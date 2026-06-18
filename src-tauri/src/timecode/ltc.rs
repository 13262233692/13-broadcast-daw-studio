use super::{Timecode, FrameRate, LtcStats};
use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use std::sync::Arc;

const LTC_FRAME_SAMPLES_MAX: usize = 65536;

pub struct LtcDecoder {
    sample_rate: f32,
    last_sample: f32,
    last_transition: u64,
    bit_buffer: u64,
    bit_count: u8,
    sync_word_detected: bool,
    samples_since_sync: u64,
    last_frame_samples: u64,
    frame_buffer: [u8; 10],
    frame_byte_index: usize,
    frame_bit_index: usize,
    valid_frame_count: u32,
    locked: bool,
    last_timecode: Option<Timecode>,
    detected_fps: Option<FrameRate>,
}

impl LtcDecoder {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            sample_rate,
            last_sample: 0.0,
            last_transition: 0,
            bit_buffer: 0,
            bit_count: 0,
            sync_word_detected: false,
            samples_since_sync: 0,
            last_frame_samples: 0,
            frame_buffer: [0u8; 10],
            frame_byte_index: 0,
            frame_bit_index: 0,
            valid_frame_count: 0,
            locked: false,
            last_timecode: None,
            detected_fps: None,
        }
    }

    pub fn process_samples(&mut self, samples: &[f32], timecode_out: &mut Vec<Timecode>) {
        for &s in samples {
            self.samples_since_sync += 1;
            let transition = (self.last_sample > 0.0 && s <= 0.0) || (self.last_sample < 0.0 && s >= 0.0);
            self.last_sample = s;
            if !transition { continue; }

            let since_last = self.samples_since_sync - self.last_transition;
            self.last_transition = self.samples_since_sync;

            let samples_per_bit_min = (self.sample_rate as u64 * 40) / 1_000_000;
            let samples_per_bit_max = (self.sample_rate as u64 * 250) / 1_000_000;

            let bit = if since_last < samples_per_bit_min { continue; }
            else if since_last <= samples_per_bit_max { 0u8 }
            else { 1u8 };

            self.bit_buffer = (self.bit_buffer << 1) | (bit as u64);
            self.bit_count = self.bit_count.saturating_add(1);

            if self.bit_count >= 16 {
                const LTC_SYNC: u64 = 0b0011111111111101;
                if (self.bit_buffer & 0xFFFF) == LTC_SYNC {
                    self.sync_word_detected = true;
                    self.frame_byte_index = 0;
                    self.frame_bit_index = 0;
                    self.bit_count = 0;
                    let frame_duration = self.samples_since_sync.wrapping_sub(self.last_frame_samples);
                    self.last_frame_samples = self.samples_since_sync;
                    if frame_duration > 0 {
                        let fps = self.sample_rate as f64 / frame_duration as f64 * 80.0;
                        self.detected_fps = Some(match fps {
                            f if (f - 24.0).abs() < 0.5 => FrameRate::Fps24,
                            f if (f - 25.0).abs() < 0.5 => FrameRate::Fps25,
                            f if (f - 29.97).abs() < 0.5 => FrameRate::Fps2997NonDrop,
                            _ => FrameRate::Fps30NonDrop,
                        });
                    }
                    continue;
                }
            }

            if self.sync_word_detected && self.frame_byte_index < 10 {
                if bit == 1 {
                    self.frame_buffer[self.frame_byte_index] |= 1 << self.frame_bit_index;
                }
                self.frame_bit_index += 1;
                if self.frame_bit_index >= 8 {
                    self.frame_bit_index = 0;
                    self.frame_byte_index += 1;
                    if self.frame_byte_index >= 10 {
                        if let Some(tc) = self.decode_frame() {
                            timecode_out.push(tc);
                            self.last_timecode = Some(tc);
                            self.valid_frame_count = self.valid_frame_count.saturating_add(1);
                            self.locked = self.valid_frame_count >= 5;
                        }
                        self.sync_word_detected = false;
                    }
                }
            }
        }
    }

    fn decode_frame(&self) -> Option<Timecode> {
        if self.frame_buffer.len() < 10 { return None; }
        let b = &self.frame_buffer;
        let frames_units = b[0] & 0x0F;
        let frames_tens = (b[1] & 0x03) << 4;
        let frames = frames_tens | frames_units;
        let df_flag = (b[1] & 0x40) != 0;
        let seconds_units = b[2] & 0x0F;
        let seconds_tens = (b[3] & 0x07) << 4;
        let seconds = seconds_tens | seconds_units;
        let minutes_units = b[4] & 0x0F;
        let minutes_tens = (b[5] & 0x07) << 4;
        let minutes = minutes_tens | minutes_units;
        let hours_units = b[6] & 0x0F;
        let hours_tens = (b[7] & 0x03) << 4;
        let hours = hours_tens | hours_units;

        let frame_rate = match self.detected_fps {
            Some(f) => f,
            None => if df_flag { FrameRate::Fps2997Drop } else { FrameRate::Fps30NonDrop },
        };

        if frames >= 60 || seconds >= 60 || minutes >= 60 || hours >= 24 { return None; }

        let mut tc = Timecode::new(hours, minutes, seconds, frames, frame_rate);
        tc.user_bits[0] = ((b[1] >> 4) & 0x03) | ((b[0] >> 4) & 0x0F) >> 4;
        tc.user_bits[0] = (b[0] >> 4) & 0x0F;
        tc.user_bits[1] = (b[2] >> 4) & 0x0F;
        tc.user_bits[2] = (b[4] >> 4) & 0x0F;
        tc.user_bits[3] = (b[6] >> 4) & 0x0F;

        Some(tc)
    }

    pub fn stats(&self) -> LtcStats {
        LtcStats {
            locked: self.locked,
            last_frame_timecode: self.last_timecode,
            drift_ms: 0.0,
            sample_rate: self.sample_rate,
            samples_since_sync: self.samples_since_sync,
            detected_frame_rate: self.detected_fps,
        }
    }

    pub fn reset(&mut self) {
        self.bit_buffer = 0;
        self.bit_count = 0;
        self.sync_word_detected = false;
        self.frame_byte_index = 0;
        self.frame_bit_index = 0;
        self.valid_frame_count = 0;
        self.locked = false;
        self.samples_since_sync = 0;
    }
}

#[derive(Clone)]
pub struct LtcSourceHandle {
    pub enabled: Arc<AtomicBool>,
    pub frame_counter: Arc<AtomicU64>,
    pub last_timecode_samples: Arc<AtomicU64>,
    pub locked: Arc<AtomicBool>,
}

impl LtcSourceHandle {
    pub fn new() -> Self {
        Self {
            enabled: Arc::new(AtomicBool::new(false)),
            frame_counter: Arc::new(AtomicU64::new(0)),
            last_timecode_samples: Arc::new(AtomicU64::new(0)),
            locked: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl Default for LtcSourceHandle { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_timecode_conversion() {
        let tc = Timecode::new(1, 0, 0, 0, FrameRate::Fps25);
        assert_eq!(tc.to_seconds(), 3600.0);
        assert_eq!(tc.to_total_frames(), 3600 * 25);
        let back = Timecode::from_total_frames(tc.to_total_frames(), FrameRate::Fps25);
        assert_eq!(back.hours, 1);
        assert_eq!(back.minutes, 0);
    }
}
