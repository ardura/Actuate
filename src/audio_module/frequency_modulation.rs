use std::f32::consts::PI;

pub fn frequency_modulation(modulating_sample: f32, carrier_sample: f32, modulation_index: f32) -> f32 {
    let phase_change = modulation_index * modulating_sample;
    (2.0 * PI * carrier_sample + phase_change).cos()
}
