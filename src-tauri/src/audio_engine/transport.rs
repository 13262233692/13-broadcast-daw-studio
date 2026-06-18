use crate::shared::types::{AudioConfig, AudioStats};
use anyhow::Result;
use parking_lot::Mutex;
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportState {
    Stopped,
    Playing,
    Recording,
    Paused,
}

#[derive(Debug, Clone)]
pub struct RecordingInfo {
    pub sample_rate: u32,
    pub channels: usize,
    pub start_time: Instant,
    pub duration: Duration,
}

pub struct Transport {
    config: AudioConfig,
    state: Arc<Mutex<TransportState>>,
    is_recording: Arc<AtomicBool>,
    recording_info: Arc<Mutex<Option<RecordingInfo>>>,
    wav_writer: Arc<Mutex<Option<BufWriter<std::fs::File>>>>,
    stats: Arc<Mutex<AudioStats>>,
    position_samples: Arc<Mutex<u64>>,
    loop_enabled: Arc<AtomicBool>,
    loop_start: Arc<Mutex<u64>>,
    loop_end: Arc<Mutex<u64>>,
}

use std::fs::File;
use std::io::BufWriter;

impl Transport {
    pub fn new(config: AudioConfig, stats: Arc<Mutex<AudioStats>>) -> Self {
        Self {
            config,
            state: Arc::new(Mutex::new(TransportState::Stopped)),
            is_recording: Arc::new(AtomicBool::new(false)),
            recording_info: Arc::new(Mutex::new(None)),
            wav_writer: Arc::new(Mutex::new(None)),
            stats,
            position_samples: Arc::new(Mutex::new(0)),
            loop_enabled: Arc::new(AtomicBool::new(false)),
            loop_start: Arc::new(Mutex::new(0)),
            loop_end: Arc::new(Mutex::new(0)),
        }
    }

    pub fn play(&self) {
        let mut state = self.state.lock();
        if *state == TransportState::Stopped || *state == TransportState::Paused {
            *state = TransportState::Playing;
        }
    }

    pub fn stop(&self) {
        let mut state = self.state.lock();
        *state = TransportState::Stopped;

        if self.is_recording.load(Ordering::SeqCst) {
            self.stop_recording_internal();
        }

        let mut position = self.position_samples.lock();
        *position = 0;
    }

    pub fn pause(&self) {
        let mut state = self.state.lock();
        if *state == TransportState::Playing {
            *state = TransportState::Paused;
        } else if *state == TransportState::Recording {
            *state = TransportState::Paused;
        }
    }

    pub fn record(&self, output_path: std::path::PathBuf) -> Result<()> {
        let mut state = self.state.lock();
        if *state == TransportState::Stopped || *state == TransportState::Paused {
            self.start_recording_internal(output_path)?;
            *state = TransportState::Recording;
        }
        Ok(())
    }

    pub fn toggle_play(&self) {
        let state = *self.state.lock();
        match state {
            TransportState::Stopped => self.play(),
            TransportState::Playing => self.pause(),
            TransportState::Paused => self.play(),
            TransportState::Recording => self.pause(),
        }
    }

    pub fn toggle_record(&self, output_path: std::path::PathBuf) -> Result<()> {
        if self.is_recording.load(Ordering::SeqCst) {
            self.stop();
        } else {
            self.record(output_path)?;
        }
        Ok(())
    }

    fn start_recording_internal(&self, output_path: std::path::PathBuf) -> Result<()> {
        let file = File::create(&output_path)?;
        let buf_writer = BufWriter::new(file);

        let channels = self.config.sample_rate;
        let sample_rate = self.config.sample_rate;
        let bits_per_sample: u16 = 32;
        let num_channels: u16 = 2;
        let byte_rate = sample_rate * num_channels as u32 * (bits_per_sample / 8) as u32;
        let block_align = num_channels * (bits_per_sample / 8);
        let data_size: u32 = 0;

        let header = Self::create_wav_header(num_channels, sample_rate, byte_rate, block_align, bits_per_sample, data_size);

        *self.wav_writer.lock() = Some(buf_writer);
        if let Some(writer) = self.wav_writer.lock().as_mut() {
            writer.write_all(&header)?;
        }

        *self.recording_info.lock() = Some(RecordingInfo {
            sample_rate: self.config.sample_rate,
            channels: 2,
            start_time: Instant::now(),
            duration: Duration::from_secs(0),
        });

        self.is_recording.store(true, Ordering::SeqCst);
        Ok(())
    }

