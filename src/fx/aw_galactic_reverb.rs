// This is a Rust implementation of Airwindows Galactic with some tweaks
// I love this reverb and couldn't make anything like it on my own so here we are borrowing as a huge fan
// Ardura + Chris @ Airwindows

use std::f32::consts::{FRAC_PI_2, TAU};

#[derive(Clone)]
struct ArrayBank {
    a_i: Vec<f32>,
    a_j: Vec<f32>,
    a_k: Vec<f32>,
    a_l: Vec<f32>,
    a_a: Vec<f32>,
    a_b: Vec<f32>,
    a_c: Vec<f32>,
    a_d: Vec<f32>,
    a_e: Vec<f32>,
    a_f: Vec<f32>,
    a_g: Vec<f32>,
    a_h: Vec<f32>,
}

#[derive(Clone)]
pub struct GalacticReverb {
    // Actuate's inputs
    sample_rate: f32,
    size: f32,
    wet: f32,
    // Complex stuff
    regen: f32,
    attenuate: f32,
    lowpass: f32,
    drift: f32,
    delay_bank: Vec<usize>,
    vibrato_memory: f32,
    old_fpd: f32,
    countI: usize,
	countJ: usize,
	countK: usize,
	countL: usize,
	countA: usize,
	countB: usize,
	countC: usize,
	countD: usize,	
	countE: usize,
	countF: usize,
	countG: usize,
	countH: usize,
    countM: usize,
    aML: Vec<f32>,
    aMR: Vec<f32>,
    // iir persistent values
    iir_a_l: f32,
    iir_a_r: f32,
    iir_b_l: f32,
    iir_b_r: f32,
    // predelay
    cycle: i32,
    // Collapse the delay banks into an array of arrays
    arr_l: ArrayBank,
    arr_r: ArrayBank,
    feedback_l: Vec<f32>,
    feedback_r: Vec<f32>,
    last_ref_l: Vec<f32>,
    last_ref_r: Vec<f32>,
}
//                            I     J     K    L    A     B     C     D    E     F     G     H
const DELAYS: [usize;12] = [3407, 1823, 859, 331, 4801, 2909, 1153, 461, 7607, 4217, 2269, 1597];
// Missed this originally          I     J     K    L    A     B     C     D    E     F     G     H
const DELAY_SIZING: [usize;12] = [6480, 3660, 1720, 680, 9700, 6000, 2320, 940, 15220, 8460, 4540, 3200];
const DELAY_M: usize = 256;
const MYRAND: f32 = 83.0 * 0.0000000000618;

