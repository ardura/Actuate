// This is a reverb inspired by a simplified Airwindows Galactic to be used in a combination
// Hence "Simple Space"
// Ardura

use std::f32::consts::{TAU};

#[derive(Clone)]
struct ArrayBank {
    a_i: Vec<f32>,
    a_j: Vec<f32>,
    a_k: Vec<f32>,
    a_l: Vec<f32>,
}

#[derive(Clone)]
pub struct SimpleSpaceReverb {
    // Actuate's inputs
    sample_rate: f32,
    size: f32,
    wet: f32,
    // Complex stuff
    regen: f32,
    attenuate: f32,
    lowpass: f32,
    drift_l: f32,
    drift_r: f32,
    delay_bank: Vec<usize>,
    vibrato_memory_l: f32,
    vibrato_memory_r: f32,
    old_fpd: f32,
    countI: usize,
	countJ: usize,
	countK: usize,
	countL: usize,
    countM: usize,
    aML: Vec<f32>,
    aMR: Vec<f32>,
    // iir persistent values
    iir_a_l: f32,
    iir_a_r: f32,
    iir_b_l: f32,
    iir_b_r: f32,
    // Collapse the delay banks into an array of arrays
    arr_l: ArrayBank,
    arr_r: ArrayBank,
    feedback_l: Vec<f32>,
    feedback_r: Vec<f32>,
    last_ref_l: Vec<f32>,
    last_ref_r: Vec<f32>,
}
//                                 I     J     K    L
const DELAYS: [usize; 4] =       [3450, 2248, 1000, 320];
const DELAY_SIZING: [usize; 4] = [7000, 4588, 2300, 680];
const DELAY_M: usize = 256;
const MYRAND: f32 = 83.0 * 0.0000000000618;

impl SimpleSpaceReverb {
    pub fn new(sample_rate: f32, size_input: f32, feedback: f32, wet: f32) -> Self {
        // My settings
        let overallscale = sample_rate/44100.0;
        let regen_val = feedback;
        let regen_calc = 0.0625 + (( 1.0 - regen_val ) * 0.0625 );
        // Make this darker - Galactic has 79 here in Actuate
        let lowpass_val = 0.70;
        // I also made the drift larger from Galactic: 0.002 vs 0.001
        let drift_val = f32::powf(0.5, 3.0) * 0.002;
        SimpleSpaceReverb {
            sample_rate: sample_rate,
            size: (size_input * 1.77) + 0.1,
            wet: wet,
            // Complex stuff
            regen: regen_calc,
            attenuate: (1.0 - (regen_calc / 0.125))*1.333,
            lowpass: f32::powf(1.00001 - (1.0 - lowpass_val), 2.0) / f32::sqrt(overallscale),
            drift_l: f32::powf(drift_val, 3.0) * 0.001,
            drift_r: f32::powf(drift_val, 3.0) * 0.002,
            delay_bank: vec![
                DELAYS[0] * ((size_input * 1.77) + 0.1) as usize,
                DELAYS[1] * ((size_input * 1.77) + 0.1) as usize,
                DELAYS[2] * ((size_input * 1.77) + 0.1) as usize,
                DELAYS[3] * ((size_input * 1.77) + 0.1) as usize,
            ],
            // Magic numbers
            vibrato_memory_l: 3.0,
            vibrato_memory_r: 3.0,
            old_fpd: 429496.7295,
            countI: 1,
	        countJ: 1,
	        countK: 1,
	        countL: 1,
            countM: 1,
            aML: vec![0.0; 3111],
            aMR: vec![0.0; 3111],
            iir_a_l: 0.0,
            iir_a_r: 0.0,
            iir_b_l: 0.0,
            iir_b_r: 0.0,
            arr_l: ArrayBank {
                a_i: vec![0.0; DELAY_SIZING[0]],
                a_j: vec![0.0; DELAY_SIZING[1]],
                a_k: vec![0.0; DELAY_SIZING[2]],
                a_l: vec![0.0; DELAY_SIZING[3]],
            },
            arr_r: ArrayBank {
                a_i: vec![0.0; DELAY_SIZING[0]],
                a_j: vec![0.0; DELAY_SIZING[1]],
                a_k: vec![0.0; DELAY_SIZING[2]],
                a_l: vec![0.0; DELAY_SIZING[3]],
            },
            feedback_l: vec![0.0; 4],
            feedback_r: vec![0.0; 4],
            last_ref_l: vec![0.0; 7],
            last_ref_r: vec![0.0; 7],
        }
    }

