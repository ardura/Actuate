// A chorus based off Airwindows ChorusEnsemble written by Ardura

use std::f32::consts::FRAC_PI_2;
use std::f32::consts::TAU;

const TOTAL_SAMPLES: usize = 16386;
const LOOP_LIMIT: usize = 8176;
const RANGE_MULT: f32 = 981.19368;

#[derive(Clone)]
pub struct ChorusEnsemble {
    // Inputs
    sample_rate: f32,
    range: f32,
    speed: f32,
    amount: f32,
    // Internals
    raw_speed: f32,
    raw_range: f32,
    sweep: f32,
    range2: f32,
    range3: f32,
    range4: f32,
    left_buffer: [f32; TOTAL_SAMPLES],
    right_buffer: [f32; TOTAL_SAMPLES],
    gcount: usize,
    airPrevL: f32,
	airEvenL: f32,
	airOddL: f32,
	airFactorL: f32,
	airPrevR: f32,
	airEvenR: f32,
	airOddR: f32,
	airFactorR: f32,
	fpFlip: bool,
}

impl ChorusEnsemble {
    pub fn new(
        sample_rate: f32,
        range: f32,
        speed: f32,
        amount: f32,
    ) -> Self {
        let mut scale = 1.0/44100.0;
        scale *= sample_rate;
        let calc_speed = (speed.powf(3.0) * 0.001)*scale;
        let calc_range = range.powf(3.0) * RANGE_MULT;
        let calc_range2 = calc_range * 2.0;
        let calc_range3 = calc_range * 3.0;
        let calc_range4 = calc_range * 4.0;
        Self {
            // Inputs
            sample_rate: sample_rate,
            range: calc_range,
            speed: calc_speed,
            amount: amount,
            // Internals
            raw_speed: speed,
            raw_range: range,
            sweep: FRAC_PI_2,
            range2: calc_range2,
            range3: calc_range3,
            range4: calc_range4,
            left_buffer: [0.0; TOTAL_SAMPLES],
            right_buffer: [0.0; TOTAL_SAMPLES],
            gcount: 0,
            airPrevL: 0.0,
	        airEvenL: 0.0,
	        airOddL: 0.0,
	        airFactorL: 0.0,
	        airPrevR: 0.0,
	        airEvenR: 0.0,
	        airOddR: 0.0,
	        airFactorR: 0.0,
	        fpFlip: false,
        }
    }

    pub fn update(&mut self, sample_rate: f32, range: f32, speed: f32, amount: f32) {
        if sample_rate != self.sample_rate {
            self.sample_rate = sample_rate;
            let mut scale = 1.0/44100.0;
            scale *= sample_rate;
            self.speed = (speed.powf(3.0) * 0.001)*scale;
        }
        if amount != self.amount {
            self.amount = amount;
        }
        if range != self.raw_range {
            self.raw_range = range;
            self.range = range.powf(3.0) * RANGE_MULT;
            self.range2 = self.range * 2.0;
            self.range3 = self.range * 3.0;
            self.range4 = self.range * 4.0;
        }
        if speed != self.raw_speed {
            self.raw_speed = self.raw_speed;
            // Scaled params
            let mut scale = 1.0/44100.0;
            scale *= sample_rate;
            self.speed = (speed.powf(3.0) * 0.001)*scale;
        }
    }

