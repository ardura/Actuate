use std::f32::consts::PI;

#[derive(Clone)]
pub struct A4iiiFilter {
    integrators: [f32; 4],
    cutoff: f32,
    sample_rate: f32,
    alpha: f32,
    resonance: f32,
    feedback: f32,
}

impl A4iiiFilter {
    pub fn new(cutoff: f32, sample_rate: f32, resonance: f32) -> Self {
        let omega = 2.0 * PI * cutoff / sample_rate;
        let alpha = omega / (omega + 1.0);
        let current_resonance = resonance.clamp(0.0, 1.0);
        let feedback = current_resonance * 0.99;

        Self {
            integrators: [0.0; 4],
            cutoff,
            sample_rate,
            alpha,
            resonance: current_resonance,
            feedback,
        }
    }

    pub fn update(&mut self, cutoff: f32, resonance: f32, sample_rate: f32) {
        if self.sample_rate != sample_rate {
            self.sample_rate = sample_rate;
        }
        self.cutoff = cutoff.max(0.0).min(self.sample_rate / 2.0);
        self.resonance = resonance.clamp(0.0, 1.0);
        self.feedback = self.resonance * 0.99;
        let omega = 2.0 * PI * self.cutoff / self.sample_rate;
        self.alpha = omega / (omega + 1.0);
    }

    pub fn process(&mut self, input: f32) -> f32 {
        let mut feedback_signal: f32 = 0.0;
        let mut driven_val: f32;

        let pre_sat_0 = self.integrators[0] + self.alpha * (input - self.integrators[0] + feedback_signal);
        driven_val = pre_sat_0 * (self.resonance + 0.6);
        self.integrators[0] = driven_val / (1.0 + driven_val.abs());
        feedback_signal = self.integrators[0] * self.feedback;

        let pre_sat_1 = self.integrators[1] + self.alpha * (self.integrators[0] - self.integrators[1] + feedback_signal);
        driven_val = pre_sat_1 * (self.resonance + 0.45);
        self.integrators[1] = driven_val / (1.0 + driven_val.abs());
        feedback_signal = self.integrators[1] * self.feedback;

        let pre_sat_2 = self.integrators[2] + self.alpha * (self.integrators[1] - self.integrators[2] + feedback_signal);
        driven_val = pre_sat_2 * (self.resonance + 0.3);
        self.integrators[2] = driven_val / (1.0 + driven_val.abs());
        feedback_signal = self.integrators[2] * self.feedback;

        let pre_sat_3 = self.integrators[3] + self.alpha * (self.integrators[2] - self.integrators[3] + feedback_signal);
        driven_val = pre_sat_3 * (self.resonance + 0.15);
        self.integrators[3] = driven_val / (1.0 + driven_val.abs());

        // Average the outputs
        let output = (self.integrators[0]
            + self.integrators[1]
            + self.integrators[2]
            + self.integrators[3])
            / 4.0;
        
        // Apply final bump
        output * 1.3
    }
}