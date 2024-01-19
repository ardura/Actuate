// Subhoofer's main inspiration - ABass Algorithm mimicking a bass plugin of Renaissance
// Ardura 2023

use nih_plug::util;

// Subhoofer's default harmonic_strength was 0.0011 and hardness 0.04
pub fn a_bass_saturation(signal: f32, harmonic_strength: f32) -> f32 {
    let num_harmonics: usize = 4;
    let mut summed: f32 = 0.0;
    for j in 1..=num_harmonics {
        match j {
            1 => {
                let harmonic_component: f32 = harmonic_strength * 170.0 * (signal * j as f32).cos() - signal;
                summed += harmonic_component;
            },
            2 => {
                let harmonic_component: f32 = harmonic_strength * 25.0 * (signal * j as f32).sin() - signal;
                summed += harmonic_component;
            },
            3 => {
                let harmonic_component: f32 = harmonic_strength * 150.0 * (signal * j as f32).cos() - signal;
                summed += harmonic_component;
            },
            4 => {
                let harmonic_component2: f32 = harmonic_strength * 80.0 * (signal * j as f32).sin() - signal;
                summed += harmonic_component2;
            },
            _ => unreachable!()
        }
    }
    if harmonic_strength > 0.0
    {
        chebyshev_tape(summed, 0.04) * util::db_to_gain(-9.0)
    }
    else {
        0.0
    }
}

// Modified function from Duro Console for different behavior - hoof hardness
fn chebyshev_tape(sample: f32, drive: f32) -> f32 {
    let dry = 1.0 - drive;
    let peak = f32::max(sample.abs(), 1.0);
    let x = sample / peak;
    let x2 = x * x;
    let x3 = x * x2;
    let x5 = x3 * x2;
    let x6 = x3 * x3;
    let y = x
        - 0.166667 * x3
        + 0.00833333 * x5
        - 0.000198413 * x6
        + 0.0000000238 * x6 * drive;
    dry * sample + (1.0 - dry) * y / (1.0 + y.abs())
}