    pub fn process(&mut self, left_in: f32, right_in: f32) -> (f32, f32) {
        let mut left_out;
        let mut right_out;

        // Left side input
        self.airFactorL = self.airPrevL - left_in;
        if self.fpFlip {
            self.airEvenL += self.airFactorL;
            self.airOddL -= self.airFactorL;
            self.airFactorL = self.airEvenL;
        } else {
            self.airOddL += self.airFactorL;
            self.airEvenL -= self.airFactorL;
            self.airFactorL = self.airOddL;
        }
        self.airOddL = (self.airOddL - ((self.airOddL - self.airEvenL)/256.0)) / 1.0001;
        self.airEvenL = (self.airEvenL - ((self.airEvenL - self.airOddL)/256.0)) / 1.0001;
        self.airPrevL = left_in;
        left_out = left_in + (self.airFactorL * self.amount);
		//air, compensates for loss of highs in flanger's interpolation

        // Right side input
        self.airFactorR = self.airPrevR - right_in;
        if self.fpFlip {
            self.airEvenR += self.airFactorR;
            self.airOddR -= self.airFactorR;
            self.airFactorR = self.airEvenR;
        } else {
            self.airOddR += self.airFactorR;
            self.airEvenR -= self.airFactorR;
            self.airFactorR = self.airOddR;
        }
        self.airOddR = (self.airOddR - ((self.airOddR - self.airEvenR)/256.0)) / 1.0001;
        self.airEvenR = (self.airEvenR - ((self.airEvenR - self.airOddR)/256.0)) / 1.0001;
        self.airPrevR = right_in;
        right_out = right_in + (self.airFactorR * self.amount);
		//air, compensates for loss of highs in flanger's interpolation

        if self.gcount < 1 || self.gcount > LOOP_LIMIT {
            self.gcount = LOOP_LIMIT;
        }

        let mut count = self.gcount;
        self.left_buffer[count] = left_out;
        self.left_buffer[count + LOOP_LIMIT] = left_out;
        self.right_buffer[count] = right_out;
        self.right_buffer[count + LOOP_LIMIT] = right_out;
        self.gcount -= 1;

        let modulation = self.range * self.amount;

        // Structure of oneof the "ensemble" chorus voices
        let mut offset = self.range + (modulation * (self.sweep).sin());
        count = self.gcount + offset.floor() as usize;

        left_out = self.left_buffer[count] * (1.0 - (offset - offset.floor()));
        left_out += self.left_buffer[count + 1];
        left_out += self.left_buffer[count + 2] * (offset - offset.floor());
        left_out -= (self.left_buffer[count] - self.left_buffer[count + 1]) - (self.left_buffer[count + 1] - self.left_buffer[count + 2])/50.0;

        right_out += self.right_buffer[count] * (1.0 - (offset - offset.floor()));
        right_out += self.right_buffer[count + 1];
        right_out += self.right_buffer[count + 2] * (offset - offset.floor());
        right_out -= (self.right_buffer[count] - self.right_buffer[count + 1]) - (self.right_buffer[count + 1] - self.right_buffer[count + 2])/50.0;

        // Voice 2
        offset = self.range2 + (modulation * (self.sweep + 1.0).sin());
        count = self.gcount + offset.floor() as usize;

        left_out += self.left_buffer[count] * (1.0 - (offset - offset.floor()));
        left_out += self.left_buffer[count + 1];
        left_out += self.left_buffer[count + 2] * (offset - offset.floor());
        left_out -= (self.left_buffer[count] - self.left_buffer[count + 1]) - (self.left_buffer[count + 1] - self.left_buffer[count + 2])/50.0;

        right_out += self.right_buffer[count] * (1.0 - (offset - offset.floor()));
        right_out += self.right_buffer[count + 1];
        right_out += self.right_buffer[count + 2] * (offset - offset.floor());
        right_out -= (self.right_buffer[count] - self.right_buffer[count + 1]) - (self.right_buffer[count + 1] - self.right_buffer[count + 2])/50.0;

        // Voice 3
        offset = self.range3 + (modulation * (self.sweep + 2.0).sin());
        count = self.gcount + offset.floor() as usize;

        left_out += self.left_buffer[count] * (1.0 - (offset - offset.floor()));
        left_out += self.left_buffer[count + 1];
        left_out += self.left_buffer[count + 2] * (offset - offset.floor());
        left_out -= (self.left_buffer[count] - self.left_buffer[count + 1]) - (self.left_buffer[count + 1] - self.left_buffer[count + 2])/50.0;

        right_out += self.right_buffer[count] * (1.0 - (offset - offset.floor()));
        right_out += self.right_buffer[count + 1];
        right_out += self.right_buffer[count + 2] * (offset - offset.floor());
        right_out -= (self.right_buffer[count] - self.right_buffer[count + 1]) - (self.right_buffer[count + 1] - self.right_buffer[count + 2])/50.0;

        // Voice 4
        offset = self.range4 + (modulation * (self.sweep + 3.0).sin());
        count = self.gcount + offset.floor() as usize;

        left_out += self.left_buffer[count] * (1.0 - (offset - offset.floor()));
        left_out += self.left_buffer[count + 1];
        left_out += self.left_buffer[count + 2] * (offset - offset.floor());
        left_out -= (self.left_buffer[count] - self.left_buffer[count + 1]) - (self.left_buffer[count + 1] - self.left_buffer[count + 2])/50.0;

        right_out += self.right_buffer[count] * (1.0 - (offset - offset.floor()));
        right_out += self.right_buffer[count + 1];
        right_out += self.right_buffer[count + 2] * (offset - offset.floor());
        right_out -= (self.right_buffer[count] - self.right_buffer[count + 1]) - (self.right_buffer[count + 1] - self.right_buffer[count + 2])/50.0;

        // Scale the added voices down
        left_out *= 0.125;
        right_out *= 0.125;

        self.sweep += self.speed;
        if self.sweep > TAU {
            self.sweep -= TAU;
        }

        if self.amount != 1.0 {
            // Mix dry and wet signals based on the amount parameter
            left_out = left_in * (1.0 - self.amount) + left_out * self.amount;
            right_out = right_in * (1.0 - self.amount) + right_out * self.amount;
        }

        self.fpFlip = !self.fpFlip;

        (left_out, right_out)
    }
}
