use super::{MidiDeviceInfo, MidiMessage, MidiDirection, MidiMapping};
use midir::{MidiInput, MidiOutput, MidiInputConnection, MidiOutputConnection};
use crossbeam_channel::{Sender, Receiver, unbounded};
use parking_lot::Mutex;
use std::sync::Arc;

pub struct MidiManager {
    midi_in: Option<MidiInput>,
    midi_out: Option<MidiOutput>,
    input_conn: Option<MidiInputConnection<()>>,
    output_conn: Option<MidiOutputConnection>,
    message_sender: Sender<MidiMessage>,
    message_receiver: Receiver<MidiMessage>,
    mappings: Arc<Mutex<Vec<MidiMapping>>>,
    active_device: Option<MidiDeviceInfo>,
}

unsafe impl Send for MidiManager {}
unsafe impl Sync for MidiManager {}

impl MidiManager {
    pub fn new() -> Self {
        let (tx, rx) = unbounded();
        Self {
            midi_in: MidiInput::new("Broadcast DAW").ok(),
            midi_out: MidiOutput::new("Broadcast DAW").ok(),
            input_conn: None,
            output_conn: None,
            message_sender: tx,
            message_receiver: rx,
            mappings: Arc::new(Mutex::new(Vec::new())),
            active_device: None,
        }
    }

    pub fn scan_devices(&self) -> Result<Vec<MidiDeviceInfo>, String> {
        let mut devices = Vec::new();
        if let Some(ref input) = self.midi_in {
            for port in input.ports() {
                if let Ok(name) = input.port_name(&port) {
                    devices.push(MidiDeviceInfo {
                        id: format!("in_{}", name),
                        name,
                        direction: MidiDirection::Input,
                    });
                }
            }
        }
        if let Some(ref output) = self.midi_out {
            for port in output.ports() {
                if let Ok(name) = output.port_name(&port) {
                    devices.push(MidiDeviceInfo {
                        id: format!("out_{}", name),
                        name,
                        direction: MidiDirection::Output,
                    });
                }
            }
        }
        Ok(devices)
    }

    pub fn connect_input(&mut self, device_name: &str) -> Result<(), String> {
        let port = {
            let input = self.midi_in.as_ref().ok_or("MIDI input not available")?;
            input.ports().into_iter()
                .find(|p| input.port_name(p).map(|n| n.contains(device_name)).unwrap_or(false))
                .ok_or_else(|| format!("Device '{}' not found", device_name))?
        };
        let input = self.midi_in.take().ok_or("MIDI input not available")?;
        let tx = self.message_sender.clone();
        let conn = input.connect(&port, "ltc-input", move |_ts, data, _| {
            let msg = parse_midi_message(data);
            if let Some(m) = msg {
                let _ = tx.send(m);
            }
        }, ()).map_err(|e| e.to_string())?;
        self.input_conn = Some(conn);
        self.active_device = Some(MidiDeviceInfo {
            id: device_name.to_string(),
            name: device_name.to_string(),
            direction: MidiDirection::Input,
        });
        Ok(())
    }

    pub fn disconnect(&mut self) {
        self.input_conn.take();
        self.output_conn.take();
        self.active_device = None;
    }

    pub fn receiver(&self) -> Receiver<MidiMessage> {
        self.message_receiver.clone()
    }

    pub fn send(&mut self, bytes: &[u8]) -> Result<(), String> {
        if let Some(ref mut out) = self.output_conn {
            out.send(bytes).map_err(|e| e.to_string())
        } else {
            Err("No output connected".to_string())
        }
    }

    pub fn add_mapping(&mut self, mapping: MidiMapping) {
        self.mappings.lock().push(mapping);
    }

    pub fn mappings(&self) -> Vec<MidiMapping> {
        self.mappings.lock().clone()
    }

    pub fn active_device(&self) -> Option<MidiDeviceInfo> {
        self.active_device.clone()
    }
}

impl Default for MidiManager { fn default() -> Self { Self::new() } }

pub fn parse_midi_message(data: &[u8]) -> Option<MidiMessage> {
    if data.is_empty() { return None; }
    let status = data[0];
    let channel = status & 0x0F;
    match status & 0xF0 {
        0x80 if data.len() >= 3 => Some(MidiMessage::NoteOff { channel, note: data[1], velocity: data[2] }),
        0x90 if data.len() >= 3 => Some(MidiMessage::NoteOn { channel, note: data[1], velocity: data[2] }),
        0xB0 if data.len() >= 3 => Some(MidiMessage::ControlChange { channel, controller: data[1], value: data[2] }),
        0xC0 if data.len() >= 2 => Some(MidiMessage::ProgramChange { channel, program: data[1] }),
        0xF0 => Some(MidiMessage::SysEx(data.to_vec())),
        0xF1 if data.len() >= 2 => Some(MidiMessage::TimecodeQuarterFrame { message_type: (data[1] >> 4) & 0x07, data: data[1] & 0x0F }),
        _ => None,
    }
}
