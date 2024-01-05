#[derive(Clone)]
pub struct StereoReverb {
    left_delay: Vec<f32>,
    right_delay: Vec<f32>,
    delay_length: usize,
    feedback: f32,
    current_index: usize,
}

impl StereoReverb {
    pub fn new(sample_rate: f32, size: f32, feedback: f32) -> Self {
        // Calculate the length of the delay line based on the desired size
        let delay_length = ((size * sample_rate) / 2.0).round() as usize;

        // Initialize left and right delay lines with zeros
        let left_delay = vec![0.0; delay_length];
        let right_delay = vec![0.0; delay_length];

        StereoReverb {
            left_delay,
            right_delay,
            delay_length,
            feedback,
            current_index: 0,
        }
    }

    pub fn set_feedback(&mut self, feedback: f32) {
        self.feedback = feedback;
    }

    pub fn set_size(&mut self, size: f32, sample_rate: f32) {
        let temp: usize = ((size * sample_rate) / 2.0).round() as usize;
        if self.delay_length != temp {
            self.delay_length = temp;
            self.left_delay = vec![0.0; temp];
            self.right_delay = vec![0.0; temp];
            self.current_index = 0;
        }
    }

    pub fn process_tdl(&mut self, input_l: f32, input_r: f32, amount: f32) -> (f32, f32) {
        // Get the current values from the delay lines
        let delayed_sample_l = self.left_delay[self.current_index];
        let delayed_sample_r = self.right_delay[self.current_index];

        // Calculate the left and right outputs
        let mut output_l = input_l + self.feedback * delayed_sample_l;
        let mut output_r = input_r + self.feedback * delayed_sample_r;

        // Store the outputs in the delay lines
        self.left_delay[self.current_index] = output_l;
        self.right_delay[self.current_index] = output_r;

        // Move the index to the next position in the delay lines
        self.current_index = (self.current_index + 1) % self.delay_length;

        output_l = input_l * (1.0 - amount) + output_l * amount;
        output_r = input_r * (1.0 - amount) + output_r * amount;
        (output_l, output_r)
    }
}
