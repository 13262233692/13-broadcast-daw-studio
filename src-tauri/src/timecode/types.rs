use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FrameRate {
    Fps24,
    Fps25,
    Fps2997Drop,
    Fps2997NonDrop,
    Fps30Drop,
    Fps30NonDrop,
}

impl FrameRate {
    pub fn frames_per_second(&self) -> f64 {
        match self {
            FrameRate::Fps24 => 24.0,
            FrameRate::Fps25 => 25.0,
            FrameRate::Fps2997Drop | FrameRate::Fps2997NonDrop => 30.0 / 1.001,
            FrameRate::Fps30Drop | FrameRate::Fps30NonDrop => 30.0,
        }
    }
    pub fn nominal_fps(&self) -> u8 {
        match self {
            FrameRate::Fps24 => 24,
            FrameRate::Fps25 => 25,
            FrameRate::Fps2997Drop | FrameRate::Fps2997NonDrop |
            FrameRate::Fps30Drop | FrameRate::Fps30NonDrop => 30,
        }
    }
    pub fn is_drop_frame(&self) -> bool {
        matches!(self, FrameRate::Fps2997Drop | FrameRate::Fps30Drop)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Timecode {
    pub hours: u8,
    pub minutes: u8,
    pub seconds: u8,
    pub frames: u8,
    pub frame_rate: FrameRate,
    pub user_bits: [u8; 4],
}

impl Timecode {
    pub fn new(hours: u8, minutes: u8, seconds: u8, frames: u8, frame_rate: FrameRate) -> Self {
        Self { hours, minutes, seconds, frames, frame_rate, user_bits: [0u8; 4] }
    }

    pub fn to_seconds(&self) -> f64 {
        let total_frames = self.to_total_frames() as f64;
        total_frames / self.frame_rate.frames_per_second()
    }

    pub fn to_total_frames(&self) -> i64 {
        let nominal = self.frame_rate.nominal_fps() as i64;
        let mut total = ((self.hours as i64 * 3600) + (self.minutes as i64 * 60) + self.seconds as i64) * nominal + self.frames as i64;
        if self.frame_rate.is_drop_frame() {
            let drop_frames = if matches!(self.frame_rate, FrameRate::Fps2997Drop) { 2 } else { 4 };
            let minutes = self.hours as i64 * 60 + self.minutes as i64;
            total -= drop_frames * (minutes - minutes / 10);
        }
        total
    }

    pub fn from_total_frames(mut total: i64, frame_rate: FrameRate) -> Self {
        let nominal = frame_rate.nominal_fps() as i64;
        if frame_rate.is_drop_frame() {
            let drop_frames = if matches!(frame_rate, FrameRate::Fps2997Drop) { 2 } else { 4 };
            let frames_per_10min = nominal * 60 * 10;
            let frames_per_min = nominal * 60 - drop_frames;
            let d = total / frames_per_10min;
            let m = total % frames_per_10min;
            if m > drop_frames {
                total += drop_frames * 9 * d + drop_frames * ((m - drop_frames) / frames_per_min);
            } else {
                total += drop_frames * 9 * d;
            }
        }
        let frames = (total % nominal) as u8;
        let total_sec = total / nominal;
        let seconds = (total_sec % 60) as u8;
        let total_min = total_sec / 60;
        let minutes = (total_min % 60) as u8;
        let hours = (total_min / 60) as u8;
        Self { hours, minutes, seconds, frames, frame_rate, user_bits: [0u8; 4] }
    }

    pub fn to_string(&self) -> String {
        let sep = if self.frame_rate.is_drop_frame() { ';' } else { ':' };
        format!("{:02}:{:02}:{:02}{}{:02}", self.hours, self.minutes, self.seconds, sep, self.frames)
    }
}

impl Default for Timecode {
    fn default() -> Self { Self::new(0,0,0,0, FrameRate::Fps25) }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct LtcStats {
    pub locked: bool,
    pub last_frame_timecode: Option<Timecode>,
    pub drift_ms: f64,
    pub sample_rate: f32,
    pub samples_since_sync: u64,
    pub detected_frame_rate: Option<FrameRate>,
}

impl Default for LtcStats {
    fn default() -> Self {
        Self { locked: false, last_frame_timecode: None, drift_ms: 0.0, sample_rate: 48000.0, samples_since_sync: 0, detected_frame_rate: None }
    }
}
