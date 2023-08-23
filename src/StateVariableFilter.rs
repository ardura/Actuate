use std::f32::consts::PI;

// Modified implementation from https://www.musicdsp.org/en/latest/Filters/23-state-variable.html
// Adapted to rust and made a little more flexible by Ardura

pub struct StateVariableFilter {
    // These are in Hz
    sample_rate: f32,
    // Filter coeff 
    filter: f32,
    // Loop amount to rerun filter code
    iterations: usize,
    // These are [0..1]
    resonance: f32,
}

impl StateVariableFilter {
    pub fn update(&mut self, cutoff: f32, resonance: f32, iterations: usize, sample_rate: f32) {
        if sample_rate != self.sample_rate {
            self.sample_rate = sample_rate;
        }
        if resonance.clamp(0.001, 1.0) != self.resonance {
            self.resonance = resonance.clamp(0.001, 1.0);
        }
        if iterations != self.iterations {
            self.iterations = iterations;
        }
        self.filter = 2.0 * ((PI * cutoff)/self.sample_rate).sin();
    }
    pub fn process(&mut self, lp_amount: f32, hp_amount: f32, bp_amount: f32, notch_amount: f32, input_left: f32, input_right: f32, dry_wet: f32) -> (f32,f32) {
        if dry_wet == 0.0 {
            return (input_left, input_right);
        }

        let mut low_l: f32 = 0.0;
        let mut high_l: f32 = 0.0;
        let mut band_l: f32 = 0.0;
        let mut notch_l: f32 = 0.0;
        
        let mut low_r: f32 = 0.0;
        let mut high_r: f32 = 0.0;
        let mut band_r: f32 = 0.0;
        let mut notch_r: f32 = 0.0;

        let mut counter:usize = 0;

        // Process left
        while counter < self.iterations {
            low_l = low_l + self.filter * band_l;
            high_l = self.resonance * input_left - low_l - self.resonance * band_l;
            band_l = self.filter * high_l + band_l;
            notch_l = high_l + low_l;
            counter += 1;
        }

        counter = 0;
        // Process right
        while counter < self.iterations {
            low_r = low_r + self.filter * band_r;
            high_r = self.resonance * input_right - low_r - self.resonance * band_r;
            band_r = self.filter * high_r + band_r;
            notch_r = high_r + low_r;
            counter += 1;
        }

        // Combine signals
        let output_l = (low_l*lp_amount + high_l*hp_amount + band_l*bp_amount + notch_l*notch_amount)*dry_wet + (1.0 - dry_wet) * input_left;
        let output_r = (low_r*lp_amount + high_r*hp_amount + band_r*bp_amount + notch_r*notch_amount)*dry_wet + (1.0 - dry_wet) * input_right;

        (output_l,output_r)
    }
}