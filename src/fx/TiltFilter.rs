// Thanks AI - it made a more stable and simpler filter than I could have

use nih_plug::prelude::Enum;
use serde::{Deserialize, Serialize};

#[derive(Enum, PartialEq, Serialize, Deserialize, Clone)]
pub enum ResponseType {
    Lowpass,
    Highpass,
}

// Have to allow dead code since rust complains about sample rate not being used despite it being used
#[allow(dead_code)]
#[derive(Clone)]
pub struct TiltFilterStruct {
    sample_rate: f32,
    lowpass: SimpleFilter,
    highpass: SimpleFilter,
    current_cutoff: f32,
    current_tilt: ResponseType,
    current_tilt_val: f32,
}

impl TiltFilterStruct {
    pub fn new(sample_rate: f32, initial_cutoff: f32, response_type_value: ResponseType) -> Self {
        let lowpass = SimpleFilter::lowpass(sample_rate, initial_cutoff);
        let highpass = SimpleFilter::highpass(sample_rate, initial_cutoff);
        let current_tilt_val = match response_type_value {
            ResponseType::Lowpass => {
                0.8
            },
            ResponseType::Highpass => {
                0.2
            }
        };

        Self {
            sample_rate,
            lowpass,
            highpass,
            current_cutoff: initial_cutoff,
            current_tilt: response_type_value,
            current_tilt_val: current_tilt_val,
        }
    }

    pub fn process(&mut self, input: f32) -> f32 {
        // Process input through lowpass and highpass filters
        let low_out = self.lowpass.process(input, ResponseType::Lowpass);
        let high_out = self.highpass.process(input, ResponseType::Highpass);

        // Crossfade between lowpass and highpass based on tilt
        let tilt_amount = self.current_tilt_val;

        low_out * tilt_amount + high_out * (1.0 - tilt_amount)
    }

    pub fn set_cutoff(&mut self, cutoff: f32) {
        self.current_cutoff = cutoff.clamp(20.0, 20_000.0); // Clamping to avoid extremes
        self.highpass.set_cutoff(self.current_cutoff);
        self.lowpass.set_cutoff(self.current_cutoff);
    }

    pub fn set_tilt(&mut self, tilt: ResponseType) {
        self.current_tilt = tilt.clone();
        self.current_tilt_val = match tilt {
            ResponseType::Lowpass => {
                0.8
            },
            ResponseType::Highpass => {
                0.2
            }
        }
    }
}

// First-order filter implementation
#[derive(Clone)]
pub struct SimpleFilter {
    a: f32,
    b: f32,
    prev_input: f32,
    prev_output: f32,
}

impl SimpleFilter {
    pub fn lowpass(sample_rate: f32, cutoff: f32) -> Self {
        let rc = 1.0 / (2.0 * std::f32::consts::PI * cutoff);
        let dt = 1.0 / sample_rate;
        let alpha = dt / (rc + dt);

        Self {
            a: alpha,
            b: 1.0 - alpha,
            prev_input: 0.0,
            prev_output: 0.0,
        }
    }

    pub fn highpass(sample_rate: f32, cutoff: f32) -> Self {
        let rc = 1.0 / (2.0 * std::f32::consts::PI * cutoff);
        let dt = 1.0 / sample_rate;
        let alpha = rc / (rc + dt);

        Self {
            a: alpha,
            b: 1.0 - alpha,
            prev_input: 0.0,
            prev_output: 0.0,
        }
    }

    pub fn set_cutoff(&mut self, cutoff: f32) {
        // Recalculate filter coefficient based on the new cutoff frequency
        let rc = 1.0 / (2.0 * std::f32::consts::PI * cutoff);
        let dt = 1.0 / 44_100.0; // Assuming sample rate
        self.a = dt / (rc + dt);
        self.b = 1.0 - self.a;
    }

    pub fn process(&mut self, input: f32, type_filter: ResponseType) -> f32 {
        match type_filter {
            ResponseType::Lowpass => {
                let output = self.a * input + self.b * self.prev_output;
                self.prev_output = output;
                output
            },
            ResponseType::Highpass => {
                let output = self.b * (self.prev_output + input - self.prev_input);
                self.prev_input = input;
                self.prev_output = output;
                output
            }
        }
    }
}
