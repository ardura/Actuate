use nih_plug::prelude::Enum;
use serde::{Deserialize, Serialize};
use std::f32::consts::PI;

// Modified implementation from https://www.musicdsp.org/en/latest/Filters/23-state-variable.html and some tweaks
// Adapted to rust by Ardura

#[derive(Enum, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub enum ResonanceType {
    // Allegedly the "ideal" response when tying Q to angular sin response
    Default,
    // Allegedly a Moog Ladder Q approximation further modified
    Moog,
    // Allegedly an approximation of a TB-303 LP further modified
    TB,
    // Allegedly an approximation of an Arp 2600 further modified
    Arp,
    // I made this up - kind of a hyper resonance while still being gentle
    Res,
    // I made this up - Gentle bump - kind of Arp-ey
    Bump,
    // I made this up - Curve based on powf behavior
    Powf,
}

#[derive(Clone)]
pub struct StateVariableFilter {
    sample_rate: f32,
    frequency: f32,
    q: f32,
    low_output: f32,
    band_output: f32,
    high_output: f32,
    res_mode: ResonanceType,
    oversample: i32,
}

impl Default for StateVariableFilter {
    fn default() -> Self {
        Self {
            sample_rate: 44100.0,
            q: 0.0,
            frequency: 20000.0,
            low_output: 0.0,
            band_output: 0.0,
            high_output: 0.0,
            res_mode: ResonanceType::Default,
            oversample: 4,
        }
    }
}

impl StateVariableFilter {
    pub fn set_oversample(mut self, oversample_amount: i32) -> Self {
        self.oversample = oversample_amount;
        self
    }

    pub fn update(
        &mut self,
        frequency: f32,
        q: f32,
        sample_rate: f32,
        resonance_mode: ResonanceType,
    ) {
        if sample_rate != self.sample_rate {
            self.sample_rate = sample_rate;
        }
        if q != self.q {
            self.q = q.clamp(0.0, 1.0);
        }
        if frequency != self.frequency {
            self.frequency = frequency.clamp(20.0, 16000.0);
        }
        if resonance_mode != self.res_mode {
            self.res_mode = resonance_mode;
        }
    }

    pub fn process(&mut self, input: f32) -> (f32, f32, f32) {
        // Calculate our normalized freq for filtering
        let normalized_freq: f32 = match self.res_mode {
            ResonanceType::Default => (2.0 * PI * self.frequency) / (self.sample_rate * 4.0),
            ResonanceType::Moog => (2.0 * PI * self.frequency) / (self.sample_rate * 0.5),
            ResonanceType::TB => (2.0 * PI * self.frequency) / (self.sample_rate * 0.5),
            ResonanceType::Arp => (2.0 * PI * self.frequency) / (self.sample_rate * 0.5),
            // Actuate v1.0.2 additions
            ResonanceType::Res => (2.0 * PI * self.frequency) / (self.sample_rate * 0.5),
            ResonanceType::Bump => (2.0 * PI * self.frequency) / (self.sample_rate * 4.0),
            ResonanceType::Powf => (2.0 * PI * self.frequency) / (self.sample_rate * 4.0),
        };

        // Calculate our resonance coefficient
        // This is here to save calls during filter sweeps even though a static filter will use more resources this way
        let resonance = match self.res_mode {
            ResonanceType::Default => (normalized_freq / (2.0 * self.q)).sin(),
            // These are all approximations I found then modified - I'm not claiming any accuracy - more like inspiration
            ResonanceType::Moog => {
                let resonance_exp = 16.0 * PI * self.q - 2.0;
                resonance_exp * (2.0 * PI * normalized_freq / self.sample_rate)
            }
            ResonanceType::TB => {
                let resonance_exp = 8.0 * PI * self.q;
                resonance_exp * (PI * normalized_freq / self.sample_rate).tan()
            }
            ResonanceType::Arp => {
                let resonance_exp = 2.0 * PI * self.q + 0.3;
                resonance_exp * (2.0 * PI * normalized_freq / self.sample_rate)
            }
            // Actuate v1.0.2 additions
            // These ones I have made based off other ideas
            ResonanceType::Res => {
                let resonance_exp = (2.0 * PI * self.q).powf(0.9);
                resonance_exp * (2.0 * PI * normalized_freq / self.sample_rate).tan()
            }
            ResonanceType::Bump => {
                let resonance_exp = self.q * (normalized_freq / (2.0 * self.q)).sin();
                resonance_exp
                    * (normalized_freq / (2.0 * (self.q + 0.001))).asinh()
                    * (self.q + 0.001).sin()
            }
            ResonanceType::Powf => {
                let resonance_exp = (2.0 * PI * self.q).powf(0.4) + 0.001;
                (resonance_exp * (2.0 * PI * normalized_freq / (2.0 * resonance_exp)).sin()).tanh()
            }
        };

        // Oversample by running multiple iterations
        for _ in 0..self.oversample {
            self.low_output += normalized_freq * self.band_output;
            self.high_output = input - self.low_output - self.q * self.band_output;
            self.band_output += resonance * self.high_output;
            self.low_output += resonance * self.band_output;
        }
        (self.low_output, self.band_output, self.high_output)
    }
}
