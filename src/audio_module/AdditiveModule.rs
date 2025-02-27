use std::f32::consts::{FRAC_PI_2, PI, TAU};
use nih_plug::util;

use super::{SingleUnisonVoice, SingleVoice};

#[derive(Clone, PartialEq)]
pub struct AdditiveHarmonic {
    pub index: usize,
    pub amplitude: f32,
}

#[derive(Clone)]
pub struct AdditiveOscillator {
    harmonics: Vec<AdditiveHarmonic>,
}

impl AdditiveOscillator {
    pub fn default() -> Self {
        AdditiveOscillator {
            harmonics: vec![AdditiveHarmonic { index: 0, amplitude: 0.0 }; 15],
        }
    }

    pub fn set_harmonics(&mut self, harmonics: Vec<AdditiveHarmonic>) {
        self.harmonics = harmonics;
    }

    pub fn next_sample(&mut self, voice: &mut SingleVoice, sample_rate: f32, detune_mod: f32) -> f32 {
        let mut sample = 0.0;
        let nyquist = sample_rate / 2.0;
        
        if voice.amp_current != 0.0 {
            let base_note = voice.note as f32 + voice._detune + detune_mod + voice.pitch_current + voice.pitch_current_2;
            let instant_frequency = util::f32_midi_note_to_freq(base_note).min(nyquist);
            voice.phase_delta = instant_frequency / sample_rate;

            for (i, harmonic) in self.harmonics.iter().enumerate() {
                if harmonic.amplitude != 0.0 {
                    let harmonic_freq = if harmonic.index == 0 {
                        instant_frequency
                    } else {
                        (harmonic.index as f32 + 1.0) * instant_frequency
                    };
                    let phase_increment = TAU * harmonic_freq / sample_rate;
                    voice.harmonic_phases[i] = (voice.harmonic_phases[i] + phase_increment) % TAU;
                    sample += fast_sine(voice.harmonic_phases[i]) * harmonic.amplitude;
                }
            }
        }

        sample
    }

    pub fn next_unison_sample(&mut self, voice: &mut SingleUnisonVoice, sample_rate: f32, detune_mod: f32) -> f32 {
        let mut sample = 0.0;
        let nyquist = sample_rate / 2.0;
        
        if voice.amp_current != 0.0 {
            let base_note = voice.note as f32 + voice._unison_detune_value + detune_mod + voice.pitch_current + voice.pitch_current_2;
            let instant_frequency = util::f32_midi_note_to_freq(base_note).min(nyquist);
            voice.phase_delta = instant_frequency / sample_rate;

            for (i, harmonic) in self.harmonics.iter_mut().enumerate() {
                if harmonic.amplitude != 0.0 {
                    let harmonic_freq = if harmonic.index == 0 {
                        instant_frequency
                    } else {
                        (harmonic.index as f32 + 1.0) * instant_frequency
                    };
                    let phase_increment = TAU * harmonic_freq / sample_rate;
                    voice.harmonic_phases[i] = (voice.harmonic_phases[i] + phase_increment) % TAU;
                    sample += fast_sine(voice.harmonic_phases[i]) * harmonic.amplitude;
                }
            }
        }

        sample
    }
}

// Moon Lander sine approximation w/ changed period and range
fn fast_sine(mut x: f32) -> f32 {
    x = x.rem_euclid(TAU);

    let sign = if x > PI { -1.0 } else { 1.0 };
    if x > PI {
        x -= PI;
    }
    if x > FRAC_PI_2 {
        x = PI - x;
    }

    let x2 = x * x;
    sign * x * (1.0 - x2 / 6.0 * (1.0 - x2 / 20.0))
}