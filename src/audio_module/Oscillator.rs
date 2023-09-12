/*
Copyright (C) 2023 Ardura

This program is free software:
you can redistribute it and/or modify it under the terms of the GNU General Public License
as published by the Free Software Foundation,either version 3 of the License, or (at your option) any later version.

This program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY;
without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
See the GNU General Public License for more details.

You should have received a copy of the GNU General Public License along with this program.
If not, see https://www.gnu.org/licenses/.

#####################################

Oscillator by Ardura

This creates an oscillator mathematically with some modifiers I made myself.
This is intended to be a building block used by other files in the Actuate synth.

#####################################
*/

use std::f32::consts::{self, PI, FRAC_2_PI};
use nih_plug::{params::enums::Enum};

#[derive(Enum, PartialEq, Eq, Debug, Copy, Clone)]
pub enum VoiceType {
    Sine,
    Tri,
    Saw,
    RSaw,
    InSaw,
    Ramp,
    Square,
    RSquare,
}

#[derive(Enum, PartialEq, Eq, Debug, Copy, Clone)]
pub enum OscState {
    Off,
    Attacking,
    Decaying,
    Sustaining,
    Releasing,
}

#[derive(Enum, PartialEq, Eq, Debug, Copy, Clone)]
pub enum SmoothStyle {
    Linear,
    Logarithmic,
    Exponential,
}

#[derive(Enum, PartialEq, Eq, Debug, Copy, Clone)]
pub enum RetriggerStyle {
    Free,
    Retrigger,
    Random,
}

// Super useful function to scale an input 0-1 into other ranges
pub(crate) fn scale_range(input: f32, min_output: f32, max_output: f32) -> f32 {
    let scaled = input * (max_output - min_output) + min_output;
    scaled.clamp(min_output, max_output)
}

/*
    I'm designing each of these waveforms to be the frequency + modifier that changes the waveform
    This way I can simplify the amount of waveforms while creating more options!
    I'm not sure if this is efficient or not, but it's my synth :)
    modifier is between 0 and 1 unlss Oscillator::scale_range is used
*/

// Sine wave oscillator modded with some sort of saw wave multiplication
pub fn calculate_sine(mod_amount: f32, phase: f32) -> f32 {
    // f(x) = sin(x * tau) {0 < x < 1}
    let mut sine: f32 = 0.0;
    let scaled_phase = scale_range(phase, -1.0, 1.0);
    if mod_amount <= 0.33 {
        sine = (phase * consts::TAU).sin();
    } else if mod_amount > 0.33 && mod_amount < 0.67 {
        // X^2 Approximation
        if scaled_phase < 0.0 {
            sine = ((2.0 * scaled_phase + 1.0).powi(2) - 1.0) * 0.99;
        }
        else {
            sine = (-(2.0 * scaled_phase - 1.0).powi(2) + 1.0) * 0.99;
        }
    } else if mod_amount >= 0.67 {
        // Allegedy other efficient approximation
        sine = ((24.5 * scaled_phase) / consts::TAU) - (((24.5 * scaled_phase) * scaled_phase.abs()) / consts::TAU);
    }
    sine
}

// Rounded Saw Wave with rounding amount
pub fn calculate_rsaw(rounding: f32, phase: f32) -> f32 {
    let rounding_amount: i32 = scale_range(rounding, 2.0, 30.0).floor() as i32;
    let scaled_phase: f32 = scale_range(phase, -1.0, 1.0);
    // n = rounding int
    // f(x) = x * (1 − x^(2n))
    scaled_phase * (1.0 - scaled_phase.powi(2 * rounding_amount))
}

// Saw Wave with half rectification in modifier
pub fn calculate_saw(mod_to_bool: f32, phase: f32) -> f32 {
    let half: bool = if mod_to_bool < 0.5 { false } else { true };
    let scaled_phase = if half { phase } else { 
        scale_range(phase, -1.0, 1.0) };
    // f(x) = x mod period
    scaled_phase % consts::TAU
}

// Ramp Wave with half rectification in modifier
pub fn calculate_ramp(mod_to_bool: f32, phase: f32) -> f32 {
    let half: bool = if mod_to_bool < 0.5 { false } else { true };
    let scaled_phase = if half { phase } else { 
        scale_range(phase, -1.0, 1.0) };
    // f(x) = x mod period
    -1.0 * (scaled_phase % consts::TAU)
}

// Inward Curved Saw Wave
pub fn calculate_inward_saw(curve_amount: f32, phase: f32) -> f32 {
    // This makes more sense to the user even though it's a little weird to modify it like this
    let mut calc_curve_amount: i32 = scale_range(curve_amount, 1.0, 4.99).floor() as i32;
    match calc_curve_amount {
        1 => calc_curve_amount = 2,
        2 => calc_curve_amount = 10,
        3 => calc_curve_amount = 3,
        4 => calc_curve_amount = 11,
        // Unreachable
        _ => calc_curve_amount = 1,
    }
    let scaled_phase: f32 = scale_range(phase, -1.0, 1.0);
    // f(x) = (x + 1)^6 {-1 <= x <= 0}
    // f(x) = -(x-1)^6 {0 <= x <= 1}
    if scaled_phase <= 0.0 {
        (scaled_phase + 1.0).powi(calc_curve_amount)
    } else {
        -(scaled_phase - 1.0).powi(calc_curve_amount)
    }
}

pub fn calculate_square(mod_amount: f32, phase: f32) -> f32 {
    let mod_scaled: f32 = scale_range(1.0 - mod_amount, 0.0625, 0.5);
    let square: f32 = 1.0;
    // Hard cut function scaling to a pulse with mod
    if phase >= mod_scaled {
        square * -1.0
    } else {
        square
    }
}

pub fn calculate_rounded_square(mod_amount: f32, phase: f32) -> f32 {
    let scaled_phase: f32 = scale_range(phase, -1.0, 1.0);
    let mod_scaled: i32 = scale_range(mod_amount, 2.0, 8.0).floor() as i32 * 2;
    // Rounding function is approximated with these exponential functions
    if scaled_phase <  0.0 {
        (2.0 * scaled_phase + 1.0).powi(mod_scaled) - 1.0
    } else {
        -(2.0 * scaled_phase - 1.0).powi(mod_scaled) + 1.0
    }
}

pub fn calculate_tri(mod_amount: f32, phase: f32) -> f32 {
    let tri: f32 = (FRAC_2_PI) * (((2.0 * PI) * phase).sin()).asin();
    let mut tan_tri: f32 = 0.0;
    // Mix in 
    if mod_amount >  0.0 {
        tan_tri = ((phase * PI).sin()).tan()/(consts::FRAC_PI_2);
    }
    // Use mod to fade between tri and weird tan tri
    tri*(1.0 - mod_amount) + tan_tri*mod_amount
}
