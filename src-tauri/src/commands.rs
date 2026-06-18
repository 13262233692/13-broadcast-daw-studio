use std::sync::Arc;
use parking_lot::Mutex;
use crossbeam_channel::{unbounded, Sender, Receiver};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::audio_engine::{AudioEngine, CpalHost};
use crate::shared::types::{AudioConfig, AudioStats, AudioDeviceInfo, EngineEvent};
use crate::shared::rt_params::{ParamUpdate, GraphCommand, RcuSnapshot, DagSnapshot};
use crate::dag::{DAGProcessor, NodeType, Node, Connection, Position};
use crate::dsp::types::{EqBand, CompressorParams};
use crate::vst3_host::{Vst3Host, PluginInfo};
use uuid::Uuid;

struct EngineRuntime {
    engine: AudioEngine,
    _reply_counter: AtomicU64,
}

unsafe impl Send for EngineRuntime {}
unsafe impl Sync for EngineRuntime {}

#[derive(Clone)]
pub struct AppState {
    engine_runtime: Arc<Mutex<Option<EngineRuntime>>>,
    vst3_host: Arc<Mutex<Vst3Host>>,
    event_receiver: Arc<Mutex<Receiver<EngineEvent>>>,
    event_sender: Sender<EngineEvent>,
}

impl AppState {
    pub fn new() -> Self {
        let (sender, receiver) = unbounded();
        Self {
            engine_runtime: Arc::new(Mutex::new(None)),
            vst3_host: Arc::new(Mutex::new(Vst3Host::new())),
            event_receiver: Arc::new(Mutex::new(receiver)),
            event_sender: sender,
        }
    }

    pub fn event_sender(&self) -> &Sender<EngineEvent> {
        &self.event_sender
    }
}

fn parse_node_type(s: &str) -> Result<NodeType, String> {
    serde_json::from_value(serde_json::Value::String(s.to_lowercase()))
        .map_err(|e| format!("Invalid node type '{}': {}", s, e))
}

fn with_engine<F, R>(state: &AppState, f: F) -> Result<R, String>
where
    F: FnOnce(&EngineRuntime) -> R,
{
    let guard = state.engine_runtime.lock();
    let rt = guard.as_ref().ok_or("Audio engine is not running")?;
    Ok(f(rt))
}

