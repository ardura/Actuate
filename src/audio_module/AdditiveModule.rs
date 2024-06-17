use std::f32::consts::PI;

use super::SingleVoice;

struct AdditiveHarmonic {
    index: usize,
    amplitude: f32,
    phase: f32,
}

struct AdditiveOscillator {
    harmonics: Vec<AdditiveHarmonic>,
    time: f32,
}

impl AdditiveOscillator {
    fn new(harmonics: Vec<AdditiveHarmonic>) -> Self {
        AdditiveOscillator {
            harmonics,
            time: 0.0,
        }
    }

    fn next_sample(&mut self, voice: SingleVoice, sample_rate: f32) -> (f32, f32) {
        let mut sample = self.harmonics.iter()
            .map(|h| {
                let harmonic_freq = (h.index as f32 + 1.0) * voice.frequency;
                let angle = 2.0 * PI * harmonic_freq * self.time + h.phase;
                angle.sin() * h.amplitude
            })
            .sum::<f32>();
        sample *= voice.amp_current;

        self.time += 1.0 / sample_rate as f32;
        (sample, sample) // Mono to stereo: left and right channels are the same
    }
}

fn main() {
    // Example harmonics with phase shifts: fundamental with no phase shift,
    // 2nd harmonic with π/2 phase shift, 3rd harmonic with π phase shift.
    let harmonics = vec![
        AdditiveHarmonic { index: 0, amplitude: 0.1, phase: 0.0 },
        AdditiveHarmonic { index: 1, amplitude: 0.05, phase: std::f32::consts::PI / 2.0 },
        AdditiveHarmonic { index: 2, amplitude: 0.03, phase: std::f32::consts::PI },
    ];

    let mut oscillator = AdditiveOscillator::new(harmonics);

    for _ in 0..SAMPLE_RATE * 2 { // Output samples for 2 seconds
        let (left, right) = oscillator.next_sample();
        // Replace with actual audio output logic
        println!("{}, {}", left, right);
    }
}