// Biquad filter structures rewritten from RBJ's Audio EQ Cookbook
// I wanted to rewrite it myself to understand it better and make things clearer
// Adapted to rust by Ardura

// I'm not using everything here in this Actuate version
#![allow(dead_code)]

use nih_plug::params::enums::Enum;

// This is for my sanity
const LEFT: usize = 0;
const RIGHT: usize = 1;

// These are the filter types implemented
#[derive(Clone, Copy, Enum, PartialEq)]
pub(crate) enum FilterType {
    Off,
    LowPass,
    HighPass,
    BandPass,
    Notch,
    Peak,
    LowShelf,
    HighShelf,
}

// I wanted these separate from the main struct for readability
#[derive(Clone, Copy)]
struct BiquadCoefficients {
    b0: f32,
    b1: f32,
    b2: f32,
    a0: f32,
    a1: f32,
    a2: f32,
}

// This assigns our coefficients when passed the intermediate variables
// Nothing to mention here, RBJ has done all the work
impl BiquadCoefficients {
    pub fn new(biquad_type: FilterType, alpha: f32, omega: f32, peak_gain: f32) -> Self {
        let b0: f32;
        let b1: f32;
        let b2: f32;
        let a0: f32;
        let a1: f32;
        let a2: f32;
        let cos_omega = omega.cos();
        let sin_omega = omega.sin();
        match biquad_type {
            FilterType::Off => {
                b0 = 0.0;
                b1 = 0.0;
                b2 = 0.0;
                a0 = 0.0;
                a1 = 0.0;
                a2 = 0.0;
            }
            FilterType::LowPass => {
                b0 = (1.0 - cos_omega) / 2.0;
                b1 = 1.0 - cos_omega;
                b2 = (1.0 - cos_omega) / 2.0;
                a0 = 1.0 + alpha;
                a1 = -2.0 * cos_omega;
                a2 = 1.0 - alpha;
            }
            FilterType::HighPass => {
                b0 = (1.0 + cos_omega) / 2.0;
                b1 = -(1.0 + cos_omega);
                b2 = (1.0 + cos_omega) / 2.0;
                a0 = 1.0 + alpha;
                a1 = -2.0 * cos_omega;
                a2 = 1.0 - alpha;
            }
            FilterType::BandPass => {
                b0 = sin_omega / 2.0;
                b1 = 0.0;
                b2 = -sin_omega / 2.0;
                a0 = 1.0 + alpha;
                a1 = -2.0 * cos_omega;
                a2 = 1.0 - alpha;
            }
            FilterType::Notch => {
                b0 = 1.0;
                b1 = -2.0 * cos_omega;
                b2 = 1.0;
                a0 = 1.0 + alpha;
                a1 = -2.0 * cos_omega;
                a2 = 1.0 - alpha;
            }
            FilterType::Peak => {
                let A = (10.0_f32.powf(peak_gain / 40.0)).sqrt();
                b0 = 1.0 + alpha * A;
                b1 = -2.0 * cos_omega;
                b2 = 1.0 - alpha * A;
                a0 = 1.0 + alpha / A;
                a1 = -2.0 * cos_omega;
                a2 = 1.0 - alpha / A;
            }
            FilterType::LowShelf => {
                let A = (10.0_f32.powf(peak_gain / 40.0)).sqrt();
                let sqrt_a_2_alpha = 2.0 * (A).sqrt() * alpha;
                b0 = A * ((A + 1.0) - (A - 1.0) * cos_omega + sqrt_a_2_alpha);
                b1 = 2.0 * A * ((A - 1.0) - (A + 1.0) * cos_omega);
                b2 = A * ((A + 1.0) - (A - 1.0) * cos_omega - sqrt_a_2_alpha);
                a0 = (A + 1.0) + (A - 1.0) * cos_omega + sqrt_a_2_alpha;
                a1 = -2.0 * ((A - 1.0) + (A + 1.0) * cos_omega);
                a2 = (A + 1.0) + (A - 1.0) * cos_omega - sqrt_a_2_alpha;
            }
            FilterType::HighShelf => {
                let A = (10.0_f32.powf(peak_gain / 40.0)).sqrt();
                let sqrt_a_2_alpha = 2.0 * (A).sqrt() * alpha;
                b0 = A * ((A + 1.0) + (A - 1.0) * cos_omega + sqrt_a_2_alpha);
                b1 = -2.0 * A * ((A - 1.0) + (A + 1.0) * cos_omega);
                b2 = A * ((A + 1.0) + (A - 1.0) * cos_omega - sqrt_a_2_alpha);
                a0 = (A + 1.0) - (A - 1.0) * cos_omega + sqrt_a_2_alpha;
                a1 = 2.0 * ((A - 1.0) - (A + 1.0) * cos_omega);
                a2 = (A + 1.0) - (A - 1.0) * cos_omega - sqrt_a_2_alpha;
            }
        }
        BiquadCoefficients {
            b0: b0,
            b1: b1,
            b2: b2,
            a0: a0,
            a1: a1,
            a2: a2,
        }
    }
}

// This is the main Biquad struct, once more trying to make things clearer
#[derive(Clone, Copy)]
pub(crate) struct Biquad {
    // Main controls for the filter
    biquad_type: FilterType,
    sample_rate: f32,
    center_freq: f32,
    gain_db: f32,
    q_factor: f32,
    // Tracks previous outputs
    input_history: [[f32; 2]; 2],
    output_history: [[f32; 2]; 2],
    // Coefficients
    coeffs: BiquadCoefficients,
}

// This is for interleaving biquad structs - Airwindows inspired
// 10 interleave max is just my decision
#[derive(Clone, Copy)]
pub(crate) struct InterleavedBiquad {
    interleaves: usize,
    current_index: usize,
    biquad_array: [Biquad; 10],
}