    fn create_wav_header(num_channels: u16, sample_rate: u32, byte_rate: u32, block_align: u16, bits_per_sample: u16, data_size: u32) -> Vec<u8> {
        let mut header = Vec::with_capacity(44);
        header.extend_from_slice(b"RIFF");
        let file_size = 36 + data_size;
        header.extend_from_slice(&file_size.to_le_bytes());
        header.extend_from_slice(b"WAVE");
        header.extend_from_slice(b"fmt ");
        header.extend_from_slice(&16u32.to_le_bytes());
        header.extend_from_slice(&3u16.to_le_bytes()); // IEEE float
        header.extend_from_slice(&num_channels.to_le_bytes());
        header.extend_from_slice(&sample_rate.to_le_bytes());
        header.extend_from_slice(&byte_rate.to_le_bytes());
        header.extend_from_slice(&block_align.to_le_bytes());
        header.extend_from_slice(&bits_per_sample.to_le_bytes());
        header.extend_from_slice(b"data");
        header.extend_from_slice(&data_size.to_le_bytes());
        header
    }

    fn stop_recording_internal(&self) {
        self.is_recording.store(false, Ordering::SeqCst);

        if let Some(mut writer) = self.wav_writer.lock().take() {
            let _ = writer.flush();
        }

        if let Some(mut info) = self.recording_info.lock().take() {
            info.duration = info.start_time.elapsed();
        }
    }

    pub fn process_audio(&self, block: &[f32], _channels: usize) {
        let state = *self.state.lock();

        if state == TransportState::Playing || state == TransportState::Recording {
            let mut position = self.position_samples.lock();
            let frames = block.len() / 2;
            *position += frames as u64;

            if self.loop_enabled.load(Ordering::SeqCst) {
                let loop_start = *self.loop_start.lock();
                let loop_end = *self.loop_end.lock();
                if loop_end > loop_start && *position >= loop_end {
                    *position = loop_start;
                }
            }
        }

        if self.is_recording.load(Ordering::SeqCst) {
            if let Some(writer) = self.wav_writer.lock().as_mut() {
                let bytes: Vec<u8> = block.iter()
                    .flat_map(|s| s.to_le_bytes())
                    .collect();
                let _ = writer.write_all(&bytes);
            }
        }
    }

    pub fn get_state(&self) -> TransportState {
        *self.state.lock()
    }

    pub fn is_playing(&self) -> bool {
        matches!(*self.state.lock(), TransportState::Playing | TransportState::Recording)
    }

    pub fn is_recording(&self) -> bool {
        self.is_recording.load(Ordering::SeqCst)
    }

    pub fn get_position(&self) -> Duration {
        let samples = *self.position_samples.lock();
        Duration::from_secs_f64(samples as f64 / self.config.sample_rate as f64)
    }

    pub fn get_position_samples(&self) -> u64 {
        *self.position_samples.lock()
    }

    pub fn set_position(&self, samples: u64) {
        let mut position = self.position_samples.lock();
        *position = samples;
    }

    pub fn set_loop(&self, start: u64, end: u64) {
        let mut loop_start = self.loop_start.lock();
        let mut loop_end = self.loop_end.lock();
        *loop_start = start;
        *loop_end = end;
    }

    pub fn set_loop_enabled(&self, enabled: bool) {
        self.loop_enabled.store(enabled, Ordering::SeqCst);
    }

    pub fn is_loop_enabled(&self) -> bool {
        self.loop_enabled.load(Ordering::SeqCst)
    }

    pub fn get_recording_info(&self) -> Option<RecordingInfo> {
        self.recording_info.lock().clone()
    }

    pub fn update_stats(&self) {
        let mut stats = self.stats.lock();
        stats.sample_rate = self.config.sample_rate as f32;
        stats.actual_buffer_size = self.config.buffer_size as f32;
    }
}

impl Drop for Transport {
    fn drop(&mut self) {
        if self.is_recording.load(Ordering::SeqCst) {
            self.stop_recording_internal();
        }
    }
}
