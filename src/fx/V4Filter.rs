// A digital filter inspired by Novation synths of old
// Ardura

use nih_plug::util;

#[derive(Clone, Copy, Debug)]
pub struct V4FilterStruct {
    scale_gain: f32,
    adjustment_factor: f32,
    filter_stage_1: f32,
    filter_stage_2: f32,
    filter_stage_3: f32,
    feedback_factor: f32,
    feedback_offset: f32,
    integrator: f32,
    filter_output: f32,
    cutoff_frequency: f32,
    sample_rate: f32,
    alpha: f32,
}

impl V4FilterStruct {
    pub fn default() -> V4FilterStruct {
        V4FilterStruct {
            scale_gain: 1.0,
            adjustment_factor: 1.0,
            filter_stage_1: 0.0,
            filter_stage_2: 0.0,
            filter_stage_3: 0.0,
            feedback_factor: 0.072,
            feedback_offset: 0.0,
            integrator: 0.0,
            filter_output: 0.0,
            cutoff_frequency: 1000.0,
            sample_rate: 44100.0,
            alpha: 1.0,
        }
    }

    pub fn update(&mut self, feedback: f32, cutoff_frequency: f32, sample_rate: f32) {
        if self.sample_rate != sample_rate {
            self.sample_rate = sample_rate;
        }
        if self.feedback_offset != feedback {
            self.feedback_offset = feedback;
            self.scale_feedback_from_cutoff();
        }
        if self.cutoff_frequency != cutoff_frequency {
            self.cutoff_frequency = cutoff_frequency;
            self.scale_feedback_from_cutoff();
            self.scale_adjustment_from_cutoff();
            self.calculate_alpha();
        }
    }

    pub fn process(&mut self, input_sample: f32) -> f32 {
        self.scale_gain_from_cutoff();
        self.filter_stage_1 = self.process_stage(self.filter_stage_1, input_sample);
        self.filter_stage_2 = self.process_stage(self.filter_stage_2, self.filter_stage_1 * 1.2);
        self.filter_stage_3 = self.process_stage(self.filter_stage_3, self.filter_stage_2 * 1.3);
        self.filter_output = self.process_stage(self.filter_output, self.filter_stage_3 * 1.4);
        self.filter_output * util::db_to_gain(8.0 + self.scale_gain)
    }

    fn process_stage(&mut self, stage_value: f32, input_value: f32) -> f32 {
        let intermediate_value1 = input_value / ((input_value.abs() - 0.9999925).max(0.0001) * self.adjustment_factor + 1.0);
        let mut filter_result = self.alpha * intermediate_value1 + (1.0 - self.alpha) * stage_value;

        // Nonlinearity funkiness that the integrator smooths out over stages when it happens
        if (filter_result - 0.25).abs() < 1.0e-6 {
            filter_result = (filter_result * 16.0).clamp(-1.0, 1.0);
        }
        
        let mut intermediate_value2 = filter_result * self.feedback_factor + self.integrator;
        self.integrator = intermediate_value2.clamp(-1.0, 1.0);
        
        intermediate_value2 = (intermediate_value1 - intermediate_value2) - stage_value * filter_result;
        let output = stage_value * self.feedback_factor + intermediate_value2;
        
        output.clamp(-1.0, 1.0)
    }

    fn scale_gain_from_cutoff(&mut self) {
        let output_min = 1.0;
        let output_max = 18.0;
    
        self.scale_gain = output_max - (self.cutoff_frequency.clamp(20.0, 20000.0) - 20.0) * (output_max - output_min) / 19980.0;
    }
    

    fn scale_adjustment_from_cutoff(&mut self) {
        let output_min = 20000.0;
        let output_max = 0.0;
    
        self.adjustment_factor = output_min + (20000.0 - self.cutoff_frequency.clamp(20.0, 20000.0)) * (output_min - output_max) / 19980.0;
    }

    fn scale_feedback_from_cutoff(&mut self) {
        self.feedback_factor = ((self.cutoff_frequency.clamp(20.0, 20000.0) - 20.0) * (0.36) / 19980.0) + (1.0 - self.feedback_offset)*0.25;
    }
    
    fn calculate_alpha(&mut self) {
        let dt = 1.0 / self.sample_rate;
        let rc = 1.0 / (std::f32::consts::TAU * self.cutoff_frequency);
        self.alpha = dt / (rc + dt);
    }
}
