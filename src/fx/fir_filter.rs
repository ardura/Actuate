use nih_plug::params::enums::Enum;
use serde::{Deserialize, Serialize};
use std::f32::consts::PI;

// FIR Supported Types
#[derive(Enum, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
pub enum FIRTypes {
    Highpass,
    Lowpass,
    Bandpass,
    Notch,
}

const TAPS: usize = 32;

//#[derive(Clone)]
pub struct FirFilter {
    sample_rate: f32,
    filter_type: FIRTypes,
    cutoff_freq: f32,
    coefficients: Vec<f32>,
    buffer: Vec<f32>,
    index: usize,
    resonance: f32,
}

impl FirFilter {
    pub fn new(sample_rate: f32) -> Self {
        // This doesn't matter since it will be updated
        let cutoff_freq = 16000.0;

        let coefficients =
            FirFilter::calculate_coefficients(FIRTypes::Lowpass, TAPS, sample_rate, cutoff_freq);
        let buffer_size = coefficients.len();
        FirFilter {
            sample_rate: sample_rate,
            filter_type: FIRTypes::Lowpass,
            cutoff_freq: cutoff_freq,
            coefficients,
            buffer: vec![0.0; buffer_size],
            index: 0,
            resonance: 0.0,
        }
    }

    pub fn update(
        &mut self,
        sample_rate: f32,
        filter_type: FIRTypes,
        cutoff_freq: f32,
        resonance: f32,
    ) {
        let mut recalc: bool = false;
        if self.sample_rate != sample_rate {
            self.sample_rate = sample_rate;
            recalc = true;
        }
        if self.filter_type != filter_type {
            self.filter_type = filter_type;
            recalc = true;
        }
        if self.cutoff_freq != cutoff_freq {
            self.cutoff_freq = cutoff_freq;
            recalc = true;
        }
        if self.resonance != resonance {
            self.resonance = resonance;
            recalc = true;
        }
        if recalc {
            self.coefficients = Self::calculate_coefficients(
                self.filter_type,
                TAPS,
                self.sample_rate,
                self.cutoff_freq,
            );
            self.buffer.resize(self.coefficients.len(), 0.0);
        }
    }

    pub fn process(&mut self, input: f32) -> f32 {
        // Add the input to the buffer
        self.buffer[self.index] = input;

        // Compute the filtered output
        let output: f32 = self
            .coefficients
            .iter()
            .zip(self.buffer.iter().rev())
            .map(|(&coef, &x)| coef * x)
            .sum();

        // Apply resonance by adding feedback
        //let feedback = self.resonance * (input - output);
        //let filtered_output = output + feedback;
        let filtered_output = output;

        // Update buffer index
        self.index = (self.index + 1) % self.buffer.len();

        filtered_output
    }

    pub fn calculate_coefficients(
        filter_type: FIRTypes,
        taps: usize,
        sample_rate: f32,
        cutoff_freq: f32,
    ) -> Vec<f32> {
        // Calculate coefficients based on filter type
        match filter_type {
            FIRTypes::Highpass => Self::highpass(taps, sample_rate, cutoff_freq),
            FIRTypes::Lowpass => Self::lowpass(taps, sample_rate, cutoff_freq),
            FIRTypes::Bandpass => Self::bandpass(taps, sample_rate, cutoff_freq),
            FIRTypes::Notch => Self::notch(taps, sample_rate, cutoff_freq),
        }
    }

    fn lowpass(taps: usize, sample_rate: f32, cutoff_freq: f32) -> Vec<f32> {
        // Calculate lowpass filter coefficients using a window function
        let nyquist = 0.5 * sample_rate;
        let normalized_cutoff = cutoff_freq / nyquist;
    
        let coefficients: Vec<f32> = (0..taps)
            .map(|n| {
                let sinc_val = if n == taps / 2 {
                    1.0
                } else {
                    (PI * (n as f32 - taps as f32 / 2.0) * normalized_cutoff).sin()
                        / (PI * (n as f32 - taps as f32 / 2.0))
                };
    
                0.54 - 0.46 * (2.0 * PI * n as f32 / taps as f32).cos() * sinc_val
            })
            .collect();
    
        coefficients
    }    

    fn highpass(taps: usize, sample_rate: f32, cutoff_freq: f32) -> Vec<f32> {
        // Calculate highpass filter coefficients by subtracting lowpass coefficients from a dirac impulse
        let lowpass_coefficients = Self::lowpass(taps, sample_rate, cutoff_freq);
        let mut dirac_impulse: Vec<f32> = vec![0.0; taps];
        dirac_impulse[taps / 2] = 1.0;

        lowpass_coefficients
            .iter()
            .zip(dirac_impulse.iter())
            .map(|(&a, &b)| b - a)
            .collect()
    }

    fn bandpass(taps: usize, sample_rate: f32, cutoff_freq: f32) -> Vec<f32> {
        // Calculate bandpass filter coefficients using two lowpass filters
        let low_cutoff = cutoff_freq - 0.5;
        let high_cutoff = cutoff_freq + 0.5;

        let lowpass1 = Self::lowpass(taps, sample_rate, low_cutoff);
        let lowpass2 = Self::lowpass(taps, sample_rate, high_cutoff);

        lowpass2
            .iter()
            .zip(lowpass1.iter())
            .map(|(&a, &b)| a - b)
            .collect()
    }

    fn notch(taps: usize, sample_rate: f32, cutoff_freq: f32) -> Vec<f32> {
        // Calculate notch filter coefficients by summing highpass and lowpass filter coefficients
        let highpass = Self::highpass(taps, sample_rate, cutoff_freq);
        let lowpass = Self::lowpass(taps, sample_rate, cutoff_freq);

        highpass
            .iter()
            .zip(lowpass.iter())
            .map(|(&a, &b)| a + b)
            .collect()
    }
}
