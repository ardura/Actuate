// Calculate and LFO and move phase similar to Oscillator.rs
// Ardura

extern crate num_traits;
use nih_plug::prelude::Enum;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct LFOController {
    frequency: f32,
    phase: f32,
    amplitude: f32,
    waveform: Waveform,
}

#[derive(Enum, PartialEq, Clone, Copy, Serialize, Deserialize)]
pub enum LFORetrigger {
    None,
    NoteOn,
}

#[derive(Enum, PartialEq, Clone, Copy, Serialize, Deserialize)]
pub enum LFOSnapValues {
    Quad,
    QuadD,
    QuadT,
    Double,
    DoubleD,
    DoubleT,
    Whole,
    WholeD,
    WholeT,
    Half,
    HalfD,
    HalfT,
    Quarter,
    QuarterD,
    QuarterT,
    Eighth,
    EighthD,
    EighthT,
    Sixteen,
    SixteenD,
    SixteenT,
    ThirtySecond,
    ThirtySecondD,
    ThirtySecondT,
}

#[derive(Enum, PartialEq, Clone, Copy, Serialize, Deserialize)]
pub enum Waveform {
    Sine,
    Square,
    Triangle,
    Sawtooth,
    Ramp,
    PulseQuarter,
    PulseEigth,
}

impl LFOController {
    pub fn new(frequency: f32, amplitude: f32, waveform: Waveform, phase: f32) -> Self {
        LFOController {
            frequency,
            phase,
            amplitude,
            waveform,
        }
    }

    pub fn set_frequency(&mut self, frequency: f32) {
        self.frequency = frequency;
    }

    pub fn set_waveform(&mut self, waveform: Waveform) {
        self.waveform = waveform;
    }

    pub fn set_phase(&mut self, phase: f32) {
        self.phase = phase;
    }

    pub fn get_frequency(&mut self) -> f32 {
        self.frequency
    }

    pub fn get_waveform(&mut self) -> Waveform {
        self.waveform
    }

    pub fn next_sample(&mut self, sample_rate: f32) -> f32 {
        let delta_time = 1.0 / sample_rate;
        self.phase += self.frequency * delta_time;

        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }

        match self.waveform {
            Waveform::Sine => self.amplitude * (2.0 * std::f32::consts::PI * self.phase).sin(),
            Waveform::Triangle => {
                if self.phase < 0.5 {
                    4.0 * self.amplitude * self.phase - self.amplitude
                } else {
                    3.0 * self.amplitude - 4.0 * self.amplitude * self.phase
                }
            }
            Waveform::Sawtooth => self.amplitude * (1.0 - 2.0 * self.phase),
            Waveform::Ramp => self.amplitude * self.phase,
            Waveform::Square => {
                if self.phase < 0.5 {
                    self.amplitude
                } else {
                    -self.amplitude
                }
            }
            Waveform::PulseQuarter => {
                if self.phase < 0.25 {
                    self.amplitude
                } else {
                    -self.amplitude
                }
            }
            Waveform::PulseEigth => {
                if self.phase < 0.125 {
                    self.amplitude
                } else {
                    -self.amplitude
                }
            }
        }
    }
}
