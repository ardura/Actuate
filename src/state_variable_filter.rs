use std::f32::consts::{self, PI};

pub struct state_variable_filter {
    // These are in Hz
    sample_rate: f32,
    // Filter coeff 
    filter: f32,
    // Loop amount to rerun filter code
    iterations: usize,
    // These are [0..1]
    resonance: f32,
}

impl state_variable_filter {
    pub fn update(&mut self, cutoff: f32, resonance: f32, iterations: usize, sample_rate: f32) {
        let new_res = resonance.clamp(0.001, 1.0);
        if sample_rate != self.sample_rate {
            self.sample_rate = sample_rate;
        }
        if resonance != self.resonance {
            self.resonance = resonance;
        }
        if iterations != self.iterations {
            self.iterations = iterations;
        }
        self.filter = 2.0 * ((PI * cutoff)/self.sample_rate).sin();
    }
    pub fn process(&mut self, lowpass_amount: f32, highpass_amount: f32, bandpass_amount: f32, notch_amount: f32, input_left: f32, input_right: f32) -> (f32,f32) {
        let mut low: f32 = 0.0;
        let mut high: f32;
        let mut band: f32 = 0.0;
        let mut notch: f32;
        let mut counter:usize = 0;
        // Process left
        while counter < self.iterations {
            low = low + self.filter * band;
            high = self.resonance * input_left - low - self.resonance * band;
            band = self.filter * high + band;
            notch = high + low;
            counter += 1;
        }
        low = 0.0;
        band = 0.0;
        counter = 0;
        // Process right
        while counter < self.iterations {
            low = low + self.filter * band;
            high = self.resonance * input_left - low - self.resonance * band;
            band = self.filter * high + band;
            notch = high + low;
            counter += 1;
        }
        (0.0,0.0)
    }
}