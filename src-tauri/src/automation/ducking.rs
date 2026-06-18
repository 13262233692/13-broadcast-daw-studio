use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct DuckingEnvelope {
    pub enabled: bool,
    pub duck_gain: f32,
    pub attack_ms: f32,
    pub hold_ms: f32,
    pub release_ms: f32,
    pub sample_rate: f32,
    pub current_gain: f32,
    target_gain: f32,
    state: DuckingState,
    samples_in_state: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
enum DuckingState { Idle, Attack, Hold, Release }

impl Default for DuckingEnvelope {
    fn default() -> Self {
        Self {
            enabled: false,
            duck_gain: 0.2,
            attack_ms: 10.0,
            hold_ms: 200.0,
            release_ms: 200.0,
            sample_rate: 48000.0,
            current_gain: 1.0,
            target_gain: 1.0,
            state: DuckingState::Idle,
            samples_in_state: 0,
        }
    }
}

impl DuckingEnvelope {
    pub fn new(sample_rate: f32) -> Self {
        let mut d = Self::default();
        d.sample_rate = sample_rate;
        d
    }

    pub fn trigger(&mut self) {
        if !self.enabled { return; }
        self.state = DuckingState::Attack;
        self.samples_in_state = 0;
        self.target_gain = self.duck_gain;
    }

    pub fn release(&mut self) {
        self.state = DuckingState::Release;
        self.samples_in_state = 0;
        self.target_gain = 1.0;
    }

    pub fn set_sample_rate(&mut self, sr: f32) { self.sample_rate = sr; }

    fn samples_per_ms(&self, ms: f32) -> f32 {
        (ms * self.sample_rate) / 1000.0
    }

    pub fn process_block(&mut self, block: &mut [f32]) {
        if !self.enabled { return; }
        for s in block.iter_mut() {
            match self.state {
                DuckingState::Attack => {
                    let attack_samples = self.samples_per_ms(self.attack_ms).max(1.0);
                    self.current_gain += (self.target_gain - self.current_gain) / attack_samples;
                    self.samples_in_state += 1;
                    if self.samples_in_state >= attack_samples as u64 {
                        self.current_gain = self.target_gain;
                        self.state = DuckingState::Hold;
                        self.samples_in_state = 0;
                    }
                }
                DuckingState::Hold => {
                    self.samples_in_state += 1;
                    let hold_samples = self.samples_per_ms(self.hold_ms);
                    if self.samples_in_state >= hold_samples as u64 {
                        self.state = DuckingState::Release;
                        self.samples_in_state = 0;
                        self.target_gain = 1.0;
                    }
                }
                DuckingState::Release => {
                    let release_samples = self.samples_per_ms(self.release_ms).max(1.0);
                    self.current_gain += (self.target_gain - self.current_gain) / release_samples;
                    self.samples_in_state += 1;
                    if (self.current_gain - 1.0).abs() < 0.001 {
                        self.current_gain = 1.0;
                        self.state = DuckingState::Idle;
                        self.samples_in_state = 0;
                    }
                }
                DuckingState::Idle => { self.current_gain = 1.0; }
            }
            *s *= self.current_gain;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_ducking_attack() {
        let mut d = DuckingEnvelope::new(48000.0);
        d.enabled = true;
        d.attack_ms = 1.0;
        d.duck_gain = 0.1;
        d.trigger();
        let mut block = [1.0f32; 480];
        d.process_block(&mut block);
        assert!(block[block.len()-1] < 0.2);
    }
}
