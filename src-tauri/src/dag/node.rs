use crate::dsp::types::AudioBlock;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeType {
    Input,
    Output,
    Mix,
    Gain,
    Eq,
    Compressor,
    Limiter,
    Vst3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PortType {
    Input,
    Output,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Port {
    pub id: Uuid,
    pub node_id: Uuid,
    pub port_type: PortType,
    pub channel: usize,
    pub name: String,
}

impl Port {
    pub fn new(node_id: Uuid, port_type: PortType, channel: usize, name: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            node_id,
            port_type,
            channel,
            name: name.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: Uuid,
    pub node_type: NodeType,
    pub name: String,
    pub position: Position,
    pub input_ports: Vec<Port>,
    pub output_ports: Vec<Port>,
    pub config: serde_json::Value,
    pub enabled: bool,
    pub bypassed: bool,
}

impl Node {
    pub fn new(
        node_type: NodeType,
        name: impl Into<String>,
        position: Position,
        input_ports: Vec<Port>,
        output_ports: Vec<Port>,
        config: serde_json::Value,
    ) -> Self {
        let id = Uuid::new_v4();
        Self {
            id,
            node_type,
            name: name.into(),
            position,
            input_ports,
            output_ports,
            config,
            enabled: true,
            bypassed: false,
        }
    }

    pub fn with_id(
        id: Uuid,
        node_type: NodeType,
        name: impl Into<String>,
        position: Position,
        input_ports: Vec<Port>,
        output_ports: Vec<Port>,
        config: serde_json::Value,
    ) -> Self {
        Self {
            id,
            node_type,
            name: name.into(),
            position,
            input_ports,
            output_ports,
            config,
            enabled: true,
            bypassed: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Connection {
    pub id: Uuid,
    pub source_port: Uuid,
    pub target_port: Uuid,
    pub source_node: Uuid,
    pub target_node: Uuid,
    pub gain: f32,
}

impl Connection {
    pub fn new(
        source_port: Uuid,
        target_port: Uuid,
        source_node: Uuid,
        target_node: Uuid,
        gain: f32,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            source_port,
            target_port,
            source_node,
            target_node,
            gain,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Edge {
    pub gain: f32,
}

impl Edge {
    pub fn new(gain: f32) -> Self {
        Self { gain }
    }
}

pub trait NodeProcessor: std::any::Any + Send + Sync {
    fn process(&mut self, inputs: Vec<AudioBlock>) -> AudioBlock;
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}
