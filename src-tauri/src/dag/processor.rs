use crate::dsp::types::AudioBlock;
use crate::dsp::{Compressor, Limiter, ParametricEq};
use super::node::NodeProcessor;

#[derive(Debug, Clone)]
pub struct InputNode {
    sample_rate: f32,
    channels: usize,
    block_size: usize,
}

impl InputNode {
    pub fn new(sample_rate: f32, channels: usize, block_size: usize) -> Self {
        Self {
            sample_rate,
            channels,
            block_size,
        }
    }
}

impl NodeProcessor for InputNode {
    fn process(&mut self, inputs: Vec<AudioBlock>) -> AudioBlock {
        if !inputs.is_empty() {
            return inputs[0].clone();
        }
        AudioBlock::new(self.channels, self.block_size, self.sample_rate)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

#[derive(Debug, Clone)]
pub struct OutputNode {
    sample_rate: f32,
    channels: usize,
    block_size: usize,
}

impl OutputNode {
    pub fn new(sample_rate: f32, channels: usize, block_size: usize) -> Self {
        Self {
            sample_rate,
            channels,
            block_size,
        }
    }
}

impl NodeProcessor for OutputNode {
    fn process(&mut self, inputs: Vec<AudioBlock>) -> AudioBlock {
        if !inputs.is_empty() {
            return inputs[0].clone();
        }
        AudioBlock::new(self.channels, self.block_size, self.sample_rate)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

#[derive(Debug, Clone)]
pub struct MixNode {
    sample_rate: f32,
    channels: usize,
    block_size: usize,
    gains: Vec<f32>,
}

impl MixNode {
    pub fn new(sample_rate: f32, channels: usize, block_size: usize, num_inputs: usize) -> Self {
        Self {
            sample_rate,
            channels,
            block_size,
            gains: vec![1.0; num_inputs],
        }
    }

    pub fn set_gain(&mut self, index: usize, gain: f32) {
        if index < self.gains.len() {
            self.gains[index] = gain;
        }
    }
}

impl NodeProcessor for MixNode {
    fn process(&mut self, inputs: Vec<AudioBlock>) -> AudioBlock {
        let mut output = AudioBlock::new(self.channels, self.block_size, self.sample_rate);

        for (input_idx, input) in inputs.iter().enumerate() {
            let gain = self.gains.get(input_idx).copied().unwrap_or(1.0);
            for ch in 0..self.channels.min(input.channels) {
                for frame in 0..self.block_size.min(input.block_size) {
                    output.samples[ch][frame] += input.samples[ch][frame] * gain;
                }
            }
        }

        output
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

#[derive(Debug, Clone)]
pub struct GainNode {
    gain: f32,
    sample_rate: f32,
    channels: usize,
    block_size: usize,
}

impl GainNode {
    pub fn new(sample_rate: f32, channels: usize, block_size: usize) -> Self {
        Self {
            gain: 1.0,
            sample_rate,
            channels,
            block_size,
        }
    }

    pub fn with_gain(sample_rate: f32, channels: usize, block_size: usize, gain: f32) -> Self {
        Self {
            gain,
            sample_rate,
            channels,
            block_size,
        }
    }

    pub fn set_gain(&mut self, gain: f32) {
        self.gain = gain;
    }

    pub fn set_gain_db(&mut self, db: f32) {
        self.gain = 10.0_f32.powf(db / 20.0);
    }
}

impl NodeProcessor for GainNode {
    fn process(&mut self, inputs: Vec<AudioBlock>) -> AudioBlock {
        let mut output = if !inputs.is_empty() {
            inputs[0].clone()
        } else {
            AudioBlock::new(self.channels, self.block_size, self.sample_rate)
        };

        if (self.gain - 1.0).abs() > f32::EPSILON {
            for ch in 0..output.channels {
                for frame in 0..output.block_size {
                    output.samples[ch][frame] *= self.gain;
                }
            }
        }

        output
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

#[derive(Debug, Clone)]
pub struct EqNode {
    eq: ParametricEq,
    sample_rate: f32,
    channels: usize,
    block_size: usize,
}

impl EqNode {
    pub fn new(eq: ParametricEq, sample_rate: f32, channels: usize, block_size: usize) -> Self {
        Self {
            eq,
            sample_rate,
            channels,
            block_size,
        }
    }

    pub fn eq(&self) -> &ParametricEq {
        &self.eq
    }

    pub fn eq_mut(&mut self) -> &mut ParametricEq {
        &mut self.eq
    }
}

impl NodeProcessor for EqNode {
    fn process(&mut self, inputs: Vec<AudioBlock>) -> AudioBlock {
        let mut output = if !inputs.is_empty() {
            inputs[0].clone()
        } else {
            AudioBlock::new(self.channels, self.block_size, self.sample_rate)
        };

        self.eq.process_block(&mut output);
        output
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

#[derive(Debug, Clone)]
pub struct CompressorNode {
    compressor: Compressor,
    sample_rate: f32,
    channels: usize,
    block_size: usize,
}

impl CompressorNode {
    pub fn new(compressor: Compressor, sample_rate: f32, channels: usize, block_size: usize) -> Self {
        Self {
            compressor,
            sample_rate,
            channels,
            block_size,
        }
    }

    pub fn compressor(&self) -> &Compressor {
        &self.compressor
    }

    pub fn compressor_mut(&mut self) -> &mut Compressor {
        &mut self.compressor
    }

    pub fn get_gain_reduction(&self) -> f32 {
        self.compressor.get_gain_reduction()
    }
}

impl NodeProcessor for CompressorNode {
    fn process(&mut self, inputs: Vec<AudioBlock>) -> AudioBlock {
        let mut output = if !inputs.is_empty() {
            inputs[0].clone()
        } else {
            AudioBlock::new(self.channels, self.block_size, self.sample_rate)
        };

        self.compressor.process_block(&mut output);
        output
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

#[derive(Debug, Clone)]
pub struct LimiterNode {
    limiter: Limiter,
    sample_rate: f32,
    channels: usize,
    block_size: usize,
}

impl LimiterNode {
    pub fn new(limiter: Limiter, sample_rate: f32, channels: usize, block_size: usize) -> Self {
        Self {
            limiter,
            sample_rate,
            channels,
            block_size,
        }
    }

    pub fn limiter(&self) -> &Limiter {
        &self.limiter
    }

    pub fn limiter_mut(&mut self) -> &mut Limiter {
        &mut self.limiter
    }

    pub fn get_gain_reduction(&self) -> f32 {
        self.limiter.get_gain_reduction()
    }
}

impl NodeProcessor for LimiterNode {
    fn process(&mut self, inputs: Vec<AudioBlock>) -> AudioBlock {
        let mut output = if !inputs.is_empty() {
            inputs[0].clone()
        } else {
            AudioBlock::new(self.channels, self.block_size, self.sample_rate)
        };

        self.limiter.process_block(&mut output);
        output
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

#[derive(Debug, Clone)]
pub struct Vst3Node {
    sample_rate: f32,
    channels: usize,
    block_size: usize,
    plugin_path: Option<String>,
    plugin_loaded: bool,
}

impl Vst3Node {
    pub fn new(sample_rate: f32, channels: usize, block_size: usize) -> Self {
        Self {
            sample_rate,
            channels,
            block_size,
            plugin_path: None,
            plugin_loaded: false,
        }
    }

    pub fn load_plugin(&mut self, path: impl Into<String>) -> Result<(), String> {
        self.plugin_path = Some(path.into());
        self.plugin_loaded = true;
        Ok(())
    }

    pub fn is_plugin_loaded(&self) -> bool {
        self.plugin_loaded
    }
}

impl NodeProcessor for Vst3Node {
    fn process(&mut self, inputs: Vec<AudioBlock>) -> AudioBlock {
        if !inputs.is_empty() {
            return inputs[0].clone();
        }
        AudioBlock::new(self.channels, self.block_size, self.sample_rate)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
