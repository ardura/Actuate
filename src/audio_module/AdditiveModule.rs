use std::f32::consts::TAU;
use nih_plug::util;

use super::SingleVoice;

#[derive(Clone)]
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
            harmonics: {
                    let mut tmp = Vec::new();
                    tmp.push(AdditiveHarmonic {
                        index: 0,
                        amplitude: 1.0,
                    });
                    tmp
            },
        }
    }

    pub fn set_harmonics(&mut self, harmonics: Vec<AdditiveHarmonic>) {
        self.harmonics = harmonics;
    }

    pub fn next_sample(&mut self, voice: &mut SingleVoice, sample_rate: f32, detune_mod: f32, unison_voice: bool) -> f32 {
        let mut sample = 0.0;
        let nyquist = sample_rate / 2.0;
        for (i, harmonic) in self.harmonics.iter_mut().enumerate() {
            let base_note: f32;
            if unison_voice {
                base_note = voice.note as f32
                + voice._unison_detune_value
                + detune_mod
                + voice.pitch_current
                + voice.pitch_current_2;
            } else {
                base_note = voice.note as f32
                + voice._detune
                + detune_mod
                + voice.pitch_current
                + voice.pitch_current_2;
            }
            let instant_frequency = util::f32_midi_note_to_freq(base_note).min(nyquist);
            
            voice.phase_delta = instant_frequency / sample_rate;

            let harmonic_freq = if harmonic.index == 0 {
                instant_frequency
            } else {
                (harmonic.index as f32 + 1.0) * instant_frequency
            };
            let phase_increment = TAU * harmonic_freq / sample_rate;
            voice.harmonic_phases[i] = (voice.harmonic_phases[i] + phase_increment) % TAU;
            sample += voice.harmonic_phases[i].sin() * harmonic.amplitude;
        }

        sample
    }
}