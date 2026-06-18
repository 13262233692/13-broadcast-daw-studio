use std::sync::Arc;
use parking_lot::Mutex;
use crossbeam_channel::{unbounded, Sender, Receiver};

use crate::audio_engine::CpalHost;
use crate::shared::types::{AudioConfig, AudioStats, AudioDeviceInfo, EngineEvent};
use crate::dag::{DAGProcessor, NodeType, Node, Connection, Position};
use crate::dsp::types::{EqBand, CompressorParams};
use crate::vst3_host::{Vst3Host, PluginInfo};

struct EngineStateHandle {
    processor_graph: Arc<parking_lot::RwLock<DAGProcessor>>,
    stats: Arc<Mutex<AudioStats>>,
    is_running: bool,
}

unsafe impl Send for EngineStateHandle {}
unsafe impl Sync for EngineStateHandle {}

#[derive(Clone)]
pub struct AppState {
    engine_state: Arc<Mutex<Option<EngineStateHandle>>>,
    vst3_host: Arc<Mutex<Vst3Host>>,
    event_receiver: Arc<Mutex<Receiver<EngineEvent>>>,
    event_sender: Sender<EngineEvent>,
}

impl AppState {
    pub fn new() -> Self {
        let (sender, receiver) = unbounded();
        Self {
            engine_state: Arc::new(Mutex::new(None)),
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
) -> Result<(), String> {
    let config = AudioConfig {
        input_device: input_device_id.unwrap_or_default(),
        output_device: output_device_id.unwrap_or_default(),
        sample_rate,
        buffer_size,
        exclusive_mode: false,
    };

    let sr = config.sample_rate as f32;
    let bs = config.buffer_size as usize;

    let processor_graph = Arc::new(parking_lot::RwLock::new(DAGProcessor::new(sr, 2, bs)));
    let stats = Arc::new(Mutex::new(AudioStats {
        cpu_usage: 0.0,
        xruns: 0,
        latency: (bs as f32 / sr) * 1000.0,
        sample_rate: sr,
        actual_buffer_size: bs as f32,
        dsp_load: 0.0,
    }));

    let handle = EngineStateHandle {
        processor_graph,
        stats,
        is_running: true,
    };

    *state.engine_state.lock() = Some(handle);
    Ok(())
}

#[tauri::command]
pub fn stop_audio_engine(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let mut guard = state.engine_state.lock();
    match guard.as_mut() {
        Some(handle) => {
            handle.is_running = false;
            *guard = None;
            Ok(())
        }
        None => Err("Audio engine is not running".to_string()),
    }
}

#[tauri::command]
pub fn get_audio_stats(state: tauri::State<'_, AppState>) -> Result<AudioStats, String> {
    let guard = state.engine_state.lock();
    match guard.as_ref() {
        Some(handle) => Ok(*handle.stats.lock()),
        None => Err("Audio engine is not running".to_string()),
    }
}

#[tauri::command]
pub fn get_patchbay(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    let guard = state.engine_state.lock();
    let handle = guard.as_ref().ok_or("Audio engine is not running")?;

    let dag = handle.processor_graph.read();
    let nodes: Vec<&Node> = dag.get_nodes();
    let connections: &[Connection] = dag.get_connections();

    serde_json::to_value(serde_json::json!({
        "nodes": nodes,
        "connections": connections,
    }))
    .map_err(|e| e.to_string())
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

    let guard = state.engine_state.lock();
    let handle = guard.as_ref().ok_or("Audio engine is not running")?;

    let mut dag = handle.processor_graph.write();
    let node_id = dag.add_node(nt, name, position, config);

    Ok(node_id.to_string())
}

#[tauri::command]
pub fn remove_node(
    node_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let uuid = uuid::Uuid::parse_str(&node_id)
        .map_err(|e| format!("Invalid node ID '{}': {}", node_id, e))?;

    let guard = state.engine_state.lock();
    let handle = guard.as_ref().ok_or("Audio engine is not running")?;

    let mut dag = handle.processor_graph.write();
    dag.remove_node(uuid);

    Ok(())
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
    let src_node = uuid::Uuid::parse_str(&source_node_id)
        .map_err(|e| format!("Invalid source node ID: {}", e))?;
    let src_port = uuid::Uuid::parse_str(&source_port_id)
        .map_err(|e| format!("Invalid source port ID: {}", e))?;
    let tgt_node = uuid::Uuid::parse_str(&target_node_id)
        .map_err(|e| format!("Invalid target node ID: {}", e))?;
    let tgt_port = uuid::Uuid::parse_str(&target_port_id)
        .map_err(|e| format!("Invalid target port ID: {}", e))?;

    let guard = state.engine_state.lock();
    let handle = guard.as_ref().ok_or("Audio engine is not running")?;

    let mut dag = handle.processor_graph.write();
    let connection_id = dag.connect(src_node, src_port, tgt_node, tgt_port, gain)
        .map_err(|e| e.to_string())?;

    Ok(connection_id.to_string())
}

#[tauri::command]
pub fn disconnect_nodes(
    connection_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let uuid = uuid::Uuid::parse_str(&connection_id)
        .map_err(|e| format!("Invalid connection ID '{}': {}", connection_id, e))?;

    let guard = state.engine_state.lock();
    let handle = guard.as_ref().ok_or("Audio engine is not running")?;

    let mut dag = handle.processor_graph.write();
    dag.disconnect(uuid).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_node_bypass(
    node_id: String,
    bypassed: bool,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let uuid = uuid::Uuid::parse_str(&node_id)
        .map_err(|e| format!("Invalid node ID '{}': {}", node_id, e))?;

    let guard = state.engine_state.lock();
    let handle = guard.as_ref().ok_or("Audio engine is not running")?;

    let mut dag = handle.processor_graph.write();

    let node_mut = dag.get_node_mut(uuid)
        .ok_or_else(|| format!("Node '{}' not found", node_id))?;
    node_mut.bypassed = bypassed;

    Ok(())
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
    let uuid = uuid::Uuid::parse_str(&node_id)
        .map_err(|e| format!("Invalid node ID '{}': {}", node_id, e))?;

    let guard = state.engine_state.lock();
    let handle = guard.as_ref().ok_or("Audio engine is not running")?;

    let mut dag = handle.processor_graph.write();

    let processor = dag.get_processor_mut::<crate::dag::EqNode>(uuid)
        .ok_or_else(|| format!("EQ processor for node '{}' not found", node_id))?;

    let eq = processor.eq_mut();
    for (i, band) in bands.into_iter().enumerate() {
        eq.set_band(i, band);
    }
    Ok(())
}

#[tauri::command]
pub fn update_compressor_params(
    node_id: String,
    params: CompressorParams,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let uuid = uuid::Uuid::parse_str(&node_id)
        .map_err(|e| format!("Invalid node ID '{}': {}", node_id, e))?;

    let guard = state.engine_state.lock();
    let handle = guard.as_ref().ok_or("Audio engine is not running")?;

    let mut dag = handle.processor_graph.write();

    let processor = dag.get_processor_mut::<crate::dag::CompressorNode>(uuid)
        .ok_or_else(|| format!("Compressor processor for node '{}' not found", node_id))?;

    processor.compressor_mut().params = params;
    Ok(())
}

#[tauri::command]
pub fn set_master_volume(
    volume: f32,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let guard = state.engine_state.lock();
    let handle = guard.as_ref().ok_or("Audio engine is not running")?;

    let mut dag = handle.processor_graph.write();

    let gain_node_ids: Vec<_> = dag.get_nodes()
        .iter()
        .filter(|n| n.node_type == NodeType::Gain)
        .map(|n| n.id)
        .collect();

    for id in gain_node_ids {
        if let Some(gain_proc) = dag.get_processor_mut::<crate::dag::GainNode>(id) {
            gain_proc.set_gain(volume);
            return Ok(());
        }
    }

    Err("No gain node found to set master volume".to_string())
}
