use super::types::{AudioBlock, CompressorParams};
use std::f32::consts::LN_2;

#[derive(Debug, Clone)]
pub struct Compressor {
    pub params: CompressorParams,
    envelope_follower: f32,
    gain_reduction: f32,
    pub sample_rate: f32,
}

impl Compressor {
    pub fn new(sample_rate: f32, params: CompressorParams) -> Self {
        Self {
            params,
            envelope_follower: 0.0,
            gain_reduction: 0.0,
            sample_rate,
        }
    }

    pub fn attack_coeff(&self) -> f32 {
        let attack_seconds = self.params.attack / 1000.0;
        if attack_seconds <= 0.0 {
            return 1.0;
        }
        let coeff = (-LN_2 / (self.sample_rate * attack_seconds)).exp();
        coeff.clamp(0.0, 1.0)
    }

    pub fn release_coeff(&self) -> f32 {
        let release_seconds = self.params.release / 1000.0;
        if release_seconds <= 0.0 {
            return 1.0;
        }
        let coeff = (-LN_2 / (self.sample_rate * release_seconds)).exp();
        coeff.clamp(0.0, 1.0)
    }

    pub fn set_params(&mut self, params: CompressorParams) {
        self.params = params;
    }

    pub fn set_threshold(&mut self, threshold: f32) {
        self.params.threshold = threshold;
    }

    pub fn set_ratio(&mut self, ratio: f32) {
        self.params.ratio = ratio.max(1.0);
    }

    pub fn set_attack(&mut self, attack: f32) {
        self.params.attack = attack.max(0.0);
    }

    pub fn set_release(&mut self, release: f32) {
        self.params.release = release.max(0.0);
    }

    pub fn set_makeup_gain(&mut self, makeup_gain: f32) {
        self.params.makeup_gain = makeup_gain;
    }

    pub fn set_knee_width(&mut self, knee_width: f32) {
        self.params.knee_width = knee_width.max(0.0);
    }

    pub fn set_bypass(&mut self, bypass: bool) {
        self.params.bypass = bypass;
    }

    pub fn get_gain_reduction(&self) -> f32 {
        self.gain_reduction
    }

    pub fn reset(&mut self) {
        self.envelope_follower = 0.0;
        self.gain_reduction = 0.0;
    }

    fn linear_to_db(linear: f32) -> f32 {
        if linear <= 0.0 {
            -100.0
        } else {
            20.0 * linear.log10()
        }
    }

    fn db_to_linear(db: f32) -> f32 {
        10.0_f32.powf(db / 20.0)
    }

    pub fn process_block(&mut self, block: &mut AudioBlock) {
        if self.params.bypass {
            return;
        }

        let attack_coeff = self.attack_coeff();
        let release_coeff = self.release_coeff();
        let threshold = self.params.threshold;
        let ratio = self.params.ratio;
        let knee_width = self.params.knee_width;
        let makeup_gain = Self::db_to_linear(self.params.makeup_gain);

        let block_size = block.block_size;
        let channels = block.channels;

        for frame in 0..block_size {
            let mut peak_sample = 0.0;
            for ch in 0..channels {
                let sample = block.samples[ch][frame].abs();
                if sample > peak_sample {
                    peak_sample = sample;
                }
            }

            let target_envelope = if peak_sample > self.envelope_follower {
                attack_coeff * self.envelope_follower + (1.0 - attack_coeff) * peak_sample
            } else {
                release_coeff * self.envelope_follower + (1.0 - release_coeff) * peak_sample
            };
            self.envelope_follower = target_envelope;

            let envelope_db = Self::linear_to_db(self.envelope_follower);

            let gain_reduction_db = if knee_width > 0.0 {
                let knee_start = threshold - knee_width / 2.0;
                let knee_end = threshold + knee_width / 2.0;

                if envelope_db < knee_start {
                    0.0
                } else if envelope_db > knee_end {
                    threshold + (envelope_db - threshold) / ratio - envelope_db
                } else {
                    let t = (envelope_db - knee_start) / knee_width;
                    let t2 = t * t;
                    let amount = t2 * (envelope_db - threshold) / ratio;
                    threshold + amount - envelope_db
                }
            } else {
                if envelope_db <= threshold {
                    0.0
                } else {
                    threshold + (envelope_db - threshold) / ratio - envelope_db
                }
            };

            self.gain_reduction = gain_reduction_db;

            let gain_linear = Self::db_to_linear(gain_reduction_db) * makeup_gain;

            for ch in 0..channels {
                block.samples[ch][frame] *= gain_linear;
            }
        }
    }
}