impl Biquad {
    pub fn new(
        sample_rate: f32,
        center_freq: f32,
        gain_db: f32,
        q_factor: f32,
        biquad_type: FilterType,
    ) -> Self {
        let omega = 2.0 * std::f32::consts::PI * center_freq / sample_rate;
        let alpha = (omega.sin()) / (2.0 * q_factor);

        Biquad {
            biquad_type: biquad_type,
            sample_rate,
            center_freq,
            gain_db,
            q_factor,
            input_history: [[0.0, 0.0]; 2],
            output_history: [[0.0, 0.0]; 2],
            coeffs: BiquadCoefficients::new(biquad_type, alpha, omega, gain_db),
        }
    }

    // This is meant to only recalculate when there's an actual update as this method runs often
    pub fn update(&mut self, sample_rate: f32, center_freq: f32, gain_db: f32, q_factor: f32) {
        let mut recalc = false;
        if self.sample_rate != sample_rate {
            self.sample_rate = sample_rate;
            recalc = true;
        }
        if self.center_freq != center_freq {
            self.center_freq = center_freq;
            recalc = true;
        }
        if self.gain_db != gain_db {
            self.gain_db = gain_db;
            recalc = true;
        }
        if self.q_factor != q_factor {
            self.q_factor = q_factor;
            recalc = true;
        }
        if recalc {
            // Calculate our intermediate variables from our new info and create new coefficients
            let omega = 2.0 * std::f32::consts::PI * center_freq / sample_rate;
            let alpha = (omega.sin()) / (2.0 * q_factor);
            self.coeffs = BiquadCoefficients::new(self.biquad_type, alpha, omega, self.gain_db);
        }
    }

    pub fn set_type(&mut self, biquad_type: FilterType) {
        if self.biquad_type != biquad_type {
            self.biquad_type = biquad_type;
            // Calculate our intermediate variables from our new info and create new coefficients
            let omega = 2.0 * std::f32::consts::PI * self.center_freq / self.sample_rate;
            let alpha = (omega.sin()) / (2.0 * self.q_factor);
            self.coeffs = BiquadCoefficients::new(self.biquad_type, alpha, omega, self.gain_db);
        }
    }

    // I'll handle the oversampling/ordering from the calling thread, I'm trying to K.I.S.S.
    pub fn process_sample(&mut self, input_l: f32, input_r: f32) -> (f32, f32) {
        if self.biquad_type == FilterType::Off {
            return (input_l, input_r);
        }
        // Using RBJ's Direct Form I straight from the cookbook
        let output_l;
        let output_r;
        // Calculate our current output for the left side
        output_l = (self.coeffs.b0 / self.coeffs.a0) * input_l
            + (self.coeffs.b1 / self.coeffs.a0) * self.input_history[0][LEFT]
            + (self.coeffs.b2 / self.coeffs.a0) * self.input_history[1][LEFT]
            - (self.coeffs.a1 / self.coeffs.a0) * self.output_history[0][LEFT]
            - (self.coeffs.a2 / self.coeffs.a0) * self.output_history[1][LEFT];
        // Reassign the history variables
        self.input_history[1][LEFT] = self.input_history[0][LEFT];
        self.input_history[0][LEFT] = input_l;
        self.output_history[1][LEFT] = self.output_history[0][LEFT];
        self.output_history[0][LEFT] = output_l;

        // Calculate our current output for the right side
        output_r = (self.coeffs.b0 / self.coeffs.a0) * input_r
            + (self.coeffs.b1 / self.coeffs.a0) * self.input_history[0][RIGHT]
            + (self.coeffs.b2 / self.coeffs.a0) * self.input_history[1][RIGHT]
            - (self.coeffs.a1 / self.coeffs.a0) * self.output_history[0][RIGHT]
            - (self.coeffs.a2 / self.coeffs.a0) * self.output_history[1][RIGHT];
        // Reassign the history variables
        self.input_history[1][RIGHT] = self.input_history[0][RIGHT];
        self.input_history[0][RIGHT] = input_r;
        self.output_history[1][RIGHT] = self.output_history[0][RIGHT];
        self.output_history[0][RIGHT] = output_r;

        (output_l, output_r)
    }
}

impl InterleavedBiquad {
    pub fn new(
        sample_rate: f32,
        center_freq: f32,
        gain_db: f32,
        q_factor: f32,
        biquad_type: FilterType,
        new_interleave: usize,
    ) -> Self {
        InterleavedBiquad {
            interleaves: new_interleave,
            current_index: 0,
            biquad_array: [Biquad::new(sample_rate, center_freq, gain_db, q_factor, biquad_type);
                10],
        }
    }

    pub fn update(&mut self, sample_rate: f32, center_freq: f32, gain_db: f32, q_factor: f32) {
        for biquad in self.biquad_array.iter_mut() {
            biquad.update(sample_rate, center_freq, gain_db, q_factor);
        }
    }

    pub fn set_type(&mut self, biquad_type: FilterType) {
        for biquad in self.biquad_array.iter_mut() {
            biquad.set_type(biquad_type);
        }
    }

    pub fn set_interleave(&mut self, new_interleave: usize) {
        self.interleaves = new_interleave.clamp(2, 10);
    }

    pub fn increment_index(&mut self) {
        // Increment our index
        self.current_index += 1;

        if self.current_index >= self.interleaves {
            self.current_index = 0;
        }
    }

    pub fn process_sample(&mut self, input_l: f32, input_r: f32) -> (f32, f32) {
        let output_l;
        let output_r;
        (output_l, output_r) =
            self.biquad_array[self.current_index].process_sample(input_l, input_r);

        // Return
        (output_l, output_r)
    }
}
