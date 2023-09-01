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
This is intended to be a building block used by the main lib.rs of the Actuate synth.
It leverages the smoothing functions built into the nih_plug crate for attack and release :)

#####################################
*/

use std::f32::consts::{self, PI, FRAC_2_PI};
use rand::Rng;
use nih_plug::{params::enums::Enum, prelude::{Smoother, SmoothingStyle}};

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

#[derive(Clone)]
pub struct Oscillator {
    // Sample rate is used to calculate the frequency of the wave
    pub sample_rate: f32,
    // Enum above that has different wave types
    pub osc_type: VoiceType,
    // Enum above that has Osc lifetime state
    pub osc_state: OscState,
    // Attack and release params stored here
    pub osc_attack: Smoother<f32>,
    pub osc_release: Smoother<f32>,
    pub prev_attack: f32,
    pub prev_release: f32,
    // Smoothing curves for attack and release
    pub attack_smoothing: SmoothStyle,
    pub prev_attack_smoothing: SmoothStyle,
    pub release_smoothing: SmoothStyle,
    pub prev_release_smoothing: SmoothStyle,
    // Mod amount is something I added since the math stuff is fun/interesting
    pub osc_mod_amount: f32,
    // This is used to have a "free" phase based off the previous note when lib.rs has retrigger disabled
    pub prev_note_phase_delta: f32,
    // This tracks the phase of our waveform(s)
    pub phase: f32,
}

impl Oscillator {
    // This updates our attack and release if needed - These are called on midi events from lib.rs
    pub fn check_update_attack(&mut self, new_attack: f32, new_smoothing: SmoothStyle) {
        // Restrict this update to non-note changes to fix curve change panics
        let mut update_assign: bool = false;
        if self.prev_attack_smoothing != new_smoothing {
            self.prev_attack_smoothing = new_smoothing;
            update_assign = true;
        }
        if self.prev_attack != new_attack {
            self.prev_attack = new_attack;
            update_assign = true;
        }
        if update_assign {
            // Reassign in struct
            self.osc_attack = match self.prev_attack_smoothing {
                SmoothStyle::Exponential => Smoother::new(SmoothingStyle::Exponential(new_attack)),
                SmoothStyle::Linear => Smoother::new(SmoothingStyle::Linear(new_attack)),
                SmoothStyle::Logarithmic => Smoother::new(SmoothingStyle::Logarithmic(new_attack.clamp(0.1, 999.9))),
            } 
        }
    }
    pub fn check_update_release(&mut self, new_release: f32, new_smoothing: SmoothStyle) {
        // Restrict this update to non-note changes to fix curve change panics
        let mut update_assign: bool = false;
        if self.prev_release_smoothing != new_smoothing {
            self.prev_release_smoothing = new_smoothing;
            update_assign = true;
        }
        if self.prev_release != new_release {
            self.prev_release = new_release;
            update_assign = true;
        }
        if update_assign {
            // Reassign in struct
            self.osc_release = match self.prev_release_smoothing {
                SmoothStyle::Exponential => Smoother::new(SmoothingStyle::Exponential(new_release)),
                SmoothStyle::Linear => Smoother::new(SmoothingStyle::Linear(new_release)),
                SmoothStyle::Logarithmic => Smoother::new(SmoothingStyle::Logarithmic(new_release.clamp(0.1, 999.9))),
            } 
        }
    }
    pub fn check_update_sample_rate(&mut self, sample_rate_if_changed: f32) {
        if sample_rate_if_changed != self.sample_rate {
            self.sample_rate = sample_rate_if_changed;
        }
    }

    // Reset our wave phase - used for retrigger
    pub fn reset_phase(&mut self) {
        self.phase = 0.0;
    }

    // Random phase reset!
    pub fn set_random_phase(&mut self) {
        let mut rng = rand::thread_rng();
        let m: f32 = rng.gen_range(0.0..1.0);
        self.phase = m;
    }

