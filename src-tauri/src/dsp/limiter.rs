use super::types::AudioBlock;
use std::f32::consts::LN_2;

#[derive(Debug, Clone)]
pub struct Limiter {
    pub threshold: f32,
    pub attack: f32,
    pub release: f32,
    pub makeup_gain: f32,
    envelope_follower: f32,
    gain_reduction: f32,
    sample_rate: f32,
}

impl Limiter {
    pub fn new(sample_rate: f32, threshold: f32, attack: f32, release: f32, makeup_gain: f32) -> Self {
        Self {
            threshold,
            attack,
            release,
            makeup_gain,
            envelope_follower: 0.0,
            gain_reduction: 0.0,
            sample_rate,
        }
    }

    fn attack_coeff(&self) -> f32 {
        let attack_seconds = self.attack / 1000.0;
        if attack_seconds <= 0.0 {
            return 1.0;
        }
        let coeff = (-LN_2 / (self.sample_rate * attack_seconds)).exp();
        coeff.clamp(0.0, 1.0)
    }

    fn release_coeff(&self) -> f32 {
        let release_seconds = self.release / 1000.0;
        if release_seconds <= 0.0 {
            return 1.0;
        }
        let coeff = (-LN_2 / (self.sample_rate * release_seconds)).exp();
        coeff.clamp(0.0, 1.0)
    }

    pub fn set_threshold(&mut self, threshold: f32) {
        self.threshold = threshold;
    }

    pub fn set_attack(&mut self, attack: f32) {
        self.attack = attack.max(0.0);
    }

    pub fn set_release(&mut self, release: f32) {
        self.release = release.max(0.0);
    }

    pub fn set_makeup_gain(&mut self, makeup_gain: f32) {
        self.makeup_gain = makeup_gain;
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
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
        let attack_coeff = self.attack_coeff();
        let release_coeff = self.release_coeff();
        let threshold_linear = Self::db_to_linear(self.threshold);
        let makeup_gain_linear = Self::db_to_linear(self.makeup_gain);

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

            let gain = if self.envelope_follower > threshold_linear {
                threshold_linear / self.envelope_follower
            } else {
                1.0
            };

            let envelope_db = Self::linear_to_db(self.envelope_follower);
            if envelope_db > self.threshold {
                self.gain_reduction = self.threshold - envelope_db;
            } else {
                self.gain_reduction = 0.0;
            }

            let final_gain = gain * makeup_gain_linear;

            for ch in 0..channels {
                block.samples[ch][frame] *= final_gain;
            }
        }
    }
}
