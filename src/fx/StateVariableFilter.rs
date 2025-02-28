use nih_plug::prelude::Enum;
use serde::{Deserialize, Serialize};
use std::f32::consts::PI;

#[derive(Enum, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub enum ResonanceType {
    Default,
    Moog,
    TB,
    Arp,
    Res,
    Bump,
    Powf,
}

#[derive(Clone)]
pub struct StateVariableFilter {
    sample_rate: f32,
    sample_rate_quad: f32,
    sample_rate_half: f32,
    frequency: f32,
    double_pi_freq: f32,
    q: f32,
    low_output: f32,
    band_output: f32,
    high_output: f32,
    res_mode: ResonanceType,
    oversample: i32,
    // Pre-computed coefficients
    normalized_freq: f32,
    resonance_coeff: f32,
}

impl Default for StateVariableFilter {
    fn default() -> Self {
        let mut filter = Self {
            sample_rate: 44100.0,
            sample_rate_quad: 44100.0 * 4.0,
            sample_rate_half: 22050.0,
            q: 0.1,
            frequency: 20000.0,
            double_pi_freq: 2.0 * PI * 20000.0,
            low_output: 0.0,
            band_output: 0.0,
            high_output: 0.0,
            res_mode: ResonanceType::Default,
            oversample: 4,
            normalized_freq: 0.0,
            resonance_coeff: 0.0,
        };
        
        // Initialize coefficients
        filter.update_coefficients();
        filter
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
        let mut parameters_changed = false;
        
        if sample_rate != self.sample_rate {
            self.sample_rate = sample_rate;
            self.sample_rate_quad = sample_rate * 4.0;
            self.sample_rate_half = sample_rate * 0.5;
            parameters_changed = true;
        }
        
        if q != self.q {
            // Apply mode-specific Q clamping
            self.q = match resonance_mode {
                ResonanceType::Default | ResonanceType::Bump => q.clamp(0.13, 1.0),
                _ => q.clamp(0.0, 1.0),
            };
            parameters_changed = true;
        }
        
        if frequency != self.frequency {
            self.frequency = frequency.clamp(20.0, 20000.0);
            self.double_pi_freq = 2.0 * PI * self.frequency;
            parameters_changed = true;
        }
        
        if resonance_mode != self.res_mode {
            self.res_mode = resonance_mode;
            parameters_changed = true;
        }
        
        // Only update coefficients if something changed
        if parameters_changed {
            self.update_coefficients();
        }
    }
    
    // New method to pre-compute coefficients
    fn update_coefficients(&mut self) {
        // Calculate normalized frequency based on mode
        self.normalized_freq = match self.res_mode {
            ResonanceType::Default | ResonanceType::Bump | ResonanceType::Powf => {
                self.double_pi_freq / self.sample_rate_quad
            },
            _ => self.double_pi_freq / self.sample_rate_half,
        };
        
        // Pre-compute resonance coefficient
        self.resonance_coeff = match self.res_mode {
            ResonanceType::Default => {
                (self.normalized_freq / (2.0 * self.q)).sin()
            },
            ResonanceType::Moog => {
                let resonance_exp = 16.0 * PI * self.q - 2.0;
                resonance_exp * (2.0 * PI * self.normalized_freq / self.sample_rate)
            },
            ResonanceType::TB => {
                let resonance_exp = 8.0 * PI * self.q;
                resonance_exp * (PI * self.normalized_freq / self.sample_rate).tan()
            },
            ResonanceType::Arp => {
                let resonance_exp = 2.0 * PI * self.q + 0.3;
                resonance_exp * (2.0 * PI * self.normalized_freq / self.sample_rate)
            },
            ResonanceType::Res => {
                let resonance_exp = (2.0 * PI * self.q).powf(0.9);
                resonance_exp * (2.0 * PI * self.normalized_freq / self.sample_rate).tan()
            },
            ResonanceType::Bump => {
                let resonance_exp = self.q * (self.normalized_freq / (2.0 * self.q)).sin();
                resonance_exp
                    * (self.normalized_freq / (2.0 * (self.q + 0.001))).asinh()
                    * (self.q + 0.001).sin()
            },
            ResonanceType::Powf => {
                let resonance_exp = (2.0 * PI * self.q).powf(0.4) + 0.001;
                (resonance_exp * (2.0 * PI * self.normalized_freq / (2.0 * resonance_exp)).sin()).tanh()
            },
        };
    }

    // More efficient processing using pre-computed coefficients
    pub fn process(&mut self, input: f32) -> (f32, f32, f32) {
        let rd_input = remove_denormals_fast(input);
        
        // Use pre-computed values for a more streamlined inner loop
        let normalized_freq = self.normalized_freq;
        let resonance = self.resonance_coeff;
        
        // Oversample by running multiple iterations
        for _ in 0..self.oversample {
            self.low_output += normalized_freq * self.band_output;
            self.high_output = rd_input - self.low_output - self.q * self.band_output;
            self.band_output += resonance * self.high_output;
            self.low_output += resonance * self.band_output;
        }
        
        // Apply anti-denormal to outputs
        self.low_output = remove_denormals_fast(self.low_output);
        self.band_output = remove_denormals_fast(self.band_output);
        self.high_output = remove_denormals_fast(self.high_output);
        
        (self.low_output, self.band_output, self.high_output)
    }
}

// More efficient denormal prevention without branching
#[inline(always)]
fn remove_denormals_fast(x: f32) -> f32 {
    // This tiny DC offset is below audible threshold but prevents denormals
    x + 1.0e-30
}