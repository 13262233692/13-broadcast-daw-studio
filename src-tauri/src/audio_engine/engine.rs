use crate::dag::DAGProcessor;
use crate::dsp::AudioBlock;
use crate::shared::types::{AudioConfig, AudioStats, EngineEvent};
use anyhow::{Context, Result};
use crossbeam_channel::Sender;
use parking_lot::{Mutex, RwLock};
use ringbuf::{HeapConsumer, HeapProducer, HeapRb};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use thread_priority::{ThreadPriority, set_current_thread_priority};

use super::cpal_impl::CpalHost;

const RING_BUFFER_SIZE: usize = 65536;
const LEVEL_UPDATE_INTERVAL: Duration = Duration::from_millis(50);
const STATS_UPDATE_INTERVAL: Duration = Duration::from_millis(100);

struct StreamHolder {
    input_stream: Option<cpal::Stream>,
    output_stream: Option<cpal::Stream>,
}

unsafe impl Send for StreamHolder {}

pub struct AudioEngine {
    config: AudioConfig,
    streams: Arc<Mutex<StreamHolder>>,
    processor_graph: Arc<RwLock<DAGProcessor>>,
    input_producer: Arc<Mutex<HeapProducer<f32>>>,
    input_consumer: Arc<Mutex<HeapConsumer<f32>>>,
    output_producer: Arc<Mutex<HeapProducer<f32>>>,
    output_consumer: Arc<Mutex<HeapConsumer<f32>>>,
    stats: Arc<Mutex<AudioStats>>,
    event_sender: Sender<EngineEvent>,
    is_running: bool,
    processing_thread_handle: Option<thread::JoinHandle<()>>,
    event_thread_handle: Option<thread::JoinHandle<()>>,
    processing_running: Arc<AtomicBool>,
    event_running: Arc<AtomicBool>,
}

