// Subhoofer's main inspiration - ABass Algorithm mimicking a bass plugin of Renaissance
// Ardura 2023

use nih_plug::util;

// My updated 2024 Algorithm used in Subhoofer
pub fn a_bass_saturation(signal: f32, harmonic_strength: f32) -> f32 {
    let mut output = custom_sincos_saturation(
        signal,
        harmonic_strength * 31.422043, 
        harmonic_strength * 189.29568, 
        harmonic_strength * 25.0, 
        harmonic_strength * 26.197401
    );
    let h_l = (output * 2.0) - output.powf(2.0);
    output += h_l * 0.0070118904;

    // I used -9.0 here instead of Subhoofer's value because Actuate signal often runs hotter/louder
    chebyshev_tape(output, 0.0093) * util::db_to_gain(-9.0)
}

fn custom_sincos_saturation(signal: f32, harmonic_strength1: f32, harmonic_strength2: f32, harmonic_strength3: f32, harmonic_strength4: f32) -> f32 {
    let mut summed: f32 = 0.0;

    let harmonic_component: f32 = harmonic_strength1 * (signal * 1.0).cos() - signal;
    summed += harmonic_component;

    let harmonic_component: f32 = harmonic_strength2 * (signal * 2.0).sin() - signal;
    summed += harmonic_component;

    let harmonic_component: f32 = harmonic_strength3 * (signal * 3.0).cos() - signal;
    summed += harmonic_component;

    let harmonic_component2: f32 = harmonic_strength4 * (signal * 4.0).sin() - signal;
    summed += harmonic_component2;

    summed
}

/* Older Subhoofer Algorithm
// Subhoofer's default harmonic_strength was 0.0011 and hardness 0.04
pub fn a_bass_saturation(signal: f32, harmonic_strength: f32) -> f32 {
    let num_harmonics: usize = 4;
    let mut summed: f32 = 0.0;
    for j in 1..=num_harmonics {
        match j {
            1 => {
                let harmonic_component: f32 =
                    harmonic_strength * 170.0 * (signal * j as f32).cos() - signal;
                summed += harmonic_component;
            }
            2 => {
                let harmonic_component: f32 =
                    harmonic_strength * 25.0 * (signal * j as f32).sin() - signal;
                summed += harmonic_component;
            }
            3 => {
                let harmonic_component: f32 =
                    harmonic_strength * 150.0 * (signal * j as f32).cos() - signal;
                summed += harmonic_component;
            }
            4 => {
                let harmonic_component2: f32 =
                    harmonic_strength * 80.0 * (signal * j as f32).sin() - signal;
                summed += harmonic_component2;
            }
            _ => unreachable!(),
        }
    }
    if harmonic_strength > 0.0 {
        chebyshev_tape(summed, 0.04) * util::db_to_gain(-9.0)
    } else {
        0.0
    }
}
*/

// Modified function from Duro Console for different behavior - hoof hardness
fn chebyshev_tape(sample: f32, drive: f32) -> f32 {
    let dry = 1.0 - drive;
    let peak = f32::max(sample.abs(), 1.0);
    let x = sample / peak;
    let x2 = x * x;
    let x3 = x * x2;
    let x5 = x3 * x2;
    let x6 = x3 * x3;
    let y = x - 0.166667 * x3 + 0.00833333 * x5 - 0.000198413 * x6 + 0.0000000238 * x6 * drive;
    dry * sample + (1.0 - dry) * y / (1.0 + y.abs())
}
