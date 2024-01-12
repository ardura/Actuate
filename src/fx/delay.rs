// This is a tapped delay line delay meant to be simple for Actuate!
// Stock synth delays are pretty ok :)
// Ardura 2023

use nih_plug::params::enums::Enum;
use serde::{Deserialize, Serialize};

#[derive(Clone, Enum, PartialEq, Serialize, Deserialize)]
pub enum DelaySnapValues {
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

#[derive(Clone, Enum, PartialEq, Serialize, Deserialize)]
pub enum DelayType {
    Stereo,
    PingPongL,
    PingPongR,
}

#[derive(Clone)]
pub(crate) struct Delay {
    sample_rate: f32,
    bpm: f32,
    length: DelaySnapValues,
    delay_buffer_l: Vec<f32>,
    delay_buffer_r: Vec<f32>,
    delay_length: usize,
    delay_type: DelayType,
    feedback: f32,
    current_index: usize,
    delay_chunk_flip: bool,
}

impl Delay {
    pub fn new(sample_rate: f32, bpm: f32, length: DelaySnapValues, feedback: f32) -> Self {
        // Recalculate delay length based on the new size
        let divisor: f32 = match length {
            DelaySnapValues::Whole => 1.0,
            DelaySnapValues::WholeD => 1.0 * 1.5,
            DelaySnapValues::WholeT => 1.0 / 3.0,
            DelaySnapValues::Half => 2.0,
            DelaySnapValues::HalfD => 2.0 * 1.5,
            DelaySnapValues::HalfT => 2.0 / 3.0,
            DelaySnapValues::Quarter => 4.0,
            DelaySnapValues::QuarterD => 4.0 * 1.5,
            DelaySnapValues::QuarterT => 4.0 / 3.0,
            DelaySnapValues::Eighth => 8.0,
            DelaySnapValues::EighthD => 8.0 * 1.5,
            DelaySnapValues::EighthT => 8.0 / 3.0,
            DelaySnapValues::Sixteen => 16.0,
            DelaySnapValues::SixteenD => 16.0 * 1.5,
            DelaySnapValues::SixteenT => 16.0 / 3.0,
            DelaySnapValues::ThirtySecond => 32.0,
            DelaySnapValues::ThirtySecondD => 32.0 * 1.5,
            DelaySnapValues::ThirtySecondT => 32.0 / 3.0,
        };

        // Calculate beats per second
        let bps = bpm / 60.0;

        // Calculate samples per beat
        let samples_per_beat = sample_rate / bps;

        // Calculate samples per note type
        let samples_per_note_type = samples_per_beat * (4.0 / divisor);
        let delay_length = samples_per_note_type as usize;

        // Create delay buffers for left and right channels initialized with zeros
        let delay_buffer_l = vec![0.0; delay_length];
        let delay_buffer_r = vec![0.0; delay_length];

        Delay {
            sample_rate,
            bpm: 138.0,
            length,
            delay_buffer_l,
            delay_buffer_r,
            delay_length,
            delay_type: DelayType::Stereo,
            feedback,
            current_index: 0,
            delay_chunk_flip: true,
        }
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32, bpm: f32) {
        if self.bpm != bpm {
            self.bpm = bpm;
        }
        if self.sample_rate != sample_rate {
            self.sample_rate = sample_rate;

            // Recalculate delay length based on the new sample rate
            let length =
                self.calculate_samples_per_note_type(Self::get_divisor(self.length.clone()));

            // Recalculate delay length based on the new size
            self.delay_length = length as usize;

            // Resize and reset the delay buffers
            self.delay_buffer_l = vec![0.0; self.delay_length];
            self.delay_buffer_r = vec![0.0; self.delay_length];
            self.current_index = 0;
        }
    }

    fn calculate_samples_per_note_type(&mut self, note_type_value: f32) -> f32 {
        // Calculate beats per second
        let bps = self.bpm / 60.0;

        // Calculate samples per beat
        let samples_per_beat = self.sample_rate / bps;

        // Calculate samples per note type
        let samples_per_note_type = samples_per_beat * (4.0 / note_type_value);

        samples_per_note_type
    }

