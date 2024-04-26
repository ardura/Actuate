use std::f32::consts::{PI};

pub fn frequency_modulation(modulating_sample: f32, carrier_sample: f32, modulation_index: f32) -> f32 {
    if modulation_index == 0.0 {
        return carrier_sample;
    }
    let phase_change = modulation_index * modulating_sample;
    let modulated_signal = (2.0 * PI * carrier_sample + phase_change).cos();

    let compensation_factor = 1.0 / (modulation_index*0.5 + 2.0);
    let compensated_signal = modulated_signal * compensation_factor;

    compensated_signal
}

pub fn double_modulation(modulating_sample: f32, carrier_sample: f32, modulation_index: f32) -> f32 {
    let first_fm_sample = frequency_modulation(modulating_sample, carrier_sample, modulation_index);
    let second_fm_sample = frequency_modulation(first_fm_sample, carrier_sample, modulation_index);
    second_fm_sample
}

pub fn triple_modulation(modulating_sample: f32, carrier_sample: f32, modulation_index: f32) -> f32 {
    let first_fm_sample = frequency_modulation(modulating_sample, carrier_sample, modulation_index);
    let second_fm_sample = frequency_modulation(first_fm_sample, carrier_sample, modulation_index);
    let third_fm_sample = frequency_modulation(second_fm_sample, carrier_sample, modulation_index);
    third_fm_sample
}