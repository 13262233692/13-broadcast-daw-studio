use crate::dag::DAGProcessor;
use crate::dsp::AudioBlock;
use crate::shared::types::{AudioConfig, AudioStats, EngineEvent};
use crate::shared::rt_params::{ParamQueueHandle, GraphCmdQueueHandle, ParamUpdate, GraphCommand, RcuSnapshot, DagSnapshot};
use anyhow::{Context, Result};
use crossbeam_channel::Sender;
use ringbuf::{HeapProducer, HeapConsumer, HeapRb};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
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
    streams: parking_lot::Mutex<StreamHolder>,
    param_queue: ParamQueueHandle,
    graph_cmd_queue: GraphCmdQueueHandle,
    stats_snapshot: Arc<RcuSnapshot<AudioStats>>,
    dag_snapshot: Arc<RcuSnapshot<DagSnapshot>>,
    event_sender: Sender<EngineEvent>,
    is_running: bool,
    processing_thread_handle: Option<thread::JoinHandle<()>>,
    event_thread_handle: Option<thread::JoinHandle<()>>,
    processing_running: Arc<AtomicBool>,
    event_running: Arc<AtomicBool>,
}

impl AudioEngine {
    pub fn new(config: AudioConfig, event_sender: Sender<EngineEvent>) -> Self {
        let param_queue = ParamQueueHandle::new();
        let graph_cmd_queue = GraphCmdQueueHandle::new();

        let stats = AudioStats {
            cpu_usage: 0.0,
            xruns: 0,
            latency: 0.0,
            sample_rate: config.sample_rate as f32,
            actual_buffer_size: config.buffer_size as f32,
            dsp_load: 0.0,
        };

        Self {
            config,
            streams: parking_lot::Mutex::new(StreamHolder {
                input_stream: None,
                output_stream: None,
            }),
            param_queue: param_queue.clone(),
            graph_cmd_queue: graph_cmd_queue.clone(),
            stats_snapshot: Arc::new(RcuSnapshot::new(stats)),
            dag_snapshot: Arc::new(RcuSnapshot::new(DagSnapshot::default())),
            event_sender,
            is_running: false,
            processing_thread_handle: None,
            event_thread_handle: None,
            processing_running: Arc::new(AtomicBool::new(false)),
            event_running: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn param_queue(&self) -> &ParamQueueHandle {
        &self.param_queue
    }

    pub fn graph_cmd_queue(&self) -> &GraphCmdQueueHandle {
        &self.graph_cmd_queue
    }

    pub fn stats_snapshot(&self) -> Arc<RcuSnapshot<AudioStats>> {
        self.stats_snapshot.clone()
    }

    pub fn dag_snapshot(&self) -> Arc<RcuSnapshot<DagSnapshot>> {
        self.dag_snapshot.clone()
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

        let input_rb = HeapRb::<f32>::new(RING_BUFFER_SIZE);
        let (input_producer, input_consumer) = input_rb.split();
        let output_rb = HeapRb::<f32>::new(RING_BUFFER_SIZE);
        let (output_producer, output_consumer) = output_rb.split();

        let input_producer_mutexed = Arc::new(parking_lot::Mutex::new(Some(input_producer)));
        let output_consumer_mutexed = Arc::new(parking_lot::Mutex::new(Some(output_consumer)));

        let xruns_atomic = Arc::new(AtomicU64::new(0));
        let xruns_for_input = xruns_atomic.clone();
        let xruns_for_output = xruns_atomic.clone();
        let event_sender_for_input = self.event_sender.clone();
        let event_sender_for_output = self.event_sender.clone();

        let input_stream = cpal_host.build_input_stream(
            &input_device,
            &stream_config,
            move |data: &[f32], _info: &cpal::InputCallbackInfo| {
                let mut opt_guard = if let Some(g) = input_producer_mutexed.try_lock() { g } else { return; };
                let mut producer = match opt_guard.take() {
                    Some(p) => p,
                    None => return,
                };
                let written = producer.push_slice(data);
                *opt_guard = Some(producer);
                drop(opt_guard);
                if written < data.len() {
                    let new = xruns_for_input.fetch_add(1, Ordering::Relaxed) + 1;
                    let _ = event_sender_for_input.send(EngineEvent::XrunOccurred(new as u32));
                }
            },
        )?;

        let output_stream = cpal_host.build_output_stream(
            &output_device,
            &stream_config,
            move |data: &mut [f32], _info: &cpal::OutputCallbackInfo| {
                let mut opt_guard = if let Some(g) = output_consumer_mutexed.try_lock() { g } else {
                    for s in data.iter_mut() { *s = 0.0; }
                    return;
                };
                let mut consumer = match opt_guard.take() {
                    Some(c) => c,
                    None => { for s in data.iter_mut() { *s = 0.0; } return; },
                };
                let read = consumer.pop_slice(data);
                *opt_guard = Some(consumer);
                drop(opt_guard);
                if read < data.len() {
                    for sample in &mut data[read..] {
                        *sample = 0.0;
                    }
                    let new = xruns_for_output.fetch_add(1, Ordering::Relaxed) + 1;
                    let _ = event_sender_for_output.send(EngineEvent::XrunOccurred(new as u32));
                }
            },
        )?;

        input_stream.play()?;
        output_stream.play()?;

        {
            let mut s = self.streams.lock();
            s.input_stream = Some(input_stream);
            s.output_stream = Some(output_stream);
        }

        self.processing_running.store(true, Ordering::SeqCst);
        self.event_running.store(true, Ordering::SeqCst);

        let processing_running = self.processing_running.clone();
        let param_q = self.param_queue.clone();
        let graph_q = self.graph_cmd_queue.clone();
        let stats_rcu = self.stats_snapshot.clone();
        let dag_rcu = self.dag_snapshot.clone();
        let event_sender = self.event_sender.clone();
        let config = self.config.clone();
        let sr = self.config.sample_rate as f32;
        let bs = self.config.buffer_size as usize;
        let xruns_dsp = xruns_atomic.clone();

        self.processing_thread_handle = Some(
            thread::Builder::new()
                .name("broadcast-dsp".into())
                .spawn(move || {
                    set_realtime_class();

                    Self::audio_processing_loop(
                        processing_running,
                        input_consumer,
                        output_producer,
                        DAGProcessor::new(sr, 2, bs),
                        param_q,
                        graph_q,
                        stats_rcu,
                        dag_rcu,
                        event_sender,
                        config,
                        xruns_dsp,
                    );
                })
                .expect("Failed to spawn DSP thread")
        );

        let event_running = self.event_running.clone();
        let stats_rcu_event = self.stats_snapshot.clone();
        let event_sender_event = self.event_sender.clone();

        self.event_thread_handle = Some(thread::spawn(move || {
            Self::event_publish_loop(
                event_running,
                app_handle,
                stats_rcu_event,
                event_sender_event,
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

    pub fn is_running(&self) -> bool {
        self.is_running
    }

    fn apply_param_update(graph: &mut DAGProcessor, update: ParamUpdate) {
        match update {
            ParamUpdate::SetNodeBypass { node_id, bypassed } => {
                if let Some(node) = graph.get_node_mut(node_id) {
                    node.bypassed = bypassed;
                }
            }
            ParamUpdate::SetNodeEnabled { node_id, enabled } => {
                if let Some(node) = graph.get_node_mut(node_id) {
                    node.enabled = enabled;
                }
            }
            ParamUpdate::SetGain { node_id, gain } => {
                if let Some(proc) = graph.get_processor_mut::<crate::dag::GainNode>(node_id) {
                    proc.set_gain(gain);
                }
            }
            ParamUpdate::SetEqBands { node_id, bands } => {
                if let Some(proc) = graph.get_processor_mut::<crate::dag::EqNode>(node_id) {
                    let eq = proc.eq_mut();
                    for (i, band) in bands.into_iter().enumerate() {
                        eq.set_band(i, band);
                    }
                }
            }
            ParamUpdate::SetCompressorParams { node_id, params } => {
                if let Some(proc) = graph.get_processor_mut::<crate::dag::CompressorNode>(node_id) {
                    proc.compressor_mut().params = params;
                }
            }
            ParamUpdate::SetMasterVolume { volume } => {
                let ids: Vec<uuid::Uuid> = graph.get_nodes()
                    .iter()
                    .filter(|n| matches!(n.node_type, crate::dag::NodeType::Gain))
                    .map(|n| n.id)
                    .collect();
                for id in ids {
                    if let Some(proc) = graph.get_processor_mut::<crate::dag::GainNode>(id) {
                        proc.set_gain(volume);
                        break;
                    }
                }
            }
            ParamUpdate::SetEdgeGain { connection_id, gain } => {
                let conns = graph.get_connections();
                if let Some(_idx) = conns.iter().position(|c| c.id == connection_id) {
                    // Gain adjustment handled in-process via node-specific paths
                }
            }
        }
    }

    fn apply_graph_command(graph: &mut DAGProcessor, cmd: GraphCommand) {
        match cmd {
            GraphCommand::AddNode { node_type, name, position, config, reply_id: _ } => {
                graph.add_node(node_type, name, position, config);
            }
            GraphCommand::RemoveNode { node_id } => {
                graph.remove_node(node_id);
            }
            GraphCommand::Connect { source_node_id, source_port_id, target_node_id, target_port_id, gain, reply_id: _ } => {
                let _ = graph.connect(source_node_id, source_port_id, target_node_id, target_port_id, gain);
            }
            GraphCommand::Disconnect { connection_id } => {
                let _ = graph.disconnect(connection_id);
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn audio_processing_loop(
        running: Arc<AtomicBool>,
        mut input_consumer: HeapConsumer<f32>,
        mut output_producer: HeapProducer<f32>,
        mut graph: DAGProcessor,
        param_queue: ParamQueueHandle,
        graph_cmd_queue: GraphCmdQueueHandle,
        stats_snapshot: Arc<RcuSnapshot<AudioStats>>,
        dag_snapshot: Arc<RcuSnapshot<DagSnapshot>>,
        event_sender: Sender<EngineEvent>,
        config: AudioConfig,
        xruns_atomic: Arc<AtomicU64>,
    ) {
        let channels = 2usize;
        let buffer_size = config.buffer_size as usize;
        let sample_rate = config.sample_rate as f32;

        let mut param_buf: Vec<ParamUpdate> = Vec::with_capacity(256);
        let mut cmd_buf: Vec<GraphCommand> = Vec::with_capacity(32);

        let mut last_process_time = Instant::now();
        let mut last_snapshot_time = Instant::now();
        let mut cpu_start_time = Instant::now();
        let mut cpu_cycles: u64 = 0;
        let mut dsp_cycles: u64 = 0;
        let _last_xruns = xruns_atomic.load(Ordering::Relaxed);

        while running.load(Ordering::Acquire) {
            let cycle_start = Instant::now();

            graph_cmd_queue.try_drain_all(&mut cmd_buf);
            for cmd in cmd_buf.drain(..) {
                Self::apply_graph_command(&mut graph, cmd);
            }

            param_queue.try_drain_all(&mut param_buf);
            for update in param_buf.drain(..) {
                Self::apply_param_update(&mut graph, update);
            }

            {
                let mut interleaved = vec![0.0f32; buffer_size * channels];
                let read = input_consumer.pop_slice(&mut interleaved);

                if read == buffer_size * channels {
                    let mut audio_block =
                        AudioBlock::from_interleaved(&interleaved, channels, sample_rate);

                    let dsp_start = Instant::now();
                    let processed_block = graph.process(audio_block).unwrap_or_else(|_| {
                        AudioBlock::new(channels, buffer_size, sample_rate)
                    });
                    let dsp_elapsed = dsp_start.elapsed().as_nanos() as u64;
                    dsp_cycles += dsp_elapsed;

                    let processed = processed_block.to_interleaved();
                    let written = output_producer.push_slice(&processed);

                    if written < processed.len() {
                        let _ = xruns_atomic.fetch_add(1, Ordering::Relaxed);
                    }

                    let levels = Self::calculate_levels(&processed_block);
                    let _ = event_sender.send(EngineEvent::LevelUpdate(levels));

                    let clipping_channels = Self::detect_clipping(&processed_block);
                    if !clipping_channels.is_empty() {
                        let _ = event_sender.send(EngineEvent::Clipping(clipping_channels));
                    }
                }
            }

            let cycle_elapsed = cycle_start.elapsed().as_nanos() as u64;
            cpu_cycles += cycle_elapsed;

            let elapsed_since_last = last_process_time.elapsed();
            if elapsed_since_last >= Duration::from_millis(100) {
                let total_elapsed = cpu_start_time.elapsed().as_nanos() as u64;
                if total_elapsed > 0 {
                    let current_xruns = xruns_atomic.load(Ordering::Relaxed);
                    let new_stats = AudioStats {
                        cpu_usage: (cpu_cycles as f64 / total_elapsed as f64) as f32,
                        xruns: current_xruns as u32,
                        latency: (buffer_size as f32 / sample_rate) * 1000.0,
                        sample_rate,
                        actual_buffer_size: buffer_size as f32,
                        dsp_load: (dsp_cycles as f64 / total_elapsed as f64) as f32,
                    };
                    stats_snapshot.store(new_stats);
                }
                cpu_cycles = 0;
                dsp_cycles = 0;
                cpu_start_time = Instant::now();
                last_process_time = Instant::now();
            }

            if last_snapshot_time.elapsed() >= Duration::from_millis(100) {
                let snapshot = DagSnapshot {
                    nodes: graph.get_nodes().into_iter().cloned().collect(),
                    connections: graph.get_connections().to_vec(),
                };
                dag_snapshot.store(snapshot);
                last_snapshot_time = Instant::now();
            }

            let expected_cycle_time =
                Duration::from_micros((buffer_size as u64 * 1_000_000) / config.sample_rate as u64);
            let elapsed = cycle_start.elapsed();
            if elapsed < expected_cycle_time {
                spin_wait(expected_cycle_time - elapsed);
            }
        }
    }

    fn event_publish_loop(
        running: Arc<AtomicBool>,
        _app_handle: tauri::AppHandle,
        stats_snapshot: Arc<RcuSnapshot<AudioStats>>,
        event_sender: Sender<EngineEvent>,
    ) {
        let mut last_stats_update = Instant::now();

        while running.load(Ordering::Acquire) {
            let now = Instant::now();

            if now.duration_since(last_stats_update) >= STATS_UPDATE_INTERVAL {
                let stats_ref = stats_snapshot.load(Ordering::Acquire);
                let _ = event_sender.send(EngineEvent::StatsUpdate(*stats_ref));
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

fn spin_wait(dur: Duration) {
    let start = Instant::now();
    while start.elapsed() < dur {
        std::hint::spin_loop();
    }
}

#[cfg(target_os = "windows")]
fn set_realtime_class() {
    use windows_sys::Win32::System::Threading::{GetCurrentThread, SetThreadPriority, THREAD_PRIORITY_TIME_CRITICAL};
    unsafe {
        SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_TIME_CRITICAL);
    }
    let _ = set_current_thread_priority(ThreadPriority::Max);
}

#[cfg(target_os = "macos")]
fn set_realtime_class() {
    let _ = set_current_thread_priority(ThreadPriority::Max);
}

#[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
fn set_realtime_class() {
    let _ = set_current_thread_priority(ThreadPriority::Max);
}