impl AudioEngine {
    pub fn new(config: AudioConfig, event_sender: Sender<EngineEvent>) -> Self {
        let input_ringbuf = HeapRb::<f32>::new(RING_BUFFER_SIZE);
        let (input_producer, input_consumer) = input_ringbuf.split();

        let output_ringbuf = HeapRb::<f32>::new(RING_BUFFER_SIZE);
        let (output_producer, output_consumer) = output_ringbuf.split();

        let processor_graph = Arc::new(RwLock::new(DAGProcessor::new(
            config.sample_rate as f32,
            2,
            config.buffer_size as usize,
        )));

        let stats = Arc::new(Mutex::new(AudioStats {
            cpu_usage: 0.0,
            xruns: 0,
            latency: 0.0,
            sample_rate: config.sample_rate as f32,
            actual_buffer_size: config.buffer_size as f32,
            dsp_load: 0.0,
        }));

        Self {
            config,
            streams: Arc::new(Mutex::new(StreamHolder {
                input_stream: None,
                output_stream: None,
            })),
            processor_graph,
            input_producer: Arc::new(Mutex::new(input_producer)),
            input_consumer: Arc::new(Mutex::new(input_consumer)),
            output_producer: Arc::new(Mutex::new(output_producer)),
            output_consumer: Arc::new(Mutex::new(output_consumer)),
            stats,
            event_sender,
            is_running: false,
            processing_thread_handle: None,
            event_thread_handle: None,
            processing_running: Arc::new(AtomicBool::new(false)),
            event_running: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn start(&mut self, app_handle: tauri::AppHandle) -> Result<()> {
        if self.is_running {
            return Ok(());
        }

        let cpal_host = CpalHost::new().context("Failed to create CPAL host")?;

        let input_device = cpal_host
            .get_default_input_device()
            .context("No input device found")?;

        let output_device = cpal_host
            .get_default_output_device()
            .context("No output device found")?;

        let stream_config = cpal::StreamConfig {
            channels: 2,
            sample_rate: cpal::SampleRate(self.config.sample_rate),
            buffer_size: cpal::BufferSize::Fixed(self.config.buffer_size),
        };

        let input_producer = self.input_producer.clone();
        let stats_for_input = self.stats.clone();
        let event_sender_for_input = self.event_sender.clone();

        let input_stream = cpal_host.build_input_stream(
            &input_device,
            &stream_config,
            move |data: &[f32], _info: &cpal::InputCallbackInfo| {
                let mut producer = input_producer.lock();
                let written = producer.push_slice(data);
                if written < data.len() {
                    let mut stats = stats_for_input.lock();
                    stats.xruns += 1;
                    let _ = event_sender_for_input.send(EngineEvent::XrunOccurred(stats.xruns));
                }
            },
        )?;

        let output_consumer = self.output_consumer.clone();
        let stats_for_output = self.stats.clone();
        let event_sender_for_output = self.event_sender.clone();

        let output_stream = cpal_host.build_output_stream(
            &output_device,
            &stream_config,
            move |data: &mut [f32], _info: &cpal::OutputCallbackInfo| {
                let mut consumer = output_consumer.lock();
                let read = consumer.pop_slice(data);
                if read < data.len() {
                    for sample in &mut data[read..] {
                        *sample = 0.0;
                    }
                    let mut stats = stats_for_output.lock();
                    stats.xruns += 1;
                    let _ = event_sender_for_output.send(EngineEvent::XrunOccurred(stats.xruns));
                }
            },
        )?;

        input_stream.play()?;
        output_stream.play()?;

        self.streams.lock().input_stream = Some(input_stream);
        self.streams.lock().output_stream = Some(output_stream);

        self.processing_running.store(true, Ordering::SeqCst);
        self.event_running.store(true, Ordering::SeqCst);

        let processing_running = self.processing_running.clone();
        let input_consumer = self.input_consumer.clone();
        let output_producer = self.output_producer.clone();
        let processor_graph = self.processor_graph.clone();
        let stats = self.stats.clone();
        let event_sender = self.event_sender.clone();
        let config = self.config.clone();

        self.processing_thread_handle = Some(thread::spawn(move || {
            let _ = set_current_thread_priority(ThreadPriority::Max);
            Self::audio_processing_loop(
                processing_running,
                input_consumer,
                output_producer,
                processor_graph,
                stats,
                event_sender,
                config,
            );
        }));

        let event_running = self.event_running.clone();
        let stats_for_event = self.stats.clone();
        let event_sender_for_event = self.event_sender.clone();
        let processor_graph_for_event = self.processor_graph.clone();
        let config_for_event = self.config.clone();

        self.event_thread_handle = Some(thread::spawn(move || {
            Self::event_publish_loop(
                event_running,
                app_handle,
                stats_for_event,
                event_sender_for_event,
                processor_graph_for_event,
                config_for_event,
            );
        }));

        self.is_running = true;
        Ok(())
    }

    pub fn stop(&mut self) -> Result<()> {
        if !self.is_running {
            return Ok(());
        }

        self.processing_running.store(false, Ordering::SeqCst);
        self.event_running.store(false, Ordering::SeqCst);

        if let Some(handle) = self.processing_thread_handle.take() {
            handle.join().ok();
        }

        if let Some(handle) = self.event_thread_handle.take() {
            handle.join().ok();
        }

        {
            let mut streams = self.streams.lock();
            streams.input_stream.take();
            streams.output_stream.take();
        }

        self.is_running = false;
        Ok(())
    }

    pub fn processor_graph(&self) -> Arc<RwLock<DAGProcessor>> {
        self.processor_graph.clone()
    }

    pub fn stats(&self) -> Arc<Mutex<AudioStats>> {
        self.stats.clone()
    }

    pub fn is_running(&self) -> bool {
        self.is_running
    }

    fn audio_processing_loop(
        running: Arc<AtomicBool>,
        input_consumer: Arc<Mutex<HeapConsumer<f32>>>,
        output_producer: Arc<Mutex<HeapProducer<f32>>>,
        processor_graph: Arc<RwLock<DAGProcessor>>,
        stats: Arc<Mutex<AudioStats>>,
        event_sender: Sender<EngineEvent>,
        config: AudioConfig,
    ) {
        let channels = 2usize;
        let buffer_size = config.buffer_size as usize;
        let sample_rate = config.sample_rate as f32;

        let mut last_process_time = Instant::now();
        let mut cpu_start_time = Instant::now();
        let mut cpu_cycles: u64 = 0;
        let mut dsp_cycles: u64 = 0;

        while running.load(Ordering::SeqCst) {
            let cycle_start = Instant::now();

            {
                let mut consumer = input_consumer.lock();
                let available = consumer.len();

                if available >= buffer_size * channels {
                    let mut interleaved = vec![0.0f32; buffer_size * channels];
                    let read = consumer.pop_slice(&mut interleaved);

                    if read == buffer_size * channels {
                        let mut audio_block =
                            AudioBlock::from_interleaved(&interleaved, channels, sample_rate);

                        let dsp_start = Instant::now();
                        let processed_block = {
                            let mut graph = processor_graph.write();
                            graph.process(audio_block).unwrap_or_else(|_| {
                                AudioBlock::new(channels, buffer_size, sample_rate)
                            })
                        };
                        let dsp_elapsed = dsp_start.elapsed().as_nanos() as u64;
                        dsp_cycles += dsp_elapsed;

                        let processed = processed_block.to_interleaved();

                        let mut producer = output_producer.lock();
                        let written = producer.push_slice(&processed);

                        if written < processed.len() {
                            let mut stats_guard = stats.lock();
                            stats_guard.xruns += 1;
                            let _ = event_sender
                                .send(EngineEvent::XrunOccurred(stats_guard.xruns));
                        }

                        let levels = Self::calculate_levels(&processed_block);
                        let _ = event_sender.send(EngineEvent::LevelUpdate(levels));

                        let clipping_channels = Self::detect_clipping(&processed_block);
                        if !clipping_channels.is_empty() {
                            let _ = event_sender.send(EngineEvent::Clipping(clipping_channels));
                        }
                    }
                }
            }

            let cycle_elapsed = cycle_start.elapsed().as_nanos() as u64;
            cpu_cycles += cycle_elapsed;

            let elapsed_since_last = last_process_time.elapsed();
            if elapsed_since_last >= Duration::from_millis(100) {
                let total_elapsed = cpu_start_time.elapsed().as_nanos() as u64;
                if total_elapsed > 0 {
                    let mut stats_guard = stats.lock();
                    stats_guard.cpu_usage = (cpu_cycles as f64 / total_elapsed as f64) as f32;
                    stats_guard.dsp_load = (dsp_cycles as f64 / total_elapsed as f64) as f32;
                    stats_guard.latency = (buffer_size as f32 / sample_rate) * 1000.0;
                }
                cpu_cycles = 0;
                dsp_cycles = 0;
                cpu_start_time = Instant::now();
                last_process_time = Instant::now();
            }

            let expected_cycle_time =
                Duration::from_micros((buffer_size as u64 * 1_000_000) / config.sample_rate as u64);
            let elapsed = cycle_start.elapsed();
            if elapsed < expected_cycle_time {
                thread::sleep(expected_cycle_time - elapsed);
            }
        }
    }

    fn event_publish_loop(
        running: Arc<AtomicBool>,
        _app_handle: tauri::AppHandle,
        stats: Arc<Mutex<AudioStats>>,
        event_sender: Sender<EngineEvent>,
        _processor_graph: Arc<RwLock<DAGProcessor>>,
        _config: AudioConfig,
    ) {
        let mut last_stats_update = Instant::now();

        while running.load(Ordering::SeqCst) {
            let now = Instant::now();

            if now.duration_since(last_stats_update) >= STATS_UPDATE_INTERVAL {
                let current_stats = *stats.lock();
                let _ = event_sender.send(EngineEvent::StatsUpdate(current_stats));
                last_stats_update = now;
            }

            thread::sleep(LEVEL_UPDATE_INTERVAL);
        }
    }

    fn calculate_levels(block: &AudioBlock) -> Vec<f32> {
        let mut levels = Vec::with_capacity(block.channels);
        for ch in 0..block.channels {
            let mut sum = 0.0f32;
            for s in 0..block.block_size {
                let sample = block.samples[ch][s];
                sum += sample * sample;
            }
            let rms = (sum / block.block_size as f32).sqrt();
            levels.push(rms);
        }
        levels
    }

    fn detect_clipping(block: &AudioBlock) -> Vec<usize> {
        let mut clipping = Vec::new();
        for ch in 0..block.channels {
            for s in 0..block.block_size {
                let sample = block.samples[ch][s];
                if sample.abs() >= 1.0 {
                    clipping.push(ch);
                    break;
                }
            }
        }
        clipping
    }
}

impl Drop for AudioEngine {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}
