// Simpler saturations for Actuate
// Based off the Duro Console Saturations
// Ardura 2023

use std::f32::consts::PI;
use nih_plug::params::enums::Enum;
use serde::{Serialize, Deserialize};

#[derive(Clone, Enum, PartialEq, Serialize, Deserialize)]
pub enum SaturationType {
    Tape,
    Clip,
    SinPow,
    CosQ,
}

#[derive(Clone, PartialEq)]
pub(crate) struct Saturation {
    sat_type: SaturationType,
}

impl Saturation {
    pub fn new() -> Self {
        Saturation {
            sat_type: SaturationType::Tape,
        }
    }

    pub fn set_type(&mut self, new_type: SaturationType) {
        self.sat_type = new_type;
    }

    // Process our saturations - amount from 0 to 1
    pub fn process(&mut self, input_l: f32, input_r: f32, amount: f32) -> (f32,f32) {
        let output_l: f32;
        let output_r: f32;
        let idrive = if amount == 0.0 { 0.0001 } else { amount };
        match self.sat_type {
            SaturationType::Tape => {
                // Define the transfer curve for the tape saturation effect
                // 1.0 addition and powf were added to make it more pronounced
                let transfer = |x: f32| -> f32 {
                    (x * (10.0 * idrive + 1.0)).tanh()
                };
                // Apply the transfer curve to the input sample
                output_l = transfer(input_l);
                output_r = transfer(input_r);
            },
            SaturationType::Clip => {
                let clipped = input_l.signum();
                // Mix clipped signal with original
                output_l = input_l * (1.0 - amount) + clipped * amount;
                output_r = input_r * (1.0 - amount) + clipped * amount;
            },
            SaturationType::SinPow => {
                let transfer = |x: f32| -> f32 {
                    (x * (idrive)).sin().powf(2.0)
                };
            
                output_l = transfer(input_l);
                output_r = transfer(input_r);
            },
            SaturationType::CosQ => {
                let transfer = |x: f32| -> f32 {
                    ((idrive * (idrive * PI * x).cos()) / 4.0) + x
                };

                output_l = transfer(input_l);
                output_r = transfer(input_r);
            }
        }
        
        (output_l, output_r)
    }
}
