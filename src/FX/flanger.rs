use std::f32::consts::PI;

#[derive(Clone)]
pub struct StereoFlanger {
    sample_rate: f32,
    depth: f32,
    lfo_rate: f32,
    delay_range: f32,
    feedback: f32,
    delay_line: Vec<(f32, f32)>,
    index: usize,
    lfo_phase: f32,
}

impl StereoFlanger {
    pub fn new(sample_rate: f32, depth: f32, lfo_rate: f32, delay_range: f32, feedback: f32, max_delay_samples: usize) -> Self {
        Self {
            sample_rate,
            depth,
            lfo_rate,
            delay_range,
            feedback,
            delay_line: vec![(0.0, 0.0); max_delay_samples],
            index: 0,
            lfo_phase: 0.0,
        }
    }

    pub fn update(&mut self, sample_rate: f32, depth: f32, lfo_rate: f32, feedback: f32) {
        self.sample_rate = sample_rate;
        self.depth = depth;
        self.lfo_rate = lfo_rate;
        self.feedback = feedback;
    }

    pub fn process(&mut self, left_in: f32, right_in: f32, amount: f32) -> (f32, f32) {
        // Update LFO phase
        self.lfo_phase += 2.0 * PI * self.lfo_rate / self.sample_rate;
        if self.lfo_phase > 2.0 * PI {
            self.lfo_phase -= 2.0 * PI;
        }

        // Calculate modulation depth
        let modulator = self.depth * (0.5 * self.lfo_phase.sin() + 0.5);

        // Calculate delay in samples
        let delay_samples = (self.delay_range * modulator) as usize;

        // Retrieve delayed samples from the delay line
        let delayed_left = self.delay_line[(self.index + delay_samples) % self.delay_line.len()].0;
        let delayed_right = self.delay_line[(self.index + delay_samples) % self.delay_line.len()].1;

        // Apply flanger effect
        let mut left_out = left_in + self.feedback * delayed_left;
        let mut right_out = right_in + self.feedback * delayed_right;

        // Update delay line
        self.delay_line[self.index] = (left_in, right_in);

        // Increment index and wrap around
        self.index = (self.index + 1) % self.delay_line.len();

        // Mix dry and wet signals based on the amount parameter
        left_out = left_in * (1.0 - amount) + left_out * amount;
        right_out = right_in * (1.0 - amount) + right_out * amount;

        (left_out, right_out)
    }
}