impl GalacticReverb {
    pub fn new(sample_rate: f32, size_input: f32, feedback: f32, wet: f32) -> Self {
        // My settings
        let overallscale = sample_rate/44100.0;
        let regen_val = feedback;
        let regen_calc = 0.0625 + (( 1.0 - regen_val ) * 0.0625 );
        let lowpass_val = 0.79;
        let drift_val = f32::powf(0.5, 3.0) * 0.001;
        GalacticReverb {
            sample_rate: sample_rate,
            size: (size_input * 1.77) + 0.1,
            wet: wet,
            // Complex stuff
            regen: regen_calc,
            attenuate: (1.0 - (regen_calc / 0.125))*1.333,
            lowpass: f32::powf(1.00001 - (1.0 - lowpass_val), 2.0) / f32::sqrt(overallscale),
            drift: f32::powf(drift_val, 3.0) * 0.001,
            delay_bank: vec![
                DELAYS[0] * ((size_input * 1.77) + 0.1) as usize,
                DELAYS[1] * ((size_input * 1.77) + 0.1) as usize,
                DELAYS[2] * ((size_input * 1.77) + 0.1) as usize,
                DELAYS[3] * ((size_input * 1.77) + 0.1) as usize,
                DELAYS[4] * ((size_input * 1.77) + 0.1) as usize,
                DELAYS[5] * ((size_input * 1.77) + 0.1) as usize,
                DELAYS[6] * ((size_input * 1.77) + 0.1) as usize,
                DELAYS[7] * ((size_input * 1.77) + 0.1) as usize,
                DELAYS[8] * ((size_input * 1.77) + 0.1) as usize,
                DELAYS[9] * ((size_input * 1.77) + 0.1) as usize,
                DELAYS[10] * ((size_input * 1.77) + 0.1) as usize,
                DELAYS[11] * ((size_input * 1.77) + 0.1) as usize,
            ],
            // Magic numbers
            vibrato_memory: 3.0,
            old_fpd: 429496.7295,
            countI: 1,
	        countJ: 1,
	        countK: 1,
	        countL: 1,
	        countA: 1,
	        countB: 1,
	        countC: 1,
	        countD: 1,
	        countE: 1,
	        countF: 1,
	        countG: 1,
	        countH: 1,
            countM: 1,
            aML: vec![0.0; 3111],
            aMR: vec![0.0; 3111],
            iir_a_l: 0.0,
            iir_a_r: 0.0,
            iir_b_l: 0.0,
            iir_b_r: 0.0,
            cycle: 0,
            arr_l: ArrayBank {
                a_i: vec![0.0; DELAY_SIZING[0]],
                a_j: vec![0.0; DELAY_SIZING[1]],
                a_k: vec![0.0; DELAY_SIZING[2]],
                a_l: vec![0.0; DELAY_SIZING[3]],
                a_a: vec![0.0; DELAY_SIZING[4]],
                a_b: vec![0.0; DELAY_SIZING[5]],
                a_c: vec![0.0; DELAY_SIZING[6]],
                a_d: vec![0.0; DELAY_SIZING[7]],
                a_e: vec![0.0; DELAY_SIZING[8]],
                a_f: vec![0.0; DELAY_SIZING[9]],
                a_g: vec![0.0; DELAY_SIZING[10]],
                a_h: vec![0.0; DELAY_SIZING[11]],
            },
            arr_r: ArrayBank {
                a_i: vec![0.0; DELAY_SIZING[0]],
                a_j: vec![0.0; DELAY_SIZING[1]],
                a_k: vec![0.0; DELAY_SIZING[2]],
                a_l: vec![0.0; DELAY_SIZING[3]],
                a_a: vec![0.0; DELAY_SIZING[4]],
                a_b: vec![0.0; DELAY_SIZING[5]],
                a_c: vec![0.0; DELAY_SIZING[6]],
                a_d: vec![0.0; DELAY_SIZING[7]],
                a_e: vec![0.0; DELAY_SIZING[8]],
                a_f: vec![0.0; DELAY_SIZING[9]],
                a_g: vec![0.0; DELAY_SIZING[10]],
                a_h: vec![0.0; DELAY_SIZING[11]],
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
                (DELAYS[4] as f32 * self.size) as usize,
                (DELAYS[5] as f32 * self.size) as usize,
                (DELAYS[6] as f32 * self.size) as usize,
                (DELAYS[7] as f32 * self.size) as usize,
                (DELAYS[8] as f32 * self.size) as usize,
                (DELAYS[9] as f32 * self.size) as usize,
                (DELAYS[10] as f32 * self.size) as usize,
                (DELAYS[11] as f32 * self.size) as usize,
            ];
        }

        if 0.0625 + (( 1.0 - feedback ) * 0.0625) != self.regen {
            let regen_val = feedback;
            let regen_calc = 0.0625 + (( 1.0 - regen_val ) * 0.0625 );
            self.regen = regen_calc;
            self.attenuate = (1.0 - (regen_calc / 0.125))*1.333;
        }

        self.wet = wet;
    }

