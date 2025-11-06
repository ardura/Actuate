// A4IV Filter - Ardura
// This is an idea of using averaging with some nonlinearity
// Gets less stable at low Hz without Res but doesn't blow up

#[derive(Clone)]
pub struct A4ivFilter {
    frequency: f32,
    resonance: f32,
    sample_rate: f32,
    a: f32,
    a2: f32,
    b: f32,
    previous_input: f32,
    previous_output: f32,
}

impl A4ivFilter {
    pub fn new(frequency: f32, resonance: f32, sample_rate: f32) -> Self {
        let a = 1.0 / (1.0 + (sample_rate / (2.0 * std::f32::consts::PI * frequency)).powi(2));
        let a2 = 1.0 / (1.0 + (sample_rate / (2.0 * std::f32::consts::PI * frequency)).powi(3));
        let b = (1.0 - a) * resonance;

        Self {
            frequency,
            resonance,
            sample_rate,
            a,
            a2,
            b,
            previous_input: 0.0,
            previous_output: 0.0,
        }
    }

    pub fn update(&mut self, frequency: f32, resonance: f32, sample_rate: f32) {
        self.frequency = frequency;
        self.resonance = resonance;
        self.sample_rate = sample_rate;

        self.a = 1.0 / (1.0 + (sample_rate / (2.0 * std::f32::consts::PI * frequency)).powi(2));
        self.a2 = 1.0 / (1.0 + (sample_rate / (2.0 * std::f32::consts::PI * frequency)).powi(3));
        self.b = (1.0 - self.a) * resonance;
    }

    pub fn process(&mut self, input: f32) -> f32 {
                          // Weird Average                          Feedback                        Low bump
        let output = (self.a * input + self.a2 * input)/2.0 + self.b * self.previous_output + ((input * 0.5) * self.resonance * 0.5)*0.25;
        self.previous_input = input;
        self.previous_output = output;

        output
    }
}