    // Increment phase - used in non retriggered oscs
    pub fn increment_phase(&mut self) {
        self.phase += self.prev_note_phase_delta;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }
    }

    // Reset Smoothing functions
    pub fn reset_attack_smoother(&mut self, reset_to: f32) {
        self.osc_attack.reset(reset_to);
    }
    pub fn reset_release_smoother(&mut self, reset_to: f32) {
        self.osc_release.reset(reset_to);
    }

    // Update our smoothers for attack and release and optionally sample rate if something has changed
    pub fn set_attack_target(&mut self, sample_rate_if_changed: f32, new_attack_target: f32) {
        self.check_update_sample_rate(sample_rate_if_changed);
        self.osc_attack.set_target(self.sample_rate, new_attack_target);
    }
    pub fn set_release_target(&mut self, sample_rate_if_changed: f32, new_release_target: f32) {
        self.check_update_sample_rate(sample_rate_if_changed);
        self.osc_release.set_target(self.sample_rate, new_release_target);
    }

    // Return our attack or release Smoothers for the main lib use
    pub fn get_attack_smoother(&mut self) -> Smoother<f32> {
        return self.osc_attack.clone();
    }
    pub fn get_release_smoother(&mut self) -> Smoother<f32> {
        return self.osc_release.clone();
    }

    /*
    get/set osc state - lib.rs uses this for ADSR:
        Off,
        Attacking,
        Decaying,
        Sustaining,
        Releasing,
    */
    pub fn set_osc_state(&mut self, new_state: OscState) {
        self.osc_state = new_state;
    }
    pub fn get_osc_state(&mut self) -> OscState {
        self.osc_state
    }

    // Super useful function to scale an input 0-1 into other ranges
    fn scale_range(input: f32, min_output: f32, max_output: f32) -> f32 {
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
    pub fn calculate_sine(&mut self, frequency: f32, mod_amount: f32) -> f32 {
        let phase_delta = frequency / self.sample_rate;
        self.prev_note_phase_delta = phase_delta;

        // f(x) = sin(x * tau) {0 < x < 1}
        let mut sine: f32 = 0.0;
        let scaled_phase = Oscillator::scale_range(self.phase, -1.0, 1.0);

        if mod_amount <= 0.33 {
            sine = (self.phase * consts::TAU).sin();
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

        self.phase += phase_delta;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }

        sine
    }

    // Rounded Saw Wave with rounding amount
    pub fn calculate_rsaw(&mut self, frequency: f32, rounding: f32) -> f32 {
        let rounding_amount: i32 = Self::scale_range(rounding, 2.0, 30.0).floor() as i32;
        let phase_delta = frequency / self.sample_rate;
        self.prev_note_phase_delta = phase_delta;
        let scaled_phase = Self::scale_range(self.phase, -1.0, 1.0);

        // n = rounding int
        // f(x) = x * (1 − x^(2n))
        let rsaw: f32 = scaled_phase * (1.0 - scaled_phase.powi(2 * rounding_amount));

        self.phase += phase_delta;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }

        rsaw
    }

    // Saw Wave with half rectification in modifier
    pub fn calculate_saw(&mut self, frequency: f32, mod_to_bool: f32) -> f32 {
        let half: bool;
        if mod_to_bool < 0.5 {
            half = false;
        }
        else {
            half = true;
        }
        let phase_delta = frequency / self.sample_rate;
        self.prev_note_phase_delta = phase_delta;

        let scaled_phase = if half {
            self.phase
        } else { 
            Self::scale_range(self.phase, -1.0, 1.0) 
        };

        // f(x) = x mod period
        let saw: f32 = scaled_phase % consts::TAU;

        self.phase += phase_delta;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }

        saw
    }

    // Ramp Wave with half rectification in modifier
    pub fn calculate_ramp(&mut self, frequency: f32, mod_to_bool: f32) -> f32 {
        let half: bool;
        if mod_to_bool < 0.5 {
            half = false;
        }
        else {
            half = true;
        }
        let phase_delta: f32 = frequency / self.sample_rate;
        self.prev_note_phase_delta = phase_delta;

        let scaled_phase: f32 = if half {
            self.phase
        } else { 
            Self::scale_range(self.phase, -1.0, 1.0) 
        };

        // f(x) = x mod period
        let saw = -1.0 * (scaled_phase % consts::TAU);

        self.phase += phase_delta;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }

        saw
    }

    // Inward Curved Saw Wave
    pub fn calculate_inward_saw(&mut self, frequency: f32, curve_amount: f32) -> f32 {
        // This makes more sense to the user even though it's a little weird to modify it like this
        let mut calc_curve_amount: i32 = Self::scale_range(curve_amount, 1.0, 4.99).floor() as i32;
        match calc_curve_amount {
            1 => calc_curve_amount = 2,
            2 => calc_curve_amount = 10,
            3 => calc_curve_amount = 3,
            4 => calc_curve_amount = 11,
            // Unreachable
            _ => calc_curve_amount = 1,
        }

        let phase_delta: f32 = frequency / self.sample_rate;
        self.prev_note_phase_delta = phase_delta;
        let scaled_phase: f32 = Self::scale_range(self.phase, -1.0, 1.0);

        // f(x) = (x + 1)^6 {-1 <= x <= 0}
        // f(x) = -(x-1)^6 {0 <= x <= 1}
        let saw: f32 = if scaled_phase <= 0.0 {
            (scaled_phase + 1.0).powi(calc_curve_amount)
        } else {
            -(scaled_phase - 1.0).powi(calc_curve_amount)
        };

        self.phase += phase_delta;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }

        saw
    }

    pub fn calculate_square(&mut self, frequency: f32, mod_amount: f32) -> f32 {
        let phase_delta: f32 = frequency / self.sample_rate;
        self.prev_note_phase_delta = phase_delta;
        let mod_scaled: f32 = Oscillator::scale_range(1.0 - mod_amount, 0.0625, 0.5);
        
        let mut square: f32 = 1.0;

        // Hard cut function scaling to a pulse with mod
        if self.phase >= mod_scaled {
            square *= -1.0;
        }

        self.phase += phase_delta;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }

        square
    }

    pub fn calculate_rounded_square(&mut self, frequency: f32, mod_amount: f32) -> f32 {
        let phase_delta: f32 = frequency / self.sample_rate;
        self.prev_note_phase_delta = phase_delta;
        let scaled_phase: f32 = Self::scale_range(self.phase, -1.0, 1.0);
        let mod_scaled: i32 = Oscillator::scale_range(mod_amount, 2.0, 8.0).floor() as i32 * 2;
        
        let rounded_square: f32;

        // Rounding function is approximated with these exponential functions
        if scaled_phase <  0.0 {
            rounded_square = (2.0 * scaled_phase + 1.0).powi(mod_scaled) - 1.0;
        } else {
            rounded_square = -(2.0 * scaled_phase - 1.0).powi(mod_scaled) + 1.0;
        }

        self.phase += phase_delta;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }

        rounded_square
    }

    pub fn calculate_tri(&mut self, frequency: f32, mod_amount: f32) -> f32 {
        let phase_delta: f32 = frequency / self.sample_rate;
        self.prev_note_phase_delta = phase_delta;
        let tri: f32 = (FRAC_2_PI) * (((2.0 * PI) * self.phase).sin()).asin();
        let mut tan_tri: f32 = 0.0;

        // Mix in 
        if mod_amount >  0.0 {
            tan_tri = ((self.phase * PI).sin()).tan()/(consts::FRAC_PI_2);
        }

        self.phase += phase_delta;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }

        // Use mod to fade between tri and weird tan tri
        tri*(1.0 - mod_amount) + tan_tri*mod_amount
    }
}