    fn get_divisor(length: DelaySnapValues) -> f32 {
        let divisor: f32 = match length {
            DelaySnapValues::Whole => 1.0,
            DelaySnapValues::WholeD => 1.0 * 1.5,
            DelaySnapValues::WholeT => 1.0 / 3.0,
            DelaySnapValues::Half => 2.0,
            DelaySnapValues::HalfD => 2.0 * 1.5,
            DelaySnapValues::HalfT => 2.0 / 3.0,
            DelaySnapValues::Quarter => 4.0,
            DelaySnapValues::QuarterD => 4.0 * 1.5,
            DelaySnapValues::QuarterT => 4.0 / 3.0,
            DelaySnapValues::Eighth => 8.0,
            DelaySnapValues::EighthD => 8.0 * 1.5,
            DelaySnapValues::EighthT => 8.0 / 3.0,
            DelaySnapValues::Sixteen => 16.0,
            DelaySnapValues::SixteenD => 16.0 * 1.5,
            DelaySnapValues::SixteenT => 16.0 / 3.0,
            DelaySnapValues::ThirtySecond => 32.0,
            DelaySnapValues::ThirtySecondD => 32.0 * 1.5,
            DelaySnapValues::ThirtySecondT => 32.0 / 3.0,
        };
        divisor
    }

    pub fn set_length(&mut self, length: DelaySnapValues) {
        if self.length != length {
            let new_length =
                self.calculate_samples_per_note_type(Self::get_divisor(self.length.clone()));

            // Recalculate delay length based on the new size
            self.delay_length = new_length as usize;

            // Resize and reset the delay buffers
            self.delay_buffer_l = vec![0.0; self.delay_length];
            self.delay_buffer_r = vec![0.0; self.delay_length];
            self.current_index = 0;

            //Reassign
            self.length = length;
        }
    }

    pub fn set_type(&mut self, delay_type: DelayType) {
        self.delay_type = delay_type;
    }

    pub fn set_feedback(&mut self, feedback: f32) {
        self.feedback = feedback;
    }

    pub fn process(&mut self, input_l: f32, input_r: f32, amount: f32) -> (f32, f32) {
        // Get the current values from the delay lines
        let delayed_sample_l: f32 = self.delay_buffer_l[self.current_index];
        let delayed_sample_r: f32 = self.delay_buffer_r[self.current_index];

        // Calculate the left and right outputs
        let mut output_l: f32;
        let mut output_r: f32;
        match self.delay_type {
            DelayType::Stereo => {
                output_l = input_l + self.feedback * delayed_sample_l;
                output_r = input_r + self.feedback * delayed_sample_r;
            }
            DelayType::PingPongL => {
                if self.delay_chunk_flip {
                    output_l = input_l + self.feedback * delayed_sample_l;
                    output_r = input_r;
                } else {
                    output_l = input_l;
                    output_r = input_r + self.feedback * delayed_sample_r;
                }
            }
            DelayType::PingPongR => {
                if self.delay_chunk_flip {
                    output_l = input_l;
                    output_r = input_r + self.feedback * delayed_sample_r;
                } else {
                    output_l = input_l + self.feedback * delayed_sample_l;
                    output_r = input_r;
                }
            }
        }

        // Store the outputs in the delay lines
        self.delay_buffer_l[self.current_index] = output_l;
        self.delay_buffer_r[self.current_index] = output_r;

        // Move the index to the next position in the delay lines
        self.current_index = (self.current_index + 1) % self.delay_length;
        if self.current_index == 0 {
            self.delay_chunk_flip = !self.delay_chunk_flip;
        }

        // Return the left and right outputs
        output_l = input_l * (1.0 - amount) + output_l * amount;
        output_r = input_r * (1.0 - amount) + output_r * amount;
        (output_l, output_r)
    }
}
