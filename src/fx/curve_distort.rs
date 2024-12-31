// Nonlinear Distortion through interpolating different lines

use std::f32::consts::PI;

/// Main function demonstrating oversampled nonlinear distortion with automatic gain staging
fn main() {
    // Example audio signal
    let audio_samples = vec![-0.9, -0.5, 0.0, 0.5, 0.9];

    // Oversampling factor
    let oversample_factor = 4;

    // Parameters for the distortion
    let intensity = 0.7;
    let curve_mix = 0.5;

    // Process audio with oversampling and automatic gain staging
    let mut gain_staged_samples = Vec::new();
    let mut input_rms_accum = 0.0;
    let mut output_rms_accum = 0.0;
    let mut sample_count = 0;

    for sample in audio_samples {
        // Oversample and process each sample
        let processed = oversample_and_process(sample, intensity, curve_mix, oversample_factor);

        // Accumulate RMS calculations
        input_rms_accum += sample * sample;
        output_rms_accum += processed[0] * processed[0];
        sample_count += 1;

        // Compute gain on-the-fly
        let input_rms = (input_rms_accum / sample_count as f32).sqrt();
        let output_rms = (output_rms_accum / sample_count as f32).sqrt();
        let gain = if output_rms > 0.0 { input_rms / output_rms } else { 1.0 };

        // Apply gain adjustment
        gain_staged_samples.push(processed[0] * gain);
    }

    // Output processed samples
    println!("{:?}", gain_staged_samples);
}

/// Oversample, process, and then downsample
fn oversample_and_process(
    input: f32,
    intensity: f32,
    curve_mix: f32,
    oversample_factor: usize,
) -> Vec<f32> {
    // Create an oversampled signal
    let mut oversampled = vec![0.0; oversample_factor];
    for i in 0..oversample_factor {
        // Simple upsampling by repeating the sample
        oversampled[i] = input;
    }

    // Apply distortion to the oversampled signal
    for sample in oversampled.iter_mut() {
        *sample = nonlinear_distortion(*sample, intensity, curve_mix);
    }

    // Apply a simple lowpass filter to remove aliasing
    oversampled = lowpass_filter(oversampled, 0.5 / oversample_factor as f32);

    // Downsample back to the original rate
    vec![oversampled[0]] // Return only the first sample (simple downsampling)
}

/// Nonlinear distortion processing
fn nonlinear_distortion(input: f32, intensity: f32, curve_mix: f32) -> f32 {
    let curve1 = |x: f32| x / (1.0 + x.abs());
    let curve2 = |x: f32| x - (x * x * x) / 3.0;
    let curve3 = |x: f32| 4.0 * x * (1.0 - x.abs());

    let interpolated_secondary = |x: f32| (1.0 - curve_mix) * curve2(x) + curve_mix * curve3(x);
    let interpolated_curve = |x: f32| {
        let t = intensity;
        (1.0 - t) * curve1(x) + t * interpolated_secondary(x)
    };

    // Removed bias application
    interpolated_curve(input)
}

/// Simple lowpass filter (single-pole IIR)
fn lowpass_filter(samples: Vec<f32>, cutoff: f32) -> Vec<f32> {
    let mut filtered = vec![0.0; samples.len()];
    let alpha = 1.0 / (1.0 + 2.0 * PI * cutoff);
    filtered[0] = samples[0];
    for i in 1..samples.len() {
        filtered[i] = alpha * samples[i] + (1.0 - alpha) * filtered[i - 1];
    }
    filtered
}