#[tauri::command]
pub fn get_audio_devices() -> Result<Vec<AudioDeviceInfo>, String> {
    CpalHost::new()
        .map_err(|e| e.to_string())?
        .scan_devices()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn start_audio_engine(
    input_device_id: Option<String>,
    output_device_id: Option<String>,
    sample_rate: u32,
    buffer_size: u32,
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let config = AudioConfig {
        input_device: input_device_id.unwrap_or_default(),
        output_device: output_device_id.unwrap_or_default(),
        sample_rate,
        buffer_size,
        exclusive_mode: false,
    };

    let mut engine = AudioEngine::new(config, state.event_sender().clone());
    engine.start(app_handle).map_err(|e| e.to_string())?;

    let runtime = EngineRuntime {
        engine,
        _reply_counter: AtomicU64::new(0),
    };

    *state.engine_runtime.lock() = Some(runtime);
    Ok(())
}

#[tauri::command]
pub fn stop_audio_engine(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let mut guard = state.engine_runtime.lock();
    match guard.as_mut() {
        Some(rt) => {
            rt.engine.stop().map_err(|e| e.to_string())?;
            *guard = None;
            Ok(())
        }
        None => Err("Audio engine is not running".to_string()),
    }
}

#[tauri::command]
pub fn get_audio_stats(state: tauri::State<'_, AppState>) -> Result<AudioStats, String> {
    with_engine(&state.inner(), |rt| {
        let rcu = rt.engine.stats_snapshot();
        *rcu.load(Ordering::Acquire)
    })
}

#[tauri::command]
pub fn get_patchbay(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    with_engine(&state.inner(), |rt| {
        let rcu = rt.engine.dag_snapshot();
        let snap: &DagSnapshot = rcu.load(Ordering::Acquire);
        let nodes: Vec<&Node> = snap.nodes.iter().collect();
        let connections: &[Connection] = &snap.connections;
        serde_json::to_value(serde_json::json!({
            "nodes": nodes,
            "connections": connections,
        }))
        .map_err(|e| e.to_string())
    })?
}

#[tauri::command]
pub fn add_node(
    node_type: String,
    name: String,
    x: f32,
    y: f32,
    config: serde_json::Value,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    let nt = parse_node_type(&node_type)?;
    let position = Position { x, y };

    with_engine(&state.inner(), |rt| {
        let reply_id = rt._reply_counter.fetch_add(1, Ordering::Relaxed);
        let cmd = GraphCommand::AddNode {
            node_type: nt,
            name,
            position,
            config,
            reply_id,
        };
        let q = rt.engine.graph_cmd_queue();
        let _ = q.push(cmd);
    })?;

    Ok(String::new())
}

#[tauri::command]
pub fn remove_node(
    node_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let uuid = Uuid::parse_str(&node_id)
        .map_err(|e| format!("Invalid node ID '{}': {}", node_id, e))?;

    with_engine(&state.inner(), |rt| {
        let cmd = GraphCommand::RemoveNode { node_id: uuid };
        let q = rt.engine.graph_cmd_queue();
        let _ = q.push(cmd);
    })
}

#[tauri::command]
pub fn connect_nodes(
    source_node_id: String,
    source_port_id: String,
    target_node_id: String,
    target_port_id: String,
    gain: f32,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    let src_node = Uuid::parse_str(&source_node_id)
        .map_err(|e| format!("Invalid source node ID: {}", e))?;
    let src_port = Uuid::parse_str(&source_port_id)
        .map_err(|e| format!("Invalid source port ID: {}", e))?;
    let tgt_node = Uuid::parse_str(&target_node_id)
        .map_err(|e| format!("Invalid target node ID: {}", e))?;
    let tgt_port = Uuid::parse_str(&target_port_id)
        .map_err(|e| format!("Invalid target port ID: {}", e))?;

    with_engine(&state.inner(), |rt| {
        let reply_id = rt._reply_counter.fetch_add(1, Ordering::Relaxed);
        let cmd = GraphCommand::Connect {
            source_node_id: src_node,
            source_port_id: src_port,
            target_node_id: tgt_node,
            target_port_id: tgt_port,
            gain,
            reply_id,
        };
        let q = rt.engine.graph_cmd_queue();
        let _ = q.push(cmd);
    })?;

    Ok(String::new())
}

#[tauri::command]
pub fn disconnect_nodes(
    connection_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let uuid = Uuid::parse_str(&connection_id)
        .map_err(|e| format!("Invalid connection ID '{}': {}", connection_id, e))?;

    with_engine(&state.inner(), |rt| {
        let cmd = GraphCommand::Disconnect { connection_id: uuid };
        let q = rt.engine.graph_cmd_queue();
        let _ = q.push(cmd);
    })
}

#[tauri::command]
pub fn set_node_bypass(
    node_id: String,
    bypassed: bool,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let uuid = Uuid::parse_str(&node_id)
        .map_err(|e| format!("Invalid node ID '{}': {}", node_id, e))?;

    with_engine(&state.inner(), |rt| {
        let update = ParamUpdate::SetNodeBypass {
            node_id: uuid,
            bypassed,
        };
        let q = rt.engine.param_queue();
        let _ = q.push(update);
    })
}

#[tauri::command]
pub fn scan_vst3_plugins(state: tauri::State<'_, AppState>) -> Result<Vec<PluginInfo>, String> {
    let mut host = state.vst3_host.lock();
    host.scan_plugins().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn load_vst3_plugin(
    plugin_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    let mut host = state.vst3_host.lock();
    host.load_plugin(plugin_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_vst3_parameter(
    instance_id: String,
    param_id: String,
    value: f32,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let mut host = state.vst3_host.lock();
    host.set_parameter(instance_id, param_id, value)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_eq_bands(
    node_id: String,
    bands: Vec<EqBand>,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let uuid = Uuid::parse_str(&node_id)
        .map_err(|e| format!("Invalid node ID '{}': {}", node_id, e))?;

    with_engine(&state.inner(), |rt| {
        let update = ParamUpdate::SetEqBands {
            node_id: uuid,
            bands,
        };
        let q = rt.engine.param_queue();
        let _ = q.push(update);
    })
}

#[tauri::command]
pub fn update_compressor_params(
    node_id: String,
    params: CompressorParams,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let uuid = Uuid::parse_str(&node_id)
        .map_err(|e| format!("Invalid node ID '{}': {}", node_id, e))?;

    with_engine(&state.inner(), |rt| {
        let update = ParamUpdate::SetCompressorParams {
            node_id: uuid,
            params,
        };
        let q = rt.engine.param_queue();
        let _ = q.push(update);
    })
}

#[tauri::command]
pub fn set_master_volume(
    volume: f32,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    with_engine(&state.inner(), |rt| {
        let update = ParamUpdate::SetMasterVolume { volume };
        let q = rt.engine.param_queue();
        let _ = q.push(update);
    })
}

#[allow(dead_code)]
fn _unused(_: DAGProcessor) {}
