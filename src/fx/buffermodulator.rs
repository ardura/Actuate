use std::f32::consts::TAU;

// This started as me trying a bunch of different ways to make a StereoChorus without success
// I ended up liking this weird modulation behavior more
// Maybe I'll figure out chorus in future but for now here's buffer mod - Ardura

#[derive(Clone)]
pub struct BufferModulator {
    sample_rate: f32,
    depth: f32,
    rate: f32,
    spread: f32,
    buffer_tracker: f32,
    delay_left: usize,
    delay_right: usize,
    time_left: f32,
    time_right: f32,
    delay_line_left: Vec<f32>,
    delay_line_right: Vec<f32>,
}

impl BufferModulator {
    pub fn new(sample_rate: f32, depth: f32, rate: f32) -> Self {
        let delay_left = (sample_rate / 3.0) as usize;
        let delay_right = (sample_rate / 3.0 * rate) as usize;

        BufferModulator {
            sample_rate,
            depth,
            rate,
            spread: 0.0,
            buffer_tracker: 0.0,
            delay_left,
            delay_right,
            time_left: 0.0,
            time_right: 0.0,
            delay_line_left: vec![0.0; delay_left],
            delay_line_right: vec![0.0; delay_right],
        }
    }

    pub fn update(&mut self, sample_rate: f32, depth: f32, rate: f32, spread: f32, buffer: f32) {
        self.sample_rate = sample_rate;
        self.depth = depth;
        self.rate = rate;
        self.spread = spread.clamp(0.0, 1.0);
        if self.buffer_tracker != buffer {
            self.buffer_tracker = buffer;
            let temp = (buffer / 3.0).max(2.0) as usize;
            self.delay_left = temp;
            self.delay_right = temp;
            self.delay_line_left = vec![0.0; self.delay_left];
            self.delay_line_right = vec![0.0; self.delay_right];
        }
    }

    pub fn process(&mut self, input_left: f32, input_right: f32, amount: f32) -> (f32, f32) {
        // Update time variables
        self.time_left += 1.0 / self.sample_rate;
        // self.time_right += rate / self.sample_rate;
        self.time_right += 1.0 + self.spread / self.sample_rate;

        // Calculate modulation signals
        let modulation_left = (self.time_left * TAU * self.rate).sin();
        let modulation_right = (self.time_right * TAU * self.rate).sin();

        // Apply effect to the left channel
        let delayed_sample_left = self.delay_line_left.remove(0);
        let output_left = self.depth * delayed_sample_left * modulation_left;
        self.delay_line_left.push(input_left + output_left);

        // Apply effect to the right channel
        let delayed_sample_right = self.delay_line_right.remove(0);
        let output_right = self.depth * delayed_sample_right * modulation_right;
        self.delay_line_right.push(input_right + output_right);

        (
            output_left * amount + input_left * (1.0 - amount),
            output_right * amount + input_right * (1.0 - amount),
        )
    }
}
