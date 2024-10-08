// A4I Filter - Averaged 4 Pole Integrated Filter
// Ardura

use nih_plug::util;

// Define the filter structure
#[derive(Clone)]
pub struct A4iFilter {
    poles: [OnePoleLowPassFilter; 4],
    iter: usize,
    osc_table_burst: [f32; 13],
    scale_gain: f32,
}

impl A4iFilter {
    pub fn new(sample_rate: f32, cutoff_freq: f32, resonance: f32) -> Self {
        let poles = [
            OnePoleLowPassFilter::new(sample_rate, cutoff_freq, resonance),
            OnePoleLowPassFilter::new(sample_rate, cutoff_freq, resonance),
            OnePoleLowPassFilter::new(sample_rate, cutoff_freq, resonance),
            OnePoleLowPassFilter::new(sample_rate, cutoff_freq, resonance),
        ];

        A4iFilter { poles, iter: 0, osc_table_burst: [
                1.0,
                1.0,
                1.0001,
                1.0,
                1.0,
                1.0,
                1.002,
                1.0,
                1.003,
                1.0,
                1.0,
                1.001,
                1.0001
            ],
            scale_gain: 1.0
        }
    }

    pub fn update(&mut self, cutoff_freq: f32, resonance: f32, sample_rate: f32) {
        self.scale_gain_from_cutoff(cutoff_freq);
        self.poles[0].update(cutoff_freq, sample_rate, resonance);
        self.poles[1].update(cutoff_freq - 80.0, sample_rate, resonance + 0.1);
        self.poles[2].update(cutoff_freq - 130.0, sample_rate, resonance + 0.2);
        self.poles[3].update(cutoff_freq - 200.0, sample_rate, resonance + 0.4);
    }

    pub fn process(&mut self, input: f32) -> f32 {
        let mut output = 0.0;
        let temp = self.poles[0].process(input);
        let temp2 = self.poles[1].process_w_res(temp);
        output += temp2;
        let temp3 = self.poles[2].process_w_res(temp2);
        let temp4 = self.poles[3].process_w_res(temp3);
        output += temp4;
        // Fun
        self.iter += 1;
        if self.iter > 12 {
            self.iter = 0;
        }
        (output * self.osc_table_burst[self.iter] * util::db_to_gain(15.0 + self.scale_gain)) / 4.0
    }

    fn scale_gain_from_cutoff(&mut self, cutoff_freq: f32) {
        let output_min = 1.0;
        let output_max = 10.0;
    
        self.scale_gain = output_max - (cutoff_freq.clamp(20.0, 20000.0) - 20.0) * (output_max - output_min) / 19980.0;
    }
    
    
}

#[derive(Clone)]
pub struct OnePoleLowPassFilter {
    alpha: f32,
    prev_output: f32,
    sample_rate: f32,
    cutoff_freq: f32,
    resonance: f32,
}

impl OnePoleLowPassFilter {
    pub fn new(sample_rate: f32, cutoff_freq: f32, resonance: f32) -> Self {
        let mut filter = OnePoleLowPassFilter {
            alpha: 0.0,
            prev_output: 0.0,
            sample_rate,
            cutoff_freq,
            resonance,
        };
        filter.update(cutoff_freq, sample_rate, resonance);
        filter
    }

    pub fn update(&mut self, cutoff_freq: f32, sample_rate: f32, resonance: f32) {
        let mut update: bool = false;
        if self.sample_rate != sample_rate {
            self.sample_rate = sample_rate;
            update = true;
        }
        if self.cutoff_freq != cutoff_freq {
            self.cutoff_freq = cutoff_freq.clamp(20.0, 20000.0);
            update = true;
        }
        if self.resonance != resonance {
            self.resonance = resonance.clamp(0.0, 2.0);
            update = true;
        }
        if update {
            let rc = 1.0 / (2.0 * std::f32::consts::PI * self.cutoff_freq);
            let dt = 1.0 / self.sample_rate;
            self.alpha = dt / (rc + dt);
        }
    }

    pub fn process_w_res(&mut self, input: f32) -> f32 {
        let feedback = self.prev_output * self.resonance;
        let filtered_input = input - feedback;
        self.prev_output = self.alpha * filtered_input + (1.0 - self.alpha) * self.prev_output;
        self.prev_output
    }

    pub fn process(&mut self, input: f32) -> f32 {
        self.prev_output = self.alpha * input + (1.0 - self.alpha) * self.prev_output;
        self.prev_output
    }
}