    pub fn update(&mut self, sample_rate: f32, size_input: f32, feedback: f32, wet: f32) {
        let lowpass_val = 0.76;

        self.sample_rate = sample_rate;
        let overallscale = sample_rate/44100.0;

        self.lowpass = f32::powf(1.00001 - (1.0 - lowpass_val), 2.0) / f32::sqrt(overallscale);

        if (size_input * 1.77) + 0.1 != self.size {
            let scaled_size = (size_input * 1.77) + 0.1;
            self.size = scaled_size;
            self.delay_bank = vec![
                (DELAYS[0] as f32 * self.size) as usize,
                (DELAYS[1] as f32 * self.size) as usize,
                (DELAYS[2] as f32 * self.size) as usize,
                (DELAYS[3] as f32 * self.size) as usize,
            ];
        }

        if 0.08 + (( 1.0 - feedback ) * 0.08) != self.regen {
            let regen_val = feedback;
            let regen_calc = 0.08 + (( 1.0 - regen_val ) * 0.08 );
            self.regen = regen_calc;
            self.attenuate = (1.0 - (regen_calc / 0.125))*1.333;
        }

        self.wet = wet;
    }

    pub fn process(&mut self, input_l: f32, input_r: f32) -> (f32, f32) {
        // Calculate vibrato
        self.vibrato_memory_l += self.old_fpd * self.drift_l;
        self.vibrato_memory_r += self.old_fpd * self.drift_r;
        if self.vibrato_memory_l > TAU {
            self.vibrato_memory_l = 0.0;
            self.old_fpd = 0.4294967295 + MYRAND;
        }
        if self.vibrato_memory_r > TAU {
            self.vibrato_memory_r = 0.0;
            self.old_fpd = 0.4294967295 + MYRAND;
        }

        self.aML[self.countM as usize] = input_l * self.attenuate;
        self.aMR[self.countM as usize] = input_r * self.attenuate;

        self.countM += 1;
        self.countM = if self.countM > DELAY_M { 0 } else { self.countM };

        // Make the vibrato
        let offsetML: f32 = (self.vibrato_memory_l.sin() + 1.0) * 127.0;
        let offsetMR: f32 = (self.vibrato_memory_r.sin() + 1.0) * 127.0;
        let workingML: usize = self.countM + offsetML as usize;
        let workingMR: usize = self.countM + offsetMR as usize;
        let mut interpolate_ML = self.aML[(workingML - if workingML > DELAY_M { DELAY_M + 1 } else { 0 }) as usize] * (1.0 - (offsetML.floor() - offsetML));
        interpolate_ML += self.aML[(workingML + 1 - if workingML + 1 > DELAY_M { DELAY_M + 1 } else { 0 }) as usize] * (offsetML - offsetML.floor());
        let mut interpolate_MR = self.aMR[(workingMR - if workingMR > DELAY_M { DELAY_M + 1 } else { 0 }) as usize] * (1.0 - (offsetMR.floor() - offsetMR));
        interpolate_MR += self.aMR[(workingMR + 1 - if workingMR + 1 > DELAY_M { DELAY_M + 1 } else { 0 }) as usize] * (offsetMR - offsetMR.floor());

        let mut output_l = interpolate_ML;
        let mut output_r = interpolate_MR;

        // Lowpass filter
        self.iir_a_l =(self.iir_a_l * (1.0 - self.lowpass)) + (output_l * self.lowpass);
        output_l = self.iir_a_l;
        self.iir_a_r =(self.iir_a_r * (1.0 - self.lowpass)) + (output_r * self.lowpass);
        output_r = self.iir_a_r;

        ///////////////////////////////////////////////////////////////////////////////////
        // Reverb Block ONE
        ///////////////////////////////////////////////////////////////////////////////////
        self.arr_l.a_i[self.countI] = output_l + (self.feedback_r[0] * self.regen);
        self.arr_l.a_j[self.countJ] = output_l + (self.feedback_r[1] * self.regen);
        self.arr_l.a_k[self.countK] = output_l + (self.feedback_r[2] * self.regen);
        self.arr_l.a_l[self.countL] = output_l + (self.feedback_r[3] * self.regen);
        self.arr_r.a_i[self.countI] = output_r + (self.feedback_l[0] * self.regen);
        self.arr_r.a_j[self.countJ] = output_r + (self.feedback_l[1] * self.regen);
        self.arr_r.a_k[self.countK] = output_r + (self.feedback_l[2] * self.regen);
        self.arr_r.a_l[self.countL] = output_r + (self.feedback_l[3] * self.regen);

        //                              I     J     K    L    A     B     C     D    E     F     G     H
        //const DELAYS: [usize; 12] = [3407, 1823, 859, 331, 4801, 2909, 1153, 461, 7607, 4217, 2269, 1597];
        self.countI += 1; if self.countI > self.delay_bank[0] { self.countI = 0; }
        self.countJ += 1; if self.countJ > self.delay_bank[1] { self.countJ = 0; }
        self.countK += 1; if self.countK > self.delay_bank[2] { self.countK = 0; }
        self.countL += 1; if self.countL > self.delay_bank[3] { self.countL = 0; }

        let outIL = self.arr_l.a_i[self.countI - if self.countI > self.delay_bank[0] { self.delay_bank[0] } else { 0 }];
        let outJL = self.arr_l.a_j[self.countJ - if self.countJ > self.delay_bank[1] { self.delay_bank[1] } else { 0 }];
        let outKL = self.arr_l.a_k[self.countK - if self.countK > self.delay_bank[2] { self.delay_bank[2] } else { 0 }];
        let outLL = self.arr_l.a_l[self.countL - if self.countL > self.delay_bank[3] { self.delay_bank[3] } else { 0 }];
        let outIR = self.arr_r.a_i[self.countI - if self.countI > self.delay_bank[0] { self.delay_bank[0] } else { 0 }];
        let outJR = self.arr_r.a_j[self.countJ - if self.countJ > self.delay_bank[1] { self.delay_bank[1] } else { 0 }];
        let outKR = self.arr_r.a_k[self.countK - if self.countK > self.delay_bank[2] { self.delay_bank[2] } else { 0 }];
        let outLR = self.arr_r.a_l[self.countL - if self.countL > self.delay_bank[3] { self.delay_bank[3] } else { 0 }];
        
        self.feedback_l[0] = outIL - (outJL + outKL + outLL);
        self.feedback_l[1] = outJL - (outIL + outKL + outLL);
        self.feedback_l[2] = outKL - (outIL + outJL + outLL);
        self.feedback_l[3] = outLL - (outIL + outJL + outKL);
        self.feedback_r[0] = outIR - (outJR + outKR + outLR);
        self.feedback_r[1] = outJR - (outIR + outKR + outLR);
        self.feedback_r[2] = outKR - (outIR + outJR + outLR);
        self.feedback_r[3] = outLR - (outIR + outJR + outKR);
        
        output_l = (outIL + outJL + outKL + outLL)/2.0;
        output_r = (outIR + outJR + outKR + outLR)/2.0;

        self.last_ref_l[0] = self.last_ref_l[4]; //start from previous last
        self.last_ref_l[2] = (self.last_ref_l[0] + output_l)/2.0; //half
        self.last_ref_l[1] = (self.last_ref_l[0] + self.last_ref_l[2])/2.0; //one quarter
        self.last_ref_l[3] = (self.last_ref_l[2] + output_l)/2.0; //three quarters
        self.last_ref_l[4] = output_l; //full
        self.last_ref_r[0] = self.last_ref_r[4]; //start from previous last
        self.last_ref_r[2] = (self.last_ref_r[0] + output_r)/2.0; //half
        self.last_ref_r[1] = (self.last_ref_r[0] + self.last_ref_r[2])/2.0; //one quarter
        self.last_ref_r[3] = (self.last_ref_r[2] + output_r)/2.0; //three quarters
        self.last_ref_r[4] = output_r; //full
            
        output_l = self.last_ref_l[4];
        output_r = self.last_ref_r[4];

        self.iir_b_l = (self.iir_b_l * (1.0 - self.lowpass)) + (output_l * self.lowpass);
        output_l = self.iir_b_l;

        self.iir_b_r = (self.iir_b_r * (1.0 - self.lowpass)) + (output_r * self.lowpass);
        output_r = self.iir_b_r;
        
        output_l = input_l * (1.0 - self.wet) + output_l * self.wet;
        output_r = input_r * (1.0 - self.wet) + output_r * self.wet;
        //Add in dry after a point
        output_l += input_l * (self.wet / 2.0);
        output_r += input_r * (self.wet / 2.0);
        (output_l, output_r)
    }
}