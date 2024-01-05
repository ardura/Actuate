#[derive(Clone)]
pub struct StereoLimiter {
    threshold: f32,
    knee_width: f32,
}

impl StereoLimiter {
    pub fn new(threshold: f32, knee_width: f32) -> Self {
        StereoLimiter { threshold, knee_width }
    }

    pub fn update(&mut self, knee_width: f32, threshold: f32) {
        self.threshold = threshold;
        self.knee_width = knee_width;
    }

    pub fn process(&self, left_in: f32, right_in: f32) -> (f32, f32) {
        let left_gain = self.limit(left_in);
        let right_gain = self.limit(right_in);
        (left_gain, right_gain)
    }

    pub fn limit(&self, input: f32) -> f32 {
        let knee_range = self.knee_width / 2.0;
        let soft_threshold = self.threshold + knee_range;

        if input.abs() > soft_threshold {
            let gain_reduction = (input.abs() - soft_threshold) / knee_range;
            let gain = 1.0 / (1.0 + gain_reduction);
            gain * input.signum() * soft_threshold.abs()
        } else {
            input
        }
    }
}
