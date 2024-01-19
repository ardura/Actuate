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
use lazy_static::lazy_static;
use nih_plug::params::enums::Enum;
use rand::{rngs::StdRng, Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use std::f32::consts::{self, FRAC_2_PI, PI};

// Make a lookup table for faster but less accurate sine approx for additive
const TABLE_SIZE: usize = 512;
lazy_static! {
    // Generating waveform tables
    static ref SIN_TABLE: [f32; TABLE_SIZE] = {
        let mut table = [0.0; TABLE_SIZE];
        for i in 0..TABLE_SIZE {
            let phase = (i as f32 / TABLE_SIZE as f32) * std::f32::consts::PI * 2.0;
            table[i] = phase.sin();
        }
        table
    };
    static ref SAW_TABLE: [f32; TABLE_SIZE] = {
        let mut table = [0.0; TABLE_SIZE];
        for i in 0..TABLE_SIZE {
            let phase = i as f32 / (TABLE_SIZE - 1) as f32;  // Adjusted phase calculation
            // Calculate the sawtooth waveform directly
            table[i] = -1.0 + 2.0 * phase;
        }
        table
    };
    // These are for creating an "analog saw" by rotating at random between WSAW tables
    static ref WSAW_TABLE_1: [f32; TABLE_SIZE] = {
        let mut table = [0.0; TABLE_SIZE];
        let mut rng = StdRng::seed_from_u64(38);  // Use a fixed seed

        for i in 0..TABLE_SIZE {
            let phase = i as f32 / (TABLE_SIZE - 1) as f32;

            // Introduce deterministic randomness to the waveform
            let randomness = rng.gen_range(-0.1..0.1);  // Adjust the range of randomness
            table[i] = -1.0 + 2.0 * (phase + randomness);
        }
        table
    };
    static ref WSAW_TABLE_2: [f32; TABLE_SIZE] = {
        let mut table = [0.0; TABLE_SIZE];
        let mut rng = StdRng::seed_from_u64(57);  // Use a fixed seed

        for i in 0..TABLE_SIZE {
            let phase = i as f32 / (TABLE_SIZE - 1) as f32;

            // Introduce deterministic randomness to the waveform
            let randomness = rng.gen_range(-0.1..0.1);  // Adjust the range of randomness
            table[i] = -1.0 + 2.0 * (phase + randomness);
        }
        table
    };
    // These are for creating an "analog saw" by rotating at random between WSAW tables
    static ref SSAW_TABLE_1: [f32; TABLE_SIZE] = {
        let mut table = [0.0; TABLE_SIZE];
        let mut rng = StdRng::seed_from_u64(22);  // Use a fixed seed

        for i in 0..TABLE_SIZE {
            let phase = i as f32 / (TABLE_SIZE - 1) as f32;

            // Introduce deterministic randomness to the waveform
            let randomness = rng.gen_range(-0.01..0.01);  // Adjust the range of randomness
            table[i] = -1.0 + 2.0 * (phase + randomness);
        }
        table
    };
    static ref SSAW_TABLE_2: [f32; TABLE_SIZE] = {
        let mut table = [0.0; TABLE_SIZE];
        let mut rng = StdRng::seed_from_u64(17);  // Use a fixed seed

        for i in 0..TABLE_SIZE {
            let phase = i as f32 / (TABLE_SIZE - 1) as f32;

            // Introduce deterministic randomness to the waveform
            let randomness = rng.gen_range(-0.01..0.01);  // Adjust the range of randomness
            table[i] = -1.0 + 2.0 * (phase + randomness);
        }
        table
    };
    // Combine the SSAW technique with the RSAW function
    // "A for Analog"
    static ref ASAW_TABLE_1: [f32; TABLE_SIZE] = {
        let mut table = [0.0; TABLE_SIZE];
        let mut rng = StdRng::seed_from_u64(44);  // Use a fixed seed
        let rounding_amount: i32 = 30; // Adjust the rounding amount as needed

        for i in 0..TABLE_SIZE {
            let phase = i as f32 / (TABLE_SIZE - 1) as f32;
            // Introduce deterministic randomness to the waveform
            let randomness = rng.gen_range(-0.009..0.009);  // Adjust the range of randomness

            let scaled_phase = -1.0 + 2.0 * (phase + randomness);

            // Calculate the rounded sawtooth waveform directly
            table[i] = scaled_phase * (1.0 - scaled_phase.powi(2 * rounding_amount));
        }

        table
    };
    static ref ASAW_TABLE_2: [f32; TABLE_SIZE] = {
        let mut table = [0.0; TABLE_SIZE];
        let mut rng = StdRng::seed_from_u64(44);  // Use a fixed seed
        let rounding_amount: i32 = 30; // Adjust the rounding amount as needed

        for i in 0..TABLE_SIZE {
            let phase = i as f32 / (TABLE_SIZE - 1) as f32;
            // Introduce deterministic randomness to the waveform
            let randomness = rng.gen_range(-0.008..0.008);  // Adjust the range of randomness

            let scaled_phase = -1.0 + 2.0 * (phase + randomness);

            // Calculate the rounded sawtooth waveform directly
            table[i] = scaled_phase * (1.0 - scaled_phase.powi(2 * rounding_amount));
        }

        table
    };
    static ref ASAW_TABLE_3: [f32; TABLE_SIZE] = {
        let mut table = [0.0; TABLE_SIZE];
        let mut rng = StdRng::seed_from_u64(44);  // Use a fixed seed
        let rounding_amount: i32 = 32; // Adjust the rounding amount as needed

        for i in 0..TABLE_SIZE {
            let phase = i as f32 / (TABLE_SIZE - 1) as f32;
            // Introduce deterministic randomness to the waveform
            let randomness = rng.gen_range(-0.01..0.01);  // Adjust the range of randomness

            let scaled_phase = -1.0 + 2.0 * (phase + randomness);

            // Calculate the rounded sawtooth waveform directly
            table[i] = scaled_phase * (1.0 - scaled_phase.powi(2 * rounding_amount));
        }

        table
    };
    static ref RSAW_TABLE: [f32; TABLE_SIZE] = {
        let mut table = [0.0; TABLE_SIZE];
        let rounding_amount: i32 = 15; // Adjust the rounding amount as needed

        for i in 0..TABLE_SIZE {
            let phase = i as f32 / (TABLE_SIZE - 1) as f32;
            let scaled_phase = -1.0 + 2.0 * phase;

            // Calculate the rounded sawtooth waveform directly
            table[i] = scaled_phase * (1.0 - scaled_phase.powi(2 * rounding_amount));
        }

        table
    };
    static ref RAMP_TABLE: [f32; TABLE_SIZE] = {
        let mut table = [0.0; TABLE_SIZE];

        for i in 0..TABLE_SIZE {
            let phase = i as f32 / (TABLE_SIZE - 1) as f32;
            let scaled_phase = -1.0 + 2.0 * phase;

            // Calculate the ramp wave directly
            table[i] = -scaled_phase % consts::TAU;
        }

        table
    };
    static ref SQUARE_TABLE: [f32; TABLE_SIZE] = {
        let mut table = [0.0; TABLE_SIZE];

        for i in 0..TABLE_SIZE {
            let phase = i as f32 / (TABLE_SIZE - 1) as f32;

            // Calculate the square wave directly
            if phase < 0.5 {
                table[i] = 1.0;  // Positive phase half
            } else {
                table[i] = -1.0;  // Negative phase half
            }
        }

        table
    };
    static ref PULSE_TABLE: [f32; TABLE_SIZE] = {
        let mut table = [0.0; TABLE_SIZE];

        for i in 0..TABLE_SIZE {
            let phase = i as f32 / (TABLE_SIZE - 1) as f32;

            // Calculate the pulse wave directly
            if phase < 0.25 {
                table[i] = 1.0;  // Positive phase quarter
            } else {
                table[i] = -1.0;  // Negative phase three-quarters
            }
        }

        table
    };
    static ref RSQUARE_TABLE: [f32; TABLE_SIZE] = {
        let mut table = [0.0; TABLE_SIZE];
        let mod_amount: f32 = 0.15;
        let mod_scaled: i32 = scale_range(mod_amount, 2.0, 8.0).floor() as i32 * 2;

        for i in 0..TABLE_SIZE {
            let phase = i as f32 / (TABLE_SIZE - 1) as f32;
            let scaled_phase = -1.0 + 2.0 * phase;

            // Calculate the rounded square wave directly
            if scaled_phase < 0.0 {
                table[i] = (2.0 * scaled_phase + 1.0).powi(mod_scaled) - 1.0;
            } else {
                table[i] = -(2.0 * scaled_phase - 1.0).powi(mod_scaled) + 1.0;
            }
        }

        table
    };
    static ref TRI_TABLE: [f32; TABLE_SIZE] = {
        let mut table = [0.0; TABLE_SIZE];

        for i in 0..TABLE_SIZE {
            let phase = i as f32 / (TABLE_SIZE - 1) as f32;
            let tri = (FRAC_2_PI) * (((2.0 * PI) * phase).sin()).asin();

            // Store the calculated triangle wave value in the table
            table[i] = tri;
        }

        table
    };
}

#[derive(Enum, PartialEq, Eq, Debug, Copy, Clone, Serialize, Deserialize)]
pub enum VoiceType {
    Sine,
    Tri,
    Saw,
    RSaw,
    WSaw,
    SSaw,
    RASaw,
    Ramp,
    Square,
    RSquare,
    Pulse,
    Noise,
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
    LogSteep,
    Exponential,
}

#[derive(Enum, PartialEq, Eq, Debug, Copy, Clone, Deserialize, Serialize)]
pub enum RetriggerStyle {
    Free,
    Retrigger,
    Random,
    UniRandom,
}

// Super useful function to scale an input 0-1 into other ranges
pub(crate) fn scale_range(input: f32, min_output: f32, max_output: f32) -> f32 {
    let scaled = input * (max_output - min_output) + min_output;
    scaled.clamp(min_output, max_output)
}

// Sine wave oscillator with lerp smoothing
pub fn get_sine(phase: f32) -> f32 {
    let index = (phase * (TABLE_SIZE - 1) as f32) as usize;
    let frac = phase * (TABLE_SIZE - 1) as f32 - index as f32;
    let next_index = index + 1;

    let sine = if next_index < TABLE_SIZE - 1 {
        SIN_TABLE[index] * (1.0 - frac) + SIN_TABLE[next_index] * frac
    } else {
        SIN_TABLE[index] // If next_index is out of bounds, use the current index
    };
    sine
}

// Rounded Saw Wave with rounding amount
pub fn get_rsaw(phase: f32) -> f32 {
    let index = (phase * (TABLE_SIZE - 1) as f32) as usize;
    return RSAW_TABLE[index];
}

// Rounded Saw Wave with analog-ey modification
pub fn get_rasaw(phase: f32) -> f32 {
    let index = (phase * (TABLE_SIZE - 1) as f32) as usize;
    let mut rng = rand::thread_rng();
    let random_int: u32 = rng.gen_range(0..=2);
    // Based on our int, use the three seed-noise tables
    match random_int {
        0 => {
            return ASAW_TABLE_1[index];
        }
        1 => {
            return ASAW_TABLE_2[index];
        }
        2 => {
            return ASAW_TABLE_3[index];
        }
        _ => {
            return 0.0;
        }
    }
}

// Saw Wave
pub fn get_saw(phase: f32) -> f32 {
    let index = (phase * (TABLE_SIZE - 1) as f32) as usize;
    return SAW_TABLE[index];
}

// "Analog" inspired "whiter" Saw wave
pub fn get_wsaw(phase: f32) -> f32 {
    let index = (phase * (TABLE_SIZE - 1) as f32) as usize;
    let mut rng = rand::thread_rng();
    let random_bool: bool = rng.gen();
    // Based on our random bool, obtain the Saw waveforms with seed-introduced randomness waveform tables
    if random_bool {
        return WSAW_TABLE_1[index];
    } else {
        return WSAW_TABLE_2[index];
    }
}

// "Analog" inspired "subtle warm" Saw wave
pub fn get_ssaw(phase: f32) -> f32 {
    let index = (phase * (TABLE_SIZE - 1) as f32) as usize;
    let mut rng = rand::thread_rng();
    let random_bool: bool = rng.gen();
    // Based on our random bool, obtain the Saw waveforms with seed-introduced randomness waveform tables
    if random_bool {
        return SSAW_TABLE_1[index];
    } else {
        return SSAW_TABLE_2[index];
    }
}

// Ramp Wave
pub fn get_ramp(phase: f32) -> f32 {
    let index = (phase * (TABLE_SIZE - 1) as f32) as usize;
    return RAMP_TABLE[index];
}

// Square Wave
pub fn get_square(phase: f32) -> f32 {
    let index = (phase * (TABLE_SIZE - 1) as f32) as usize;
    return SQUARE_TABLE[index];
}

// 1/4 Pulse Wave
pub fn get_pulse(phase: f32) -> f32 {
    let index = (phase * (TABLE_SIZE - 1) as f32) as usize;
    return PULSE_TABLE[index];
}

pub fn get_rsquare(phase: f32) -> f32 {
    let index = (phase * (TABLE_SIZE - 1) as f32) as usize;
    return RSQUARE_TABLE[index];
}

pub fn get_tri(phase: f32) -> f32 {
    let index = (phase * (TABLE_SIZE - 1) as f32) as usize;
    return TRI_TABLE[index];
}

// Bard helped me out on this one
#[derive(Clone)]
pub struct DeterministicWhiteNoiseGenerator {
    seed: u64,
}

impl DeterministicWhiteNoiseGenerator {
    pub fn new(seed: u64) -> Self {
        // Magic number seed I made up to have same noise pattern every time
        DeterministicWhiteNoiseGenerator { seed }
    }

    pub fn generate_sample(&mut self) -> f32 {
        let random_value = self.xorshift();
        // Scale the random value to be between -1.0 and 1.0
        let sample = (random_value as f32 / u64::MAX as f32) * 2.0 - 1.0;
        sample
    }

    fn xorshift(&mut self) -> u64 {
        let mut x = self.seed;
        x ^= x << 21;
        x ^= x >> 35;
        x ^= x << 4;
        self.seed = x;
        x
    }
}
