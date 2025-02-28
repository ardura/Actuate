// Simpler saturations for Actuate
// Ardura 2023, modified again in 2025

use nih_plug::params::enums::Enum;
use serde::{Deserialize, Serialize};
use std::f32::consts::PI;

#[derive(Clone, Enum, PartialEq, Serialize, Deserialize)]
pub enum SaturationType {
    Tape,
    Clip,
    SinPow,
    Subtle,
    Sine,
}

// Define a type for saturation processing function
type SaturationFn = fn(f32, f32, f32) -> (f32, f32);

#[derive(Clone, PartialEq)]
pub(crate) struct Saturation {
    sat_type: SaturationType,
    process_fn: SaturationFn,
}

impl Saturation {
    pub fn new() -> Self {
        let mut s = Saturation {
            sat_type: SaturationType::Tape,
            process_fn: Self::process_tape,
        };
        s.update_process_fn();
        s
    }

    pub fn set_type(&mut self, new_type: SaturationType) {
        if self.sat_type != new_type {
            self.sat_type = new_type;
            self.update_process_fn();
        }
    }

    // Update the function pointer based on saturation type
    fn update_process_fn(&mut self) {
        self.process_fn = match self.sat_type {
            SaturationType::Tape => Self::process_tape,
            SaturationType::Clip => Self::process_clip,
            SaturationType::SinPow => Self::process_sinpow,
            SaturationType::Subtle => Self::process_subtle,
            SaturationType::Sine => Self::process_sine,
        };
    }

    // Process our saturations - amount from 0 to 1
    pub fn process(&mut self, input_l: f32, input_r: f32, amount: f32) -> (f32, f32) {
        let idrive = if amount == 0.0 { 0.0001 } else { amount };
        (self.process_fn)(input_l, input_r, idrive)
    }

    // Individual processing functions for each saturation type
    fn process_tape(input_l: f32, input_r: f32, idrive: f32) -> (f32, f32) {
        let factor = 10.0 * idrive + 1.0;
        (
            (input_l * factor).tanh(),
            (input_r * factor).tanh(),
        )
    }

    fn process_clip(input_l: f32, input_r: f32, amount: f32) -> (f32, f32) {
        let one_minus_amount = 1.0 - amount;
        (
            input_l * one_minus_amount + input_l.signum() * amount,
            input_r * one_minus_amount + input_r.signum() * amount,
        )
    }

    fn process_sinpow(input_l: f32, input_r: f32, idrive: f32) -> (f32, f32) {
        (
            (input_l * idrive).sin().powf(2.0),
            (input_r * idrive).sin().powf(2.0),
        )
    }

    fn process_subtle(input_l: f32, input_r: f32, idrive: f32) -> (f32, f32) {
        (
            ((idrive * (idrive * PI * input_l).cos()) / 4.0) + input_l,
            ((idrive * (idrive * PI * input_r).cos()) / 4.0) + input_r,
        )
    }

    fn process_sine(input_l: f32, input_r: f32, idrive: f32) -> (f32, f32) {
        (
            input_l.signum() * (input_l.abs() + idrive).sin(),
            input_r.signum() * (input_r.abs() + idrive).sin(),
        )
    }
}