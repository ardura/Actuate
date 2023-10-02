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
use serde::{Deserialize, Serialize};
use std::f32::consts::{self, PI, FRAC_2_PI};
use nih_plug::{params::enums::Enum};

#[derive(Enum, PartialEq, Eq, Debug, Copy, Clone, Serialize, Deserialize)]
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

#[derive(Enum, PartialEq, Eq, Debug, Copy, Clone, Deserialize, Serialize)]
pub enum SmoothStyle {
    Linear,
    Logarithmic,
    Exponential,
}

#[derive(Enum, PartialEq, Eq, Debug, Copy, Clone, Deserialize, Serialize)]
pub enum RetriggerStyle {
    Free,
    Retrigger,
    Random,
    UniRandom
}

// Super useful function to scale an input 0-1 into other ranges
pub(crate) fn scale_range(input: f32, min_output: f32, max_output: f32) -> f32 {
    let scaled = input * (max_output - min_output) + min_output;
    scaled.clamp(min_output, max_output)
}

// Sine wave oscillator modded with some sort of saw wave multiplication
pub fn calculate_sine(mod_amount: f32, phase: f32) -> f32 {
    // f(x) = sin(x * tau) {0 < x < 1}
    let scaled_phase = scale_range(phase, -1.0, 1.0);
    let tau = consts::TAU;
    let sine: f32;

    match mod_amount {
        mod_amount if mod_amount <= 0.33 => {
            sine = (phase * tau).sin();
        }
        mod_amount if mod_amount < 0.67 => {
            // X^2 Approximation
            let x = 2.0 * scaled_phase;
            sine = if x < 0.0 {
                ((x + 1.0).powi(2) - 1.0) * 0.99
            } else {
                (-(x - 1.0).powi(2) + 1.0) * 0.99
            };
        }
        _ => {
            // Allegedly other efficient approximation
            sine = (24.5 * scaled_phase / tau) - (24.5 * scaled_phase * scaled_phase.abs() / tau);
        }
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
    let half = mod_to_bool >= 0.5;
    let scaled_phase = if half {
        phase
    } else {
        scale_range(phase, -1.0, 1.0)
    };
    
    // f(x) = x mod period
    scaled_phase % consts::TAU
}


// Ramp Wave with half rectification in modifier
pub fn calculate_ramp(mod_to_bool: f32, phase: f32) -> f32 {
    let half = mod_to_bool >= 0.5;
    let scaled_phase = if half {
        phase
    } else {
        scale_range(phase, -1.0, 1.0)
    };

    // f(x) = -x mod period
    -scaled_phase % consts::TAU
}

// Inward Curved Saw Wave
pub fn calculate_inward_saw(curve_amount: f32, phase: f32) -> f32 {
    let mut calc_curve_amount: i32 = scale_range(curve_amount, 1.0, 4.99).floor() as i32;
    
    // Direct mappings of curve_amount
    match calc_curve_amount {
        1 => calc_curve_amount = 2,
        2 => calc_curve_amount = 10,
        3 => calc_curve_amount = 3,
        4 => calc_curve_amount = 11,
        // Unreachable
        _ => calc_curve_amount = 1,
    }

    let scaled_phase = scale_range(phase, -1.0, 1.0);

    // Calculate the inward curved saw wave directly
    let result = if scaled_phase <= 0.0 {
        (scaled_phase + 1.0).powi(calc_curve_amount)
    } else {
        -(scaled_phase - 1.0).powi(calc_curve_amount)
    };

    result
}

pub fn calculate_square(mod_amount: f32, phase: f32) -> f32 {
    let mod_scaled: f32 = scale_range(1.0 - mod_amount, 0.0625, 0.5);
    // Hard cut function scaling to a pulse with mod
    if phase >= mod_scaled {
        -1.0
    } else {
        1.0
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
