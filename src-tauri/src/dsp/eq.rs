use super::types::{AudioBlock, EqBand, FilterType};
use std::f32::consts::PI;

#[derive(Debug, Clone)]
struct EqBandState {
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
}

impl Default for EqBandState {
    fn default() -> Self {
        Self {
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ParametricEq {
    pub bands: Vec<EqBand>,
    pub filters: Vec<(f32, f32, f32, f32, f32)>,
    pub sample_rate: f32,
    states: Vec<Vec<EqBandState>>,
}

impl ParametricEq {
    pub fn new(sample_rate: f32, bands: Vec<EqBand>) -> Self {
        let filters: Vec<(f32, f32, f32, f32, f32)> = bands
            .iter()
            .map(|band| Self::create_filter(band, sample_rate))
            .collect();

        Self {
            bands,
            filters,
            sample_rate,
            states: Vec::new(),
        }
    }

    pub fn create_filter(band: &EqBand, sample_rate: f32) -> (f32, f32, f32, f32, f32) {
        let w0 = 2.0 * PI * band.frequency / sample_rate;
        let sin_w0 = w0.sin();
        let cos_w0 = w0.cos();
        let alpha = sin_w0 / (2.0 * band.q);
        let a = 10.0_f32.powf(band.gain / 40.0);

        let (b0, b1, b2, a0, a1, a2) = match band.filter_type {
            FilterType::Peaking => {
                let b0 = 1.0 + alpha * a;
                let b1 = -2.0 * cos_w0;
                let b2 = 1.0 - alpha * a;
                let a0 = 1.0 + alpha / a;
                let a1 = -2.0 * cos_w0;
                let a2 = 1.0 - alpha / a;
                (b0, b1, b2, a0, a1, a2)
            }
            FilterType::LowShelf => {
                let ap1 = a + 1.0;
                let am1 = a - 1.0;
                let sqrt_2a = (2.0 * a).sqrt() * band.q;
                let b0 = a * (ap1 - am1 * cos_w0 + sqrt_2a * sin_w0);
                let b1 = 2.0 * a * (am1 - ap1 * cos_w0);
                let b2 = a * (ap1 - am1 * cos_w0 - sqrt_2a * sin_w0);
                let a0 = ap1 + am1 * cos_w0 + sqrt_2a * sin_w0;
                let a1 = -2.0 * (am1 + ap1 * cos_w0);
                let a2 = ap1 + am1 * cos_w0 - sqrt_2a * sin_w0;
                (b0, b1, b2, a0, a1, a2)
            }
            FilterType::HighShelf => {
                let ap1 = a + 1.0;
                let am1 = a - 1.0;
                let sqrt_2a = (2.0 * a).sqrt() * band.q;
                let b0 = a * (ap1 + am1 * cos_w0 + sqrt_2a * sin_w0);
                let b1 = -2.0 * a * (am1 + ap1 * cos_w0);
                let b2 = a * (ap1 + am1 * cos_w0 - sqrt_2a * sin_w0);
                let a0 = ap1 - am1 * cos_w0 + sqrt_2a * sin_w0;
                let a1 = 2.0 * (am1 - ap1 * cos_w0);
                let a2 = ap1 - am1 * cos_w0 - sqrt_2a * sin_w0;
                (b0, b1, b2, a0, a1, a2)
            }
            FilterType::LowPass => {
                let b0 = (1.0 - cos_w0) / 2.0;
                let b1 = 1.0 - cos_w0;
                let b2 = (1.0 - cos_w0) / 2.0;
                let a0 = 1.0 + alpha;
                let a1 = -2.0 * cos_w0;
                let a2 = 1.0 - alpha;
                (b0, b1, b2, a0, a1, a2)
            }
            FilterType::HighPass => {
                let b0 = (1.0 + cos_w0) / 2.0;
                let b1 = -(1.0 + cos_w0);
                let b2 = (1.0 + cos_w0) / 2.0;
                let a0 = 1.0 + alpha;
                let a1 = -2.0 * cos_w0;
                let a2 = 1.0 - alpha;
                (b0, b1, b2, a0, a1, a2)
            }
            FilterType::BandPass => {
                let b0 = alpha;
                let b1 = 0.0;
                let b2 = -alpha;
                let a0 = 1.0 + alpha;
                let a1 = -2.0 * cos_w0;
                let a2 = 1.0 - alpha;
                (b0, b1, b2, a0, a1, a2)
            }
            FilterType::Notch => {
                let b0 = 1.0;
                let b1 = -2.0 * cos_w0;
                let b2 = 1.0;
                let a0 = 1.0 + alpha;
                let a1 = -2.0 * cos_w0;
                let a2 = 1.0 - alpha;
                (b0, b1, b2, a0, a1, a2)
            }
            FilterType::AllPass => {
                let b0 = 1.0 - alpha;
                let b1 = -2.0 * cos_w0;
                let b2 = 1.0 + alpha;
                let a0 = 1.0 + alpha;
                let a1 = -2.0 * cos_w0;
                let a2 = 1.0 - alpha;
                (b0, b1, b2, a0, a1, a2)
            }
        };

        (b0 / a0, b1 / a0, b2 / a0, a1 / a0, a2 / a0)
    }

    pub fn set_band(&mut self, index: usize, band: EqBand) {
        if index < self.bands.len() {
            self.bands[index] = band;
            self.filters[index] = Self::create_filter(&band, self.sample_rate);
        }
    }

    pub fn update_band_params(&mut self, index: usize, frequency: f32, gain: f32, q: f32) {
        if index < self.bands.len() {
            self.bands[index].frequency = frequency;
            self.bands[index].gain = gain;
            self.bands[index].q = q;
            self.filters[index] = Self::create_filter(&self.bands[index], self.sample_rate);
        }
    }

    pub fn set_band_bypass(&mut self, index: usize, bypass: bool) {
        if index < self.bands.len() {
            self.bands[index].bypass = bypass;
        }
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        for (i, band) in self.bands.iter().enumerate() {
            self.filters[i] = Self::create_filter(band, sample_rate);
        }
    }

    pub fn reset(&mut self) {
        self.states.clear();
    }

    pub fn process_block(&mut self, block: &mut AudioBlock) {
        let channels = block.channels;
        let num_bands = self.bands.len();

        if self.states.len() != channels {
            self.states = vec![vec![EqBandState::default(); num_bands]; channels];
        }

        for ch in 0..channels {
            let channel = &mut block.samples[ch];
            for (band_idx, band) in self.bands.iter().enumerate() {
                if band.bypass {
                    continue;
                }

                let (b0, b1, b2, a1, a2) = self.filters[band_idx];
                let state = &mut self.states[ch][band_idx];

                for sample in channel.iter_mut() {
                    let x0 = *sample;
                    let y0 = b0 * x0 + b1 * state.x1 + b2 * state.x2
                        - a1 * state.y1 - a2 * state.y2;

                    state.x2 = state.x1;
                    state.x1 = x0;
                    state.y2 = state.y1;
                    state.y1 = y0;

                    *sample = y0;
                }
            }
        }
    }
}
