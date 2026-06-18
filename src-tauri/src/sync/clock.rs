use crate::timecode::{Timecode, FrameRate};
use serde::{Serialize, Deserialize};
use std::sync::atomic::{AtomicU64, AtomicI64, AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ClockState {
    pub sample_rate: f32,
    pub samples_elapsed: u64,
    pub wall_time_us: u64,
    pub locked_to_external: bool,
    pub drift_ppm: f64,
    pub last_timecode: Option<Timecode>,
    pub reference_frames: u64,
}

impl Default for ClockState {
    fn default() -> Self {
        Self {
            sample_rate: 48000.0,
            samples_elapsed: 0,
            wall_time_us: 0,
            locked_to_external: false,
            drift_ppm: 0.0,
            last_timecode: None,
            reference_frames: 0,
        }
    }
}

#[derive(Clone)]
pub struct ClockHandle {
    pub state: Arc<ClockStateAtomic>,
    pub start_instant: Arc<parking_lot::Mutex<Option<Instant>>>,
    pub playback_started: Arc<AtomicBool>,
    pub position_frames: Arc<AtomicI64>,
    pub sample_rate: f32,
}

pub struct ClockStateAtomic {
    pub samples_elapsed: AtomicU64,
    pub wall_time_us: AtomicU64,
    pub drift_ppm: AtomicU64,
    pub locked_to_external: AtomicBool,
}

impl ClockStateAtomic {
    pub fn new() -> Self {
        Self {
            samples_elapsed: AtomicU64::new(0),
            wall_time_us: AtomicU64::new(0),
            drift_ppm: AtomicU64::new(0),
            locked_to_external: AtomicBool::new(false),
        }
    }

    pub fn snapshot(&self) -> ClockState {
        ClockState {
            sample_rate: 48000.0,
            samples_elapsed: self.samples_elapsed.load(Ordering::Acquire),
            wall_time_us: self.wall_time_us.load(Ordering::Acquire),
            locked_to_external: self.locked_to_external.load(Ordering::Acquire),
            drift_ppm: f64::from_bits(self.drift_ppm.load(Ordering::Acquire)),
            last_timecode: None,
            reference_frames: 0,
        }
    }
}

impl Default for ClockStateAtomic { fn default() -> Self { Self::new() } }

impl ClockHandle {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            state: Arc::new(ClockStateAtomic::new()),
            start_instant: Arc::new(parking_lot::Mutex::new(None)),
            playback_started: Arc::new(AtomicBool::new(false)),
            position_frames: Arc::new(AtomicI64::new(0)),
            sample_rate,
        }
    }

    pub fn start_playback(&self, at_timecode: Option<Timecode>) {
        *self.start_instant.lock() = Some(Instant::now());
        self.playback_started.store(true, Ordering::Release);
        if let Some(tc) = at_timecode {
            self.position_frames.store(tc.to_total_frames(), Ordering::Release);
        }
        self.state.samples_elapsed.store(0, Ordering::Release);
    }

    pub fn stop_playback(&self) {
        self.playback_started.store(false, Ordering::Release);
    }

    pub fn advance_samples(&self, samples: u32) {
        self.state.samples_elapsed.fetch_add(samples as u64, Ordering::AcqRel);
        if let Some(start) = *self.start_instant.lock() {
            let us = start.elapsed().as_micros() as u64;
            self.state.wall_time_us.store(us, Ordering::Release);
        }
    }

    pub fn current_timecode(&self, frame_rate: FrameRate) -> Timecode {
        let samples = self.state.samples_elapsed.load(Ordering::Acquire) as f64;
        let frames = (samples * frame_rate.frames_per_second() / self.sample_rate as f64) as i64;
        Timecode::from_total_frames(frames, frame_rate)
    }

    pub fn current_position_seconds(&self) -> f64 {
        self.state.samples_elapsed.load(Ordering::Acquire) as f64 / self.sample_rate as f64
    }

    pub fn apply_timecode_sync(&self, tc: &Timecode) {
        let ltc_frames = tc.to_total_frames();
        let our_frames = self.position_frames.load(Ordering::Acquire);
        let diff = ltc_frames - our_frames;
        let samples_diff = (diff as f64 * self.sample_rate as f64 / tc.frame_rate.frames_per_second()) as i64;
        if samples_diff.abs() > (self.sample_rate as i64 / 10) {
            self.position_frames.store(ltc_frames, Ordering::Release);
            self.state.samples_elapsed.fetch_add(samples_diff.max(0) as u64, Ordering::AcqRel);
            self.state.locked_to_external.store(true, Ordering::Release);
        } else {
            let ppm = if our_frames != 0 {
                (diff as f64 / our_frames as f64) * 1_000_000.0
            } else { 0.0 };
            self.state.drift_ppm.store(ppm.to_bits(), Ordering::Release);
        }
    }

    pub fn wait_until(&self, deadline_samples: u64) {
        while self.state.samples_elapsed.load(Ordering::Acquire) < deadline_samples {
            std::hint::spin_loop();
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct PositionBroadcast {
    pub session_id: u64,
    pub sequence: u64,
    pub wall_time_us: u64,
    pub position_seconds: f64,
    pub timecode_hh: u8,
    pub timecode_mm: u8,
    pub timecode_ss: u8,
    pub timecode_ff: u8,
    pub is_playing: bool,
    pub speed_ratio: f32,
}

impl PositionBroadcast {
    pub fn to_bytes(&self) -> [u8; 56] {
        let mut buf = [0u8; 56];
        buf[0..8].copy_from_slice(&self.session_id.to_be_bytes());
        buf[8..16].copy_from_slice(&self.sequence.to_be_bytes());
        buf[16..24].copy_from_slice(&self.wall_time_us.to_be_bytes());
        buf[24..32].copy_from_slice(&self.position_seconds.to_be_bytes());
        buf[32] = self.timecode_hh;
        buf[33] = self.timecode_mm;
        buf[34] = self.timecode_ss;
        buf[35] = self.timecode_ff;
        buf[36] = self.is_playing as u8;
        buf[37..41].copy_from_slice(&self.speed_ratio.to_be_bytes());
        buf
    }

    pub fn from_bytes(buf: &[u8]) -> Option<Self> {
        if buf.len() < 41 { return None; }
        let session_id = u64::from_be_bytes(buf[0..8].try_into().ok()?);
        let sequence = u64::from_be_bytes(buf[8..16].try_into().ok()?);
        let wall_time_us = u64::from_be_bytes(buf[16..24].try_into().ok()?);
        let position_seconds = f64::from_be_bytes(buf[24..32].try_into().ok()?);
        let speed_ratio = f32::from_be_bytes(buf[37..41].try_into().ok()?);
        Some(Self {
            session_id, sequence, wall_time_us, position_seconds,
            timecode_hh: buf[32], timecode_mm: buf[33], timecode_ss: buf[34], timecode_ff: buf[35],
            is_playing: buf[36] != 0, speed_ratio,
        })
    }
}
