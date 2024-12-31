// A4II Filter - Averaged 4 Pole Integrated Filter - Another take
// Ardura

use std::f32::consts::PI;

#[derive(Clone)]
pub struct A4iiFilter {
    integrators: [f32; 4],
    cutoff: f32,
    sample_rate: f32,
    alpha: f32,
    resonance: f32,
    feedback: f32,
}

impl A4iiFilter {
    pub fn new(cutoff: f32, sample_rate: f32, resonance: f32) -> Self {
        let omega = 2.0 * PI * cutoff / sample_rate;
        let alpha = omega / (omega + 1.0);
        let resonance = resonance.clamp(0.0, 1.0); // Clamp resonance to valid range
        let feedback = resonance * 0.99;

        Self {
            integrators: [0.0; 4],
            cutoff,
            sample_rate,
            alpha,
            resonance,
            feedback
        }
    }

    pub fn update(&mut self, cutoff: f32, resonance: f32, sample_rate: f32) {
        if self.sample_rate != sample_rate {
            self.sample_rate = sample_rate;
        }
        self.cutoff = cutoff.max(0.0).min(self.sample_rate / 2.0);
        self.resonance = resonance.clamp(0.0, 1.0);
        self.feedback = self.resonance * 0.99;
        // Calculate the integration coefficient
        let omega = 2.0 * PI * self.cutoff / self.sample_rate;
        self.alpha = omega / (omega + 1.0);
    }

    pub fn process(&mut self, input: f32) -> f32 {
        let mut feedback_signal: f32 = 0.0;

        // Process the input through four integrators
        self.integrators[0] += self.alpha * (input - self.integrators[0] + feedback_signal);
        feedback_signal = self.integrators[0] * self.feedback; //Feedback from first stage
        self.integrators[1] += self.alpha * (self.integrators[0] - self.integrators[1] + feedback_signal);
        feedback_signal = self.integrators[1] * self.feedback; //Feedback from second stage
        self.integrators[2] += self.alpha * (self.integrators[1] - self.integrators[2] + feedback_signal);
        feedback_signal = self.integrators[2] * self.feedback; //Feedback from third stage
        self.integrators[3] += self.alpha * (self.integrators[2] - self.integrators[3] + feedback_signal);


        // Average the outputs of the four integrators
        let output = (self.integrators[0]
            + self.integrators[1]
            + self.integrators[2]
            + self.integrators[3])
            / 4.0;

        output
    }
}
