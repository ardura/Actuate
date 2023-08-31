use std::f32::consts::PI;
use nih_plug::{prelude::Enum, util::db_to_gain};

// Modified implementation from https://www.musicdsp.org/en/latest/Filters/23-state-variable.html and AI
//
// Adapted to rust by Ardura

#[derive(Enum, PartialEq, Eq)]
pub enum ResonanceType {
    // Allegedly the "ideal" response when tying Q to angular sin response
    Default,
    // Allegedly a Moog Ladder Q calculation
    Moog,
    // Allegedly an approximation of a TB-303 LP
    TB,
    // Allegedly an approximation of an Arp 2600
    Arp,
}

pub struct StateVariableFilter {
    sample_rate: f32,
    frequency: f32,
    q: f32,
    low_output: f32,
    band_output: f32,
    high_output: f32,
    res_mode: ResonanceType,
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
        }
    }
}

impl StateVariableFilter {
    pub fn update(&mut self, frequency: f32, q: f32, sample_rate: f32, resonance_mode: ResonanceType) {
        if sample_rate != self.sample_rate {
            self.sample_rate = sample_rate;
        }
        if q != self.q {
            self.q = q;
        }
        if frequency != self.frequency {
            self.frequency = frequency;
        }
        if resonance_mode != self.res_mode {
            self.res_mode = resonance_mode;
        }
    }

    pub fn process(&mut self, input: f32) -> (f32, f32, f32) {
        // Prevent large DC spikes by changing freq
        match self.res_mode {
            ResonanceType::Moog => { self.frequency = self.frequency.clamp(1100.0, 16000.0); },
            ResonanceType::TB => { self.frequency = self.frequency.clamp(1100.0, 16000.0); },
            ResonanceType::Arp => { self.frequency = self.frequency.clamp(1100.0, 16000.0); },
            _ => {}
        }

        // Calculate our normalized freq for filtering
        let normalized_freq: f32 = (2.0 * PI * self.frequency) / (self.sample_rate*2.0);
        
        // I made this magic number by tesing resonance sweeps and monitoring
        // With single saw wave. Resonance would go wild otherwise and clip to infinity in some scenarios.
        let resonance_scaler: f32 = db_to_gain(-17.0);

        // Calculate our resonance coefficient
        // This is here to save calls during filter sweeps even though a static filter will use more resources this way
        let resonance = match self.res_mode {
            ResonanceType::Default => (normalized_freq / (2.0 * self.q)).sin(),
            ResonanceType::Moog => ((2.0 * PI * normalized_freq / self.sample_rate) / (4.0 * PI * self.q - 2.0)) * resonance_scaler,
            ResonanceType::TB => ((PI * normalized_freq / self.sample_rate).tan() / self.q) * resonance_scaler,
            ResonanceType::Arp => ((2.0 * PI * normalized_freq / self.sample_rate) / (2.0 * PI * self.q + 0.3)) * resonance_scaler,
        };

        // Oversample by running multiple iterations
        for _ in 0..4 {
            self.low_output += normalized_freq * self.band_output;
            self.high_output = input - self.low_output - self.q * self.band_output;
            self.band_output += resonance * self.high_output;
            self.low_output += resonance * self.band_output;
        }
        (self.low_output, self.band_output, self.high_output)
    }
}