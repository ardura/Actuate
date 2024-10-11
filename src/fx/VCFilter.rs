use nih_plug::params::enums::Enum;
use serde::{Deserialize, Serialize};

// Rust port of https://www.musicdsp.org/en/latest/Filters/24-moog-vcf.html
// Ardura

#[derive(Enum, PartialEq, Serialize, Deserialize, Clone)]
pub enum ResponseType {
    Lowpass,
    Bandpass,
    Highpass,
}

#[derive(Clone)]
pub struct VCFilter {
    // Parameters
    center_freq: f32,
    resonance: f32,
    shape: ResponseType,
    // Internal
    f: f32,
    k: f32,
    p: f32,
    r: f32,
    olds: [f32; 4],
    y: [f32; 4],
    sample_rate: f32,
}

impl VCFilter {
    pub fn new() -> Self {
        VCFilter {
            center_freq: 1000.0,
            resonance: 0.1,
            shape: ResponseType::Lowpass,
            f: 0.0,
            k: 0.0,
            p: 0.0,
            r: 0.0,
            olds: [0.0; 4],
            y: [0.0; 4],
            sample_rate: 44100.0,
        }
    }

    pub fn update(
        &mut self,
        center_freq: f32,
        resonance: f32,
        shape: ResponseType,
        sample_rate: f32,
    ) {
        let mut recalculate = false;
        if self.center_freq.clamp(20.0, 17000.0) != center_freq.clamp(20.0, 17000.0) {
            self.center_freq = center_freq.clamp(20.0, 17000.0);
            recalculate = true;
        }
        if self.resonance != resonance {
            self.resonance = resonance;
            recalculate = true;
        }
        if self.shape != shape {
            self.shape = shape;
        }
        if self.sample_rate != sample_rate {
            self.sample_rate = sample_rate;
            recalculate = true;
        }
        if recalculate {
            self.f = 2.0 * self.center_freq / self.sample_rate;
            self.k = 3.6 * self.f - 1.6 * self.f * self.f - 1.0;
            self.p = (self.k + 1.0) * 0.5;
            let scale = (1.0 - self.p).exp() * 0.9;
            self.r = (1.01 - self.resonance) * scale;
        }
    }

    pub fn process(&mut self, input: f32) -> f32 {
        let x = input - self.r * self.y[3];
        self.y[0] = x * self.p + self.olds[0] * self.p - self.k * self.y[0];
        self.y[1] = self.y[0] * self.p + self.olds[1] * self.p - self.k * self.y[1];
        self.y[2] = self.y[1] * self.p + self.olds[2] * self.p - self.k * self.y[2];
        self.y[3] = self.y[2] * self.p + self.olds[3] * self.p - self.k * self.y[3];
        self.y[3] = self.y[3] - (self.y[3].powf(3.0)) / 6.0;
        self.olds[0] = x;
        self.olds[1] = self.y[0];
        self.olds[2] = self.y[1];
        self.olds[3] = self.y[2];
        match self.shape {
            ResponseType::Lowpass => self.y[3],
            ResponseType::Highpass => input - self.y[3],
            ResponseType::Bandpass => self.y[3] - (input - self.y[3]),
        }
    }
}
