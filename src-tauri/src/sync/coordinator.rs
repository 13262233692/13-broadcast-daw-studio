use crate::timecode::{Timecode, FrameRate, LtcDecoder, LtcStats};
use crate::midi_control::{MidiManager, MidiMessage};
use crate::automation::{TimecodeTrigger, TriggerAction, DuckingEnvelope};
use crate::sync::{ClockHandle, ClockState, MulticastBroadcaster, PositionBroadcast};
use crate::shared::types::EngineEvent;
use crossbeam_channel::Sender;
use parking_lot::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

pub struct ExternalSyncCoordinator {
    pub midi: MidiManager,
    pub clock: ClockHandle,
    pub ltc_decoder: Option<LtcDecoder>,
    pub triggers: Arc<Mutex<Vec<TimecodeTrigger>>>,
    pub ducking: DuckingEnvelope,
    pub broadcaster: MulticastBroadcaster,
    pub last_broadcast_time: Instant,
    pub current_timecode: Timecode,
    pub total_frames_processed: u64,
    pub ltc_enabled: bool,
    pub multicast_enabled: bool,
    sample_rate: f32,
}

unsafe impl Send for ExternalSyncCoordinator {}
unsafe impl Sync for ExternalSyncCoordinator {}

impl ExternalSyncCoordinator {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            midi: MidiManager::new(),
            clock: ClockHandle::new(sample_rate),
            ltc_decoder: None,
            triggers: Arc::new(Mutex::new(Vec::new())),
            ducking: DuckingEnvelope::new(sample_rate),
            broadcaster: MulticastBroadcaster::new("239.255.0.1", 6789)
                .unwrap_or_else(|_| MulticastBroadcaster::new("239.255.0.1", 6789).unwrap()),
            last_broadcast_time: Instant::now(),
            current_timecode: Timecode::new(0, 0, 0, 0, FrameRate::Fps25),
            total_frames_processed: 0,
            ltc_enabled: false,
            multicast_enabled: false,
            sample_rate,
        }
    }

    pub fn enable_ltc(&mut self) {
        self.ltc_decoder = Some(LtcDecoder::new(self.sample_rate));
        self.ltc_enabled = true;
    }

    pub fn disable_ltc(&mut self) {
        self.ltc_decoder = None;
        self.ltc_enabled = false;
    }

    pub fn start_multicast(&mut self, interface: Option<&str>) -> Result<(), String> {
        self.broadcaster.start(interface)?;
        self.multicast_enabled = true;
        Ok(())
    }

    pub fn stop_multicast(&mut self) {
        self.broadcaster.stop();
        self.multicast_enabled = false;
    }

    pub fn add_trigger(&mut self, trigger: TimecodeTrigger) {
        self.triggers.lock().push(trigger);
    }

    pub fn remove_trigger(&mut self, id: uuid::Uuid) -> bool {
        let mut t = self.triggers.lock();
        let len_before = t.len();
        t.retain(|x| x.id != id);
        t.len() < len_before
    }

    pub fn triggers(&self) -> Vec<TimecodeTrigger> {
        self.triggers.lock().clone()
    }

    pub fn process_ltc_samples(&mut self, samples: &[f32], event_sender: &Sender<EngineEvent>) {
        if !self.ltc_enabled { return; }
        if let Some(ref mut decoder) = self.ltc_decoder {
            let mut tcs: Vec<Timecode> = Vec::new();
            decoder.process_samples(samples, &mut tcs);
            for tc in tcs {
                self.current_timecode = tc;
                self.clock.apply_timecode_sync(&tc);
                let _ = event_sender.send(EngineEvent::TimecodeUpdate(tc));
                let locked = decoder.stats().locked;
                if locked && !self.clock.state.locked_to_external.load(Ordering::Relaxed) {
                    self.clock.state.locked_to_external.store(true, Ordering::Release);
                    let _ = event_sender.send(EngineEvent::LtcLockStatus(true));
                }
            }
        }
    }

    pub fn process_midi_messages(&mut self, event_sender: &Sender<EngineEvent>) {
        let rx = self.midi.receiver();
        while let Ok(msg) = rx.try_recv() {
            let _ = event_sender.send(EngineEvent::MidiMessage(msg.clone()));
            match msg {
                MidiMessage::ShowControl(cmd) => {
                    self.handle_msc_command(cmd, event_sender);
                }
                MidiMessage::NoteOn { note: 60, .. } => {
                    self.ducking.trigger();
                    let _ = event_sender.send(EngineEvent::TriggerFired("ducking".into()));
                }
                MidiMessage::NoteOff { note: 60, .. } => {
                    self.ducking.release();
                }
                _ => {}
            }
        }
    }

    fn handle_msc_command(&self, cmd: crate::midi_control::ShowControlCommand, event_sender: &Sender<EngineEvent>) {
        use crate::midi_control::ShowControlCommand::*;
        let name = match cmd {
            Go => "GO",
            Stop => "STOP",
            Resume => "RESUME",
            Pause => "PAUSE",
            Reset => {
                let mut t = self.triggers.lock();
                for tr in t.iter_mut() { tr.reset(); }
                "RESET"
            }
            GoOff => "GO_OFF",
            Fire(n) => { let s = format!("FIRE_{}", n); s.leak(); "FIRE" }
            GoCue(c) => { let _s = c.clone(); "GO_CUE" }
            LoadCue(c) => { let _s = c.clone(); "LOAD_CUE" }
        };
        let _ = event_sender.send(EngineEvent::TriggerFired(name.into()));
    }

    pub fn tick_triggers(&mut self, event_sender: &Sender<EngineEvent>) -> Vec<TriggerAction> {
        let mut actions: Vec<TriggerAction> = Vec::new();
        if let Some(tc) = self.current_timecode() {
            let mut trigs = self.triggers.lock();
            for t in trigs.iter_mut() {
                if let Some(action) = t.check(&tc, tc.to_total_frames()) {
                    let _ = event_sender.send(EngineEvent::TriggerFired(t.name.clone()));
                    actions.push(action);
                }
            }
        }
        actions
    }

    pub fn apply_trigger_actions(actions: &[TriggerAction], param_queue: &crate::shared::rt_params::ParamQueueHandle, ducking: &mut DuckingEnvelope) {
        use crate::shared::rt_params::ParamUpdate;
        use TriggerAction::*;
        for a in actions {
            match a {
                MuteTrack { track_id, mute } => {
                    let _ = param_queue.push(ParamUpdate::SetNodeBypass { node_id: *track_id, bypassed: *mute });
                }
                SoloTrack { track_id: _track_id, solo: _solo } => {}
                BypassNode { node_id, bypass } => {
                    let _ = param_queue.push(ParamUpdate::SetNodeBypass { node_id: *node_id, bypassed: *bypass });
                }
                StartDucking => { ducking.trigger(); }
                StopDucking => { ducking.release(); }
                SetTrackGain { track_id, value } => {
                    let _ = param_queue.push(ParamUpdate::SetGain { node_id: *track_id, gain: *value });
                }
                SetMasterVolume { value } => {
                    let _ = param_queue.push(ParamUpdate::SetMasterVolume { volume: *value });
                }
                TriggerCue { cue_id: _ } => {}
                StartTransport | StopTransport => {}
                BroadcastPosition => {}
            }
        }
    }

    pub fn current_timecode(&self) -> Option<Timecode> {
        if self.ltc_enabled { Some(self.current_timecode) }
        else {
            Some(self.clock.current_timecode(self.current_timecode.frame_rate))
        }
    }

    pub fn ltc_stats(&self) -> LtcStats {
        if let Some(ref d) = self.ltc_decoder { d.stats() } else { LtcStats::default() }
    }

    pub fn clock_state(&self) -> ClockState {
        let mut s = self.clock.state.snapshot();
        s.sample_rate = self.sample_rate;
        s.last_timecode = self.current_timecode();
        s.reference_frames = self.total_frames_processed;
        s
    }

    pub fn maybe_broadcast_position(&mut self) {
        if !self.multicast_enabled { return; }
        if self.last_broadcast_time.elapsed().as_millis() >= 40 {
            let tc = self.current_timecode().unwrap_or_default();
            let msg = PositionBroadcast {
                session_id: 0,
                sequence: 0,
                wall_time_us: self.clock.state.wall_time_us.load(Ordering::Relaxed),
                position_seconds: tc.to_seconds(),
                timecode_hh: tc.hours,
                timecode_mm: tc.minutes,
                timecode_ss: tc.seconds,
                timecode_ff: tc.frames,
                is_playing: self.clock.playback_started.load(Ordering::Relaxed),
                speed_ratio: 1.0,
            };
            let _ = self.broadcaster.broadcast(&msg);
            self.last_broadcast_time = Instant::now();
        }
    }

    pub fn advance_samples(&mut self, samples: u32) {
        self.clock.advance_samples(samples);
    }

    pub fn set_sample_rate(&mut self, sr: f32) {
        self.sample_rate = sr;
        self.ducking.set_sample_rate(sr);
    }

    pub fn process_ducking(&mut self, block: &mut [f32]) {
        self.ducking.process_block(block);
    }
}

#[derive(Clone)]
pub struct SyncRuntimeHandle {
    pub midi_connected: Arc<AtomicBool>,
    pub ltc_active: Arc<AtomicBool>,
}

impl SyncRuntimeHandle {
    pub fn new() -> Self {
        Self {
            midi_connected: Arc::new(AtomicBool::new(false)),
            ltc_active: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl Default for SyncRuntimeHandle { fn default() -> Self { Self::new() } }
