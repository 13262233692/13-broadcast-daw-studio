use serde::{Serialize, Deserialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MidiDeviceInfo {
    pub id: String,
    pub name: String,
    pub direction: MidiDirection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MidiDirection { Input, Output }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MidiMessage {
    NoteOn { channel: u8, note: u8, velocity: u8 },
    NoteOff { channel: u8, note: u8, velocity: u8 },
    ControlChange { channel: u8, controller: u8, value: u8 },
    ProgramChange { channel: u8, program: u8 },
    SysEx(Vec<u8>),
    ShowControl(ShowControlCommand),
    TimecodeQuarterFrame { message_type: u8, data: u8 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShowControlCommand {
    Go,
    Stop,
    Resume,
    Pause,
    Reset,
    GoOff,
    Fire(u8),
    GoCue(String),
    LoadCue(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MidiMapping {
    pub id: Uuid,
    pub name: String,
    pub trigger: MidiTrigger,
    pub action: MidiAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MidiTrigger {
    ControlChange { channel: u8, controller: u8 },
    NoteOn { channel: u8, note: u8 },
    ShowControl(ShowControlCommand),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MidiAction {
    SetTrackMute { track_id: Uuid, mute: bool },
    SetTrackSolo { track_id: Uuid, solo: bool },
    SetTrackGain { track_id: Uuid, value: f32 },
    SetMasterVolume { value: f32 },
    TriggerCue { cue_id: Uuid },
    StartTransport,
    StopTransport,
}
