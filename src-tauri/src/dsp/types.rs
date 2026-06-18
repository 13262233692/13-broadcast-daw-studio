use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioBlock {
    pub samples: Vec<Vec<f32>>,
    pub sample_rate: f32,
    pub channels: usize,
    pub block_size: usize,
}

impl AudioBlock {
    pub fn new(channels: usize, block_size: usize, sample_rate: f32) -> Self {
        Self {
            samples: vec![vec![0.0; block_size]; channels],
            sample_rate,
            channels,
            block_size,
        }
    }

    pub fn from_interleaved(samples: &[f32], channels: usize, sample_rate: f32) -> Self {
        let block_size = samples.len() / channels;
        let mut deinterleaved = vec![vec![0.0; block_size]; channels];
        for (i, sample) in samples.iter().enumerate() {
            let ch = i % channels;
            let frame = i / channels;
            deinterleaved[ch][frame] = *sample;
        }
        Self {
            samples: deinterleaved,
            sample_rate,
            channels,
            block_size,
        }
    }

    pub fn to_interleaved(&self) -> Vec<f32> {
        let mut interleaved = Vec::with_capacity(self.channels * self.block_size);
        for frame in 0..self.block_size {
            for ch in 0..self.channels {
                interleaved.push(self.samples[ch][frame]);
            }
        }
        interleaved
    }

    pub fn clear(&mut self) {
        for ch in 0..self.channels {
            for s in 0..self.block_size {
                self.samples[ch][s] = 0.0;
            }
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum FilterType {
    Peaking,
    LowShelf,
    HighShelf,
    LowPass,
    HighPass,
    BandPass,
    Notch,
    AllPass,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct EqBand {
    pub filter_type: FilterType,
    pub frequency: f32,
    pub gain: f32,
    pub q: f32,
    pub bypass: bool,
}

impl EqBand {
    pub fn new(filter_type: FilterType, frequency: f32, gain: f32, q: f32) -> Self {
        Self {
            filter_type,
            frequency,
            gain,
            q,
            bypass: false,
        }
    }

    pub fn peaking(frequency: f32, gain: f32, q: f32) -> Self {
        Self::new(FilterType::Peaking, frequency, gain, q)
    }

    pub fn low_shelf(frequency: f32, gain: f32, q: f32) -> Self {
        Self::new(FilterType::LowShelf, frequency, gain, q)
    }

    pub fn high_shelf(frequency: f32, gain: f32, q: f32) -> Self {
        Self::new(FilterType::HighShelf, frequency, gain, q)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CompressorParams {
    pub threshold: f32,
    pub ratio: f32,
    pub attack: f32,
    pub release: f32,
    pub makeup_gain: f32,
    pub knee_width: f32,
    pub bypass: bool,
}

impl CompressorParams {
    pub fn new(
        threshold: f32,
        ratio: f32,
        attack: f32,
        release: f32,
        makeup_gain: f32,
        knee_width: f32,
    ) -> Self {
        Self {
            threshold,
            ratio,
            attack,
            release,
            makeup_gain,
            knee_width,
            bypass: false,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct LimiterParams {
    pub threshold: f32,
    pub attack: f32,
    pub release: f32,
    pub makeup_gain: f32,
}

impl LimiterParams {
    pub fn new(threshold: f32, attack: f32, release: f32, makeup_gain: f32) -> Self {
        Self {
            threshold,
            attack,
            release,
            makeup_gain,
        }
    }
}