    pub fn process(&mut self, input_l: f32, input_r: f32) -> (f32, f32) {
        // Calculate vibrato
        self.vibrato_memory += self.old_fpd * self.drift;
        if self.vibrato_memory > TAU {
            self.vibrato_memory = 0.0;
            self.old_fpd = 0.4294967295 + MYRAND;
        }

        self.aML[self.countM as usize] = input_l * self.attenuate;
        self.aMR[self.countM as usize] = input_r * self.attenuate;

        self.countM += 1;
        self.countM = if self.countM > DELAY_M { 0 } else { self.countM };

        // Make the vibrato
        let offsetML: f32 = (self.vibrato_memory.sin() + 1.0) * 127.0;
        let offsetMR: f32 = ((self.vibrato_memory + FRAC_PI_2).sin() + 1.0) * 127.0;
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

        // Cycle calculation - mainly for non 44100 sample rates
        let cycle_end = ((self.sample_rate/44100.0).floor() as i32).clamp(1, 4);
        self.cycle = if self.cycle > (cycle_end - 1) { cycle_end - 1 } else { self.cycle };
        self.cycle += 1;

        // We reach this point and do a reverb sample
        if self.cycle == cycle_end {
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

            ///////////////////////////////////////////////////////////////////////////////////
            // Reverb Block TWO
            ///////////////////////////////////////////////////////////////////////////////////
            self.arr_l.a_a[self.countA] = outIL - (outJL + outKL + outLL);
            self.arr_l.a_b[self.countB] = outJL - (outIL + outKL + outLL);
            self.arr_l.a_c[self.countC] = outKL - (outIL + outJL + outLL);
            self.arr_l.a_d[self.countD] = outLL - (outIL + outJL + outKL);

            self.arr_r.a_a[self.countA] = outIR - (outJR + outKR + outLR);
            self.arr_r.a_b[self.countB] = outJR - (outIR + outKR + outLR);
            self.arr_r.a_c[self.countC] = outKR - (outIR + outJR + outLR);
            self.arr_r.a_d[self.countD] = outLR - (outIR + outJR + outKR);

            //                              I     J     K    L    A     B     C     D    E     F     G     H
            //const DELAYS: [usize; 12] = [3407, 1823, 859, 331, 4801, 2909, 1153, 461, 7607, 4217, 2269, 1597];
            self.countA += 1; if self.countA > self.delay_bank[4] { self.countA = 0; }
            self.countB += 1; if self.countB > self.delay_bank[5] { self.countB = 0; }
            self.countC += 1; if self.countC > self.delay_bank[6] { self.countC = 0; }
            self.countD += 1; if self.countD > self.delay_bank[7] { self.countD = 0; }

            let outAL = self.arr_l.a_a[self.countA - if self.countA > self.delay_bank[4] { self.delay_bank[4] } else { 0 }];
            let outBL = self.arr_l.a_b[self.countB - if self.countB > self.delay_bank[5] { self.delay_bank[5] } else { 0 }];
            let outCL = self.arr_l.a_c[self.countC - if self.countC > self.delay_bank[6] { self.delay_bank[6] } else { 0 }];
            let outDL = self.arr_l.a_d[self.countD - if self.countD > self.delay_bank[7] { self.delay_bank[7] } else { 0 }];
            let outAR = self.arr_r.a_a[self.countA - if self.countA > self.delay_bank[4] { self.delay_bank[4] } else { 0 }];
            let outBR = self.arr_r.a_b[self.countB - if self.countB > self.delay_bank[5] { self.delay_bank[5] } else { 0 }];
            let outCR = self.arr_r.a_c[self.countC - if self.countC > self.delay_bank[6] { self.delay_bank[6] } else { 0 }];
            let outDR = self.arr_r.a_d[self.countD - if self.countD > self.delay_bank[7] { self.delay_bank[7] } else { 0 }];

            ///////////////////////////////////////////////////////////////////////////////////
            // Reverb Block THREE
            ///////////////////////////////////////////////////////////////////////////////////
            self.arr_l.a_e[self.countE] = outAL - (outBL + outCL + outDL);
            self.arr_l.a_f[self.countF] = outBL - (outAL + outCL + outDL);
            self.arr_l.a_g[self.countG] = outCL - (outAL + outBL + outDL);
            self.arr_l.a_h[self.countH] = outDL - (outAL + outBL + outCL);

            self.arr_r.a_e[self.countE] = outAR - (outBR + outCR + outDR);
            self.arr_r.a_f[self.countF] = outBR - (outAR + outCR + outDR);
            self.arr_r.a_g[self.countG] = outCR - (outAR + outBR + outDR);
            self.arr_r.a_h[self.countH] = outDR - (outAR + outBR + outCR);

            //                              I     J     K    L    A     B     C     D    E     F     G     H
            //const DELAYS: [usize; 12] = [3407, 1823, 859, 331, 4801, 2909, 1153, 461, 7607, 4217, 2269, 1597];
            self.countE += 1; if self.countE > self.delay_bank[8] { self.countE = 0; }
            self.countF += 1; if self.countF > self.delay_bank[9] { self.countF = 0; }
            self.countG += 1; if self.countG > self.delay_bank[10] { self.countG = 0; }
            self.countH += 1; if self.countH > self.delay_bank[11] { self.countH = 0; }

            let outEL = self.arr_l.a_e[self.countE - if self.countE > self.delay_bank[8]  { self.delay_bank[8]   } else { 0 }];
            let outFL = self.arr_l.a_f[self.countF - if self.countF > self.delay_bank[9]  { self.delay_bank[9]   } else { 0 }];
            let outGL = self.arr_l.a_g[self.countG - if self.countG > self.delay_bank[10] { self.delay_bank[10]   } else { 0 }];
            let outHL = self.arr_l.a_h[self.countH - if self.countH > self.delay_bank[11] { self.delay_bank[11]   } else { 0 }];
            let outER = self.arr_r.a_e[self.countE - if self.countE > self.delay_bank[8]  { self.delay_bank[8]   } else { 0 }];
            let outFR = self.arr_r.a_f[self.countF - if self.countF > self.delay_bank[9]  { self.delay_bank[9]   } else { 0 }];
            let outGR = self.arr_r.a_g[self.countG - if self.countG > self.delay_bank[10] { self.delay_bank[10]   } else { 0 }];
            let outHR = self.arr_r.a_h[self.countH - if self.countH > self.delay_bank[11] { self.delay_bank[11]   } else { 0 }];

            self.feedback_l[0] = outEL - (outFL + outGL + outHL);
            self.feedback_l[1] = outFL - (outEL + outGL + outHL);
            self.feedback_l[2] = outGL - (outEL + outFL + outHL);
            self.feedback_l[3] = outHL - (outEL + outFL + outGL);
            self.feedback_r[0] = outER - (outFR + outGR + outHR);
            self.feedback_r[1] = outFR - (outER + outGR + outHR);
            self.feedback_r[2] = outGR - (outER + outFR + outHR);
            self.feedback_r[3] = outHR - (outER + outFR + outGR);
            
            output_l = (outEL + outFL + outGL + outHL) / 8.0;
            output_r = (outER + outFR + outGR + outHR) / 8.0;

            // Combine the final sum of outputs
            if cycle_end == 4 {
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
            }
            else if cycle_end == 3 {
                self.last_ref_l[0] = self.last_ref_l[3]; //start from previous last
                self.last_ref_l[2] = (self.last_ref_l[0] + self.last_ref_l[0] + output_l)/3.0; //one third
                self.last_ref_l[1] = (self.last_ref_l[0] + output_l + output_l)/3.0; //two thirds
                self.last_ref_l[3] = output_l; //full

                self.last_ref_r[0] = self.last_ref_r[3]; //start from previous last
                self.last_ref_r[2] = (self.last_ref_r[0] + self.last_ref_r[0] + output_r)/3.0; //one third
                self.last_ref_r[1] = (self.last_ref_r[0] + output_r + output_r)/3.0; //two thirds
                self.last_ref_r[3] = output_r; //full
            }
            else if cycle_end == 2 {
				self.last_ref_l[0] = self.last_ref_l[2]; //start from previous last
                self.last_ref_l[1] = (self.last_ref_l[0] + output_l)/2.0; //one half
                self.last_ref_l[2] = output_l; //full

                self.last_ref_r[0] = self.last_ref_r[2]; //start from previous last
                self.last_ref_r[1] = (self.last_ref_r[0] + output_r)/2.0; //one half
                self.last_ref_r[2] = output_r; //full
			}
            else if cycle_end == 1 {
                self.last_ref_l[0] = output_l;
                self.last_ref_r[0] = output_r;
            }
            self.cycle = 0;
            output_l = self.last_ref_l[self.cycle as usize];
            output_r = self.last_ref_r[self.cycle as usize];

        } else {
            output_l = self.last_ref_l[self.cycle as usize];
            output_r = self.last_ref_r[self.cycle as usize];
        }

        self.iir_b_l = (self.iir_b_l * (1.0 - self.lowpass)) + (output_l * self.lowpass);
        output_l = self.iir_b_l;

        self.iir_b_r = (self.iir_b_r * (1.0 - self.lowpass)) + (output_r * self.lowpass);
        output_r = self.iir_b_r;
        
        // Changed this wet summing to match my other reverb
        output_l = input_l + output_l * self.wet;
        output_r = input_r + output_r * self.wet;
        (output_l, output_r)
    }
}