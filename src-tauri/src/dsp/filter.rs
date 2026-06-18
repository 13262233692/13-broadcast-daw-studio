use super::types::AudioBlock;
use std::f32::consts::PI;

#[derive(Debug, Clone)]
pub struct BiquadFilter {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
    sample_rate: f32,
}

impl BiquadFilter {
    pub fn new(b0: f32, b1: f32, b2: f32, a1: f32, a2: f32, sample_rate: f32) -> Self {
        Self {
            b0,
            b1,
            b2,
            a1,
            a2,
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
            sample_rate,
        }
    }

    pub fn set_coefficients(&mut self, b0: f32, b1: f32, b2: f32, a1: f32, a2: f32) {
        self.b0 = b0;
        self.b1 = b1;
        self.b2 = b2;
        self.a1 = a1;
        self.a2 = a2;
    }

    pub fn process_sample(&mut self, x0: f32) -> f32 {
        let y0 = self.b0 * x0 + self.b1 * self.x1 + self.b2 * self.x2
            - self.a1 * self.y1 - self.a2 * self.y2;

        self.x2 = self.x1;
        self.x1 = x0;
        self.y2 = self.y1;
        self.y1 = y0;

        y0
    }

    pub fn reset(&mut self) {
        self.x1 = 0.0;
        self.x2 = 0.0;
        self.y1 = 0.0;
        self.y2 = 0.0;
    }

    pub fn process_block(&mut self, block: &mut AudioBlock) {
        for ch in 0..block.channels {
            let mut state = BiquadFilterState::default();
            let channel = &mut block.samples[ch];
            for sample in channel.iter_mut() {
                let x0 = *sample;
                let y0 = self.b0 * x0 + self.b1 * state.x1 + self.b2 * state.x2
                    - self.a1 * state.y1 - self.a2 * state.y2;
                state.x2 = state.x1;
                state.x1 = x0;
                state.y2 = state.y1;
                state.y1 = y0;
                *sample = y0;
            }
        }
    }
}

#[derive(Debug, Clone, Default)]
struct BiquadFilterState {
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
}

#[derive(Debug, Clone)]
pub struct LowPassFilter {
    filter: BiquadFilter,
    cutoff: f32,
    q: f32,
}

impl LowPassFilter {
    pub fn new(sample_rate: f32, cutoff: f32, q: f32) -> Self {
        let (b0, b1, b2, a1, a2) = Self::create_lowpass_coefficients(cutoff, q, sample_rate);
        Self {
            filter: BiquadFilter::new(b0, b1, b2, a1, a2, sample_rate),
            cutoff,
            q,
        }
    }

    fn create_lowpass_coefficients(cutoff: f32, q: f32, sample_rate: f32) -> (f32, f32, f32, f32, f32) {
        let w0 = 2.0 * PI * cutoff / sample_rate;
        let sin_w0 = w0.sin();
        let cos_w0 = w0.cos();
        let alpha = sin_w0 / (2.0 * q);

        let b0 = (1.0 - cos_w0) / 2.0;
        let b1 = 1.0 - cos_w0;
        let b2 = (1.0 - cos_w0) / 2.0;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos_w0;
        let a2 = 1.0 - alpha;

        (b0 / a0, b1 / a0, b2 / a0, a1 / a0, a2 / a0)
    }

    pub fn set_cutoff(&mut self, cutoff: f32) {
        self.cutoff = cutoff;
        self.update_coefficients();
    }

    pub fn set_q(&mut self, q: f32) {
        self.q = q;
        self.update_coefficients();
    }

    fn update_coefficients(&mut self) {
        let (b0, b1, b2, a1, a2) = Self::create_lowpass_coefficients(
            self.cutoff,
            self.q,
            self.filter.sample_rate,
        );
        self.filter.set_coefficients(b0, b1, b2, a1, a2);
    }

    pub fn process_sample(&mut self, sample: f32) -> f32 {
        self.filter.process_sample(sample)
    }

    pub fn process_block(&mut self, block: &mut AudioBlock) {
        self.filter.process_block(block);
    }

    pub fn reset(&mut self) {
        self.filter.reset();
    }
}

#[derive(Debug, Clone)]
pub struct HighPassFilter {
    filter: BiquadFilter,
    cutoff: f32,
    q: f32,
}

impl HighPassFilter {
    pub fn new(sample_rate: f32, cutoff: f32, q: f32) -> Self {
        let (b0, b1, b2, a1, a2) = Self::create_highpass_coefficients(cutoff, q, sample_rate);
        Self {
            filter: BiquadFilter::new(b0, b1, b2, a1, a2, sample_rate),
            cutoff,
            q,
        }
    }

    fn create_highpass_coefficients(cutoff: f32, q: f32, sample_rate: f32) -> (f32, f32, f32, f32, f32) {
        let w0 = 2.0 * PI * cutoff / sample_rate;
        let sin_w0 = w0.sin();
        let cos_w0 = w0.cos();
        let alpha = sin_w0 / (2.0 * q);

        let b0 = (1.0 + cos_w0) / 2.0;
        let b1 = -(1.0 + cos_w0);
        let b2 = (1.0 + cos_w0) / 2.0;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos_w0;
        let a2 = 1.0 - alpha;

        (b0 / a0, b1 / a0, b2 / a0, a1 / a0, a2 / a0)
    }

    pub fn set_cutoff(&mut self, cutoff: f32) {
        self.cutoff = cutoff;
        self.update_coefficients();
    }

    pub fn set_q(&mut self, q: f32) {
        self.q = q;
        self.update_coefficients();
    }

    fn update_coefficients(&mut self) {
        let (b0, b1, b2, a1, a2) = Self::create_highpass_coefficients(
            self.cutoff,
            self.q,
            self.filter.sample_rate,
        );
        self.filter.set_coefficients(b0, b1, b2, a1, a2);
    }

    pub fn process_sample(&mut self, sample: f32) -> f32 {
        self.filter.process_sample(sample)
    }

    pub fn process_block(&mut self, block: &mut AudioBlock) {
        self.filter.process_block(block);
    }

    pub fn reset(&mut self) {
        self.filter.reset();
    }
}
