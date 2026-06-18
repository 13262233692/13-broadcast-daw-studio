use crate::dsp::types::{EqBand, CompressorParams};
use crate::dag::{NodeType, Position, Node, Connection};
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use std::sync::atomic::{AtomicPtr, Ordering};
use uuid::Uuid;
use ringbuf::{HeapConsumer, HeapProducer, HeapRb};

pub const PARAM_QUEUE_CAPACITY: usize = 8192;
pub const GRAPH_CMD_QUEUE_CAPACITY: usize = 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DagSnapshot {
    pub nodes: Vec<Node>,
    pub connections: Vec<Connection>,
}

impl Default for DagSnapshot {
    fn default() -> Self {
        Self {
            nodes: Vec::new(),
            connections: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ParamUpdate {
    SetNodeBypass { node_id: Uuid, bypassed: bool },
    SetNodeEnabled { node_id: Uuid, enabled: bool },
    SetGain { node_id: Uuid, gain: f32 },
    SetEqBands { node_id: Uuid, bands: Vec<EqBand> },
    SetCompressorParams { node_id: Uuid, params: CompressorParams },
    SetMasterVolume { volume: f32 },
    SetEdgeGain { connection_id: Uuid, gain: f32 },
}

unsafe impl Send for ParamUpdate {}

#[derive(Debug, Clone)]
pub enum GraphCommand {
    AddNode {
        node_type: NodeType,
        name: String,
        position: Position,
        config: serde_json::Value,
        reply_id: u64,
    },
    RemoveNode {
        node_id: Uuid,
    },
    Connect {
        source_node_id: Uuid,
        source_port_id: Uuid,
        target_node_id: Uuid,
        target_port_id: Uuid,
        gain: f32,
        reply_id: u64,
    },
    Disconnect {
        connection_id: Uuid,
    },
}

unsafe impl Send for GraphCommand {}

pub struct ParamQueueHandle {
    pub producer: Arc<std::sync::Mutex<HeapProducer<ParamUpdate>>>,
    pub consumer: Arc<std::sync::Mutex<HeapConsumer<ParamUpdate>>>,
}

impl ParamQueueHandle {
    pub fn new() -> Self {
        let rb = HeapRb::<ParamUpdate>::new(PARAM_QUEUE_CAPACITY);
        let (prod, cons) = rb.split();
        Self {
            producer: Arc::new(std::sync::Mutex::new(prod)),
            consumer: Arc::new(std::sync::Mutex::new(cons)),
        }
    }

    pub fn push(&self, update: ParamUpdate) -> Result<(), ParamUpdate> {
        match self.producer.lock() {
            Ok(mut guard) => {
                match guard.push(update) {
                    Ok(()) => Ok(()),
                    Err(skipped) => Err(skipped),
                }
            }
            Err(_) => Err(update),
        }
    }

    pub fn try_drain_all(&self, buf: &mut Vec<ParamUpdate>) {
        if let Ok(mut guard) = self.consumer.try_lock() {
            buf.clear();
            while let Some(item) = guard.pop() {
                buf.push(item);
                if buf.len() >= PARAM_QUEUE_CAPACITY {
                    break;
                }
            }
        }
    }
}

impl Clone for ParamQueueHandle {
    fn clone(&self) -> Self {
        Self {
            producer: self.producer.clone(),
            consumer: self.consumer.clone(),
        }
    }
}

impl Default for ParamQueueHandle {
    fn default() -> Self {
        Self::new()
    }
}

pub struct GraphCmdQueueHandle {
    pub producer: Arc<std::sync::Mutex<HeapProducer<GraphCommand>>>,
    pub consumer: Arc<std::sync::Mutex<HeapConsumer<GraphCommand>>>,
}

impl GraphCmdQueueHandle {
    pub fn new() -> Self {
        let rb = HeapRb::<GraphCommand>::new(GRAPH_CMD_QUEUE_CAPACITY);
        let (prod, cons) = rb.split();
        Self {
            producer: Arc::new(std::sync::Mutex::new(prod)),
            consumer: Arc::new(std::sync::Mutex::new(cons)),
        }
    }

    pub fn push(&self, cmd: GraphCommand) -> Result<(), GraphCommand> {
        match self.producer.lock() {
            Ok(mut guard) => {
                match guard.push(cmd) {
                    Ok(()) => Ok(()),
                    Err(skipped) => Err(skipped),
                }
            }
            Err(_) => Err(cmd),
        }
    }

    pub fn try_drain_all(&self, buf: &mut Vec<GraphCommand>) {
        if let Ok(mut guard) = self.consumer.try_lock() {
            buf.clear();
            while let Some(item) = guard.pop() {
                buf.push(item);
                if buf.len() >= GRAPH_CMD_QUEUE_CAPACITY {
                    break;
                }
            }
        }
    }
}

impl Clone for GraphCmdQueueHandle {
    fn clone(&self) -> Self {
        Self {
            producer: self.producer.clone(),
            consumer: self.consumer.clone(),
        }
    }
}

impl Default for GraphCmdQueueHandle {
    fn default() -> Self {
        Self::new()
    }
}

pub struct RcuSnapshot<T: Send + 'static> {
    ptr: AtomicPtr<T>,
}

impl<T: Send + 'static> RcuSnapshot<T> {
    pub fn new(initial: T) -> Self {
        let boxed = Box::new(initial);
        let raw = Box::into_raw(boxed);
        Self {
            ptr: AtomicPtr::new(raw),
        }
    }

    pub fn load(&self, order: Ordering) -> &'static T {
        let raw = self.ptr.load(order);
        unsafe { &*raw }
    }

    pub fn store(&self, new_value: T) {
        let boxed = Box::new(new_value);
        let new_raw = Box::into_raw(boxed);
        let old_raw = self.ptr.swap(new_raw, Ordering::AcqRel);
        unsafe {
            if !old_raw.is_null() {
                let _ = Box::from_raw(old_raw);
            }
        }
    }

    pub fn swap(&self, new_value: T) -> T {
        let boxed = Box::new(new_value);
        let new_raw = Box::into_raw(boxed);
        let old_raw = self.ptr.swap(new_raw, Ordering::AcqRel);
        unsafe { *Box::from_raw(old_raw) }
    }
}

impl<T: Send + 'static> Drop for RcuSnapshot<T> {
    fn drop(&mut self) {
        let raw = self.ptr.load(Ordering::SeqCst);
        if !raw.is_null() {
            unsafe {
                let _ = Box::from_raw(raw);
            }
        }
    }
}

unsafe impl<T: Send + 'static> Send for RcuSnapshot<T> {}
unsafe impl<T: Send + 'static> Sync for RcuSnapshot<T> {}

