use nih_plug::params::enums::Enum;
use serde::{Deserialize, Serialize};
use std::f32::consts::PI;

// Inspired by https://www.musicdsp.org/en/latest/Filters/267-simple-tilt-equalizer.html
// Lowpass, Bandpass, Highpass based off tilt filter code
// Ardura

const SLOPE_NEG: f32 = -60.0;

#[derive(Enum, PartialEq, Serialize, Deserialize, Clone)]
pub enum ResponseType {
    Lowpass,
    Bandpass,
    Highpass,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ArduraFilter {
    // Filter parameters
    sample_rate: f32,
    center_freq: f32,
    steepness: f32,
    shape: ResponseType,

    // Filter tracking/internal
    sample_rate_x3: f32,
    lgain: f32,
    hgain: f32,
    a0: f32,
    b1: f32,
    lp_out: f32,
    // Band pass separate vars
    band_a0_low: f32,
    band_b1_low: f32,
    band_out_low: f32,
    band_a0_high: f32,
    band_b1_high: f32,
    band_out_high: f32,
}

impl ArduraFilter {
    pub fn new(sample_rate: f32, center_freq: f32, steepness: f32, shape: ResponseType) -> Self {
        let amp = 6.0 / f32::ln(2.0);
        let sample_rate_x3 = 3.0 * sample_rate;
        let lgain;
        let hgain;
        match shape {
            // These are the gains for the slopes when math happens later
            ResponseType::Lowpass => {
                lgain = f32::exp(0.0 / amp) - 1.0;
                hgain = f32::exp(SLOPE_NEG / amp) - 1.0;
            }
            ResponseType::Bandpass => {
                lgain = f32::exp(0.0 / amp) - 1.0;
                hgain = f32::exp(SLOPE_NEG / amp) - 1.0;
            }
            ResponseType::Highpass => {
                lgain = f32::exp(SLOPE_NEG / amp) - 1.0;
                hgain = f32::exp(0.0 / amp) - 1.0;
            }
        }

        let omega = 2.0 * PI * center_freq;
        let n = 1.0 / (Self::scale_range(steepness, 0.98, 1.2) * (sample_rate_x3 + omega));
        let a0 = 2.0 * omega * n;
        let b1 = (sample_rate_x3 - omega) * n;
        let lp_out = 0.0; // Initial value for lp_out

        ArduraFilter {
            center_freq,
            sample_rate_x3,
            lgain,
            hgain,
            a0,
            b1,
            lp_out,
            steepness,
            sample_rate: sample_rate,
            shape,
            band_a0_low: a0,
            band_b1_low: b1,
            band_out_low: lp_out,
            band_a0_high: a0,
            band_b1_high: b1,
            band_out_high: lp_out,
        }
    }

    pub fn update(
        &mut self,
        sample_rate: f32,
        center_freq: f32,
        steepness: f32,
        shape: ResponseType,
    ) {
        let mut recalculate = false;
        if self.sample_rate != sample_rate {
            self.sample_rate = sample_rate;
            self.sample_rate_x3 = self.sample_rate * 3.0;
            recalculate = true;
        }
        if self.center_freq != center_freq {
            self.center_freq = center_freq;
            recalculate = true;
        }
        if self.steepness != steepness {
            self.steepness = steepness;
            recalculate = true;
        }
        if self.shape != shape {
            self.shape = shape;
            recalculate = true;
        }
        if recalculate {
            let amp = 6.0 / f32::ln(2.0);
            match self.shape {
                ResponseType::Lowpass => {
                    let omega = 2.0 * PI * center_freq;
                    let n = 1.0
                        / (Self::scale_range(self.steepness, 0.98, 1.2)
                            * (self.sample_rate_x3 + omega));
                    self.b1 = (self.sample_rate_x3 - omega) * n;
                    self.lgain = f32::exp(0.0 / amp) - 1.0;
                    self.hgain = f32::exp(SLOPE_NEG / amp) - 1.0;
                }
                ResponseType::Bandpass => {
                    let width = self.steepness * self.steepness * 500.0;
                    let l_omega = 2.0 * PI * (self.center_freq - width).clamp(20.0, 16000.0);
                    let l_n = 1.0
                        / (Self::scale_range(self.steepness, 0.98, 1.2)
                            * (self.sample_rate_x3 + l_omega));
                    self.band_a0_low = 2.0 * l_omega * l_n;
                    self.band_b1_low = (self.sample_rate_x3 - l_omega) * l_n;

                    let h_omega = 2.0 * PI * (self.center_freq + width).clamp(20.0, 16000.0);
                    let h_n = 1.0
                        / (Self::scale_range(self.steepness, 0.98, 1.2)
                            * (self.sample_rate_x3 + h_omega));
                    self.band_a0_high = 2.0 * h_omega * h_n;
                    self.band_b1_high = (self.sample_rate_x3 - h_omega) * h_n;

                    self.lgain = f32::exp(0.0 / amp) - 1.0;
                    self.hgain = f32::exp(SLOPE_NEG / amp) - 1.0;
                }
                ResponseType::Highpass => {
                    let omega = 2.0 * PI * center_freq;
                    let n = 1.0
                        / (Self::scale_range(self.steepness, 0.98, 1.2)
                            * (self.sample_rate_x3 + omega));
                    self.a0 = 2.0 * omega * n;
                    self.b1 = (self.sample_rate_x3 - omega) * n;
                    self.lgain = f32::exp(SLOPE_NEG / amp) - 1.0;
                    self.hgain = f32::exp(0.0 / amp) - 1.0;
                }
            }
        }
    }

    pub fn process(&mut self, input: f32) -> f32 {
        let denorm = 1.0 / 4294967295.0;
        // Process the input using the tilt equalizer logic
        if self.shape == ResponseType::Bandpass {
            self.band_out_low = self.band_a0_low * input + self.band_b1_low * self.band_out_low;
            let temp =
                input + self.hgain * self.band_out_low + self.lgain * (input - self.band_out_low);

            self.band_out_high = self.band_a0_high * temp + self.band_b1_high * self.band_out_high;
            temp + self.lgain * self.band_out_high
                + self.hgain * (temp - self.band_out_high)
                + denorm
        } else {
            self.lp_out = self.a0 * input + self.b1 * self.lp_out;
            input + self.lgain * self.lp_out + self.hgain * (input - self.lp_out) + denorm
        }
    }

    // Super useful function to scale an input 0-1 into other ranges
    fn scale_range(input: f32, min_output: f32, max_output: f32) -> f32 {
        let scaled = input * (max_output - min_output) + min_output;
        scaled.clamp(min_output, max_output)
    }
}
