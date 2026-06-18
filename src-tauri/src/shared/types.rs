use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioBlock {
    pub sample_rate: f32,
    pub buffer_size: usize,
    pub channels: usize,
    pub data: Vec<Vec<f32>>,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum EqBandType {
    LowShelf,
    HighShelf,
    Bell,
    Notch,
    HighPass,
    LowPass,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct EqBand {
    pub frequency: f32,
    pub gain: f32,
    pub q: f32,
    pub band_type: EqBandType,
    pub enabled: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CompressorParams {
    pub threshold: f32,
    pub ratio: f32,
    pub attack: f32,
    pub release: f32,
    pub makeup_gain: f32,
    pub knee_width: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioConfig {
    pub input_device: String,
    pub output_device: String,
    pub sample_rate: u32,
    pub buffer_size: u32,
    pub exclusive_mode: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct AudioStats {
    pub cpu_usage: f32,
    pub xruns: u32,
    pub latency: f32,
    pub sample_rate: f32,
    pub actual_buffer_size: f32,
    pub dsp_load: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EngineEvent {
    LevelUpdate(Vec<f32>),
    StatsUpdate(AudioStats),
    XrunOccurred(u32),
    Clipping(Vec<usize>),
    PluginStatus(String, bool),
    TimecodeUpdate(crate::timecode::Timecode),
    LtcLockStatus(bool),
    MidiMessage(crate::midi_control::MidiMessage),
    TriggerFired(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncState {
    pub master_clock: crate::sync::ClockState,
    pub ltc: crate::timecode::LtcStats,
    pub multicast_enabled: bool,
    pub ducking_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioDeviceInfo {
    pub id: String,
    pub name: String,
    pub device_type: String,
    pub channels: usize,
    pub sample_rates: Vec<u32>,
    pub buffer_sizes: Vec<u32>,
    pub is_exclusive: bool,
}
