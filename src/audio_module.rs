/*
Copyright (C) 2023 Ardura

This program is free software:
you can redistribute it and/or modify it under the terms of the GNU General Public License
as published by the Free Software Foundation,either version 3 of the License, or (at your option) any later version.

This program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY;
without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
See the GNU General Public License for more details.

You should have received a copy of the GNU General Public License along with this program.
If not, see https://www.gnu.org/licenses/.

#####################################

Audio Module by Ardura

This is intended to be a generic implementation that can be extended for other audio code as generators

#####################################
*/

use egui_file::{FileDialog, State};
use nih_plug::{
    params::enums::Enum, prelude::{NoteEvent, ParamSetter, Smoother, SmoothingStyle}, util
};
use nih_plug_egui::egui::{self, Pos2, Rect, RichText, Rounding, ScrollArea, Ui};
use pitch_shift::PitchShifter;
use rand::Rng;
use serde::{Deserialize, Serialize};
use AdditiveModule::{AdditiveHarmonic, AdditiveOscillator};
use std::{collections::VecDeque, f32::consts::SQRT_2, path::{Path, PathBuf}, sync::Arc};

// Audio module files
pub(crate) mod Oscillator;
pub(crate) mod frequency_modulation;
pub(crate) mod AdditiveModule;
use self::Oscillator::{DeterministicWhiteNoiseGenerator, OscState, RetriggerStyle, SmoothStyle};
use crate::{
    actuate_enums::StereoAlgorithm, ActuateParams, CustomWidgets::{ui_knob::{self, KnobLayout}, CustomVerticalSlider}, PitchRouting, DARK_GREY_UI_COLOR, FONT_COLOR, LIGHTER_GREY_UI_COLOR, MEDIUM_GREY_UI_COLOR, SMALLER_FONT, WIDTH, YELLOW_MUSTARD
};
use crate::{CustomWidgets::{BeizerButton::{self, ButtonLayout}, BoolButton}, DARKER_GREY_UI_COLOR};
use CustomVerticalSlider::ParamSlider as VerticalParamSlider;

// When you create a new audio module, you should add it here
#[derive(Debug, Enum, PartialEq, Clone, Copy, Serialize, Deserialize)]
pub enum AudioModuleType {
    Off,
    Sampler,
    Granulizer,
    Additive,
    Sine,       // These Osc values are added as of the generator dropdown menu stuff
    Tri,
    Saw,
    RSaw,
    WSaw,
    SSaw,
    RASaw,
    Ramp,
    Square,
    RSquare,
    Pulse,
    Noise,
    UnsetAm,
}

#[derive(Clone)]
struct VoiceVec {
    /// The identifier for this voice
    voices: VecDeque<SingleVoice>,
}

// This is the information used to track voices and midi inputs
// I made single voice a struct so that we can add and remove via struct instead of running
// into any threading issues trying to modify different vecs in the same function rather than 1 struct
// Underscores are to get rid of the compiler warning thinking it's not used but it's stored for debugging or passed between structs
// and still functional.
#[derive(Clone)]
pub struct SingleVoice {
    /// The note's key/note, in `0..128`. Only used for the voice terminated event.
    note: u8,
    /// Velocity of our note
    _velocity: f32,
    /// Mod amount for velocity inputted to this AM
    vel_mod_amount: f32,
    /// The voice's current phase.
    phase: f32,
    /// The phase increment. This is based on the voice's frequency, derived from the note index.
    phase_delta: f32,
    /// Oscillator state for amplitude controlling
    state: Oscillator::OscState,
    // These are the attack and release smoothers
    amp_current: f32,
    osc_attack: Smoother<f32>,
    osc_decay: Smoother<f32>,
    osc_release: Smoother<f32>,
    // Pitch modulation info
    pitch_enabled: bool,
    pitch_current: f32,
    pitch_env_peak: f32,
    pitch_state: Oscillator::OscState,
    pitch_attack: Smoother<f32>,
    pitch_decay: Smoother<f32>,
    pitch_release: Smoother<f32>,
    // Pitch modulation info 2
    pitch_enabled_2: bool,
    pitch_current_2: f32,
    pitch_env_peak_2: f32,
    pitch_state_2: Oscillator::OscState,
    pitch_attack_2: Smoother<f32>,
    pitch_decay_2: Smoother<f32>,
    pitch_release_2: Smoother<f32>,
    // Final info for a note to work
    _detune: f32,
    _unison_detune_value: f32,
    frequency: f32,
    _attack_time: f32,
    _decay_time: f32,
    _release_time: f32,
    _retrigger: RetriggerStyle,
    _voice_type: AudioModuleType,

    // This is only used for unison detunes
    _angle: f32,

    // Sampler/Granulizer Pos
    sample_pos: usize,
    loop_it: bool,
    grain_start_pos: usize,
    _granular_hold: i32,
    _granular_gap: i32,
    granular_hold_end: usize,
    next_grain_pos: usize,
    _end_position: usize,
    _granular_crossfade: i32,
    grain_attack: Smoother<f32>,
    grain_release: Smoother<f32>,
    grain_state: GrainState,

    // Additive
    harmonic_phases: Vec<f32>,
}

#[derive(Enum, PartialEq, Clone, Copy, Serialize, Deserialize)]
enum GrainState {
    Attacking,
    Releasing,
}

#[derive(Clone)]
pub struct AudioModule {
    // Stored sample rate in case the audio module needs it
    sample_rate: f32,
    pub audio_module_type: AudioModuleType,

    // This flipflops stereo with 2 voices to make it make some sense to our ears
    two_voice_stereo_flipper: bool,

    ///////////////////////////////////////////////////////////
    // Audio modules should go here as a struct
    //
    // Osc doesn't have one though
    //
    ///////////////////////////////////////////////////////////

    // Granulizer/Sampler
    pub loaded_sample: Vec<Vec<f32>>,
    // Hold calculated notes
    pub sample_lib: Vec<Vec<Vec<f32>>>,
    // Treat this like a wavetable synth would
    pub loop_wavetable: bool,
    // Shift notes like a single cycle - aligned wth 3xosc
    pub single_cycle: bool,
    // Restretch length with tracking bool
    pub restretch: bool,
    pub prev_restretch: bool,

    // Granulizer other options
    pub start_position: f32,
    pub _end_position: f32,
    pub grain_hold: i32,
    pub grain_gap: i32,
    pub grain_crossfade: i32,

    ///////////////////////////////////////////////////////////

    // Stored params from main lib here on a per-module basis

    // Osc module knob storage
    pub osc_octave: i32,
    pub osc_semitones: i32,
    pub osc_detune: f32,
    pub osc_attack: f32,
    pub osc_decay: f32,
    pub osc_sustain: f32,
    pub osc_release: f32,
    pub osc_mod_amount: f32,
    pub osc_retrigger: RetriggerStyle,
    pub osc_atk_curve: SmoothStyle,
    pub osc_dec_curve: SmoothStyle,
    pub osc_rel_curve: SmoothStyle,
    pub osc_unison: i32,
    pub osc_unison_detune: f32,
    pub osc_stereo: f32,

    // Voice storage
    playing_voices: VoiceVec,
    unison_voices: VoiceVec,

    // Tracking stopping voices too
    is_playing: bool,

    // Noise variables
    noise_obj: Oscillator::DeterministicWhiteNoiseGenerator,

    // Pitch mod storage
    pitch_enable: bool,
    pitch_env_peak: f32,
    pitch_env_attack: f32,
    pitch_env_decay: f32,
    pitch_env_sustain: f32,
    pitch_env_release: f32,
    pitch_env_atk_curve: SmoothStyle,
    pitch_env_dec_curve: SmoothStyle,
    pitch_env_rel_curve: SmoothStyle,
    pitch_enable_2: bool,
    pitch_env_peak_2: f32,
    pitch_env_attack_2: f32,
    pitch_env_decay_2: f32,
    pitch_env_sustain_2: f32,
    pitch_env_release_2: f32,
    pitch_env_atk_curve_2: SmoothStyle,
    pitch_env_dec_curve_2: SmoothStyle,
    pitch_env_rel_curve_2: SmoothStyle,

    // Additive Engine
    additive_module: AdditiveModule::AdditiveOscillator,

    // Additive param storage
    pub ah0: f32,
    pub ah1: f32,
    pub ah2: f32,
    pub ah3: f32,
    pub ah4: f32,
    pub ah5: f32,
    pub ah6: f32,
    pub ah7: f32,
    pub ah8: f32,
    pub ah9: f32,
    pub ah10: f32,
    pub ah11: f32,
    pub ah12: f32,
    pub ah13: f32,
    pub ah14: f32,
    pub ah15: f32,

    // Previous additive param
    // Huge CPU saver with this here in return for more storage
    pub prev_ah0: f32,
    pub prev_ah1: f32,
    pub prev_ah2: f32,
    pub prev_ah3: f32,
    pub prev_ah4: f32,
    pub prev_ah5: f32,
    pub prev_ah6: f32,
    pub prev_ah7: f32,
    pub prev_ah8: f32,
    pub prev_ah9: f32,
    pub prev_ah10: f32,
    pub prev_ah11: f32,
    pub prev_ah12: f32,
    pub prev_ah13: f32,
    pub prev_ah14: f32,
    pub prev_ah15: f32,
}

// When you create a new audio module you need to add its default creation here as well
#[allow(overflowing_literals)]
impl Default for AudioModule {
    fn default() -> Self {
        Self {
            // Audio modules will use these
            sample_rate: 44100.0,
            audio_module_type: AudioModuleType::Sine,
            two_voice_stereo_flipper: true,

            // Granulizer/Sampler
            loaded_sample: vec![vec![0.0, 0.0]],
            sample_lib: vec![vec![vec![0.0, 0.0]]], //Vec<Vec<Vec<f32>>>
            loop_wavetable: false,
            single_cycle: false,
            restretch: true,
            prev_restretch: false,
            start_position: 0.0,
            _end_position: 1.0,
            grain_hold: 200,
            grain_gap: 200,
            grain_crossfade: 50,

            // Osc module knob storage
            osc_octave: 0,
            osc_semitones: 0,
            osc_detune: 0.0,
            osc_attack: 0.0001,
            osc_decay: 0.0001,
            osc_sustain: 999.9,
            osc_release: 0.07,
            osc_mod_amount: 0.0,
            osc_retrigger: RetriggerStyle::Free,
            osc_atk_curve: SmoothStyle::Linear,
            osc_rel_curve: SmoothStyle::Linear,
            osc_dec_curve: SmoothStyle::Linear,
            osc_unison: 1,
            osc_unison_detune: 0.0,
            osc_stereo: 1.0,

            // Voice storage
            playing_voices: VoiceVec {
                voices: VecDeque::new(),
            },
            unison_voices: VoiceVec {
                voices: VecDeque::new(),
            },

            // Tracking stopping voices
            is_playing: false,

            // Noise variables
            noise_obj: DeterministicWhiteNoiseGenerator::new(371722539),

            // Pitch mod storage
            pitch_enable: false,
            pitch_env_peak: 0.0,
            pitch_env_attack: 0.0,
            pitch_env_decay: 300.0,
            pitch_env_sustain: 0.0,
            pitch_env_release: 0.0,
            pitch_env_atk_curve: SmoothStyle::Linear,
            pitch_env_dec_curve: SmoothStyle::Linear,
            pitch_env_rel_curve: SmoothStyle::Linear,

            pitch_enable_2: false,
            pitch_env_peak_2: 0.0,
            pitch_env_attack_2: 0.0,
            pitch_env_decay_2: 300.0,
            pitch_env_sustain_2: 0.0,
            pitch_env_release_2: 0.0,
            pitch_env_atk_curve_2: SmoothStyle::Linear,
            pitch_env_dec_curve_2: SmoothStyle::Linear,
            pitch_env_rel_curve_2: SmoothStyle::Linear,

            // Additive Engine
            additive_module: AdditiveOscillator::default(),
            ah0: 0.0,
            ah1: 0.0,
            ah2: 0.0,
            ah3: 0.0,
            ah4: 0.0,
            ah5: 0.0,
            ah6: 0.0,
            ah7: 0.0,
            ah8: 0.0,
            ah9: 0.0,
            ah10: 0.0,
            ah11: 0.0,
            ah12: 0.0,
            ah13: 0.0,
            ah14: 0.0,
            ah15: 0.0,

            prev_ah0: 0.0,
            prev_ah1: 0.0,
            prev_ah2: 0.0,
            prev_ah3: 0.0,
            prev_ah4: 0.0,
            prev_ah5: 0.0,
            prev_ah6: 0.0,
            prev_ah7: 0.0,
            prev_ah8: 0.0,
            prev_ah9: 0.0,
            prev_ah10: 0.0,
            prev_ah11: 0.0,
            prev_ah12: 0.0,
            prev_ah13: 0.0,
            prev_ah14: 0.0,
            prev_ah15: 0.0,
        }
    }
}

impl AudioModule {
    // Passing the params here is not the nicest thing but we have to move things around to get past the threading stuff + egui's gui separation
    pub fn draw_module(
        ui: &mut Ui,
        egui_ctx: &nih_plug_egui::egui::Context,
        setter: &ParamSetter<'_>,
        params: Arc<ActuateParams>,
        dialog: &mut FileDialog,
        index: u8,
        module1: &Arc<std::sync::Mutex<AudioModule>>,
        module2: &Arc<std::sync::Mutex<AudioModule>>,
        module3: &Arc<std::sync::Mutex<AudioModule>>,
    ) {
        let am_type;
        let osc_retrigger;
        let osc_octave;
        let osc_semitones;
        let osc_stereo;
        let osc_unison;
        let osc_detune;
        let osc_unison_detune;
        let osc_attack;
        let osc_decay;
        let osc_sustain;
        let osc_release;
        let osc_atk_curve;
        let osc_dec_curve;
        let osc_rel_curve;
        let load_sample;
        let restretch;
        let loop_sample;
        let single_cycle;
        let start_position;
        let end_position;
        let grain_crossfade;
        let grain_hold;
        let grain_gap;
        let additive_harmonic_0;
        let additive_harmonic_1;
        let additive_harmonic_2;
        let additive_harmonic_3;
        let additive_harmonic_4;
        let additive_harmonic_5;
        let additive_harmonic_6;
        let additive_harmonic_7;
        let additive_harmonic_8;
        let additive_harmonic_9;
        let additive_harmonic_10;
        let additive_harmonic_11;
        let additive_harmonic_12;
        let additive_harmonic_13;
        let additive_harmonic_14;
        let additive_harmonic_15;
        match index {
            1 => {
                am_type = &params.audio_module_1_type;
                osc_retrigger = &params.osc_1_retrigger;
                osc_octave = &params.osc_1_octave;
                osc_semitones = &params.osc_1_semitones;
                osc_stereo = &params.osc_1_stereo;
                osc_unison = &params.osc_1_unison;
                osc_detune = &params.osc_1_detune;
                osc_unison_detune = &params.osc_1_unison_detune;
                osc_attack = &params.osc_1_attack;
                osc_decay = &params.osc_1_decay;
                osc_sustain = &params.osc_1_sustain;
                osc_release = &params.osc_1_release;
                osc_atk_curve = &params.osc_1_atk_curve;
                osc_dec_curve = &params.osc_1_dec_curve;
                osc_rel_curve = &params.osc_1_rel_curve;
                load_sample = &params.load_sample_1;
                restretch = &params.restretch_1;
                loop_sample = &params.loop_sample_1;
                single_cycle = &params.single_cycle_1;
                start_position = &params.start_position_1;
                end_position = &params.end_position_1;
                grain_crossfade = &params.grain_crossfade_1;
                grain_hold = &params.grain_hold_1;
                grain_gap = &params.grain_gap_1;
                additive_harmonic_0 = &params.additive_amp_1_0;
                additive_harmonic_1 = &params.additive_amp_1_1;
                additive_harmonic_2 = &params.additive_amp_1_2;
                additive_harmonic_3 = &params.additive_amp_1_3;
                additive_harmonic_4 = &params.additive_amp_1_4;
                additive_harmonic_5 = &params.additive_amp_1_5;
                additive_harmonic_6 = &params.additive_amp_1_6;
                additive_harmonic_7 = &params.additive_amp_1_7;
                additive_harmonic_8 = &params.additive_amp_1_8;
                additive_harmonic_9 = &params.additive_amp_1_9;
                additive_harmonic_10 = &params.additive_amp_1_10;
                additive_harmonic_11 = &params.additive_amp_1_11;
                additive_harmonic_12 = &params.additive_amp_1_12;
                additive_harmonic_13 = &params.additive_amp_1_13;
                additive_harmonic_14 = &params.additive_amp_1_14;
                additive_harmonic_15 = &params.additive_amp_1_15;
            },
            2 => {
                am_type = &params.audio_module_2_type;
                osc_retrigger = &params.osc_2_retrigger;
                osc_octave = &params.osc_2_octave;
                osc_semitones = &params.osc_2_semitones;
                osc_stereo = &params.osc_2_stereo;
                osc_unison = &params.osc_2_unison;
                osc_detune = &params.osc_2_detune;
                osc_unison_detune = &params.osc_2_unison_detune;
                osc_attack = &params.osc_2_attack;
                osc_decay = &params.osc_2_decay;
                osc_sustain = &params.osc_2_sustain;
                osc_release = &params.osc_2_release;
                osc_atk_curve = &params.osc_2_atk_curve;
                osc_dec_curve = &params.osc_2_dec_curve;
                osc_rel_curve = &params.osc_2_rel_curve;
                load_sample = &params.load_sample_2;
                restretch = &params.restretch_2;
                loop_sample = &params.loop_sample_2;
                single_cycle = &params.single_cycle_2;
                start_position = &params.start_position_2;
                end_position = &params.end_position_2;
                grain_crossfade = &params.grain_crossfade_2;
                grain_hold = &params.grain_hold_2;
                grain_gap = &params.grain_gap_2;
                additive_harmonic_0 = &params.additive_amp_2_0;
                additive_harmonic_1 = &params.additive_amp_2_1;
                additive_harmonic_2 = &params.additive_amp_2_2;
                additive_harmonic_3 = &params.additive_amp_2_3;
                additive_harmonic_4 = &params.additive_amp_2_4;
                additive_harmonic_5 = &params.additive_amp_2_5;
                additive_harmonic_6 = &params.additive_amp_2_6;
                additive_harmonic_7 = &params.additive_amp_2_7;
                additive_harmonic_8 = &params.additive_amp_2_8;
                additive_harmonic_9 = &params.additive_amp_2_9;
                additive_harmonic_10 = &params.additive_amp_2_10;
                additive_harmonic_11 = &params.additive_amp_2_11;
                additive_harmonic_12 = &params.additive_amp_2_12;
                additive_harmonic_13 = &params.additive_amp_2_13;
                additive_harmonic_14 = &params.additive_amp_2_14;
                additive_harmonic_15 = &params.additive_amp_2_15;
            },
            3 => {
                am_type = &params.audio_module_3_type;
                osc_retrigger = &params.osc_3_retrigger;
                osc_octave = &params.osc_3_octave;
                osc_semitones = &params.osc_3_semitones;
                osc_stereo = &params.osc_3_stereo;
                osc_unison = &params.osc_3_unison;
                osc_detune = &params.osc_3_detune;
                osc_unison_detune = &params.osc_3_unison_detune;
                osc_attack = &params.osc_3_attack;
                osc_decay = &params.osc_3_decay;
                osc_sustain = &params.osc_3_sustain;
                osc_release = &params.osc_3_release;
                osc_atk_curve = &params.osc_3_atk_curve;
                osc_dec_curve = &params.osc_3_dec_curve;
                osc_rel_curve = &params.osc_3_rel_curve;
                load_sample = &params.load_sample_3;
                restretch = &params.restretch_3;
                loop_sample = &params.loop_sample_3;
                single_cycle = &params.single_cycle_3;
                start_position = &params.start_position_3;
                end_position = &params.end_position_3;
                grain_crossfade = &params.grain_crossfade_3;
                grain_hold = &params.grain_hold_3;
                grain_gap = &params.grain_gap_3;
                additive_harmonic_0 = &params.additive_amp_3_0;
                additive_harmonic_1 = &params.additive_amp_3_1;
                additive_harmonic_2 = &params.additive_amp_3_2;
                additive_harmonic_3 = &params.additive_amp_3_3;
                additive_harmonic_4 = &params.additive_amp_3_4;
                additive_harmonic_5 = &params.additive_amp_3_5;
                additive_harmonic_6 = &params.additive_amp_3_6;
                additive_harmonic_7 = &params.additive_amp_3_7;
                additive_harmonic_8 = &params.additive_amp_3_8;
                additive_harmonic_9 = &params.additive_amp_3_9;
                additive_harmonic_10 = &params.additive_amp_3_10;
                additive_harmonic_11 = &params.additive_amp_3_11;
                additive_harmonic_12 = &params.additive_amp_3_12;
                additive_harmonic_13 = &params.additive_amp_3_13;
                additive_harmonic_14 = &params.additive_amp_3_14;
                additive_harmonic_15 = &params.additive_amp_3_15;
            },
            #[allow(unreachable_code)]
            _ => !unreachable!(),
        }

        const VERT_BAR_HEIGHT: f32 = 76.0;
        const VERT_LARGER: f32 = 78.0;
        const VERT_BAR_WIDTH: f32 = 12.0;
        const DISABLED_SPACE: f32 = 104.0;

        match am_type.value() {
            AudioModuleType::UnsetAm => {
                ui.label("UNSET - Err");
            }
            AudioModuleType::Off => {
                // Blank space
                ui.label("Disabled");
                ui.add_space(DISABLED_SPACE);
            }
            AudioModuleType::Sine |
            AudioModuleType::Tri |
            AudioModuleType::Saw |
            AudioModuleType::RSaw |
            AudioModuleType::WSaw |
            AudioModuleType::SSaw |
            AudioModuleType::RASaw |
            AudioModuleType::Ramp |
            AudioModuleType::Square |
            AudioModuleType::RSquare |
            AudioModuleType::Pulse |
            AudioModuleType::Noise => {
                const KNOB_SIZE: f32 = 22.0;
                const TEXT_SIZE: f32 = 10.0;
                // Oscillator
                ui.vertical(|ui| {
                    ui.add_space(1.0);
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            /*let osc_1_type_knob = ui_knob::ArcKnob::for_param(
                                _osc_voice,
                                setter,
                                KNOB_SIZE,
                                KnobLayout::Horizonal,
                            )
                            .preset_style(ui_knob::KnobStyle::Preset1)
                            .set_fill_color(DARK_GREY_UI_COLOR)
                            .set_line_color(YELLOW_MUSTARD)
                            .set_text_size(TEXT_SIZE)
                            .set_hover_text("Oscillator wave form type".to_string());
                            ui.add(osc_1_type_knob);
                            */

                            let osc_1_retrigger_knob = ui_knob::ArcKnob::for_param(
                                osc_retrigger,
                                setter,
                                KNOB_SIZE,
                                KnobLayout::Horizonal,
                            )
                            .preset_style(ui_knob::KnobStyle::Preset1)
                            .set_fill_color(DARK_GREY_UI_COLOR)
                            .set_line_color(YELLOW_MUSTARD)
                            .use_outline(true)
                            .set_text_size(TEXT_SIZE)
                            .set_hover_text("Retrigger behavior on MIDI note input:
Free: constantly running phase based off previous note
Retrigger: wave form restarts at every new note
Random: Wave and all unisons use a new random phase every note
UniRandom: Every voice uses its own unique random phase every note".to_string());
                            ui.add(osc_1_retrigger_knob);
                        });

                        ui.vertical(|ui| {
                            let osc_1_octave_knob = ui_knob::ArcKnob::for_param(
                                osc_octave,
                                setter,
                                KNOB_SIZE,
                                KnobLayout::Horizonal,
                            )
                            .preset_style(ui_knob::KnobStyle::Preset1)
                            .set_fill_color(DARK_GREY_UI_COLOR)
                            .set_line_color(YELLOW_MUSTARD)
                            .use_outline(true)
                            .set_text_size(TEXT_SIZE)
                            .set_hover_text("Adjust the MIDI input by octave".to_string());
                            ui.add(osc_1_octave_knob);

                            let osc_1_semitones_knob = ui_knob::ArcKnob::for_param(
                                osc_semitones,
                                setter,
                                KNOB_SIZE,
                                KnobLayout::Horizonal,
                            )
                            .preset_style(ui_knob::KnobStyle::Preset1)
                            .set_fill_color(DARK_GREY_UI_COLOR)
                            .set_line_color(YELLOW_MUSTARD)
                            .use_outline(true)
                            .set_text_size(TEXT_SIZE)
                            .set_hover_text("Adjust the MIDI input by semitone".to_string());
                            ui.add(osc_1_semitones_knob);
                        });

                        ui.vertical(|ui| {
                            let osc_1_stereo_knob = ui_knob::ArcKnob::for_param(
                                osc_stereo,
                                setter,
                                KNOB_SIZE,
                                KnobLayout::Horizonal,
                            )
                            .preset_style(ui_knob::KnobStyle::Preset1)
                            .set_fill_color(DARK_GREY_UI_COLOR)
                            .set_line_color(YELLOW_MUSTARD)
                            .use_outline(true)
                            .set_text_size(TEXT_SIZE)
                            .set_hover_text("Oscillator voice stereo spread. 0 is Mono.".to_string());
                            ui.add(osc_1_stereo_knob);

                            let osc_1_unison_knob = ui_knob::ArcKnob::for_param(
                                osc_unison,
                                setter,
                                KNOB_SIZE,
                                KnobLayout::Horizonal,
                            )
                            .preset_style(ui_knob::KnobStyle::Preset1)
                            .set_fill_color(DARK_GREY_UI_COLOR)
                            .set_line_color(YELLOW_MUSTARD.gamma_multiply(2.0))
                            .use_outline(true)
                            .set_text_size(TEXT_SIZE)
                            .set_hover_text("How many voices should play in unison".to_string());
                            ui.add(osc_1_unison_knob);
                        });

                        ui.vertical(|ui| {
                            let osc_1_detune_knob = ui_knob::ArcKnob::for_param(
                                osc_detune,
                                setter,
                                KNOB_SIZE,
                                KnobLayout::Horizonal,
                            )
                            .preset_style(ui_knob::KnobStyle::Preset1)
                            .set_fill_color(DARK_GREY_UI_COLOR)
                            .set_line_color(YELLOW_MUSTARD)
                            .use_outline(true)
                            .set_text_size(TEXT_SIZE)
                            .set_hover_text("Move the pitch to fine tune it".to_string());
                            ui.add(osc_1_detune_knob);

                            let osc_1_unison_detune_knob = ui_knob::ArcKnob::for_param(
                                osc_unison_detune,
                                setter,
                                KNOB_SIZE,
                                KnobLayout::Horizonal,
                            )
                            .preset_style(ui_knob::KnobStyle::Preset1)
                            .set_fill_color(DARK_GREY_UI_COLOR)
                            .set_line_color(YELLOW_MUSTARD.gamma_multiply(2.0))
                            .use_outline(true)
                            .set_text_size(TEXT_SIZE)
                            .set_hover_text("Spread the pitches of the unison voices apart".to_string());
                            ui.add(osc_1_unison_detune_knob);
                        });

                        // Trying to draw background box as rect
                        ui.painter().rect_filled(
                            Rect::from_two_pos(
                                Pos2 {
                                    x: ui.cursor().left_top().x - 4.0,
                                    y: ui.cursor().left_top().y - 4.0,
                                },
                                Pos2 {
                                    x: ui.cursor().left_top().x + VERT_BAR_WIDTH * 6.0 + 8.0,
                                    y: ui.cursor().left_top().y + VERT_BAR_HEIGHT + 12.0 + 8.0,
                                },
                            ),
                            Rounding::from(4.0),
                            DARKER_GREY_UI_COLOR,
                        );
                        ui.add_space(2.0);

                        // ADSR
                        ui.add(
                            VerticalParamSlider::for_param(osc_attack, setter)
                                .with_width(VERT_BAR_WIDTH)
                                .with_height(VERT_BAR_HEIGHT)
                                .set_reversed(true)
                                .override_colors(LIGHTER_GREY_UI_COLOR, YELLOW_MUSTARD),
                        );
                        ui.add(
                            VerticalParamSlider::for_param(osc_decay, setter)
                                .with_width(VERT_BAR_WIDTH)
                                .with_height(VERT_BAR_HEIGHT)
                                .set_reversed(true)
                                .override_colors(LIGHTER_GREY_UI_COLOR, YELLOW_MUSTARD),
                        );
                        ui.add(
                            VerticalParamSlider::for_param(osc_sustain, setter)
                                .with_width(VERT_BAR_WIDTH)
                                .with_height(VERT_BAR_HEIGHT)
                                .set_reversed(true)
                                .override_colors(LIGHTER_GREY_UI_COLOR, YELLOW_MUSTARD),
                        );
                        ui.add(
                            VerticalParamSlider::for_param(osc_release, setter)
                                .with_width(VERT_BAR_WIDTH)
                                .with_height(VERT_BAR_HEIGHT)
                                .set_reversed(true)
                                .override_colors(LIGHTER_GREY_UI_COLOR, YELLOW_MUSTARD),
                        );

                        // Curves
                        ui.vertical(|ui|{
                            ui.add(
                                BeizerButton::BeizerButton::for_param(
                                    osc_atk_curve,
                                    setter,
                                    //3.2,
                                    //5.3,
                                    5.1,
                                    1.66,
                                    ButtonLayout::HorizontalInline,
                                    true,
                                )
                                .with_background_color(MEDIUM_GREY_UI_COLOR)
                                .with_line_color(YELLOW_MUSTARD),
                            ).on_hover_text_at_pointer("The behavior of Attack movement in the envelope".to_string());
                            ui.add(
                                BeizerButton::BeizerButton::for_param(
                                    osc_dec_curve,
                                    setter,
                                    5.1,
                                    1.66,
                                    ButtonLayout::HorizontalInline,
                                    false,
                                )
                                .with_background_color(MEDIUM_GREY_UI_COLOR)
                                .with_line_color(YELLOW_MUSTARD),
                            ).on_hover_text_at_pointer("The behavior of Decay movement in the envelope".to_string());
                            ui.add(
                                BeizerButton::BeizerButton::for_param(
                                    osc_rel_curve,
                                    setter,
                                    5.1,
                                    1.66,
                                    ButtonLayout::HorizontalInline,
                                    false,
                                )
                                .with_background_color(MEDIUM_GREY_UI_COLOR)
                                .with_line_color(YELLOW_MUSTARD),
                            ).on_hover_text_at_pointer("The behavior of Release movement in the envelope".to_string());
                        });
                    });
                });
                ui.add_space(20.0);
            }
            AudioModuleType::Sampler => {
                const KNOB_SIZE: f32 = 22.0;
                const TEXT_SIZE: f32 = 10.0;
                // Even up with OSC spacing
                ui.add_space(1.0);
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        let load_sample_boolButton = BoolButton::BoolButton::for_param(load_sample, setter, 3.5, 1.0, SMALLER_FONT);
                        if ui.add(load_sample_boolButton).clicked() || params.load_sample_1.value() || params.load_sample_2.value() || params.load_sample_3.value() {
                            dialog.open();
                            let mut dvar = Some(dialog);
                            
                            if let Some(dialog) = &mut dvar {
                                if dialog.show(egui_ctx).selected() {
                                  if let Some(file) = dialog.path() {
                                    let opened_file = Some(file.to_path_buf());
                                    if Option::is_some(&opened_file) {
                                        match index {
                                            1 => {
                                                if params.load_sample_1.value() {
                                                    module1
                                                    .lock()
                                                    .unwrap()
                                                    .load_new_sample(opened_file.unwrap());
                                                    *params.am1_sample.lock().unwrap() = module1.lock().unwrap().loaded_sample.clone();
                                                    setter.set_parameter(&params.load_sample_1, false);
                                                    dialog.set_path(dialog.directory().to_path_buf());
                                                    //dialog.deselect();
                                                }
                                            },
                                            2 => {
                                                if params.load_sample_2.value() {
                                                    module2
                                                        .lock()
                                                        .unwrap()
                                                        .load_new_sample(opened_file.unwrap());
                                                    *params.am2_sample.lock().unwrap() = module2.lock().unwrap().loaded_sample.clone();
                                                    setter.set_parameter(&params.load_sample_2, false);
                                                    dialog.set_path(dialog.directory().to_path_buf());
                                                    //dialog.deselect();
                                                }
                                            },
                                            3 => {
                                                if params.load_sample_3.value() {
                                                    module3
                                                        .lock()
                                                        .unwrap()
                                                        .load_new_sample(opened_file.unwrap());
                                                    *params.am3_sample.lock().unwrap() = module3.lock().unwrap().loaded_sample.clone();
                                                    setter.set_parameter(&params.load_sample_3, false);
                                                    dialog.set_path(dialog.directory().to_path_buf());
                                                    //dialog.deselect();
                                                }
                                            },
                                            _ => {}
                                        }
                                    }
                                  }
                                }
                                match dialog.state() {
                                    State::Cancelled | State::Closed => {
                                        match index {
                                            1 => {
                                                setter.set_parameter(&params.load_sample_1, false);
                                                dialog.set_path(dialog.directory().to_path_buf());
                                                //dialog.deselect();
                                            },
                                            2 => {
                                                setter.set_parameter(&params.load_sample_2, false);
                                                dialog.set_path(dialog.directory().to_path_buf());
                                                //dialog.deselect();
                                            },
                                            3 => {
                                                setter.set_parameter(&params.load_sample_3, false);
                                                dialog.set_path(dialog.directory().to_path_buf());
                                                //dialog.deselect();
                                            },
                                            _ => {}
                                        }
                                    },
                                    _ => {}
                                }
                            }
                        }
                        let restretch_button = BoolButton::BoolButton::for_param(restretch, setter, 3.5, 1.0, SMALLER_FONT);
                        ui.add(restretch_button);
                        let loop_toggle = BoolButton::BoolButton::for_param(loop_sample, setter, 3.5, 1.0, SMALLER_FONT);
                        ui.add(loop_toggle);
                        let sc_toggle = BoolButton::BoolButton::for_param(single_cycle, setter, 3.5, 1.0, SMALLER_FONT);
                        ui.add(sc_toggle);
                    });
                    ui.vertical(|ui| {
                        let osc_1_octave_knob = ui_knob::ArcKnob::for_param(
                            osc_octave,
                            setter,
                            KNOB_SIZE,
                            KnobLayout::Horizonal,
                        )
                        .preset_style(ui_knob::KnobStyle::Preset1)
                        .set_fill_color(DARK_GREY_UI_COLOR)
                        .set_line_color(YELLOW_MUSTARD)
                        .use_outline(true)
                        .set_text_size(TEXT_SIZE)
                        .set_hover_text("Adjust the MIDI input by octave".to_string());
                        ui.add(osc_1_octave_knob);
                        let osc_1_retrigger_knob = ui_knob::ArcKnob::for_param(
                            osc_retrigger,
                            setter,
                            KNOB_SIZE,
                            KnobLayout::Horizonal,
                        )
                        .preset_style(ui_knob::KnobStyle::Preset1)
                        .set_fill_color(DARK_GREY_UI_COLOR)
                        .set_line_color(YELLOW_MUSTARD)
                        .use_outline(true)
                        .set_text_size(TEXT_SIZE)
                        .set_hover_text("Retrigger behavior on MIDI note input:
Retrigger: Sample restarts at every new note
Random: Sample uses a new random position every note".to_string());
                        ui.add(osc_1_retrigger_knob);
                    });
                    ui.vertical(|ui| {
                        let osc_1_semitones_knob = ui_knob::ArcKnob::for_param(
                            osc_semitones,
                            setter,
                            KNOB_SIZE,
                            KnobLayout::Horizonal,
                        )
                        .preset_style(ui_knob::KnobStyle::Preset1)
                        .set_fill_color(DARK_GREY_UI_COLOR)
                        .set_line_color(YELLOW_MUSTARD)
                        .use_outline(true)
                        .set_text_size(TEXT_SIZE)
                        .set_hover_text("Adjust the MIDI input by semitone".to_string());
                        ui.add(osc_1_semitones_knob);
                        let osc_1_unison_knob = ui_knob::ArcKnob::for_param(
                            osc_unison,
                            setter,
                            KNOB_SIZE,
                            KnobLayout::Horizonal,
                        )
                        .preset_style(ui_knob::KnobStyle::Preset1)
                        .set_fill_color(DARK_GREY_UI_COLOR)
                        .set_line_color(YELLOW_MUSTARD.gamma_multiply(2.0))
                        .use_outline(true)
                        .set_text_size(TEXT_SIZE)
                        .set_hover_text("How many voices should play in unison".to_string());
                        ui.add(osc_1_unison_knob);
                    });
                    ui.vertical(|ui| {
                        let start_position_1_knob = ui_knob::ArcKnob::for_param(
                            start_position,
                            setter,
                            KNOB_SIZE,
                            KnobLayout::Horizonal,
                        )
                        .preset_style(ui_knob::KnobStyle::Preset1)
                        .set_fill_color(DARK_GREY_UI_COLOR)
                        .set_line_color(YELLOW_MUSTARD)
                        .set_text_size(TEXT_SIZE)
                        .set_hover_text("Where the sample should start".to_string());
                        ui.add(start_position_1_knob);
                        let end_position_1_knob = ui_knob::ArcKnob::for_param(
                            end_position,
                            setter,
                            KNOB_SIZE,
                            KnobLayout::Horizonal,
                        )
                        .preset_style(ui_knob::KnobStyle::Preset1)
                        .set_fill_color(DARK_GREY_UI_COLOR)
                        .set_line_color(YELLOW_MUSTARD)
                        .set_text_size(TEXT_SIZE)
                        .set_hover_text("Where the sample should end".to_string());
                        ui.add(end_position_1_knob);
                    });
                    // Trying to draw background box as rect
                    ui.painter().rect_filled(
                        Rect::from_two_pos(
                            Pos2 {
                                x: ui.cursor().left_top().x - 4.0,
                                y: ui.cursor().left_top().y - 4.0,
                            },
                            Pos2 {
                                x: ui.cursor().left_top().x + VERT_BAR_WIDTH * 6.0 + 8.0,
                                y: ui.cursor().left_top().y + VERT_BAR_HEIGHT + 12.0 + 8.0,
                            },
                        ),
                        Rounding::from(4.0),
                        DARKER_GREY_UI_COLOR,
                    );
                    ui.add_space(2.0);
                    // ADSR
                    ui.add(
                        VerticalParamSlider::for_param(osc_attack, setter)
                            .with_width(VERT_BAR_WIDTH)
                            .with_height(VERT_BAR_HEIGHT)
                            .set_reversed(true)
                            .override_colors(LIGHTER_GREY_UI_COLOR, YELLOW_MUSTARD),
                    );
                    ui.add(
                        VerticalParamSlider::for_param(osc_decay, setter)
                            .with_width(VERT_BAR_WIDTH)
                            .with_height(VERT_BAR_HEIGHT)
                            .set_reversed(true)
                            .override_colors(LIGHTER_GREY_UI_COLOR, YELLOW_MUSTARD),
                    );
                    ui.add(
                        VerticalParamSlider::for_param(osc_sustain, setter)
                            .with_width(VERT_BAR_WIDTH)
                            .with_height(VERT_BAR_HEIGHT)
                            .set_reversed(true)
                            .override_colors(LIGHTER_GREY_UI_COLOR, YELLOW_MUSTARD),
                    );
                    ui.add(
                        VerticalParamSlider::for_param(osc_release, setter)
                            .with_width(VERT_BAR_WIDTH)
                            .with_height(VERT_BAR_HEIGHT)
                            .set_reversed(true)
                            .override_colors(LIGHTER_GREY_UI_COLOR, YELLOW_MUSTARD),
                    );
                    // Curves
                    ui.vertical(|ui|{
                        ui.add(
                            BeizerButton::BeizerButton::for_param(
                                osc_atk_curve,
                                setter,
                                5.1,
                                1.66,
                                ButtonLayout::HorizontalInline,
                                true,
                            )
                            .with_background_color(MEDIUM_GREY_UI_COLOR)
                            .with_line_color(YELLOW_MUSTARD),
                        ).on_hover_text_at_pointer("The behavior of Attack movement in the envelope".to_string());
                        ui.add(
                            BeizerButton::BeizerButton::for_param(
                                osc_dec_curve,
                                setter,
                                5.1,
                                1.66,
                                ButtonLayout::HorizontalInline,
                                false,
                            )
                            .with_background_color(MEDIUM_GREY_UI_COLOR)
                            .with_line_color(YELLOW_MUSTARD),
                        ).on_hover_text_at_pointer("The behavior of Decay movement in the envelope".to_string());
                        ui.add(
                            BeizerButton::BeizerButton::for_param(
                                osc_rel_curve,
                                setter,
                                5.1,
                                1.66,
                                ButtonLayout::HorizontalInline,
                                false,
                            )
                            .with_background_color(MEDIUM_GREY_UI_COLOR)
                            .with_line_color(YELLOW_MUSTARD),
                        ).on_hover_text_at_pointer("The behavior of Release movement in the envelope".to_string());
                    });
                });
                ui.add_space(20.0);
            }
            AudioModuleType::Granulizer => {
                const KNOB_SIZE: f32 = 22.0;
                const TEXT_SIZE: f32 = 10.0;
                // This fixes the granulizer release being longer than a grain itself
                if grain_hold.value() < grain_crossfade.value() {
                    setter.set_parameter(grain_crossfade, grain_hold.value());
                }
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        let load_sample_boolButton = BoolButton::BoolButton::for_param(load_sample, setter, 3.5, 0.8, SMALLER_FONT);
                        if ui.add(load_sample_boolButton).clicked() || params.load_sample_1.value() || params.load_sample_2.value() || params.load_sample_3.value() {
                            dialog.open();
                            let mut dvar = Some(dialog);
                            
                            if let Some(dialog) = &mut dvar {
                                if dialog.show(egui_ctx).selected() {
                                  if let Some(file) = dialog.path() {
                                    let opened_file = Some(file.to_path_buf());
                                    if Option::is_some(&opened_file) && Path::is_file(file) {
                                        match index {
                                            1 => {
                                                if params.load_sample_1.value() {
                                                    module1
                                                        .lock()
                                                        .unwrap()
                                                        .load_new_sample(opened_file.unwrap());
                                                    *params.am1_sample.lock().unwrap() = module1.lock().unwrap().loaded_sample.clone();
                                                    setter.set_parameter(&params.load_sample_1, false);
                                                    dialog.set_path(dialog.directory().to_path_buf());
                                                    //dialog.deselect();
                                                }
                                            },
                                            2 => {
                                                if params.load_sample_2.value() {
                                                    module2
                                                        .lock()
                                                        .unwrap()
                                                        .load_new_sample(opened_file.unwrap());
                                                    *params.am2_sample.lock().unwrap() = module2.lock().unwrap().loaded_sample.clone();
                                                    setter.set_parameter(&params.load_sample_2, false);
                                                    dialog.set_path(dialog.directory().to_path_buf());
                                                    //dialog.deselect();
                                                }
                                            },
                                            3 => {
                                                if params.load_sample_3.value() {
                                                    module3
                                                        .lock()
                                                        .unwrap()
                                                        .load_new_sample(opened_file.unwrap());
                                                    *params.am3_sample.lock().unwrap() = module3.lock().unwrap().loaded_sample.clone();
                                                    setter.set_parameter(&params.load_sample_3, false);
                                                    dialog.set_path(dialog.directory().to_path_buf());
                                                    //dialog.deselect();
                                                }
                                            },
                                            _ => {}
                                        }
                                    }
                                  }
                                }
                                match dialog.state() {
                                    State::Cancelled | State::Closed => {
                                        match index {
                                            1 => {
                                                setter.set_parameter(&params.load_sample_1, false);
                                                dialog.set_path(dialog.directory().to_path_buf());
                                                //dialog.deselect();
                                            },
                                            2 => {
                                                setter.set_parameter(&params.load_sample_2, false);
                                                dialog.set_path(dialog.directory().to_path_buf());
                                                //dialog.deselect();
                                            },
                                            3 => {
                                                setter.set_parameter(&params.load_sample_3, false);
                                                dialog.set_path(dialog.directory().to_path_buf());
                                                //dialog.deselect();
                                            },
                                            _ => {}
                                        }
                                    },
                                    _ => {}
                                }
                            }
                        }
                        let loop_toggle = BoolButton::BoolButton::for_param(loop_sample, setter, 3.5, 0.8, SMALLER_FONT);
                        ui.add(loop_toggle);

                        ui.add_space(10.0);
                        ui.label(
                            RichText::new("Note: ADSR is per note, Shape is AR per grain")
                                .font(SMALLER_FONT)
                                .color(FONT_COLOR),
                        )
                        .on_hover_text("ADSR is per note, Shape is AR per grain");
                    });
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            let osc_1_octave_knob = ui_knob::ArcKnob::for_param(
                                osc_octave,
                                setter,
                                KNOB_SIZE,
                                KnobLayout::Horizonal,
                            )
                            .preset_style(ui_knob::KnobStyle::Preset1)
                            .set_fill_color(DARK_GREY_UI_COLOR)
                            .set_line_color(YELLOW_MUSTARD)
                            .use_outline(true)
                            .set_text_size(TEXT_SIZE)
                            .set_hover_text("Adjust the MIDI input by octave".to_string());
                            ui.add(osc_1_octave_knob);

                            let osc_1_semitones_knob = ui_knob::ArcKnob::for_param(
                                osc_semitones,
                                setter,
                                KNOB_SIZE,
                                KnobLayout::Horizonal,
                            )
                            .preset_style(ui_knob::KnobStyle::Preset1)
                            .set_fill_color(DARK_GREY_UI_COLOR)
                            .set_line_color(YELLOW_MUSTARD)
                            .use_outline(true)
                            .set_text_size(TEXT_SIZE)
                            .set_hover_text("Adjust the MIDI input by semitone".to_string());
                            ui.add(osc_1_semitones_knob);
                        });

                        ui.vertical(|ui| {
                            let osc_1_retrigger_knob = ui_knob::ArcKnob::for_param(
                                osc_retrigger,
                                setter,
                                KNOB_SIZE,
                                KnobLayout::Horizonal,
                            )
                            .preset_style(ui_knob::KnobStyle::Preset1)
                            .set_fill_color(DARK_GREY_UI_COLOR)
                            .set_line_color(YELLOW_MUSTARD)
                            .use_outline(true)
                            .set_text_size(TEXT_SIZE)
                            .set_hover_text("Retrigger behavior on MIDI note input:
Retrigger: Sample restarts at every new note
Random: Sample uses a new random position every note".to_string());
                            ui.add(osc_1_retrigger_knob);

                            let grain_crossfade_1_knob = ui_knob::ArcKnob::for_param(
                                grain_crossfade,
                                setter,
                                KNOB_SIZE,
                                KnobLayout::Horizonal,
                            )
                            .preset_style(ui_knob::KnobStyle::Preset1)
                            .set_fill_color(DARK_GREY_UI_COLOR)
                            .set_line_color(YELLOW_MUSTARD)
                            .set_text_size(TEXT_SIZE)
                            .set_hover_text("How much crossfading between grains".to_string());
                            ui.add(grain_crossfade_1_knob);
                        });

                        ui.vertical(|ui| {
                            let grain_hold_1_knob = ui_knob::ArcKnob::for_param(
                                grain_hold,
                                setter,
                                KNOB_SIZE,
                                KnobLayout::Horizonal,
                            )
                            .preset_style(ui_knob::KnobStyle::Preset1)
                            .set_fill_color(DARK_GREY_UI_COLOR)
                            .set_line_color(YELLOW_MUSTARD)
                            .set_text_size(TEXT_SIZE)
                            .set_hover_text("How long do grains last".to_string());
                            ui.add(grain_hold_1_knob);

                            let grain_gap_1_knob = ui_knob::ArcKnob::for_param(
                                grain_gap,
                                setter,
                                KNOB_SIZE,
                                KnobLayout::Horizonal,
                            )
                            .preset_style(ui_knob::KnobStyle::Preset1)
                            .set_fill_color(DARK_GREY_UI_COLOR)
                            .set_line_color(YELLOW_MUSTARD)
                            .set_text_size(TEXT_SIZE)
                            .set_hover_text("The space between grains".to_string());
                            ui.add(grain_gap_1_knob);
                        });

                        ui.vertical(|ui| {
                            let start_position_1_knob = ui_knob::ArcKnob::for_param(
                                start_position,
                                setter,
                                KNOB_SIZE,
                                KnobLayout::Horizonal,
                            )
                            .preset_style(ui_knob::KnobStyle::Preset1)
                            .set_fill_color(DARK_GREY_UI_COLOR)
                            .set_line_color(YELLOW_MUSTARD)
                            .set_text_size(TEXT_SIZE)
                            .set_hover_text("Where the sample should start".to_string());
                            ui.add(start_position_1_knob);

                            let end_position_1_knob = ui_knob::ArcKnob::for_param(
                                end_position,
                                setter,
                                KNOB_SIZE,
                                KnobLayout::Horizonal,
                            )
                            .preset_style(ui_knob::KnobStyle::Preset1)
                            .set_fill_color(DARK_GREY_UI_COLOR)
                            .set_line_color(YELLOW_MUSTARD)
                            .set_text_size(TEXT_SIZE)
                            .set_hover_text("Where the sample should end".to_string());
                            ui.add(end_position_1_knob);
                        });
                        // Trying to draw background box as rect
                        ui.painter().rect_filled(
                            Rect::from_two_pos(
                                Pos2 {
                                    x: ui.cursor().left_top().x - 4.0,
                                    y: ui.cursor().left_top().y - 4.0,
                                },
                                Pos2 {
                                    x: ui.cursor().left_top().x + VERT_BAR_WIDTH * 6.0 + 8.0,
                                    y: ui.cursor().left_top().y + VERT_BAR_HEIGHT + 12.0 + 8.0,
                                },
                            ),
                            Rounding::from(4.0),
                            DARKER_GREY_UI_COLOR,
                        );
                        ui.add_space(2.0);
                        // ADSR
                        ui.add(
                            VerticalParamSlider::for_param(osc_attack, setter)
                                .with_width(VERT_BAR_WIDTH)
                                .with_height(VERT_BAR_HEIGHT)
                                .set_reversed(true)
                                .override_colors(LIGHTER_GREY_UI_COLOR, YELLOW_MUSTARD),
                        );
                        ui.add(
                            VerticalParamSlider::for_param(osc_decay, setter)
                                .with_width(VERT_BAR_WIDTH)
                                .with_height(VERT_BAR_HEIGHT)
                                .set_reversed(true)
                                .override_colors(LIGHTER_GREY_UI_COLOR, YELLOW_MUSTARD),
                        );
                        ui.add(
                            VerticalParamSlider::for_param(osc_sustain, setter)
                                .with_width(VERT_BAR_WIDTH)
                                .with_height(VERT_BAR_HEIGHT)
                                .set_reversed(true)
                                .override_colors(LIGHTER_GREY_UI_COLOR, YELLOW_MUSTARD),
                        );
                        ui.add(
                            VerticalParamSlider::for_param(osc_release, setter)
                                .with_width(VERT_BAR_WIDTH)
                                .with_height(VERT_BAR_HEIGHT)
                                .set_reversed(true)
                                .override_colors(LIGHTER_GREY_UI_COLOR, YELLOW_MUSTARD),
                        );
                        // Curves
                        ui.vertical(|ui|{
                            ui.add(
                                BeizerButton::BeizerButton::for_param(
                                    osc_atk_curve,
                                    setter,
                                    5.1,
                                    1.66,
                                    ButtonLayout::HorizontalInline,
                                    true,
                                )
                                .with_background_color(MEDIUM_GREY_UI_COLOR)
                                .with_line_color(YELLOW_MUSTARD),
                            ).on_hover_text_at_pointer("The behavior of Attack movement in the envelope".to_string());
                            ui.add(
                                BeizerButton::BeizerButton::for_param(
                                    osc_dec_curve,
                                    setter,
                                    5.1,
                                    1.66,
                                    ButtonLayout::HorizontalInline,
                                    false,
                                )
                                .with_background_color(MEDIUM_GREY_UI_COLOR)
                                .with_line_color(YELLOW_MUSTARD),
                            ).on_hover_text_at_pointer("The behavior of Decay movement in the envelope".to_string());
                            ui.add(
                                BeizerButton::BeizerButton::for_param(
                                    osc_rel_curve,
                                    setter,
                                    5.1,
                                    1.66,
                                    ButtonLayout::HorizontalInline,
                                    false,
                                )
                                .with_background_color(MEDIUM_GREY_UI_COLOR)
                                .with_line_color(YELLOW_MUSTARD),
                            ).on_hover_text_at_pointer("The behavior of Release movement in the envelope".to_string());
                        });
                    });
                });
            }
            AudioModuleType::Additive => {
                const KNOB_SIZE: f32 = 22.0;
                const TEXT_SIZE: f32 = 10.0;
                // Oscillator
                ui.vertical(|ui| {
                    ui.add_space(1.0);
                    ui.horizontal(|ui| {
                    ScrollArea::horizontal()
                        .drag_to_scroll(true)
                        .enable_scrolling(true)
                        .hscroll(true)
                        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
                        .max_width(WIDTH as f32 - 238.0)
                        .show(ui, |ui| {
                        // Additive Amplitude
                        ui.add(
                            VerticalParamSlider::for_param(additive_harmonic_0, setter)
                                .use_padding(false)
                                .override_text_size(8.0)
                                .with_width(VERT_BAR_WIDTH)
                                .with_height(VERT_LARGER)
                                .set_reversed(true)
                                .override_colors(DARKER_GREY_UI_COLOR, YELLOW_MUSTARD),
                        );
                        ui.add(
                            VerticalParamSlider::for_param(additive_harmonic_1, setter)
                                .use_padding(false)
                                .override_text_size(8.0)
                                .with_width(VERT_BAR_WIDTH)
                                .with_height(VERT_LARGER)
                                .set_reversed(true)
                                .override_colors(DARKER_GREY_UI_COLOR, YELLOW_MUSTARD),
                        );
                        ui.add(
                            VerticalParamSlider::for_param(additive_harmonic_2, setter)
                                .use_padding(false)
                                .override_text_size(8.0)
                                .with_width(VERT_BAR_WIDTH)
                                .with_height(VERT_LARGER)
                                .set_reversed(true)
                                .override_colors(DARKER_GREY_UI_COLOR, YELLOW_MUSTARD),
                        );
                        ui.add(
                            VerticalParamSlider::for_param(additive_harmonic_3, setter)
                                .use_padding(false)
                                .override_text_size(8.0)
                                .with_width(VERT_BAR_WIDTH)
                                .with_height(VERT_LARGER)
                                .set_reversed(true)
                                .override_colors(DARKER_GREY_UI_COLOR, YELLOW_MUSTARD),
                        );
                        ui.add(
                            VerticalParamSlider::for_param(additive_harmonic_4, setter)
                                .use_padding(false)
                                .override_text_size(8.0)
                                .with_width(VERT_BAR_WIDTH)
                                .with_height(VERT_LARGER)
                                .set_reversed(true)
                                .override_colors(DARKER_GREY_UI_COLOR, YELLOW_MUSTARD),
                        );
                        ui.add(
                            VerticalParamSlider::for_param(additive_harmonic_5, setter)
                                .use_padding(false)
                                .override_text_size(8.0)
                                .with_width(VERT_BAR_WIDTH)
                                .with_height(VERT_LARGER)
                                .set_reversed(true)
                                .override_colors(DARKER_GREY_UI_COLOR, YELLOW_MUSTARD),
                        );
                        ui.add(
                            VerticalParamSlider::for_param(additive_harmonic_6, setter)
                                .use_padding(false)
                                .override_text_size(8.0)
                                .with_width(VERT_BAR_WIDTH)
                                .with_height(VERT_LARGER)
                                .set_reversed(true)
                                .override_colors(DARKER_GREY_UI_COLOR, YELLOW_MUSTARD),
                        );
                        ui.add(
                            VerticalParamSlider::for_param(additive_harmonic_7, setter)
                                .use_padding(false)
                                .override_text_size(8.0)
                                .with_width(VERT_BAR_WIDTH)
                                .with_height(VERT_LARGER)
                                .set_reversed(true)
                                .override_colors(DARKER_GREY_UI_COLOR, YELLOW_MUSTARD),
                        );

                        ui.add(
                            VerticalParamSlider::for_param(additive_harmonic_8, setter)
                                .use_padding(false)
                                .override_text_size(8.0)
                                .with_width(VERT_BAR_WIDTH)
                                .with_height(VERT_LARGER)
                                .set_reversed(true)
                                .override_colors(DARKER_GREY_UI_COLOR, YELLOW_MUSTARD),
                        );
                        ui.add(
                            VerticalParamSlider::for_param(additive_harmonic_9, setter)
                                .use_padding(false)
                                .override_text_size(8.0)
                                .with_width(VERT_BAR_WIDTH)
                                .with_height(VERT_LARGER)
                                .set_reversed(true)
                                .override_colors(DARKER_GREY_UI_COLOR, YELLOW_MUSTARD),
                        );
                        ui.add(
                            VerticalParamSlider::for_param(additive_harmonic_10, setter)
                                .use_padding(false)
                                .override_text_size(8.0)
                                .with_width(VERT_BAR_WIDTH)
                                .with_height(VERT_LARGER)
                                .set_reversed(true)
                                .override_colors(DARKER_GREY_UI_COLOR, YELLOW_MUSTARD),
                        );
                        ui.add(
                            VerticalParamSlider::for_param(additive_harmonic_11, setter)
                                .use_padding(false)
                                .override_text_size(8.0)
                                .with_width(VERT_BAR_WIDTH)
                                .with_height(VERT_LARGER)
                                .set_reversed(true)
                                .override_colors(DARKER_GREY_UI_COLOR, YELLOW_MUSTARD),
                        );
                        ui.add(
                            VerticalParamSlider::for_param(additive_harmonic_12, setter)
                                .use_padding(false)
                                .override_text_size(8.0)
                                .with_width(VERT_BAR_WIDTH)
                                .with_height(VERT_LARGER)
                                .set_reversed(true)
                                .override_colors(DARKER_GREY_UI_COLOR, YELLOW_MUSTARD),
                        );
                        ui.add(
                            VerticalParamSlider::for_param(additive_harmonic_13, setter)
                                .use_padding(false)
                                .override_text_size(8.0)
                                .with_width(VERT_BAR_WIDTH)
                                .with_height(VERT_LARGER)
                                .set_reversed(true)
                                .override_colors(DARKER_GREY_UI_COLOR, YELLOW_MUSTARD),
                        );
                        ui.add(
                            VerticalParamSlider::for_param(additive_harmonic_14, setter)
                                .use_padding(false)
                                .override_text_size(8.0)
                                .with_width(VERT_BAR_WIDTH)
                                .with_height(VERT_LARGER)
                                .set_reversed(true)
                                .override_colors(DARKER_GREY_UI_COLOR, YELLOW_MUSTARD),
                        );
                        ui.add(
                            VerticalParamSlider::for_param(additive_harmonic_15, setter)
                                .use_padding(false)
                                .override_text_size(8.0)
                                .with_width(VERT_BAR_WIDTH)
                                .with_height(VERT_LARGER)
                                .set_reversed(true)
                                .override_colors(DARKER_GREY_UI_COLOR, YELLOW_MUSTARD),
                        );

                        ui.vertical(|ui| {
                            let osc_1_retrigger_knob = ui_knob::ArcKnob::for_param(
                                osc_retrigger,
                                setter,
                                KNOB_SIZE,
                                KnobLayout::Horizonal,
                            )
                            .preset_style(ui_knob::KnobStyle::Preset1)
                            .set_fill_color(DARK_GREY_UI_COLOR)
                            .set_line_color(YELLOW_MUSTARD)
                            .use_outline(true)
                            .set_text_size(TEXT_SIZE)
                            .set_hover_text("Retrigger behavior on MIDI note input:
Free: constantly running phase based off previous note
Retrigger: wave form restarts at every new note
Random: Wave and all unisons use a new random phase every note
UniRandom: Every voice uses its own unique random phase every note".to_string());
                            ui.add(osc_1_retrigger_knob);
                        });

                        ui.vertical(|ui| {
                            let osc_1_octave_knob = ui_knob::ArcKnob::for_param(
                                osc_octave,
                                setter,
                                KNOB_SIZE,
                                KnobLayout::Horizonal,
                            )
                            .preset_style(ui_knob::KnobStyle::Preset1)
                            .set_fill_color(DARK_GREY_UI_COLOR)
                            .set_line_color(YELLOW_MUSTARD)
                            .use_outline(true)
                            .set_text_size(TEXT_SIZE)
                            .set_hover_text("Adjust the MIDI input by octave".to_string());
                            ui.add(osc_1_octave_knob);

                            let osc_1_semitones_knob = ui_knob::ArcKnob::for_param(
                                osc_semitones,
                                setter,
                                KNOB_SIZE,
                                KnobLayout::Horizonal,
                            )
                            .preset_style(ui_knob::KnobStyle::Preset1)
                            .set_fill_color(DARK_GREY_UI_COLOR)
                            .set_line_color(YELLOW_MUSTARD)
                            .use_outline(true)
                            .set_text_size(TEXT_SIZE)
                            .set_hover_text("Adjust the MIDI input by semitone".to_string());
                            ui.add(osc_1_semitones_knob);
                        });

                        ui.vertical(|ui| {
                            let osc_1_stereo_knob = ui_knob::ArcKnob::for_param(
                                osc_stereo,
                                setter,
                                KNOB_SIZE,
                                KnobLayout::Horizonal,
                            )
                            .preset_style(ui_knob::KnobStyle::Preset1)
                            .set_fill_color(DARK_GREY_UI_COLOR)
                            .set_line_color(YELLOW_MUSTARD)
                            .use_outline(true)
                            .set_text_size(TEXT_SIZE)
                            .set_hover_text("Oscillator voice stereo spread. 0 is Mono.".to_string());
                            ui.add(osc_1_stereo_knob);

                            let osc_1_unison_knob = ui_knob::ArcKnob::for_param(
                                osc_unison,
                                setter,
                                KNOB_SIZE,
                                KnobLayout::Horizonal,
                            )
                            .preset_style(ui_knob::KnobStyle::Preset1)
                            .set_fill_color(DARK_GREY_UI_COLOR)
                            .set_line_color(YELLOW_MUSTARD.gamma_multiply(2.0))
                            .use_outline(true)
                            .set_text_size(TEXT_SIZE)
                            .set_hover_text("How many voices should play in unison".to_string());
                            ui.add(osc_1_unison_knob);
                        });

                        ui.vertical(|ui| {
                            let osc_1_detune_knob = ui_knob::ArcKnob::for_param(
                                osc_detune,
                                setter,
                                KNOB_SIZE,
                                KnobLayout::Horizonal,
                            )
                            .preset_style(ui_knob::KnobStyle::Preset1)
                            .set_fill_color(DARK_GREY_UI_COLOR)
                            .set_line_color(YELLOW_MUSTARD)
                            .use_outline(true)
                            .set_text_size(TEXT_SIZE)
                            .set_hover_text("Move the pitch to fine tune it".to_string());
                            ui.add(osc_1_detune_knob);

                            let osc_1_unison_detune_knob = ui_knob::ArcKnob::for_param(
                                osc_unison_detune,
                                setter,
                                KNOB_SIZE,
                                KnobLayout::Horizonal,
                            )
                            .preset_style(ui_knob::KnobStyle::Preset1)
                            .set_fill_color(DARK_GREY_UI_COLOR)
                            .set_line_color(YELLOW_MUSTARD.gamma_multiply(2.0))
                            .use_outline(true)
                            .set_text_size(TEXT_SIZE)
                            .set_hover_text("Spread the pitches of the unison voices apart".to_string());
                            ui.add(osc_1_unison_detune_knob);
                        });

                        // Trying to draw background box as rect
                        ui.painter().rect_filled(
                            Rect::from_two_pos(
                                Pos2 {
                                    x: ui.cursor().left_top().x - 4.0,
                                    y: ui.cursor().left_top().y - 4.0,
                                },
                                Pos2 {
                                    x: ui.cursor().left_top().x + VERT_BAR_WIDTH * 6.0 + 8.0,
                                    y: ui.cursor().left_top().y + VERT_BAR_HEIGHT + 12.0 + 8.0,
                                },
                            ),
                            Rounding::from(4.0),
                            DARKER_GREY_UI_COLOR,
                        );
                        ui.add_space(2.0);

                        // ADSR
                        ui.add(
                            VerticalParamSlider::for_param(osc_attack, setter)
                                .with_width(VERT_BAR_WIDTH)
                                .with_height(VERT_BAR_HEIGHT)
                                .set_reversed(true)
                                .override_colors(LIGHTER_GREY_UI_COLOR, YELLOW_MUSTARD),
                        );
                        ui.add(
                            VerticalParamSlider::for_param(osc_decay, setter)
                                .with_width(VERT_BAR_WIDTH)
                                .with_height(VERT_BAR_HEIGHT)
                                .set_reversed(true)
                                .override_colors(LIGHTER_GREY_UI_COLOR, YELLOW_MUSTARD),
                        );
                        ui.add(
                            VerticalParamSlider::for_param(osc_sustain, setter)
                                .with_width(VERT_BAR_WIDTH)
                                .with_height(VERT_BAR_HEIGHT)
                                .set_reversed(true)
                                .override_colors(LIGHTER_GREY_UI_COLOR, YELLOW_MUSTARD),
                        );
                        ui.add(
                            VerticalParamSlider::for_param(osc_release, setter)
                                .with_width(VERT_BAR_WIDTH)
                                .with_height(VERT_BAR_HEIGHT)
                                .set_reversed(true)
                                .override_colors(LIGHTER_GREY_UI_COLOR, YELLOW_MUSTARD),
                        );

                        // Curves
                        ui.vertical(|ui|{
                            ui.add(
                                BeizerButton::BeizerButton::for_param(
                                    osc_atk_curve,
                                    setter,
                                    5.1,
                                    1.66,
                                    ButtonLayout::HorizontalInline,
                                    true,
                                )
                                .with_background_color(MEDIUM_GREY_UI_COLOR)
                                .with_line_color(YELLOW_MUSTARD),
                            ).on_hover_text_at_pointer("The behavior of Attack movement in the envelope".to_string());
                            ui.add(
                                BeizerButton::BeizerButton::for_param(
                                    osc_dec_curve,
                                    setter,
                                    5.1,
                                    1.66,
                                    ButtonLayout::HorizontalInline,
                                    false,
                                )
                                .with_background_color(MEDIUM_GREY_UI_COLOR)
                                .with_line_color(YELLOW_MUSTARD),
                            ).on_hover_text_at_pointer("The behavior of Decay movement in the envelope".to_string());
                            ui.add(
                                BeizerButton::BeizerButton::for_param(
                                    osc_rel_curve,
                                    setter,
                                    5.1,
                                    1.66,
                                    ButtonLayout::HorizontalInline,
                                    false,
                                )
                                .with_background_color(MEDIUM_GREY_UI_COLOR)
                                .with_line_color(YELLOW_MUSTARD),
                            ).on_hover_text_at_pointer("The behavior of Release movement in the envelope".to_string());
                        });
                    });
                    });
                });
                ui.add_space(20.0);
            }
        }
    }

    // Index proper params from knobs
    // This lets us have a copy for voices, and also track changes like restretch changing or ADR slopes
    pub fn consume_params(&mut self, params: Arc<ActuateParams>, voice_index: usize) -> AudioModuleType {
        match voice_index {
            1 => {
                self.audio_module_type = params.audio_module_1_type.value();
                if self.osc_octave != params.osc_1_octave.value() {
                    let oct_shift = self.osc_octave - params.osc_1_octave.value();
                    for voice in self.playing_voices.voices.iter_mut() {
                        voice.note -= (oct_shift * 12) as u8;
                    }
                    for uni_voice in self.unison_voices.voices.iter_mut() {
                        uni_voice.note -= (oct_shift * 12) as u8;
                    }
                }
                self.osc_octave = params.osc_1_octave.value();
                if self.osc_semitones != params.osc_1_semitones.value() {
                    let semi_shift = self.osc_semitones - params.osc_1_semitones.value();
                    for voice in self.playing_voices.voices.iter_mut() {
                        voice.note -= semi_shift as u8;
                    }
                    for uni_voice in self.unison_voices.voices.iter_mut() {
                        uni_voice.note -= semi_shift as u8;
                    }
                }
                match params.pitch_routing.value() {
                    PitchRouting::Osc1
                    | PitchRouting::Osc1_Osc2
                    | PitchRouting::Osc1_Osc3
                    | PitchRouting::All => {
                        self.pitch_enable = params.pitch_enable.value();
                        self.pitch_env_peak = params.pitch_env_peak.value();
                        self.pitch_env_attack = params.pitch_env_attack.value();
                        self.pitch_env_decay = params.pitch_env_decay.value();
                        self.pitch_env_sustain = params.pitch_env_sustain.value();
                        self.pitch_env_release = params.pitch_env_release.value();
                        self.pitch_env_atk_curve = params.pitch_env_atk_curve.value();
                        self.pitch_env_dec_curve = params.pitch_env_dec_curve.value();
                        self.pitch_env_rel_curve = params.pitch_env_rel_curve.value();
                    }
                    _ => {
                        self.pitch_enable = false;
                    }
                }
                match params.pitch_routing_2.value() {
                    PitchRouting::Osc1
                    | PitchRouting::Osc1_Osc2
                    | PitchRouting::Osc1_Osc3
                    | PitchRouting::All => {
                        self.pitch_enable_2 = params.pitch_enable_2.value();
                        self.pitch_env_peak_2 = params.pitch_env_peak_2.value();
                        self.pitch_env_attack_2 = params.pitch_env_attack_2.value();
                        self.pitch_env_decay_2 = params.pitch_env_decay_2.value();
                        self.pitch_env_sustain_2 = params.pitch_env_sustain_2.value();
                        self.pitch_env_release_2 = params.pitch_env_release_2.value();
                        self.pitch_env_atk_curve_2 = params.pitch_env_atk_curve_2.value();
                        self.pitch_env_dec_curve_2 = params.pitch_env_dec_curve_2.value();
                        self.pitch_env_rel_curve_2 = params.pitch_env_rel_curve_2.value();
                    }
                    _ => {
                        self.pitch_enable_2 = false;
                    }
                }
                self.osc_semitones = params.osc_1_semitones.value();
                self.osc_detune = params.osc_1_detune.value();
                self.osc_attack = params.osc_1_attack.value();
                self.osc_decay = params.osc_1_decay.value();
                self.osc_sustain = params.osc_1_sustain.value();
                self.osc_release = params.osc_1_release.value();
                self.osc_retrigger = params.osc_1_retrigger.value();
                self.osc_atk_curve = params.osc_1_atk_curve.value();
                self.osc_dec_curve = params.osc_1_dec_curve.value();
                self.osc_rel_curve = params.osc_1_rel_curve.value();
                self.osc_unison = params.osc_1_unison.value();
                self.osc_unison_detune = params.osc_1_unison_detune.value();
                self.osc_stereo = params.osc_1_stereo.value();
                self.loop_wavetable = params.loop_sample_1.value();
                self.single_cycle = params.single_cycle_1.value();
                self.restretch = params.restretch_1.value();
                self.start_position = params.start_position_1.value();
                self._end_position = params.end_position_1.value();
                self.grain_hold = params.grain_hold_1.value();
                self.grain_gap = params.grain_gap_1.value();
                self.grain_crossfade = params.grain_crossfade_1.value();
                self.ah0 = params.additive_amp_1_0.value();
                self.ah1 = params.additive_amp_1_1.value();
                self.ah2 = params.additive_amp_1_2.value();
                self.ah3 = params.additive_amp_1_3.value();
                self.ah4 = params.additive_amp_1_4.value();
                self.ah5 = params.additive_amp_1_5.value();
                self.ah6 = params.additive_amp_1_6.value();
                self.ah7 = params.additive_amp_1_7.value();
                self.ah8 = params.additive_amp_1_8.value();
                self.ah9 = params.additive_amp_1_9.value();
                self.ah10 = params.additive_amp_1_10.value();
                self.ah11 = params.additive_amp_1_11.value();
                self.ah12 = params.additive_amp_1_12.value();
                self.ah13 = params.additive_amp_1_13.value();
                self.ah14 = params.additive_amp_1_14.value();
                self.ah15 = params.additive_amp_1_15.value();
            }
            2 => {
                self.audio_module_type = params.audio_module_2_type.value();
                if self.osc_octave != params.osc_2_octave.value() {
                    let oct_shift = self.osc_octave - params.osc_2_octave.value();
                    for voice in self.playing_voices.voices.iter_mut() {
                        voice.note -= (oct_shift * 12) as u8;
                    }
                    for uni_voice in self.unison_voices.voices.iter_mut() {
                        uni_voice.note -= (oct_shift * 12) as u8;
                    }
                }
                self.osc_octave = params.osc_2_octave.value();
                if self.osc_semitones != params.osc_2_semitones.value() {
                    let semi_shift = self.osc_semitones - params.osc_2_semitones.value();
                    for voice in self.playing_voices.voices.iter_mut() {
                        voice.note -= semi_shift as u8;
                    }
                    for uni_voice in self.unison_voices.voices.iter_mut() {
                        uni_voice.note -= semi_shift as u8;
                    }
                }
                match params.pitch_routing.value() {
                    PitchRouting::Osc2
                    | PitchRouting::Osc1_Osc2
                    | PitchRouting::Osc2_Osc3
                    | PitchRouting::All => {
                        self.pitch_enable = params.pitch_enable.value();
                        self.pitch_env_peak = params.pitch_env_peak.value();
                        self.pitch_env_attack = params.pitch_env_attack.value();
                        self.pitch_env_decay = params.pitch_env_decay.value();
                        self.pitch_env_sustain = params.pitch_env_sustain.value();
                        self.pitch_env_release = params.pitch_env_release.value();
                        self.pitch_env_atk_curve = params.pitch_env_atk_curve.value();
                        self.pitch_env_dec_curve = params.pitch_env_dec_curve.value();
                        self.pitch_env_rel_curve = params.pitch_env_rel_curve.value();
                    }
                    _ => {
                        self.pitch_enable = false;
                    }
                }
                match params.pitch_routing_2.value() {
                    PitchRouting::Osc2
                    | PitchRouting::Osc1_Osc2
                    | PitchRouting::Osc2_Osc3
                    | PitchRouting::All => {
                        self.pitch_enable_2 = params.pitch_enable_2.value();
                        self.pitch_env_peak_2 = params.pitch_env_peak_2.value();
                        self.pitch_env_attack_2 = params.pitch_env_attack_2.value();
                        self.pitch_env_decay_2 = params.pitch_env_decay_2.value();
                        self.pitch_env_sustain_2 = params.pitch_env_sustain_2.value();
                        self.pitch_env_release_2 = params.pitch_env_release_2.value();
                        self.pitch_env_atk_curve_2 = params.pitch_env_atk_curve_2.value();
                        self.pitch_env_dec_curve_2 = params.pitch_env_dec_curve_2.value();
                        self.pitch_env_rel_curve_2 = params.pitch_env_rel_curve_2.value();
                    }
                    _ => {
                        self.pitch_enable_2 = false;
                    }
                }
                self.osc_semitones = params.osc_2_semitones.value();
                self.osc_detune = params.osc_2_detune.value();
                self.osc_attack = params.osc_2_attack.value();
                self.osc_decay = params.osc_2_decay.value();
                self.osc_sustain = params.osc_2_sustain.value();
                self.osc_release = params.osc_2_release.value();
                self.osc_retrigger = params.osc_2_retrigger.value();
                self.osc_atk_curve = params.osc_2_atk_curve.value();
                self.osc_dec_curve = params.osc_2_dec_curve.value();
                self.osc_rel_curve = params.osc_2_rel_curve.value();
                self.osc_unison = params.osc_2_unison.value();
                self.osc_unison_detune = params.osc_2_unison_detune.value();
                self.osc_stereo = params.osc_2_stereo.value();
                self.loop_wavetable = params.loop_sample_2.value();
                self.single_cycle = params.single_cycle_2.value();
                self.restretch = params.restretch_2.value();
                self.start_position = params.start_position_2.value();
                self._end_position = params.end_position_2.value();
                self.grain_hold = params.grain_hold_2.value();
                self.grain_gap = params.grain_gap_2.value();
                self.grain_crossfade = params.grain_crossfade_2.value();
                self.ah0 = params.additive_amp_2_0.value();
                self.ah1 = params.additive_amp_2_1.value();
                self.ah2 = params.additive_amp_2_2.value();
                self.ah3 = params.additive_amp_2_3.value();
                self.ah4 = params.additive_amp_2_4.value();
                self.ah5 = params.additive_amp_2_5.value();
                self.ah6 = params.additive_amp_2_6.value();
                self.ah7 = params.additive_amp_2_7.value();
                self.ah8 = params.additive_amp_2_8.value();
                self.ah9 = params.additive_amp_2_9.value();
                self.ah10 = params.additive_amp_2_10.value();
                self.ah11 = params.additive_amp_2_11.value();
                self.ah12 = params.additive_amp_2_12.value();
                self.ah13 = params.additive_amp_2_13.value();
                self.ah14 = params.additive_amp_2_14.value();
                self.ah15 = params.additive_amp_2_15.value();
            }
            3 => {
                self.audio_module_type = params.audio_module_3_type.value();
                if self.osc_octave != params.osc_3_octave.value() {
                    let oct_shift = self.osc_octave - params.osc_3_octave.value();
                    for voice in self.playing_voices.voices.iter_mut() {
                        voice.note -= (oct_shift * 12) as u8;
                    }
                    for uni_voice in self.unison_voices.voices.iter_mut() {
                        uni_voice.note -= (oct_shift * 12) as u8;
                    }
                }
                self.osc_octave = params.osc_3_octave.value();
                if self.osc_semitones != params.osc_3_semitones.value() {
                    let semi_shift = self.osc_semitones - params.osc_3_semitones.value();
                    for voice in self.playing_voices.voices.iter_mut() {
                        voice.note -= semi_shift as u8;
                    }
                    for uni_voice in self.unison_voices.voices.iter_mut() {
                        uni_voice.note -= semi_shift as u8;
                    }
                }
                match params.pitch_routing.value() {
                    PitchRouting::Osc3
                    | PitchRouting::Osc2_Osc3
                    | PitchRouting::Osc1_Osc3
                    | PitchRouting::All => {
                        self.pitch_enable = params.pitch_enable.value();
                        self.pitch_env_peak = params.pitch_env_peak.value();
                        self.pitch_env_attack = params.pitch_env_attack.value();
                        self.pitch_env_decay = params.pitch_env_decay.value();
                        self.pitch_env_sustain = params.pitch_env_sustain.value();
                        self.pitch_env_release = params.pitch_env_release.value();
                        self.pitch_env_atk_curve = params.pitch_env_atk_curve.value();
                        self.pitch_env_dec_curve = params.pitch_env_dec_curve.value();
                        self.pitch_env_rel_curve = params.pitch_env_rel_curve.value();
                    }
                    _ => {
                        self.pitch_enable = false;
                    }
                }
                match params.pitch_routing_2.value() {
                    PitchRouting::Osc3
                    | PitchRouting::Osc2_Osc3
                    | PitchRouting::Osc1_Osc3
                    | PitchRouting::All => {
                        self.pitch_enable_2 = params.pitch_enable_2.value();
                        self.pitch_env_peak_2 = params.pitch_env_peak_2.value();
                        self.pitch_env_attack_2 = params.pitch_env_attack_2.value();
                        self.pitch_env_decay_2 = params.pitch_env_decay_2.value();
                        self.pitch_env_sustain_2 = params.pitch_env_sustain_2.value();
                        self.pitch_env_release_2 = params.pitch_env_release_2.value();
                        self.pitch_env_atk_curve_2 = params.pitch_env_atk_curve_2.value();
                        self.pitch_env_dec_curve_2 = params.pitch_env_dec_curve_2.value();
                        self.pitch_env_rel_curve_2 = params.pitch_env_rel_curve_2.value();
                    }
                    _ => {
                        self.pitch_enable_2 = false;
                    }
                }
                self.osc_semitones = params.osc_3_semitones.value();
                self.osc_detune = params.osc_3_detune.value();
                self.osc_attack = params.osc_3_attack.value();
                self.osc_decay = params.osc_3_decay.value();
                self.osc_sustain = params.osc_3_sustain.value();
                self.osc_release = params.osc_3_release.value();
                self.osc_retrigger = params.osc_3_retrigger.value();
                self.osc_atk_curve = params.osc_3_atk_curve.value();
                self.osc_dec_curve = params.osc_3_dec_curve.value();
                self.osc_rel_curve = params.osc_3_rel_curve.value();
                self.osc_unison = params.osc_3_unison.value();
                self.osc_unison_detune = params.osc_3_unison_detune.value();
                self.osc_stereo = params.osc_3_stereo.value();
                self.loop_wavetable = params.loop_sample_3.value();
                self.single_cycle = params.single_cycle_3.value();
                self.restretch = params.restretch_3.value();
                self.start_position = params.start_position_3.value();
                self._end_position = params.end_position_3.value();
                self.grain_hold = params.grain_hold_3.value();
                self.grain_gap = params.grain_gap_3.value();
                self.grain_crossfade = params.grain_crossfade_3.value();
                self.ah0 = params.additive_amp_3_0.value();
                self.ah1 = params.additive_amp_3_1.value();
                self.ah2 = params.additive_amp_3_2.value();
                self.ah3 = params.additive_amp_3_3.value();
                self.ah4 = params.additive_amp_3_4.value();
                self.ah5 = params.additive_amp_3_5.value();
                self.ah6 = params.additive_amp_3_6.value();
                self.ah7 = params.additive_amp_3_7.value();
                self.ah8 = params.additive_amp_3_8.value();
                self.ah9 = params.additive_amp_3_9.value();
                self.ah10 = params.additive_amp_3_10.value();
                self.ah11 = params.additive_amp_3_11.value();
                self.ah12 = params.additive_amp_3_12.value();
                self.ah13 = params.additive_amp_3_13.value();
                self.ah14 = params.additive_amp_3_14.value();
                self.ah15 = params.additive_amp_3_15.value();
            }
            _ => {}
        }
        self.audio_module_type
    }

    // I was looking at the PolyModSynth Example and decided on this
    // Handle the audio module midi events and regular pricessing
    // This is an INDIVIDUAL instance process unlike the GUI function
    // This sends back the OSC output + note on for filter to reset
    pub fn process(
        &mut self,
        _sample_id: usize,
        event_passed: Option<NoteEvent<()>>,
        voice_max: usize,
        detune_mod: f32,
        uni_detune_mod: f32,
        velocity_mod: f32,
        uni_velocity_mod: f32,
        vel_gain_mod: f32,
        vel_lfo_gain_mod: f32,
        stereo_algorithm: StereoAlgorithm,
    ) -> (f32, f32, bool, bool) {
        // If the process is in here the file dialog is not open per lib.rs

        // Midi events are processed here
        let mut note_on: bool = false;
        let mut note_off: bool = false;
        match event_passed {
            // The event was valid
            Some(mut event) => {
                event = event_passed.unwrap();
                match event {
                    ////////////////////////////////////////////////////////////
                    // MIDI EVENT NOTE ON
                    ////////////////////////////////////////////////////////////
                    NoteEvent::NoteOn {
                        mut note, velocity, ..
                    } => {
                        // Osc + generic stuff
                        note_on = true;
                        let mut new_phase: f32 = 0.0;

                        // Calculate our pitch mod stuff if applicable
                        let pitch_attack_smoother: Smoother<f32>;
                        let pitch_decay_smoother: Smoother<f32>;
                        let pitch_release_smoother: Smoother<f32>;
                        let pitch_mod_current: f32;
                        let pitch_attack_smoother_2: Smoother<f32>;
                        let pitch_decay_smoother_2: Smoother<f32>;
                        let pitch_release_smoother_2: Smoother<f32>;
                        let pitch_mod_current_2: f32;
                        if self.pitch_enable {
                            pitch_attack_smoother = match self.pitch_env_atk_curve {
                                SmoothStyle::Linear => {
                                    Smoother::new(SmoothingStyle::Linear(self.pitch_env_attack))
                                }
                                SmoothStyle::Logarithmic => {
                                    Smoother::new(SmoothingStyle::Logarithmic(
                                        self.pitch_env_attack.clamp(0.0001, 999.9),
                                    ))
                                }
                                SmoothStyle::Exponential => Smoother::new(
                                    SmoothingStyle::Exponential(self.pitch_env_attack),
                                ),
                                SmoothStyle::LogSteep => {
                                    Smoother::new(SmoothingStyle::LogSteep(
                                        self.pitch_env_attack.clamp(0.0001, 999.9)
                                    ))
                                }
                            };

                            pitch_decay_smoother = match self.pitch_env_dec_curve {
                                SmoothStyle::Linear => {
                                    Smoother::new(SmoothingStyle::Linear(self.pitch_env_decay))
                                }
                                SmoothStyle::Logarithmic => {
                                    Smoother::new(SmoothingStyle::Logarithmic(
                                        self.pitch_env_decay.clamp(0.0001, 999.9),
                                    ))
                                }
                                SmoothStyle::Exponential => {
                                    Smoother::new(SmoothingStyle::Exponential(self.pitch_env_decay))
                                }
                                SmoothStyle::LogSteep => {
                                    Smoother::new(SmoothingStyle::LogSteep(
                                        self.pitch_env_decay.clamp(0.0001, 999.9)
                                    ))
                                }
                            };

                            pitch_release_smoother = match self.pitch_env_rel_curve {
                                SmoothStyle::Linear => {
                                    Smoother::new(SmoothingStyle::Linear(self.pitch_env_release))
                                }
                                SmoothStyle::Logarithmic => {
                                    Smoother::new(SmoothingStyle::Logarithmic(
                                        self.pitch_env_release.clamp(0.0001, 999.9),
                                    ))
                                }
                                SmoothStyle::Exponential => Smoother::new(
                                    SmoothingStyle::Exponential(self.pitch_env_release),
                                ),
                                SmoothStyle::LogSteep => {
                                    Smoother::new(SmoothingStyle::LogSteep(
                                        self.pitch_env_release.clamp(0.0001, 999.9)
                                    ))
                                }
                            };

                            match pitch_attack_smoother.style {
                                SmoothingStyle::Logarithmic(_) | SmoothingStyle::LogSteep(_) => {
                                    pitch_attack_smoother.reset(0.0001);
                                    pitch_attack_smoother.set_target(
                                        self.sample_rate,
                                        self.pitch_env_peak.max(0.0001),
                                    );
                                }
                                _ => {
                                    pitch_attack_smoother.reset(0.0);
                                    pitch_attack_smoother
                                        .set_target(self.sample_rate, self.pitch_env_peak);
                                }
                            }

                            pitch_mod_current = pitch_attack_smoother.next();
                        } else {
                            pitch_attack_smoother = Smoother::new(SmoothingStyle::None);
                            pitch_decay_smoother = Smoother::new(SmoothingStyle::None);
                            pitch_release_smoother = Smoother::new(SmoothingStyle::None);
                            pitch_mod_current = 0.0;
                        }
                        // Pitch mod 2
                        if self.pitch_enable_2 {
                            pitch_attack_smoother_2 = match self.pitch_env_atk_curve_2 {
                                SmoothStyle::Linear => {
                                    Smoother::new(SmoothingStyle::Linear(self.pitch_env_attack_2))
                                }
                                SmoothStyle::Logarithmic => {
                                    Smoother::new(SmoothingStyle::Logarithmic(
                                        self.pitch_env_attack_2.clamp(0.0001, 999.9),
                                    ))
                                }
                                SmoothStyle::Exponential => Smoother::new(
                                    SmoothingStyle::Exponential(self.pitch_env_attack_2),
                                ),
                                SmoothStyle::LogSteep => {
                                    Smoother::new(SmoothingStyle::LogSteep(
                                        self.pitch_env_attack_2.clamp(0.0001, 999.9)
                                    ))
                                }
                            };

                            pitch_decay_smoother_2 = match self.pitch_env_dec_curve_2 {
                                SmoothStyle::Linear => {
                                    Smoother::new(SmoothingStyle::Linear(self.pitch_env_decay_2))
                                }
                                SmoothStyle::Logarithmic => {
                                    Smoother::new(SmoothingStyle::Logarithmic(
                                        self.pitch_env_decay_2.clamp(0.0001, 999.9),
                                    ))
                                }
                                SmoothStyle::Exponential => Smoother::new(
                                    SmoothingStyle::Exponential(self.pitch_env_decay_2),
                                ),
                                SmoothStyle::LogSteep => {
                                    Smoother::new(SmoothingStyle::LogSteep(
                                        self.pitch_env_decay_2.clamp(0.0001, 999.9)
                                    ))
                                }
                            };

                            pitch_release_smoother_2 = match self.pitch_env_rel_curve_2 {
                                SmoothStyle::Linear => {
                                    Smoother::new(SmoothingStyle::Linear(self.pitch_env_release_2))
                                }
                                SmoothStyle::Logarithmic => {
                                    Smoother::new(SmoothingStyle::Logarithmic(
                                        self.pitch_env_release_2.clamp(0.0001, 999.9),
                                    ))
                                }
                                SmoothStyle::Exponential => Smoother::new(
                                    SmoothingStyle::Exponential(self.pitch_env_release_2),
                                ),
                                SmoothStyle::LogSteep => Smoother::new(SmoothingStyle::LogSteep(
                                    self.pitch_env_release_2.clamp(0.0001, 999.9),
                                )),
                            };

                            match pitch_attack_smoother_2.style {
                                SmoothingStyle::Logarithmic(_) | SmoothingStyle::LogSteep(_)=> {
                                    pitch_attack_smoother_2.reset(0.0001);
                                    pitch_attack_smoother_2.set_target(
                                        self.sample_rate,
                                        self.pitch_env_peak_2.max(0.0001),
                                    );
                                }
                                _ => {
                                    pitch_attack_smoother_2.reset(0.0);
                                    pitch_attack_smoother_2
                                        .set_target(self.sample_rate, self.pitch_env_peak_2);
                                }
                            }

                            pitch_mod_current_2 = pitch_attack_smoother_2.next();
                        } else {
                            pitch_attack_smoother_2 = Smoother::new(SmoothingStyle::None);
                            pitch_decay_smoother_2 = Smoother::new(SmoothingStyle::None);
                            pitch_release_smoother_2 = Smoother::new(SmoothingStyle::None);
                            pitch_mod_current_2 = 0.0;
                        }

                        // Sampler when single cycle needs this!!!
                        if self.single_cycle {
                            // 31 comes from comparing with 3xOsc position in MIDI notes
                            note += 31;
                        }
                        // Shift our note per octave
                        match self.osc_octave {
                            -2 => {
                                note -= 24;
                            }
                            -1 => {
                                note -= 12;
                            }
                            0 => {
                                note -= 0;
                            }
                            1 => {
                                note += 12;
                            }
                            2 => {
                                note += 24;
                            }
                            _ => {}
                        }
                        // Shift our note per semitones
                        note += self.osc_semitones as u8;
                        // Shift our note per detune
                        // I'm so glad nih-plug has this helper for f32 conversions!
                        let base_note = if velocity_mod <= 0.0 {
                            note as f32
                                + self.osc_detune
                                + detune_mod
                                + pitch_mod_current
                                + pitch_mod_current_2
                        } else {
                            note as f32
                                + self.osc_detune
                                + detune_mod
                                + velocity_mod.clamp(0.0, 1.0) * velocity
                                + pitch_mod_current
                                + pitch_mod_current_2
                        };

                        // Reset the retrigger on Oscs
                        match self.osc_retrigger {
                            RetriggerStyle::Retrigger => {
                                // Start our phase back at 0
                                new_phase = 0.0;
                            }
                            RetriggerStyle::Random | RetriggerStyle::UniRandom => {
                                match self.audio_module_type {
                                    AudioModuleType::Sampler => {
                                        let mut rng = rand::thread_rng();
                                        // Prevent panic when no sample loaded yet
                                        if self.sample_lib.len() > 1 {
                                            if self.sample_lib[note as usize][0].len() > 0 {
                                                new_phase = rng.gen_range(
                                                    0.0..self.sample_lib[note as usize][0].len() as f32,
                                                );
                                            } else {
                                                // There's probably no sample loaded
                                                new_phase = 0.0;
                                            }
                                        }
                                    },
                                    AudioModuleType::Granulizer => {
                                        let mut rng = rand::thread_rng();
                                        // Prevent panic when no sample loaded yet
                                        if self.sample_lib.len() > 1 {
                                            if self.sample_lib[note as usize][0].len() > 0 {
                                                new_phase = rng.gen_range(
                                                    0.0..self.sample_lib[note as usize][0].len() as f32,
                                                );
                                            } else {
                                                // There's probably no sample loaded
                                                new_phase = 0.0;
                                            }
                                        }
                                    },
                                    _ => {
                                        // Get a random phase to use
                                        // Poly solution is to pass the phase to the struct
                                        // instead of the osc alone
                                        let mut rng = rand::thread_rng();
                                        new_phase = rng.gen_range(0.0..1.0);
                                    }
                                }
                            }
                            RetriggerStyle::Free => {
                                // Do nothing for osc
                                match self.audio_module_type {
                                    AudioModuleType::Sampler | AudioModuleType::Granulizer => {
                                        new_phase = 0.0;
                                    }
                                    _ => {}
                                }
                            }
                        }

                        // Create an array of unison notes based off the param for how many unison voices we need
                        let mut unison_notes: Vec<f32> = vec![0.0; self.osc_unison as usize];
                        // If we have any unison voices
                        if self.osc_unison > 1 {
                            // Calculate the detune step amount per amount of voices
                            let detune_step = self.osc_unison_detune / self.osc_unison as f32;
                            for unison_voice in 0..(self.osc_unison as usize - 1) {
                                // Create the detuned notes around the base note
                                if unison_voice % 2 == 1 {
                                    unison_notes[unison_voice] = util::f32_midi_note_to_freq(
                                        base_note
                                            + uni_detune_mod
                                            + (uni_velocity_mod.clamp(0.0, 1.0) * velocity)
                                            + detune_step * (unison_voice + 1) as f32
                                            + pitch_mod_current
                                            + pitch_mod_current_2,
                                    );
                                } else {
                                    unison_notes[unison_voice] = util::f32_midi_note_to_freq(
                                        base_note
                                            - uni_detune_mod
                                            - (uni_velocity_mod.clamp(0.0, 1.0) * velocity)
                                            - detune_step * (unison_voice) as f32
                                            - pitch_mod_current
                                            - pitch_mod_current_2,
                                    );
                                }
                            }
                        }

                        let attack_smoother: Smoother<f32> = match self.osc_atk_curve {
                            SmoothStyle::Linear => {
                                Smoother::new(SmoothingStyle::Linear(self.osc_attack))
                            }
                            SmoothStyle::Logarithmic => Smoother::new(SmoothingStyle::Logarithmic(
                                self.osc_attack.clamp(0.0001, 999.9),
                            )),
                            SmoothStyle::Exponential => {
                                Smoother::new(SmoothingStyle::Exponential(self.osc_attack))
                            }
                            SmoothStyle::LogSteep => {
                                Smoother::new(SmoothingStyle::LogSteep(
                                    self.osc_attack.clamp(0.0001, 999.9)
                                ))
                            }
                        };

                        let decay_smoother: Smoother<f32> = match self.osc_dec_curve {
                            SmoothStyle::Linear => {
                                Smoother::new(SmoothingStyle::Linear(self.osc_decay))
                            }
                            SmoothStyle::Logarithmic => Smoother::new(SmoothingStyle::Logarithmic(
                                self.osc_decay.clamp(0.0001, 999.9),
                            )),
                            SmoothStyle::Exponential => {
                                Smoother::new(SmoothingStyle::Exponential(self.osc_decay))
                            }
                            SmoothStyle::LogSteep => {
                                Smoother::new(SmoothingStyle::LogSteep(
                                    self.osc_decay.clamp(0.0001, 999.9)
                                ))
                            }
                        };

                        let release_smoother: Smoother<f32> = match self.osc_rel_curve {
                            SmoothStyle::Linear => {
                                Smoother::new(SmoothingStyle::Linear(self.osc_release))
                            }
                            SmoothStyle::Logarithmic => Smoother::new(SmoothingStyle::Logarithmic(
                                self.osc_release.clamp(0.0001, 999.9),
                            )),
                            SmoothStyle::Exponential => {
                                Smoother::new(SmoothingStyle::Exponential(self.osc_release))
                            }
                            SmoothStyle::LogSteep => {
                                Smoother::new(SmoothingStyle::LogSteep(
                                    self.osc_release.clamp(0.0001, 999.9)
                                ))
                            }
                        };

                        match attack_smoother.style {
                            SmoothingStyle::Logarithmic(_) | SmoothingStyle::LogSteep(_) => {
                                attack_smoother.reset(0.0001);
                                attack_smoother
                                    .set_target(self.sample_rate, velocity.clamp(0.0001, 999.9));
                            }
                            _ => {
                                attack_smoother.reset(0.0);
                                attack_smoother.set_target(self.sample_rate, velocity);
                            }
                        }

                        let scaled_sample_pos;
                        let scaled_end_pos;
                        match self.audio_module_type {
                            AudioModuleType::Sampler | AudioModuleType::Granulizer => {
                                // If ANY Sample content
                                if self.loaded_sample.len() > 0 && self.sample_lib.len() > 0 {
                                    // If our loaded sample variable or generated sample library has any content
                                    if self.loaded_sample[0].len() > 1
                                        && self.sample_lib[0][0].len() > 1
                                        && self.sample_lib.len() > 1
                                    {
                                        // Create our granulizer/sampler starting position from our knob scale
                                        scaled_sample_pos = if self.start_position > 0.0
                                            && self.osc_retrigger != RetriggerStyle::Random
                                            && self.osc_retrigger != RetriggerStyle::UniRandom
                                        {
                                            (self.sample_lib[note as usize][0].len() as f32
                                                * self.start_position)
                                                .floor()
                                                as usize
                                        }
                                        // Retrigger and use 0
                                        else if self.osc_retrigger != RetriggerStyle::Random
                                            && self.osc_retrigger != RetriggerStyle::UniRandom
                                        {
                                            0_usize
                                        }
                                        // Retrigger with random
                                        else {
                                            new_phase.floor() as usize
                                        };

                                        scaled_end_pos = if self._end_position < 1.0 {
                                            (self.sample_lib[note as usize][0].len() as f32
                                                * self._end_position)
                                                .ceil()
                                                as usize
                                        }
                                        // use end positions
                                        else {
                                            self.sample_lib[note as usize][0].len()
                                        };
                                    } else {
                                        // Nothing is in our sample library, skip attempting audio output
                                        return (0.0, 0.0, false, false);
                                    }
                                } else {
                                    // Nothing is in our sample library, skip attempting audio output
                                    return (0.0, 0.0, false, false);
                                }
                            }
                            _ => {
                                // These fields aren't used by Osc, Off
                                scaled_sample_pos = 0;
                                scaled_end_pos = 0;
                            }
                        }

                        // Osc Updates
                        let new_voice: SingleVoice = SingleVoice {
                            note: note,
                            _velocity: velocity,
                            vel_mod_amount: velocity_mod,
                            phase: new_phase,
                            //phase_delta: detuned_note / self.sample_rate,
                            phase_delta: 0.0,
                            state: OscState::Attacking,
                            // These get cloned since smoother cannot be copied
                            amp_current: 0.0,
                            osc_attack: attack_smoother.clone(),
                            osc_decay: decay_smoother.clone(),
                            osc_release: release_smoother.clone(),
                            pitch_enabled: self.pitch_enable,
                            pitch_env_peak: self.pitch_env_peak,
                            pitch_current: pitch_mod_current,
                            pitch_state: OscState::Attacking,
                            pitch_attack: pitch_attack_smoother.clone(),
                            pitch_decay: pitch_decay_smoother.clone(),
                            pitch_release: pitch_release_smoother.clone(),
                            pitch_enabled_2: self.pitch_enable_2,
                            pitch_env_peak_2: self.pitch_env_peak_2,
                            pitch_current_2: pitch_mod_current_2,
                            pitch_state_2: OscState::Attacking,
                            pitch_attack_2: pitch_attack_smoother_2.clone(),
                            pitch_decay_2: pitch_decay_smoother_2.clone(),
                            pitch_release_2: pitch_release_smoother_2.clone(),
                            _detune: self.osc_detune,
                            _unison_detune_value: self.osc_unison_detune,
                            //frequency: detuned_note,
                            frequency: 0.0,
                            _attack_time: self.osc_attack,
                            _decay_time: self.osc_decay,
                            _release_time: self.osc_release,
                            _retrigger: self.osc_retrigger,
                            _voice_type: self.audio_module_type,
                            _angle: 0.0,
                            sample_pos: scaled_sample_pos,
                            loop_it: self.loop_wavetable,
                            grain_start_pos: scaled_sample_pos,
                            _granular_gap: self.grain_gap,
                            _granular_hold: self.grain_hold,
                            granular_hold_end: scaled_sample_pos + self.grain_hold as usize,
                            next_grain_pos: scaled_sample_pos
                                + self.grain_hold as usize
                                + self.grain_gap as usize,
                            _end_position: scaled_end_pos,
                            _granular_crossfade: self.grain_crossfade,
                            grain_attack: Smoother::new(SmoothingStyle::Linear(
                                self.grain_crossfade as f32,
                            )),
                            grain_release: Smoother::new(SmoothingStyle::Linear(
                                self.grain_crossfade as f32,
                            )),
                            grain_state: GrainState::Attacking,
                            // Additive
                            harmonic_phases: {
                                let mut vector: Vec<f32> = Vec::with_capacity(16);
                                vector.push(0.0);
                                vector.push(0.0);
                                vector.push(0.0);
                                vector.push(0.0);
                                vector.push(0.0);
                                vector.push(0.0);
                                vector.push(0.0);
                                vector.push(0.0);
                                vector.push(0.0);
                                vector.push(0.0);
                                vector.push(0.0);
                                vector.push(0.0);
                                vector.push(0.0);
                                vector.push(0.0);
                                vector.push(0.0);
                                vector.push(0.0);
                                vector
                            },
                        };

                        // Add our voice struct to our voice tracking deque
                        self.playing_voices.voices.push_back(new_voice);

                        // Add unison voices to our voice tracking deque
                        if self.osc_unison > 1 && ( 
                            self.audio_module_type == AudioModuleType::Sine ||
                            self.audio_module_type == AudioModuleType::Tri ||
                            self.audio_module_type == AudioModuleType::Saw ||
                            self.audio_module_type == AudioModuleType::RSaw ||
                            self.audio_module_type == AudioModuleType::WSaw ||
                            self.audio_module_type == AudioModuleType::SSaw ||
                            self.audio_module_type == AudioModuleType::RASaw ||
                            self.audio_module_type == AudioModuleType::Ramp ||
                            self.audio_module_type == AudioModuleType::Square ||
                            self.audio_module_type == AudioModuleType::RSquare ||
                            self.audio_module_type == AudioModuleType::Pulse ||
                            self.audio_module_type == AudioModuleType::Noise ||
                            self.audio_module_type == AudioModuleType::Sampler ||
                            self.audio_module_type == AudioModuleType::Additive ) {
                            let unison_even_voices = if self.osc_unison % 2 == 0 {
                                self.osc_unison
                            } else {
                                self.osc_unison - 1
                            };
                            let mut unison_angles = vec![0.0; unison_even_voices as usize];
                            for i in 1..(unison_even_voices + 1) {
                                let voice_angle = self.calculate_panning(i - 1, self.osc_unison, stereo_algorithm);
                                unison_angles[(i - 1) as usize] = voice_angle;
                            }

                            for unison_voice in 0..(self.osc_unison as usize - 1) {
                                let uni_phase = match self.osc_retrigger {
                                    RetriggerStyle::UniRandom => {
                                        match self.audio_module_type {
                                            AudioModuleType::Additive |
                                            AudioModuleType::Sine |
                                            AudioModuleType::Tri |
                                            AudioModuleType::Saw |
                                            AudioModuleType::RSaw |
                                            AudioModuleType::WSaw |
                                            AudioModuleType::SSaw |
                                            AudioModuleType::RASaw |
                                            AudioModuleType::Ramp |
                                            AudioModuleType::Square |
                                            AudioModuleType::RSquare |
                                            AudioModuleType::Pulse |
                                            AudioModuleType::Noise => {
                                                let mut rng = rand::thread_rng();
                                                rng.gen_range(0.0..1.0)
                                            },
                                            AudioModuleType::Sampler | AudioModuleType::Granulizer=> {
                                                let mut rng = rand::thread_rng();
                                                // Prevent panic when no sample loaded yet
                                                if self.sample_lib.len() > 1 {
                                                    if self.sample_lib[note as usize][0].len() > 0 {
                                                        rng.gen_range(
                                                            0.0..self.sample_lib[note as usize][0].len() as f32,
                                                        ).floor()
                                                    } else {
                                                        // There's probably no sample loaded
                                                        0.0
                                                    }
                                                } else {
                                                    0.0
                                                }
                                            },
                                            AudioModuleType::Off | AudioModuleType::UnsetAm => {
                                                0.0
                                            },
                                        }
                                        
                                    }
                                    _ => new_phase,
                                };

                                let new_unison_voice: SingleVoice = SingleVoice {
                                    note: note,
                                    _velocity: velocity,
                                    vel_mod_amount: uni_velocity_mod,
                                    phase: uni_phase,
                                    phase_delta: unison_notes[unison_voice] / self.sample_rate,
                                    state: OscState::Attacking,
                                    // These get cloned since smoother cannot be copied
                                    amp_current: 0.0,
                                    osc_attack: attack_smoother.clone(),
                                    osc_decay: decay_smoother.clone(),
                                    osc_release: release_smoother.clone(),
                                    pitch_enabled: self.pitch_enable,
                                    pitch_env_peak: self.pitch_env_peak,
                                    pitch_current: pitch_mod_current,
                                    pitch_state: OscState::Attacking,
                                    pitch_attack: pitch_attack_smoother.clone(),
                                    pitch_decay: pitch_decay_smoother.clone(),
                                    pitch_release: pitch_release_smoother.clone(),
                                    pitch_enabled_2: self.pitch_enable_2,
                                    pitch_env_peak_2: self.pitch_env_peak_2,
                                    pitch_current_2: pitch_mod_current_2,
                                    pitch_state_2: OscState::Attacking,
                                    pitch_attack_2: pitch_attack_smoother_2.clone(),
                                    pitch_decay_2: pitch_decay_smoother_2.clone(),
                                    pitch_release_2: pitch_release_smoother_2.clone(),
                                    _detune: self.osc_detune,
                                    _unison_detune_value: self.osc_unison_detune,
                                    //frequency: unison_notes[unison_voice],
                                    frequency: 0.0,
                                    //frequency: detuned_note,
                                    _attack_time: self.osc_attack,
                                    _decay_time: self.osc_decay,
                                    _release_time: self.osc_release,
                                    _retrigger: self.osc_retrigger,
                                    _voice_type: self.audio_module_type,
                                    _angle: unison_angles[unison_voice],
                                    sample_pos: match self.audio_module_type {
                                        AudioModuleType::Additive |
                                        AudioModuleType::Sine |
                                        AudioModuleType::Tri |
                                        AudioModuleType::Saw |
                                        AudioModuleType::RSaw |
                                        AudioModuleType::WSaw |
                                        AudioModuleType::SSaw |
                                        AudioModuleType::RASaw |
                                        AudioModuleType::Ramp |
                                        AudioModuleType::Square |
                                        AudioModuleType::RSquare |
                                        AudioModuleType::Pulse |
                                        AudioModuleType::Noise => {
                                            0
                                        },
                                        AudioModuleType::Granulizer | AudioModuleType::Sampler => {
                                            uni_phase as usize
                                        },
                                        AudioModuleType::Off | AudioModuleType::UnsetAm => {
                                            0
                                        },
                                    },
                                    grain_start_pos: 0,
                                    loop_it: self.loop_wavetable,
                                    _granular_gap: 200,
                                    _granular_hold: 200,
                                    granular_hold_end: 200,
                                    next_grain_pos: 400,
                                    _end_position: scaled_end_pos,
                                    _granular_crossfade: 50,
                                    grain_attack: Smoother::new(SmoothingStyle::Linear(5.0)),
                                    grain_release: Smoother::new(SmoothingStyle::Linear(5.0)),
                                    grain_state: GrainState::Attacking,
                                    // Additive
                                    harmonic_phases: {
                                        let mut vector: Vec<f32> = Vec::with_capacity(16);
                                        vector.push(0.0);
                                        vector.push(0.0);
                                        vector.push(0.0);
                                        vector.push(0.0);
                                        vector.push(0.0);
                                        vector.push(0.0);
                                        vector.push(0.0);
                                        vector.push(0.0);
                                        vector.push(0.0);
                                        vector.push(0.0);
                                        vector.push(0.0);
                                        vector.push(0.0);
                                        vector.push(0.0);
                                        vector.push(0.0);
                                        vector.push(0.0);
                                        vector.push(0.0);
                                        vector
                                    },
                                };

                                self.unison_voices.voices.push_back(new_unison_voice);
                            }
                        }

                        // Remove the last voice when > voice_max
                        if self.playing_voices.voices.len() > voice_max {
                            self.playing_voices.voices.resize(
                                voice_max,
                                // Insert a dummy "Off" entry when resizing UP
                                SingleVoice {
                                    note: 0,
                                    _velocity: 0.0,
                                    vel_mod_amount: 0.0,
                                    phase: 0.0,
                                    phase_delta: 0.0,
                                    state: OscState::Off,
                                    amp_current: 0.0,
                                    osc_attack: Smoother::new(SmoothingStyle::None),
                                    osc_decay: Smoother::new(SmoothingStyle::None),
                                    osc_release: Smoother::new(SmoothingStyle::None),
                                    pitch_enabled: self.pitch_enable,
                                    pitch_env_peak: self.pitch_env_peak,
                                    pitch_current: 0.0,
                                    pitch_state: OscState::Attacking,
                                    pitch_attack: Smoother::new(SmoothingStyle::None),
                                    pitch_decay: Smoother::new(SmoothingStyle::None),
                                    pitch_release: Smoother::new(SmoothingStyle::None),
                                    pitch_enabled_2: self.pitch_enable_2,
                                    pitch_env_peak_2: self.pitch_env_peak_2,
                                    pitch_current_2: 0.0,
                                    pitch_state_2: OscState::Attacking,
                                    pitch_attack_2: Smoother::new(SmoothingStyle::None),
                                    pitch_decay_2: Smoother::new(SmoothingStyle::None),
                                    pitch_release_2: Smoother::new(SmoothingStyle::None),
                                    _detune: 0.0,
                                    _unison_detune_value: 0.0,
                                    frequency: 0.0,
                                    _attack_time: self.osc_attack,
                                    _decay_time: self.osc_decay,
                                    _release_time: self.osc_release,
                                    _retrigger: self.osc_retrigger,
                                    _voice_type: self.audio_module_type,
                                    _angle: 0.0,
                                    sample_pos: 0,
                                    loop_it: self.loop_wavetable,
                                    grain_start_pos: 0,
                                    _granular_gap: 200,
                                    _granular_hold: 200,
                                    granular_hold_end: 200,
                                    next_grain_pos: 400,
                                    _end_position: scaled_end_pos,
                                    _granular_crossfade: 50,
                                    grain_attack: Smoother::new(SmoothingStyle::Linear(5.0)),
                                    grain_release: Smoother::new(SmoothingStyle::Linear(5.0)),
                                    grain_state: GrainState::Attacking,
                                    // Additive
                                    harmonic_phases: Vec::new(),
                                },
                            );

                            if self.osc_unison > 1 && ( self.audio_module_type == AudioModuleType::Sine ||
                                self.audio_module_type == AudioModuleType::Tri ||
                                self.audio_module_type == AudioModuleType::Saw ||
                                self.audio_module_type == AudioModuleType::RSaw ||
                                self.audio_module_type == AudioModuleType::WSaw ||
                                self.audio_module_type == AudioModuleType::SSaw ||
                                self.audio_module_type == AudioModuleType::RASaw ||
                                self.audio_module_type == AudioModuleType::Ramp ||
                                self.audio_module_type == AudioModuleType::Square ||
                                self.audio_module_type == AudioModuleType::RSquare ||
                                self.audio_module_type == AudioModuleType::Pulse ||
                                self.audio_module_type == AudioModuleType::Noise ||
                                self.audio_module_type == AudioModuleType::Sampler )
                            {
                                self.unison_voices.voices.resize(
                                    voice_max as usize,
                                    // Insert a dummy "Off" entry when resizing UP
                                    SingleVoice {
                                        note: 0,
                                        _velocity: 0.0,
                                        vel_mod_amount: 0.0,
                                        phase: new_phase,
                                        phase_delta: 0.0,
                                        state: OscState::Off,
                                        amp_current: 0.0,
                                        osc_attack: Smoother::new(SmoothingStyle::None),
                                        osc_decay: Smoother::new(SmoothingStyle::None),
                                        osc_release: Smoother::new(SmoothingStyle::None),
                                        pitch_enabled: self.pitch_enable,
                                        pitch_env_peak: 0.0,
                                        pitch_current: 0.0,
                                        pitch_state: OscState::Off,
                                        pitch_attack: Smoother::new(SmoothingStyle::None),
                                        pitch_decay: Smoother::new(SmoothingStyle::None),
                                        pitch_release: Smoother::new(SmoothingStyle::None),
                                        pitch_enabled_2: self.pitch_enable_2,
                                        pitch_env_peak_2: 0.0,
                                        pitch_current_2: 0.0,
                                        pitch_state_2: OscState::Off,
                                        pitch_attack_2: Smoother::new(SmoothingStyle::None),
                                        pitch_decay_2: Smoother::new(SmoothingStyle::None),
                                        pitch_release_2: Smoother::new(SmoothingStyle::None),
                                        _detune: 0.0,
                                        _unison_detune_value: 0.0,
                                        frequency: 0.0,
                                        _attack_time: self.osc_attack,
                                        _decay_time: self.osc_decay,
                                        _release_time: self.osc_release,
                                        _retrigger: self.osc_retrigger,
                                        _voice_type: self.audio_module_type,
                                        _angle: 0.0,
                                        sample_pos: 0,
                                        grain_start_pos: 0,
                                        loop_it: self.loop_wavetable,
                                        _granular_gap: 200,
                                        _granular_hold: 200,
                                        granular_hold_end: 200,
                                        next_grain_pos: 400,
                                        _end_position: scaled_end_pos,
                                        _granular_crossfade: 50,
                                        grain_attack: Smoother::new(SmoothingStyle::Linear(5.0)),
                                        grain_release: Smoother::new(SmoothingStyle::Linear(5.0)),
                                        grain_state: GrainState::Attacking,
                                        // Additive
                                        harmonic_phases: Vec::new(),
                                    },
                                );
                            }
                        }

                        // Remove any off notes
                        self.playing_voices.voices.retain(|voice| {
                            voice.state != OscState::Off &&
                            !(voice.grain_state == GrainState::Releasing && voice.grain_release.steps_left() == 0)
                        });
                        if self.audio_module_type == AudioModuleType::Sine ||
                        self.audio_module_type == AudioModuleType::Tri ||
                        self.audio_module_type == AudioModuleType::Saw ||
                        self.audio_module_type == AudioModuleType::RSaw ||
                        self.audio_module_type == AudioModuleType::WSaw ||
                        self.audio_module_type == AudioModuleType::SSaw ||
                        self.audio_module_type == AudioModuleType::RASaw ||
                        self.audio_module_type == AudioModuleType::Ramp ||
                        self.audio_module_type == AudioModuleType::Square ||
                        self.audio_module_type == AudioModuleType::RSquare ||
                        self.audio_module_type == AudioModuleType::Pulse ||
                        self.audio_module_type == AudioModuleType::Noise ||
                        self.audio_module_type == AudioModuleType::Sampler {
                            self.unison_voices.voices.retain(|unison_voice| {
                                unison_voice.state != OscState::Off
                            });
                        }
                    }
                    ////////////////////////////////////////////////////////////
                    // MIDI EVENT NOTE OFF
                    ////////////////////////////////////////////////////////////
                    NoteEvent::NoteOff { note, .. } => {
                        // Set note off variable to pass back to filter
                        note_off = true;

                        // Get voices on our note and not already releasing
                        // When a voice reaches 0.0 target on releasing

                        let mut shifted_note: u8 = note;

                        // Sampler when single cycle needs this!!!
                        if self.single_cycle {
                            // 31 comes from comparing with 3xOsc position in MIDI notes
                            shifted_note += 31;
                        }

                        // Calculate note shifting to match note on shifts
                        let semi_shift: u8 = self.osc_semitones as u8;
                        shifted_note = match self.osc_octave {
                            -2 => shifted_note - 24 + semi_shift,
                            -1 => shifted_note - 12 + semi_shift,
                            0 => shifted_note + semi_shift,
                            1 => shifted_note + 12 + semi_shift,
                            2 => shifted_note + 24 + semi_shift,
                            _ => shifted_note + semi_shift,
                        };

                        if self.audio_module_type == AudioModuleType::Sine ||
                        self.audio_module_type == AudioModuleType::Tri ||
                        self.audio_module_type == AudioModuleType::Saw ||
                        self.audio_module_type == AudioModuleType::RSaw ||
                        self.audio_module_type == AudioModuleType::WSaw ||
                        self.audio_module_type == AudioModuleType::SSaw ||
                        self.audio_module_type == AudioModuleType::RASaw ||
                        self.audio_module_type == AudioModuleType::Ramp ||
                        self.audio_module_type == AudioModuleType::Square ||
                        self.audio_module_type == AudioModuleType::RSquare ||
                        self.audio_module_type == AudioModuleType::Pulse ||
                        self.audio_module_type == AudioModuleType::Noise ||
                        self.audio_module_type == AudioModuleType::Sampler ||
                        self.audio_module_type == AudioModuleType::Additive {
                            // Update the matching unison voices
                            for unison_voice in self.unison_voices.voices.iter_mut() {
                                if unison_voice.note == shifted_note
                                    && unison_voice.state != OscState::Releasing
                                {
                                    // Start our release level from our current gain on the voice
                                    unison_voice.osc_release.reset(unison_voice.amp_current);
                                    // Set our new release target to 0.0 so the note fades
                                    match unison_voice.osc_release.style {
                                        SmoothingStyle::Logarithmic(_)
                                        | SmoothingStyle::LogSteep(_) => {
                                            unison_voice
                                                .osc_release
                                                .set_target(self.sample_rate, 0.0001);
                                        }
                                        _ => {
                                            unison_voice
                                                .osc_release
                                                .set_target(self.sample_rate, 0.0);
                                        }
                                    }
                                    // Update our current amp
                                    unison_voice.amp_current = unison_voice.osc_release.next();
                                    // Update our voice state
                                    unison_voice.state = OscState::Releasing;
                                }
                            }
                        }

                        // Iterate through our voice vecdeque to find the one to update
                        for voice in self.playing_voices.voices.iter_mut() {
                            // Update current voices to releasing state if they're valid
                            if voice.note == shifted_note && voice.state != OscState::Releasing {
                                // Start our release level from our current gain on the voice
                                voice.osc_release.reset(voice.amp_current);

                                // Set our new release target to 0.0 so the note fades
                                match voice.osc_release.style {
                                    SmoothingStyle::Logarithmic(_)
                                    | SmoothingStyle::LogSteep(_) => {
                                        voice.osc_release.set_target(self.sample_rate, 0.0001);
                                    }
                                    _ => {
                                        voice.osc_release.set_target(self.sample_rate, 0.0);
                                    }
                                }
                                // Update our current amp
                                voice.amp_current = voice.osc_release.next();

                                // Update our base voice state to releasing
                                voice.state = OscState::Releasing;
                            }
                        }
                    }
                    // Stop event - doesn't seem to work from FL Studio but left in here
                    NoteEvent::Choke { .. } => {
                        self.playing_voices.voices.clear();
                        self.unison_voices.voices.clear();
                    }
                    _ => (),
                }
            }
            // The event was invalid - do nothing
            None => (),
        }

        let mut new_grain: bool = false;
        //if self.audio_module_type == AudioModuleType::Granulizer {
            // This is a dummy entry
            let mut next_grain: SingleVoice = SingleVoice {
                note: 0,
                _velocity: 0.0,
                vel_mod_amount: 0.0,
                phase: 0.0,
                phase_delta: 0.0,
                state: OscState::Off,
                // These get cloned since smoother cannot be copied
                amp_current: 0.0,
                osc_attack: Smoother::new(SmoothingStyle::None),
                osc_decay: Smoother::new(SmoothingStyle::None),
                osc_release: Smoother::new(SmoothingStyle::None),
                pitch_enabled: false,
                pitch_env_peak: 0.0,
                pitch_current: 0.0,
                pitch_state: OscState::Off,
                pitch_attack: Smoother::new(SmoothingStyle::None),
                pitch_decay: Smoother::new(SmoothingStyle::None),
                pitch_release: Smoother::new(SmoothingStyle::None),
                pitch_enabled_2: false,
                pitch_env_peak_2: 0.0,
                pitch_current_2: 0.0,
                pitch_state_2: OscState::Off,
                pitch_attack_2: Smoother::new(SmoothingStyle::None),
                pitch_decay_2: Smoother::new(SmoothingStyle::None),
                pitch_release_2: Smoother::new(SmoothingStyle::None),
                _detune: 0.0,
                _unison_detune_value: 0.0,
                frequency: 0.0,
                _attack_time: self.osc_attack,
                _decay_time: self.osc_decay,
                _release_time: self.osc_release,
                _retrigger: self.osc_retrigger,
                _voice_type: self.audio_module_type,
                _angle: 0.0,
                sample_pos: 0,
                loop_it: self.loop_wavetable,
                grain_start_pos: 0,
                _granular_gap: 200,
                _granular_hold: 200,
                granular_hold_end: 200,
                next_grain_pos: 400,
                _end_position: 800,
                _granular_crossfade: 50,
                grain_attack: Smoother::new(SmoothingStyle::Linear(5.0)),
                grain_release: Smoother::new(SmoothingStyle::Linear(5.0)),
                grain_state: GrainState::Attacking,
                // Additive
                harmonic_phases: Vec::new(),
            };
        //}
        

        // Second check for off notes before output to cut down on iterating
        // Remove any off notes
        self.playing_voices.voices.retain(|voice| {
            voice.state != OscState::Off &&
            !(voice.grain_state == GrainState::Releasing && voice.grain_release.steps_left() == 0)
        });
        if self.audio_module_type != AudioModuleType::Off {
            self.unison_voices.voices.retain(|unison_voice| {
                unison_voice.state != OscState::Off
            });
        }

        ////////////////////////////////////////////////////////////
        // Update our voices before output
        ////////////////////////////////////////////////////////////
        
        if self.audio_module_type == AudioModuleType::Additive {
            if check_inequality(self.ah0, self.prev_ah0, self.ah1, self.prev_ah1, 
                self.ah2, self.prev_ah2, self.ah3, self.prev_ah3, self.ah4, 
                self.prev_ah4, self.ah5, self.prev_ah5, 
                self.ah6, self.prev_ah6, self.ah7, self.prev_ah7,
                self.ah8, self.prev_ah8, self.ah9, self.prev_ah9, 
                self.ah10, self.prev_ah10, self.ah11, self.prev_ah11, self.ah12, 
                self.prev_ah12, self.ah13, self.prev_ah13, 
                self.ah14, self.prev_ah14, self.ah15, self.prev_ah15) {
                    self.prev_ah0 = self.ah0;
                    self.prev_ah1 = self.ah1;
                    self.prev_ah2 = self.ah2;
                    self.prev_ah3 = self.ah3;
                    self.prev_ah4 = self.ah4;
                    self.prev_ah5 = self.ah5;
                    self.prev_ah6 = self.ah6;
                    self.prev_ah7 = self.ah7;
                    self.prev_ah8 = self.ah8;
                    self.prev_ah9 = self.ah9;
                    self.prev_ah10 = self.ah10;
                    self.prev_ah11 = self.ah11;
                    self.prev_ah12 = self.ah12;
                    self.prev_ah13 = self.ah13;
                    self.prev_ah14 = self.ah14;
                    self.prev_ah15 = self.ah15;
                    let mut new_harmonics = Vec::with_capacity(16);
                    new_harmonics.push(AdditiveHarmonic {
                        index: 0,
                        amplitude: self.ah0,
                    });
                    new_harmonics.push(AdditiveHarmonic {
                        index: 1,
                        amplitude: self.ah1,
                    });
                    new_harmonics.push(AdditiveHarmonic {
                        index: 2,
                        amplitude: self.ah2,
                    });
                    new_harmonics.push(AdditiveHarmonic {
                        index: 3,
                        amplitude: self.ah3,
                    });
                    new_harmonics.push(AdditiveHarmonic {
                        index: 4,
                        amplitude: self.ah4,
                    });
                    new_harmonics.push(AdditiveHarmonic {
                        index: 5,
                        amplitude: self.ah5,
                    });
                    new_harmonics.push(AdditiveHarmonic {
                        index: 6,
                        amplitude: self.ah6,
                    });
                    new_harmonics.push(AdditiveHarmonic {
                        index: 7,
                        amplitude: self.ah7,
                    });
                    new_harmonics.push(AdditiveHarmonic {
                        index: 8,
                        amplitude: self.ah8,
                    });
                    new_harmonics.push(AdditiveHarmonic {
                        index: 9,
                        amplitude: self.ah9,
                    });
                    new_harmonics.push(AdditiveHarmonic {
                        index: 10,
                        amplitude: self.ah10,
                    });
                    new_harmonics.push(AdditiveHarmonic {
                        index: 11,
                        amplitude: self.ah11,
                    });
                    new_harmonics.push(AdditiveHarmonic {
                        index: 12,
                        amplitude: self.ah12,
                    });
                    new_harmonics.push(AdditiveHarmonic {
                        index: 13,
                        amplitude: self.ah13,
                    });
                    new_harmonics.push(AdditiveHarmonic {
                        index: 14,
                        amplitude: self.ah14,
                    });
                    new_harmonics.push(AdditiveHarmonic {
                        index: 15,
                        amplitude: self.ah15,
                    });
                    self.additive_module.set_harmonics(new_harmonics);
                }
        }
         
        for voice in self.playing_voices.voices.iter_mut() {
            if self.audio_module_type != AudioModuleType::Granulizer
                && self.audio_module_type != AudioModuleType::Off
                && self.audio_module_type != AudioModuleType::UnsetAm
            {
                // Move our phase outside of the midi events
                // I couldn't find much on how to model this so I based it off previous note phase
                voice.phase += voice.phase_delta;
                if voice.phase > 1.0 {
                    voice.phase -= 1.0;
                }
                // This happens on extreme pitch envelope values only and catches wild increments
                // or pitches above nyquist that would alias into other pitches
                if voice.phase > 1.0 {
                    voice.phase = voice.phase % 1.0;
                }

                // Move our pitch envelopes if this is an Osc
                if voice.pitch_enabled {
                    // Attack is over so use decay amount to reach sustain level - reusing current smoother
                    if voice.pitch_attack.steps_left() == 0
                        && voice.pitch_state == OscState::Attacking
                    {
                        voice.pitch_state = OscState::Decaying;
                        voice.pitch_current = voice.pitch_attack.next();
                        // Now we will use decay smoother from here
                        voice.pitch_decay.reset(voice.pitch_current);
                        let sustain_scaled = self.pitch_env_sustain / 999.9;
                        voice
                            .pitch_decay
                            .set_target(self.sample_rate, sustain_scaled.clamp(0.0001, 999.9));
                    }

                    // Move from Decaying to Sustain hold
                    if voice.pitch_decay.steps_left() == 0
                        && voice.pitch_state == OscState::Decaying
                    {
                        let sustain_scaled = self.pitch_env_sustain / 999.9;
                        voice.pitch_current = sustain_scaled;
                        voice
                            .pitch_decay
                            .set_target(self.sample_rate, sustain_scaled.clamp(0.0001, 999.9));
                        voice.pitch_state = OscState::Sustaining;
                    }

                    // End of release
                    if voice.pitch_state == OscState::Releasing
                        && voice.pitch_release.steps_left() == 0
                    {
                        voice.pitch_state = OscState::Off;
                    }
                } else {
                    // Reassign here for safety
                    voice.pitch_current = 0.0;
                    voice.pitch_state = OscState::Off;
                }
                if (self.audio_module_type != AudioModuleType::Granulizer
                && self.audio_module_type != AudioModuleType::Off
                && self.audio_module_type != AudioModuleType::Sampler
                && self.audio_module_type != AudioModuleType::UnsetAm) && voice.pitch_enabled_2 {
                    // Attack is over so use decay amount to reach sustain level - reusing current smoother
                    if voice.pitch_attack_2.steps_left() == 0
                        && voice.pitch_state_2 == OscState::Attacking
                    {
                        voice.pitch_state_2 = OscState::Decaying;
                        voice.pitch_current_2 = voice.pitch_attack_2.next();
                        // Now we will use decay smoother from here
                        voice.pitch_decay_2.reset(voice.pitch_current_2);
                        let sustain_scaled_2 = self.pitch_env_sustain_2 / 999.9;
                        voice
                            .pitch_decay_2
                            .set_target(self.sample_rate, sustain_scaled_2.clamp(0.0001, 999.9));
                    }

                    // Move from Decaying to Sustain hold
                    if voice.pitch_decay_2.steps_left() == 0
                        && voice.pitch_state_2 == OscState::Decaying
                    {
                        let sustain_scaled_2 = self.pitch_env_sustain_2 / 999.9;
                        voice.pitch_current_2 = sustain_scaled_2;
                        voice
                            .pitch_decay_2
                            .set_target(self.sample_rate, sustain_scaled_2.clamp(0.0001, 999.9));
                        voice.pitch_state_2 = OscState::Sustaining;
                    }

                    // End of release
                    if voice.pitch_state_2 == OscState::Releasing
                        && voice.pitch_release_2.steps_left() == 0
                    {
                        voice.pitch_state_2 = OscState::Off;
                    }
                } else {
                    // Reassign here for safety
                    voice.pitch_current_2 = 0.0;
                    voice.pitch_state_2 = OscState::Off;
                }

                // Move from attack to decay if needed
                // Attack is over so use decay amount to reach sustain level - reusing current smoother
                if voice.osc_attack.steps_left() == 0 && voice.state == OscState::Attacking {
                    voice.state = OscState::Decaying;
                    voice.amp_current = voice.osc_attack.next();
                    // Now we will use decay smoother from here
                    voice.osc_decay.reset(voice.amp_current);
                    let sustain_scaled = self.osc_sustain / 999.9;
                    voice.osc_decay.set_target(self.sample_rate, sustain_scaled);
                }

                // Move from Decaying to Sustain hold
                if voice.osc_decay.steps_left() == 0 && voice.state == OscState::Decaying {
                    let sustain_scaled = self.osc_sustain / 999.9;
                    voice.amp_current = sustain_scaled;
                    voice.osc_decay.set_target(self.sample_rate, sustain_scaled);
                    voice.state = OscState::Sustaining;
                }

                // End of release
                if voice.state == OscState::Releasing && voice.osc_release.steps_left() == 0 {
                    voice.state = OscState::Off;
                }
            } else if self.audio_module_type == AudioModuleType::Granulizer {
                // Move from attack to decay if needed
                // Attack is over so use decay amount to reach sustain level - reusing current smoother
                if voice.osc_attack.steps_left() == 0 && voice.state == OscState::Attacking {
                    voice.state = OscState::Decaying;
                    voice.amp_current = voice.osc_attack.next();
                    // Now we will use decay smoother from here
                    voice.osc_decay.reset(voice.amp_current);
                    let sustain_scaled = self.osc_sustain / 999.9;
                    voice.osc_decay.set_target(self.sample_rate, sustain_scaled);
                }

                // Move from Decaying to Sustain hold
                if voice.osc_decay.steps_left() == 0 && voice.state == OscState::Decaying {
                    let sustain_scaled = self.osc_sustain / 999.9;
                    voice.amp_current = sustain_scaled;
                    voice.osc_decay.set_target(self.sample_rate, sustain_scaled);
                    voice.state = OscState::Sustaining;
                }

                let scaled_start_position =
                    (self.loaded_sample[0].len() as f32 * self.start_position).floor() as usize;
                let scaled_end_position =
                    (self.loaded_sample[0].len() as f32 * self._end_position).floor() as usize;

                // If our end grain marker goes outside of our sample length it should wrap around if looping
                if voice.granular_hold_end > self.loaded_sample[0].len()
                    || voice.granular_hold_end > voice._end_position
                {
                    if voice.loop_it {
                        voice.granular_hold_end = voice.granular_hold_end
                            - self.loaded_sample[0].len()
                            + scaled_start_position;
                    } else {
                        voice.granular_hold_end = scaled_end_position;
                    }
                }
                // If our next grain goes outside of our sample length it should also wrap on loop
                if voice.next_grain_pos > self.loaded_sample[0].len()
                    || voice.next_grain_pos > voice._end_position
                {
                    if voice.loop_it {
                        voice.next_grain_pos -= self.loaded_sample[0].len() - scaled_start_position;
                    } else {
                        voice.next_grain_pos = scaled_end_position;
                    }
                }
                // If we are in the start grain crossfade
                if voice.sample_pos == voice.grain_start_pos {
                    voice.grain_state = GrainState::Attacking;
                    voice.grain_attack.reset(0.0);
                    voice.grain_attack.set_target(self.sample_rate, 1.0);
                }
                // If we are in the end grain crossfade
                else if voice.sample_pos > voice.granular_hold_end
                    && voice.grain_state == GrainState::Attacking
                {
                    voice.grain_state = GrainState::Releasing;
                    voice.grain_release.reset(1.0);
                    voice.grain_release.set_target(self.sample_rate, 0.0);
                    // If we are at the end of our grain and need to create a new one
                    new_grain = true;
                    let new_end = voice.next_grain_pos + self.grain_hold as usize;
                    next_grain = SingleVoice {
                        note: voice.note,
                        _velocity: voice._velocity,
                        vel_mod_amount: 0.0,
                        phase: voice.phase,
                        phase_delta: voice.phase_delta,
                        state: voice.state,
                        // These get cloned since smoother cannot be copied
                        amp_current: voice.amp_current,
                        osc_attack: voice.osc_attack.clone(),
                        osc_decay: voice.osc_decay.clone(),
                        osc_release: voice.osc_release.clone(),
                        pitch_enabled: voice.pitch_enabled,
                        pitch_env_peak: voice.pitch_env_peak,
                        pitch_current: voice.pitch_current,
                        pitch_state: voice.pitch_state,
                        pitch_attack: voice.pitch_attack.clone(),
                        pitch_decay: voice.pitch_decay.clone(),
                        pitch_release: voice.pitch_release.clone(),
                        pitch_enabled_2: voice.pitch_enabled_2,
                        pitch_env_peak_2: voice.pitch_env_peak_2,
                        pitch_current_2: voice.pitch_current_2,
                        pitch_state_2: voice.pitch_state_2,
                        pitch_attack_2: voice.pitch_attack_2.clone(),
                        pitch_decay_2: voice.pitch_decay_2.clone(),
                        pitch_release_2: voice.pitch_release_2.clone(),
                        _detune: voice._detune,
                        _unison_detune_value: voice._unison_detune_value,
                        frequency: voice.frequency,
                        _attack_time: voice._attack_time,
                        _decay_time: voice._decay_time,
                        _release_time: voice._release_time,
                        _retrigger: voice._retrigger,
                        _voice_type: voice._voice_type,
                        _angle: voice._angle,
                        sample_pos: voice.next_grain_pos,
                        loop_it: voice.loop_it,
                        grain_start_pos: voice.next_grain_pos,
                        _granular_gap: self.grain_gap,
                        _granular_hold: self.grain_hold,
                        granular_hold_end: new_end,
                        next_grain_pos: new_end + self.grain_gap as usize,
                        _end_position: voice._end_position,
                        _granular_crossfade: self.grain_crossfade,
                        grain_attack: Smoother::new(SmoothingStyle::Linear(
                            self.grain_crossfade as f32,
                        )),
                        grain_release: Smoother::new(SmoothingStyle::Linear(
                            self.grain_crossfade as f32,
                        )),
                        grain_state: GrainState::Attacking,
                        // Additive
                        harmonic_phases: Vec::new(),
                    };
                }

                // End of release
                if (voice.state == OscState::Releasing && voice.osc_release.steps_left() == 0)
                    || (voice.grain_state == GrainState::Releasing
                        && voice.grain_release.steps_left() == 0)
                {
                    voice.state = OscState::Off;
                }
            }
        }

        match self.audio_module_type {
            AudioModuleType::Sine |
            AudioModuleType::Tri |
            AudioModuleType::Saw |
            AudioModuleType::RSaw |
            AudioModuleType::WSaw |
            AudioModuleType::SSaw |
            AudioModuleType::RASaw |
            AudioModuleType::Ramp |
            AudioModuleType::Square |
            AudioModuleType::RSquare |
            AudioModuleType::Pulse |
            AudioModuleType::Noise |
            AudioModuleType::Additive => {
                // Update our matching unison voices
                for unison_voice in self.unison_voices.voices.iter_mut() {
                    // Move our phase outside of the midi events
                    // I couldn't find much on how to model this so I based it off previous note phase
                    unison_voice.phase += unison_voice.phase_delta;
                    if unison_voice.phase > 1.0 {
                        unison_voice.phase -= 1.0;
                    }
                    // This happens on extreme pitch envelope values only and catches wild increments
                    // or pitches above nyquist that would alias into other pitches
                    if unison_voice.phase > 1.0 {
                        unison_voice.phase = unison_voice.phase % 1.0;
                    }

                    // Move our pitch envelopes if this is an Osc
                    if unison_voice.pitch_enabled
                    {
                        // Attack is over so use decay amount to reach sustain level - reusing current smoother
                        if unison_voice.pitch_attack.steps_left() == 0
                            && unison_voice.pitch_state == OscState::Attacking
                        {
                            unison_voice.pitch_state = OscState::Decaying;
                            unison_voice.pitch_current = unison_voice.pitch_attack.next();
                            // Now we will use decay smoother from here
                            unison_voice.pitch_decay.reset(unison_voice.pitch_current);
                            let sustain_scaled = self.pitch_env_sustain / 999.9;
                            unison_voice
                                .pitch_decay
                                .set_target(self.sample_rate, sustain_scaled.clamp(0.0001, 999.9));
                        }

                        // Move from Decaying to Sustain hold
                        if unison_voice.pitch_decay.steps_left() == 0
                            && unison_voice.pitch_state == OscState::Decaying
                        {
                            let sustain_scaled = self.pitch_env_sustain / 999.9;
                            unison_voice.pitch_current = sustain_scaled;
                            unison_voice
                                .pitch_decay
                                .set_target(self.sample_rate, sustain_scaled.clamp(0.0001, 999.9));
                            unison_voice.pitch_state = OscState::Sustaining;
                        }

                        // End of release
                        if unison_voice.pitch_state == OscState::Releasing
                            && unison_voice.pitch_release.steps_left() == 0
                        {
                            unison_voice.pitch_state = OscState::Off;
                        }
                    } else {
                        // Reassign here for safety
                        unison_voice.pitch_current = 0.0;
                        unison_voice.pitch_state = OscState::Off;
                    }
                    if unison_voice.pitch_enabled_2
                    {
                        // Attack is over so use decay amount to reach sustain level - reusing current smoother
                        if unison_voice.pitch_attack_2.steps_left() == 0
                            && unison_voice.pitch_state_2 == OscState::Attacking
                        {
                            unison_voice.pitch_state_2 = OscState::Decaying;
                            unison_voice.pitch_current_2 = unison_voice.pitch_attack_2.next();
                            // Now we will use decay smoother from here
                            unison_voice
                                .pitch_decay_2
                                .reset(unison_voice.pitch_current_2);
                            let sustain_scaled_2 = self.pitch_env_sustain_2 / 999.9;
                            unison_voice.pitch_decay_2.set_target(
                                self.sample_rate,
                                sustain_scaled_2.clamp(0.0001, 999.9),
                            );
                        }

                        // Move from Decaying to Sustain hold
                        if unison_voice.pitch_decay_2.steps_left() == 0
                            && unison_voice.pitch_state_2 == OscState::Decaying
                        {
                            let sustain_scaled_2 = self.pitch_env_sustain_2 / 999.9;
                            unison_voice.pitch_current_2 = sustain_scaled_2;
                            unison_voice.pitch_decay_2.set_target(
                                self.sample_rate,
                                sustain_scaled_2.clamp(0.0001, 999.9),
                            );
                            unison_voice.pitch_state_2 = OscState::Sustaining;
                        }

                        // End of release
                        if unison_voice.pitch_state_2 == OscState::Releasing
                            && unison_voice.pitch_release_2.steps_left() == 0
                        {
                            unison_voice.pitch_state_2 = OscState::Off;
                        }
                    } else {
                        // Reassign here for safety
                        unison_voice.pitch_current_2 = 0.0;
                        unison_voice.pitch_state_2 = OscState::Off;
                    }

                    // Move from attack to decay if needed
                    // Attack is over so use decay amount to reach sustain level - reusing current smoother
                    if unison_voice.osc_attack.steps_left() == 0
                        && unison_voice.state == OscState::Attacking
                    {
                        unison_voice.state = OscState::Decaying;
                        unison_voice.amp_current = unison_voice.osc_attack.next();
                        // Now we will use decay smoother from here
                        unison_voice.osc_decay.reset(unison_voice.amp_current);
                        let sustain_scaled = self.osc_sustain / 999.9;
                        unison_voice
                            .osc_decay
                            .set_target(self.sample_rate, sustain_scaled);
                    }
                    // Move from Decaying to Sustain hold
                    if unison_voice.osc_decay.steps_left() == 0
                        && unison_voice.state == OscState::Decaying
                    {
                        unison_voice.state = OscState::Sustaining;
                        let sustain_scaled = self.osc_sustain / 999.9;
                        unison_voice.amp_current = sustain_scaled;
                        unison_voice
                            .osc_decay
                            .set_target(self.sample_rate, sustain_scaled);
                    }
                    // End of release
                    if unison_voice.state == OscState::Releasing
                        && unison_voice.osc_release.steps_left() == 0
                    {
                        unison_voice.state = OscState::Off;
                    }
                }
            }
            _ => {}
        }

        // Add our new grain to our voices
        if new_grain {
            self.playing_voices.voices.push_back(next_grain);
        }

        ////////////////////////////////////////////////////////////
        // Create output
        ////////////////////////////////////////////////////////////
        let output_signal_l: f32;
        let output_signal_r: f32;
        (output_signal_l, output_signal_r) = match self.audio_module_type {
            AudioModuleType::Sine |
            AudioModuleType::Tri |
            AudioModuleType::Saw |
            AudioModuleType::RSaw |
            AudioModuleType::WSaw |
            AudioModuleType::SSaw |
            AudioModuleType::RASaw |
            AudioModuleType::Ramp |
            AudioModuleType::Square |
            AudioModuleType::RSquare |
            AudioModuleType::Pulse |
            AudioModuleType::Noise => {
                let mut summed_voices_l: f32 = 0.0;
                let mut summed_voices_r: f32 = 0.0;
                let mut stereo_voices_l: f32 = 0.0;
                let mut stereo_voices_r: f32 = 0.0;
                let mut center_voices: f32 = 0.0;
                for voice in self.playing_voices.voices.iter_mut() {
                    // Move the pitch envelope stuff independently of the MIDI info
                    if voice.pitch_enabled {
                        voice.pitch_current = 
                        match voice.pitch_state {
                            OscState::Attacking => voice.pitch_attack.next(),
                            OscState::Decaying => voice.pitch_decay.next(),
                            OscState::Sustaining => self.pitch_env_sustain / 999.9,
                            OscState::Releasing => voice.pitch_release.next(),
                            OscState::Off => 0.0,
                        }
                    }
                    if voice.pitch_enabled_2 {
                        voice.pitch_current_2 = match voice.pitch_state_2 {
                            OscState::Attacking => voice.pitch_attack_2.next(),
                            OscState::Decaying => voice.pitch_decay_2.next(),
                            OscState::Sustaining => self.pitch_env_sustain_2 / 999.9,
                            OscState::Releasing => voice.pitch_release_2.next(),
                            OscState::Off => 0.0,
                        }
                    }

                    let temp_osc_gain_multiplier: f32;
                    // Get our current gain amount for use in match below
                    // Include gain scaling if mod is there
                    if vel_gain_mod != -2.0 {
                        temp_osc_gain_multiplier = match voice.state {
                            OscState::Attacking => {
                                voice.osc_attack.next() * vel_gain_mod * vel_lfo_gain_mod
                            }
                            OscState::Decaying => {
                                voice.osc_decay.next() * vel_gain_mod * vel_lfo_gain_mod
                            }
                            OscState::Sustaining => {
                                (self.osc_sustain / 999.9) * vel_gain_mod * vel_lfo_gain_mod
                            }
                            OscState::Releasing => {
                                voice.osc_release.next() * vel_gain_mod * vel_lfo_gain_mod
                            }
                            OscState::Off => 0.0,
                        };
                    } else {
                        temp_osc_gain_multiplier = match voice.state {
                            OscState::Attacking => voice.osc_attack.next() * vel_lfo_gain_mod,
                            OscState::Decaying => voice.osc_decay.next() * vel_lfo_gain_mod,
                            OscState::Sustaining => (self.osc_sustain / 999.9) * vel_lfo_gain_mod,
                            OscState::Releasing => voice.osc_release.next() * vel_lfo_gain_mod,
                            OscState::Off => 0.0,
                        };
                    }

                    voice.amp_current = temp_osc_gain_multiplier;

                    let nyquist = self.sample_rate / 2.0;
                    if voice.vel_mod_amount == 0.0 {
                        let base_note = voice.note as f32
                            + voice._detune
                            + detune_mod
                            + voice.pitch_current
                            + voice.pitch_current_2;
                        voice.phase_delta =
                            util::f32_midi_note_to_freq(base_note).min(nyquist) / self.sample_rate;
                    } else {
                        let base_note = voice.note as f32
                            + voice._detune
                            + detune_mod
                            + (voice.vel_mod_amount * voice._velocity)
                            + voice.pitch_current
                            + voice.pitch_current_2;
                        voice.phase_delta =
                            util::f32_midi_note_to_freq(base_note).min(nyquist) / self.sample_rate;
                    }

                    center_voices += match self.audio_module_type {
                        AudioModuleType::Sine => {
                            Oscillator::get_sine(voice.phase) * temp_osc_gain_multiplier
                        },
                        AudioModuleType::Tri => {
                            Oscillator::get_tri(voice.phase) * temp_osc_gain_multiplier
                        },
                        AudioModuleType::Saw => {
                            Oscillator::get_saw(voice.phase) * temp_osc_gain_multiplier
                        },
                        AudioModuleType::RSaw => {
                            Oscillator::get_rsaw(voice.phase) * temp_osc_gain_multiplier
                        },
                        AudioModuleType::WSaw => {
                            Oscillator::get_wsaw(voice.phase) * temp_osc_gain_multiplier
                        },
                        AudioModuleType::RASaw => {
                            Oscillator::get_rasaw(voice.phase) * temp_osc_gain_multiplier
                        },
                        AudioModuleType::SSaw => {
                            Oscillator::get_ssaw(voice.phase) * temp_osc_gain_multiplier
                        },
                        AudioModuleType::Ramp => {
                            Oscillator::get_ramp(voice.phase) * temp_osc_gain_multiplier
                        },
                        AudioModuleType::Square => {
                            Oscillator::get_square(voice.phase) * temp_osc_gain_multiplier
                        },
                        AudioModuleType::RSquare => {
                            Oscillator::get_rsquare(voice.phase) * temp_osc_gain_multiplier
                        },
                        AudioModuleType::Pulse => {
                            Oscillator::get_pulse(voice.phase) * temp_osc_gain_multiplier
                        },
                        AudioModuleType::Noise => {
                            self.noise_obj.generate_sample() * temp_osc_gain_multiplier
                        },
                        AudioModuleType::Additive | AudioModuleType::Granulizer | AudioModuleType::Off | AudioModuleType::UnsetAm | AudioModuleType::Sampler => 0.0,
                    };
                }
                // Stereo applies to unison voices
                for unison_voice in self.unison_voices.voices.iter_mut() {
                    // Move the pitch envelope stuff independently of the MIDI info
                    if unison_voice.pitch_enabled {
                        unison_voice.pitch_current = 
                        match unison_voice.pitch_state {
                            OscState::Attacking => unison_voice.pitch_attack.next(),
                            OscState::Decaying => unison_voice.pitch_decay.next(),
                            OscState::Sustaining => self.pitch_env_sustain / 999.9,
                            OscState::Releasing => unison_voice.pitch_release.next(),
                            OscState::Off => 0.0,
                        }
                    }
                    if unison_voice.pitch_enabled_2 {
                        unison_voice.pitch_current_2 = match unison_voice.pitch_state_2 {
                            OscState::Attacking => unison_voice.pitch_attack_2.next(),
                            OscState::Decaying => unison_voice.pitch_decay_2.next(),
                            OscState::Sustaining => self.pitch_env_sustain_2 / 999.9,
                            OscState::Releasing => unison_voice.pitch_release_2.next(),
                            OscState::Off => 0.0,
                        }
                    }

                    let temp_osc_gain_multiplier: f32;
                    // Get our current gain amount for use in match below
                    // Include gain scaling if mod is there
                    if vel_gain_mod != -2.0 {
                        temp_osc_gain_multiplier = match unison_voice.state {
                            OscState::Attacking => {
                                unison_voice.osc_attack.next() * vel_gain_mod * vel_lfo_gain_mod
                            }
                            OscState::Decaying => {
                                unison_voice.osc_decay.next() * vel_gain_mod * vel_lfo_gain_mod
                            }
                            OscState::Sustaining => {
                                (self.osc_sustain / 999.9) * vel_gain_mod * vel_lfo_gain_mod
                            }
                            OscState::Releasing => {
                                unison_voice.osc_release.next() * vel_gain_mod * vel_lfo_gain_mod
                            }
                            OscState::Off => 0.0,
                        };
                    } else {
                        temp_osc_gain_multiplier = match unison_voice.state {
                            OscState::Attacking => {
                                unison_voice.osc_attack.next() * vel_lfo_gain_mod
                            }
                            OscState::Decaying => unison_voice.osc_decay.next() * vel_lfo_gain_mod,
                            OscState::Sustaining => (self.osc_sustain / 999.9) * vel_lfo_gain_mod,
                            OscState::Releasing => {
                                unison_voice.osc_release.next() * vel_lfo_gain_mod
                            }
                            OscState::Off => 0.0,
                        };
                    }

                    unison_voice.amp_current = temp_osc_gain_multiplier;

                    if unison_voice.vel_mod_amount == 0.0 {
                        let base_note = unison_voice.note as f32
                            + unison_voice._unison_detune_value
                            + uni_detune_mod
                            + unison_voice.pitch_current
                            + unison_voice.pitch_current_2;
                        unison_voice.phase_delta =
                            util::f32_midi_note_to_freq(base_note) / self.sample_rate;
                    } else {
                        let base_note = unison_voice.note as f32
                            + unison_voice._unison_detune_value
                            + uni_detune_mod
                            + (unison_voice.vel_mod_amount * unison_voice._velocity)
                            + unison_voice.pitch_current
                            + unison_voice.pitch_current_2;
                        unison_voice.phase_delta =
                            util::f32_midi_note_to_freq(base_note) / self.sample_rate;
                    }

                    if self.osc_unison > 1 {
                        let temp_unison_voice: f32;
                        temp_unison_voice = match self.audio_module_type {
                            AudioModuleType::Sine => {
                                Oscillator::get_sine(unison_voice.phase)
                                    * temp_osc_gain_multiplier
                            },
                            AudioModuleType::Tri => {
                                Oscillator::get_tri(unison_voice.phase)
                                    * temp_osc_gain_multiplier
                            },
                            AudioModuleType::Saw => {
                                Oscillator::get_saw(unison_voice.phase)
                                    * temp_osc_gain_multiplier
                            },
                            AudioModuleType::RSaw => {
                                Oscillator::get_rsaw(unison_voice.phase)
                                    * temp_osc_gain_multiplier
                            },
                            AudioModuleType::WSaw => {
                                Oscillator::get_wsaw(unison_voice.phase)
                                    * temp_osc_gain_multiplier
                            },
                            AudioModuleType::SSaw => {
                                Oscillator::get_ssaw(unison_voice.phase)
                                    * temp_osc_gain_multiplier
                            },
                            AudioModuleType::RASaw => {
                                Oscillator::get_rasaw(unison_voice.phase)
                                    * temp_osc_gain_multiplier
                            },
                            AudioModuleType::Ramp => {
                                Oscillator::get_ramp(unison_voice.phase)
                                    * temp_osc_gain_multiplier
                            },
                            AudioModuleType::Square => {
                                Oscillator::get_square(unison_voice.phase)
                                    * temp_osc_gain_multiplier
                            },
                            AudioModuleType::RSquare => {
                                Oscillator::get_rsquare(unison_voice.phase)
                                    * temp_osc_gain_multiplier
                            },
                            AudioModuleType::Pulse => {
                                Oscillator::get_pulse(unison_voice.phase)
                                    * temp_osc_gain_multiplier
                            },
                            AudioModuleType::Noise => {
                                self.noise_obj.generate_sample() * temp_osc_gain_multiplier
                            },
                            _ => 0.0,
                        };

                        // Create our stereo pan for unison

                        // Our angle comes back as radians
                        let pan = unison_voice._angle;
                                            
                        // Precompute sine and cosine of the angle
                        let cos_pan = pan.cos();
                        let sin_pan = pan.sin();
                                            
                        // Calculate the amplitudes for the panned voice using vector operations
                        let scale = SQRT_2 / 2.0;
                        let temp_unison_voice_scaled = scale * temp_unison_voice;
                                            
                        let left_amp = temp_unison_voice_scaled * (cos_pan + sin_pan);
                        let right_amp = temp_unison_voice_scaled * (cos_pan - sin_pan);

                        // Add the voice to the sum of stereo voices
                        stereo_voices_l += left_amp;
                        stereo_voices_r += right_amp;
                    }
                }
                // Sum our voices for output
                summed_voices_l += center_voices;
                summed_voices_r += center_voices;
                // Scaling of output based on stereo voices and unison
                summed_voices_l += stereo_voices_l / (self.osc_unison - 1).clamp(1, 9) as f32;
                summed_voices_r += stereo_voices_r / (self.osc_unison - 1).clamp(1, 9) as f32;

                // Blending when multi-voiced
                if self.osc_unison > 1 {
                    summed_voices_l = (summed_voices_l + summed_voices_r * 0.8)/2.0;
                    summed_voices_r = (summed_voices_r + summed_voices_l * 0.8)/2.0;
                }

                // Stereo Spreading code
                let width_coeff = match stereo_algorithm {
                    StereoAlgorithm::Original => {
                        self.osc_stereo * 0.5
                    }
                    StereoAlgorithm::CubeSpread => {
                        self.osc_stereo
                    },
                    StereoAlgorithm::ExpSpread => {
                        self.osc_stereo * 1.8
                    },
                };
                let mid = (summed_voices_l + summed_voices_r) * 0.5;
                let stereo = (summed_voices_r - summed_voices_l) * width_coeff;
                summed_voices_l = mid - stereo;
                summed_voices_r = mid + stereo;

                // Return output
                (summed_voices_l, summed_voices_r)
            }
            AudioModuleType::Additive => {
                let mut summed_voices_l: f32 = 0.0;
                let mut summed_voices_r: f32 = 0.0;
                let mut stereo_voices_l: f32 = 0.0;
                let mut stereo_voices_r: f32 = 0.0;
                let mut center_voices: f32 = 0.0;
                for voice in self.playing_voices.voices.iter_mut() {
                    // Move the pitch envelope stuff independently of the MIDI info
                    if voice.pitch_enabled {
                        voice.pitch_current = 
                        match voice.pitch_state {
                            OscState::Attacking => voice.pitch_attack.next(),
                            OscState::Decaying => voice.pitch_decay.next(),
                            OscState::Sustaining => self.pitch_env_sustain / 999.9,
                            OscState::Releasing => voice.pitch_release.next(),
                            OscState::Off => 0.0,
                        }
                    }
                    if voice.pitch_enabled_2 {
                        voice.pitch_current_2 = match voice.pitch_state_2 {
                            OscState::Attacking => voice.pitch_attack_2.next(),
                            OscState::Decaying => voice.pitch_decay_2.next(),
                            OscState::Sustaining => self.pitch_env_sustain_2 / 999.9,
                            OscState::Releasing => voice.pitch_release_2.next(),
                            OscState::Off => 0.0,
                        }
                    }

                    let temp_osc_gain_multiplier: f32;
                    // Get our current gain amount for use in match below
                    // Include gain scaling if mod is there
                    if vel_gain_mod != -2.0 {
                        temp_osc_gain_multiplier = match voice.state {
                            OscState::Attacking => {
                                voice.osc_attack.next() * vel_gain_mod * vel_lfo_gain_mod
                            }
                            OscState::Decaying => {
                                voice.osc_decay.next() * vel_gain_mod * vel_lfo_gain_mod
                            }
                            OscState::Sustaining => {
                                (self.osc_sustain / 999.9) * vel_gain_mod * vel_lfo_gain_mod
                            }
                            OscState::Releasing => {
                                voice.osc_release.next() * vel_gain_mod * vel_lfo_gain_mod
                            }
                            OscState::Off => 0.0,
                        };
                    } else {
                        temp_osc_gain_multiplier = match voice.state {
                            OscState::Attacking => voice.osc_attack.next() * vel_lfo_gain_mod,
                            OscState::Decaying => voice.osc_decay.next() * vel_lfo_gain_mod,
                            OscState::Sustaining => (self.osc_sustain / 999.9) * vel_lfo_gain_mod,
                            OscState::Releasing => voice.osc_release.next() * vel_lfo_gain_mod,
                            OscState::Off => 0.0,
                        };
                    }

                    voice.amp_current = temp_osc_gain_multiplier;

                    let nyquist = self.sample_rate / 2.0;
                    if voice.vel_mod_amount == 0.0 {
                        let base_note = voice.note as f32
                            + voice._detune
                            + detune_mod
                            + voice.pitch_current
                            + voice.pitch_current_2;
                        voice.phase_delta =
                            util::f32_midi_note_to_freq(base_note).min(nyquist) / self.sample_rate;
                    } else {
                        let base_note = voice.note as f32
                            + voice._detune
                            + detune_mod
                            + (voice.vel_mod_amount * voice._velocity)
                            + voice.pitch_current
                            + voice.pitch_current_2;
                        voice.phase_delta =
                            util::f32_midi_note_to_freq(base_note).min(nyquist) / self.sample_rate;
                    }

                    // TODO make this use the additive engine
                    center_voices += self.additive_module.next_sample(voice, self.sample_rate, detune_mod, false) * voice.amp_current;
                }
                // Stereo applies to unison voices
                for unison_voice in self.unison_voices.voices.iter_mut() {
                    // Move the pitch envelope stuff independently of the MIDI info
                    if unison_voice.pitch_enabled {
                        unison_voice.pitch_current = 
                        match unison_voice.pitch_state {
                            OscState::Attacking => unison_voice.pitch_attack.next(),
                            OscState::Decaying => unison_voice.pitch_decay.next(),
                            OscState::Sustaining => self.pitch_env_sustain / 999.9,
                            OscState::Releasing => unison_voice.pitch_release.next(),
                            OscState::Off => 0.0,
                        }
                    }
                    if unison_voice.pitch_enabled_2 {
                        unison_voice.pitch_current_2 = match unison_voice.pitch_state_2 {
                            OscState::Attacking => unison_voice.pitch_attack_2.next(),
                            OscState::Decaying => unison_voice.pitch_decay_2.next(),
                            OscState::Sustaining => self.pitch_env_sustain_2 / 999.9,
                            OscState::Releasing => unison_voice.pitch_release_2.next(),
                            OscState::Off => 0.0,
                        }
                    }

                    let temp_osc_gain_multiplier: f32;
                    // Get our current gain amount for use in match below
                    // Include gain scaling if mod is there
                    if vel_gain_mod != -2.0 {
                        temp_osc_gain_multiplier = match unison_voice.state {
                            OscState::Attacking => {
                                unison_voice.osc_attack.next() * vel_gain_mod * vel_lfo_gain_mod
                            }
                            OscState::Decaying => {
                                unison_voice.osc_decay.next() * vel_gain_mod * vel_lfo_gain_mod
                            }
                            OscState::Sustaining => {
                                (self.osc_sustain / 999.9) * vel_gain_mod * vel_lfo_gain_mod
                            }
                            OscState::Releasing => {
                                unison_voice.osc_release.next() * vel_gain_mod * vel_lfo_gain_mod
                            }
                            OscState::Off => 0.0,
                        };
                    } else {
                        temp_osc_gain_multiplier = match unison_voice.state {
                            OscState::Attacking => {
                                unison_voice.osc_attack.next() * vel_lfo_gain_mod
                            }
                            OscState::Decaying => unison_voice.osc_decay.next() * vel_lfo_gain_mod,
                            OscState::Sustaining => (self.osc_sustain / 999.9) * vel_lfo_gain_mod,
                            OscState::Releasing => {
                                unison_voice.osc_release.next() * vel_lfo_gain_mod
                            }
                            OscState::Off => 0.0,
                        };
                    }

                    unison_voice.amp_current = temp_osc_gain_multiplier;

                    if unison_voice.vel_mod_amount == 0.0 {
                        let base_note = unison_voice.note as f32
                            + unison_voice._unison_detune_value
                            + uni_detune_mod
                            + unison_voice.pitch_current
                            + unison_voice.pitch_current_2;
                        unison_voice.phase_delta =
                            util::f32_midi_note_to_freq(base_note) / self.sample_rate;
                    } else {
                        let base_note = unison_voice.note as f32
                            + unison_voice._unison_detune_value
                            + uni_detune_mod
                            + (unison_voice.vel_mod_amount * unison_voice._velocity)
                            + unison_voice.pitch_current
                            + unison_voice.pitch_current_2;
                        unison_voice.phase_delta =
                            util::f32_midi_note_to_freq(base_note) / self.sample_rate;
                    }

                    if self.osc_unison > 1 {
                        let temp_unison_voice: f32 = self.additive_module.next_sample(unison_voice, self.sample_rate, detune_mod, true) * unison_voice.amp_current;
                        
                        // TODO make the unison voice sum
                        //temp_unison_voice = 0.0;

                        // Create our stereo pan for unison

                        // Our angle comes back as radians
                        let pan = unison_voice._angle;
                                            
                        // Precompute sine and cosine of the angle
                        let cos_pan = pan.cos();
                        let sin_pan = pan.sin();
                                            
                        // Calculate the amplitudes for the panned voice using vector operations
                        let scale = SQRT_2 / 2.0;
                        let temp_unison_voice_scaled = scale * temp_unison_voice;
                                            
                        let left_amp = temp_unison_voice_scaled * (cos_pan + sin_pan);
                        let right_amp = temp_unison_voice_scaled * (cos_pan - sin_pan);

                        // Add the voice to the sum of stereo voices
                        stereo_voices_l += left_amp;
                        stereo_voices_r += right_amp;
                    }
                }
                // Sum our voices for output
                summed_voices_l += center_voices;
                summed_voices_r += center_voices;
                // Scaling of output based on stereo voices and unison
                summed_voices_l += stereo_voices_l / (self.osc_unison - 1).clamp(1, 9) as f32;
                summed_voices_r += stereo_voices_r / (self.osc_unison - 1).clamp(1, 9) as f32;

                // Blending
                if self.osc_unison > 1 {
                    summed_voices_l = (summed_voices_l + summed_voices_r * 0.8)/2.0;
                    summed_voices_r = (summed_voices_r + summed_voices_l * 0.8)/2.0;
                }

                // Stereo Spreading code
                let width_coeff = match stereo_algorithm {
                    StereoAlgorithm::Original => {
                        self.osc_stereo * 0.5
                    }
                    StereoAlgorithm::CubeSpread => {
                        self.osc_stereo
                    },
                    StereoAlgorithm::ExpSpread => {
                        self.osc_stereo * 1.8
                    },
                };
                let mid = (summed_voices_l + summed_voices_r) * 0.5;
                let stereo = (summed_voices_r - summed_voices_l) * width_coeff;
                summed_voices_l = mid - stereo;
                summed_voices_r = mid + stereo;

                // Return output
                (summed_voices_l, summed_voices_r)
            }
            AudioModuleType::Sampler => {
                let mut summed_voices_l: f32 = 0.0;
                let mut summed_voices_r: f32 = 0.0;
                let mut center_voices_l: f32 = 0.0;
                let mut center_voices_r: f32 = 0.0;
                let mut stereo_voices_l: f32 = 0.0;
                let mut stereo_voices_r: f32 = 0.0;
                for voice in self.playing_voices.voices.iter_mut() {
                    // Get our current gain amount for use in match below
                    let temp_osc_gain_multiplier: f32 = match voice.state {
                        OscState::Attacking => voice.osc_attack.next(),
                        OscState::Decaying => voice.osc_decay.next(),
                        OscState::Sustaining => self.osc_sustain / 999.9,
                        OscState::Releasing => voice.osc_release.next(),
                        OscState::Off => 0.0,
                    };
                    voice.amp_current = temp_osc_gain_multiplier;

                    let usize_note = voice.note as usize;

                    // If we even have valid samples loaded
                    if self.sample_lib[0][0].len() > 1
                        && self.loaded_sample[0].len() > 1
                        && self.sample_lib.len() > 1
                    {
                        // Use our Vec<midi note value<VectorOfChannels<VectorOfSamples>>>
                        // If our note is valid 0-127
                        if usize_note < self.sample_lib.len() {
                            // If our sample position is valid for our note
                            if voice.sample_pos < self.sample_lib[usize_note][0].len() {
                                // Get our channels of sample vectors
                                let NoteVector = &self.sample_lib[usize_note];
                                // We don't need to worry about mono/stereo here because it's been setup in load_new_sample()
                                center_voices_l +=
                                    NoteVector[0][voice.sample_pos] * temp_osc_gain_multiplier;
                                center_voices_r +=
                                    NoteVector[1][voice.sample_pos] * temp_osc_gain_multiplier;
                            }
                        }

                        let scaled_start_position = (self.sample_lib[usize_note][0].len() as f32
                            * self.start_position)
                            .floor() as usize;
                        let scaled_end_position = (self.sample_lib[usize_note][0].len() as f32
                            * self._end_position)
                            .floor() as usize;
                        // Sampler moves position
                        voice.sample_pos += 1;
                        if voice.loop_it
                            && (voice.sample_pos > self.sample_lib[usize_note][0].len()
                                || voice.sample_pos > scaled_end_position)
                        {
                            voice.sample_pos = scaled_start_position;
                        } else if voice.sample_pos > scaled_end_position {
                            voice.sample_pos = self.sample_lib[usize_note][0].len();
                            voice.state = OscState::Off;
                        }
                    }
                }

                let mut temp_unison_voice_l = 0.0;
                let mut temp_unison_voice_r = 0.0;
                // Stereo applies to unison voices
                for unison_voice in self.unison_voices.voices.iter_mut() {
                    // Get our current gain amount for use in match below
                    let temp_osc_gain_multiplier: f32 = match unison_voice.state {
                        OscState::Attacking => unison_voice.osc_attack.next(),
                        OscState::Decaying => unison_voice.osc_decay.next(),
                        OscState::Sustaining => self.osc_sustain / 999.9,
                        OscState::Releasing => unison_voice.osc_release.next(),
                        OscState::Off => 0.0,
                    };
                    unison_voice.amp_current = temp_osc_gain_multiplier;

                    let usize_note = unison_voice.note as usize;

                    // If we even have valid samples loaded
                    if self.sample_lib[0][0].len() > 1
                        && self.loaded_sample[0].len() > 1
                        && self.sample_lib.len() > 1
                    {
                        // Use our Vec<midi note value<VectorOfChannels<VectorOfSamples>>>
                        // If our note is valid 0-127
                        if usize_note < self.sample_lib.len() {
                            // If our sample position is valid for our note
                            if unison_voice.sample_pos < self.sample_lib[usize_note][0].len() {
                                // Get our channels of sample vectors
                                let NoteVector = &self.sample_lib[usize_note];
                                // We don't need to worry about mono/stereo here because it's been setup in load_new_sample()
                                temp_unison_voice_l +=
                                    NoteVector[0][unison_voice.sample_pos] * temp_osc_gain_multiplier;
                                temp_unison_voice_r +=
                                    NoteVector[1][unison_voice.sample_pos] * temp_osc_gain_multiplier;
                            }
                        }

                        let scaled_start_position = (self.sample_lib[usize_note][0].len() as f32
                            * self.start_position)
                            .floor() as usize;
                        let scaled_end_position = (self.sample_lib[usize_note][0].len() as f32
                            * self._end_position)
                            .floor() as usize;
                        // Sampler moves position
                        unison_voice.sample_pos += 1;
                        if unison_voice.loop_it
                            && (unison_voice.sample_pos > self.sample_lib[usize_note][0].len()
                                || unison_voice.sample_pos > scaled_end_position)
                        {
                            unison_voice.sample_pos = scaled_start_position;
                        } else if unison_voice.sample_pos > scaled_end_position {
                            unison_voice.sample_pos = self.sample_lib[usize_note][0].len();
                            unison_voice.state = OscState::Off;
                        }
                    }
                        // Create our stereo pan for unison

                        // Our angle comes back as radians
                        let pan = unison_voice._angle;
                                            
                        // Precompute sine and cosine of the angle
                        let cos_pan = pan.cos();
                        let sin_pan = pan.sin();
                                            
                        // Calculate the amplitudes for the panned voice using vector operations
                        let scale = SQRT_2 / 2.0;
                        let temp_unison_voice_scaled_l = scale * temp_unison_voice_l;
                        let temp_unison_voice_scaled_r = scale * temp_unison_voice_r;
                                            
                        let left_amp = temp_unison_voice_scaled_l * (cos_pan + sin_pan);
                        let right_amp = temp_unison_voice_scaled_r * (cos_pan - sin_pan);

                        // Add the voice to the sum of stereo voices
                        stereo_voices_l += left_amp;
                        stereo_voices_r += right_amp;
                }

                // Sum our voices for output
                summed_voices_l += center_voices_l;
                summed_voices_r += center_voices_r;
                // Scaling of output based on stereo voices and unison
                summed_voices_l += stereo_voices_l / (self.osc_unison - 1).clamp(1, 9) as f32;
                summed_voices_r += stereo_voices_r / (self.osc_unison - 1).clamp(1, 9) as f32;

                // Stereo Spreading code
                let width_coeff = self.osc_stereo * 0.5;
                let mid = (summed_voices_l + summed_voices_r) * 0.5;
                let stereo = (summed_voices_r - summed_voices_l) * width_coeff;
                summed_voices_l = mid - stereo;
                summed_voices_r = mid + stereo;

                (summed_voices_l, summed_voices_r)
            }
            AudioModuleType::Off | AudioModuleType::UnsetAm => {
                // Do nothing, return 0.0
                (0.0, 0.0)
            }
            AudioModuleType::Granulizer => {
                let mut summed_voices_l: f32 = 0.0;
                let mut summed_voices_r: f32 = 0.0;
                for voice in self.playing_voices.voices.iter_mut() {
                    // Get our current gain amount for use in match below
                    let temp_osc_gain_multiplier: f32 = match voice.state {
                        OscState::Attacking => voice.osc_attack.next(),
                        OscState::Decaying => voice.osc_decay.next(),
                        OscState::Sustaining => self.osc_sustain / 999.9,
                        OscState::Releasing => voice.osc_release.next(),
                        OscState::Off => 0.0,
                    };
                    voice.amp_current = temp_osc_gain_multiplier;

                    let usize_note = voice.note as usize;

                    // If we even have valid samples loaded
                    if self.sample_lib[0][0].len() > 1
                        && self.loaded_sample[0].len() > 1
                        && self.sample_lib.len() > 1
                    {
                        // Use our Vec<midi note value<VectorOfChannels<VectorOfSamples>>>
                        // If our note is valid 0-127
                        if usize_note < self.sample_lib.len() {
                            // If our sample position is valid for our note
                            if voice.sample_pos < self.sample_lib[usize_note][0].len() {
                                // Get our channels of sample vectors
                                let NoteVector = &self.sample_lib[usize_note];
                                // If we are in crossfade or in middle of grain after atttack ends
                                if voice.grain_state == GrainState::Attacking {
                                    // Add our current grain
                                    if voice.grain_attack.steps_left() != 0 {
                                        // This format is: Output = CurrentSample * Voice ADSR * GrainRelease
                                        summed_voices_l += NoteVector[0][voice.sample_pos]
                                            * temp_osc_gain_multiplier
                                            * voice.grain_attack.next();
                                        summed_voices_r += NoteVector[1][voice.sample_pos]
                                            * temp_osc_gain_multiplier
                                            * voice.grain_attack.next();
                                    } else {
                                        // This format is: Output = CurrentSample * Voice ADSR * GrainRelease
                                        summed_voices_l += NoteVector[0][voice.sample_pos]
                                            * temp_osc_gain_multiplier;
                                        summed_voices_r += NoteVector[1][voice.sample_pos]
                                            * temp_osc_gain_multiplier;
                                    }
                                }
                                // If we are in crossfade
                                else if voice.grain_state == GrainState::Releasing {
                                    summed_voices_l += NoteVector[0][voice.sample_pos]
                                        * temp_osc_gain_multiplier
                                        * voice.grain_release.next();
                                    summed_voices_r += NoteVector[1][voice.sample_pos]
                                        * temp_osc_gain_multiplier
                                        * voice.grain_release.next();
                                }
                            }
                        }
                        let scaled_start_position = (self.loaded_sample[0].len() as f32
                            * self.start_position)
                            .floor() as usize;
                        let scaled_end_position = (self.loaded_sample[0].len() as f32
                            * self._end_position)
                            .floor() as usize;
                        // Granulizer moves position
                        voice.sample_pos += 1;
                        if voice.loop_it
                            && (voice.sample_pos > self.loaded_sample[0].len()
                                || voice.sample_pos > scaled_end_position)
                        {
                            voice.sample_pos = scaled_start_position;
                        } else if voice.sample_pos > scaled_end_position {
                            voice.sample_pos = self.sample_lib[usize_note][0].len();
                            voice.state = OscState::Off;
                        }
                    }
                }
                (summed_voices_l, summed_voices_r)
            }
        };

        // Send it back
        (output_signal_l, output_signal_r, note_on, note_off)
    }

    pub fn set_playing(&mut self, new_bool: bool) {
        self.is_playing = new_bool;
    }

    pub fn get_playing(&mut self) -> bool {
        self.is_playing
    }

    pub fn clear_voices(&mut self) {
        self.playing_voices.voices.clear();
        self.unison_voices.voices.clear();
    }

    pub fn load_new_sample(&mut self, path: PathBuf) {
        let reader = hound::WavReader::open(&path);
        if let Ok(mut reader) = reader {
            let spec = reader.spec();
            //let inner_sample_rate = spec.sample_rate as f32;
            let channels = spec.channels as usize;
            let samples;

            if spec.bits_per_sample == 8 {
                // Since 16 bit is loud I'm scaling this one too for safety
                samples = match spec.sample_format {
                    hound::SampleFormat::Int => reader
                        .samples::<i8>()
                        .map(|s| {
                            util::db_to_gain(-36.0)
                                * ((s.unwrap_or_default() as f32 * 256.0) / i8::MAX as f32)
                        })
                        .collect::<Vec<f32>>(),
                    hound::SampleFormat::Float => reader
                        .samples::<f32>()
                        .map(|s| s.unwrap_or_default())
                        .collect::<Vec<f32>>(),
                };
            } else if spec.bits_per_sample == 16 {
                // I noticed 16 bit can be LOUD so I tried to scale it
                samples = match spec.sample_format {
                    hound::SampleFormat::Int => reader
                        .samples::<i16>()
                        .map(|s| {
                            util::db_to_gain(-36.0)
                                * ((s.unwrap_or_default() as f32 * 256.0) / i16::MAX as f32)
                        })
                        .collect::<Vec<f32>>(),
                    hound::SampleFormat::Float => reader
                        .samples::<f32>()
                        .map(|s| s.unwrap_or_default())
                        .collect::<Vec<f32>>(),
                };
            } else {
                // Attempt 32 bit cast/decode if 8/16 are invalid - no scaling
                samples = match spec.sample_format {
                    hound::SampleFormat::Int => reader
                        .samples::<i32>()
                        .map(|s| (s.unwrap_or_default() as f32 * 256.0) / i32::MAX as f32)
                        .collect::<Vec<f32>>(),
                    hound::SampleFormat::Float => reader
                        .samples::<f32>()
                        .map(|s| s.unwrap_or_default())
                        .collect::<Vec<f32>>(),
                };
            }

            // Uninterleave sample format to chunks for resampling
            let mut new_samples = vec![Vec::with_capacity(samples.len() / channels); channels];

            for sample_chunk in samples.chunks(channels) {
                // sample_chunk is a chunk like [a, b]
                for (i, sample) in sample_chunk.into_iter().enumerate() {
                    new_samples[i].push(*sample);
                }
            }

            self.loaded_sample = new_samples;

            // Based off restretch vs non stretch use different algorithms
            // To generate a sample library
            self.regenerate_samples();
        };
    }

    // This method performs the sample recalculations when restretch is toggled
    pub fn regenerate_samples(&mut self) {
        if !self.sample_lib.is_empty() {
            if self.audio_module_type == AudioModuleType::Sampler {
                // Compare our restretch change
                if self.restretch != self.prev_restretch {
                    self.prev_restretch = self.restretch;
                }
            } else if self.audio_module_type == AudioModuleType::Granulizer {
                self.restretch = true;
                self.prev_restretch = false;
            } else {
                return;
            }

            self.sample_lib.clear();
        }

        if self.restretch {
            let middle_c: f32 = 256.0;
            // Generate our sample library from our sample
            for i in 0..127 {
                let target_pitch_factor = util::f32_midi_note_to_freq(i as f32) / middle_c;

                // Calculate the number of samples in the shifted frame
                let shifted_num_samples =
                    (self.loaded_sample[0].len() as f32 / target_pitch_factor).round() as usize;

                // Apply pitch shifting by interpolating between the original samples
                let mut shifted_samples_l = Vec::with_capacity(shifted_num_samples);
                let mut shifted_samples_r = Vec::with_capacity(shifted_num_samples);

                for j in 0..shifted_num_samples {
                    let original_index: usize;
                    let fractional_part: f32;

                    original_index = (j as f32 * target_pitch_factor).floor() as usize;
                    fractional_part = j as f32 * target_pitch_factor - original_index as f32;

                    if original_index < self.loaded_sample[0].len() - 1 {
                        // Linear interpolation between adjacent samples
                        let interpolated_sample_r;
                        let interpolated_sample_l = (1.0 - fractional_part)
                            * self.loaded_sample[0][original_index]
                            + fractional_part * self.loaded_sample[0][original_index + 1];
                        if self.loaded_sample.len() > 1 {
                            interpolated_sample_r = (1.0 - fractional_part)
                                * self.loaded_sample[1][original_index]
                                + fractional_part * self.loaded_sample[1][original_index + 1];
                        } else {
                            interpolated_sample_r = interpolated_sample_l;
                        }

                        shifted_samples_l.push(interpolated_sample_l);
                        shifted_samples_r.push(interpolated_sample_r);
                    } else {
                        // If somehow through buffer shenanigans we are past our length we shouldn't do anything here
                        if original_index < self.loaded_sample[0].len() {
                            shifted_samples_l.push(self.loaded_sample[0][original_index]);
                            if self.loaded_sample.len() > 1 {
                                shifted_samples_r.push(self.loaded_sample[1][original_index]);
                            } else {
                                shifted_samples_r.push(self.loaded_sample[0][original_index]);
                            }
                        }
                    }
                }

                let mut NoteVector = Vec::with_capacity(2);
                NoteVector.insert(0, shifted_samples_l);
                NoteVector.insert(1, shifted_samples_r);
                self.sample_lib.insert(i, NoteVector);
            }
        }
        // If we are just pitch shifting instead of restretching
        else {
            let mut shifter = PitchShifter::new(50, self.sample_rate as usize);
            for i in 0..127 {
                let translated_i = (i as i32 - 60_i32) as f32;
                let mut out_buffer_left = vec![0.0; self.loaded_sample[0].len()];
                let mut out_buffer_right = vec![0.0; self.loaded_sample[0].len()];

                let loaded_left = self.loaded_sample[0].as_slice();
                let loaded_right;
                if self.loaded_sample.len() > 1 {
                    loaded_right = self.loaded_sample[1].as_slice();
                } else {
                    loaded_right = self.loaded_sample[0].as_slice();
                }

                shifter.shift_pitch(3, translated_i, loaded_left, &mut out_buffer_left);
                shifter.shift_pitch(3, translated_i, loaded_right, &mut out_buffer_right);

                let mut NoteVector = Vec::with_capacity(2);
                NoteVector.insert(0, out_buffer_left);
                NoteVector.insert(1, out_buffer_right);
                self.sample_lib.insert(i, NoteVector);
            }
        }
    }

    fn calculate_panning(&mut self, voice_index: i32, num_voices: i32, stereo_algorithm: StereoAlgorithm) -> f32 {
        // Ensure the voice index is within bounds.
        let voice_index = voice_index.min(num_voices - 1);

        let sign = if self.two_voice_stereo_flipper {
            1.0
        } else {
            -1.0
        };

        // Handle the special case for 2 voices.
        if num_voices == 2 {
            // multiplied by sign of stereo flipper to avoid pan
            return if voice_index == 0 {
                self.two_voice_stereo_flipper = !self.two_voice_stereo_flipper;
                -0.25 * std::f32::consts::PI * sign // First voice panned left
            } else {
                0.25 * std::f32::consts::PI * sign // Second voice panned right
            };
        }

        // Handle the special case for 3 voices.
        if num_voices == 3 {
            return match voice_index {
                0 => {
                    self.two_voice_stereo_flipper = !self.two_voice_stereo_flipper;
                    -0.25 * std::f32::consts::PI * sign
                } // First voice panned left
                1 => 0.0,                                // Second voice panned center
                2 => 0.25 * std::f32::consts::PI * sign, // Third voice panned right
                _ => 0.0,                                // Handle other cases gracefully
            };
        }

        // Calculate the pan angle for voices with index 0 and 1.
        let base_angle = ((voice_index / 2) as f32) / ((num_voices / 2) as f32 - 1.0) - 0.5;

        let angle: f32;
        match stereo_algorithm {
            StereoAlgorithm::CubeSpread => {
                let poly_base_angle = base_angle.powf(3.0)/0.3;
                // Determine the final angle based on even or odd index.
                angle = if voice_index % 2 == 0 {
                    -poly_base_angle
                } else {
                    poly_base_angle
                };
            },
            StereoAlgorithm::ExpSpread => {
                let exp_base_angle = base_angle.exp() - 1.0;  // Exponential transformation
                let max_exp_angle = (1.0f32).exp() - 1.0;
                let normalized_exp_angle = exp_base_angle / max_exp_angle;

                // Determine the final angle based on even or odd index.
                angle = if voice_index % 2 == 0 {
                    -normalized_exp_angle
                } else {
                    normalized_exp_angle
                };
            },
            StereoAlgorithm::Original => {
                // Determine the final angle based on even or odd index.
                angle = if voice_index % 2 == 0 {
                    -base_angle
                } else {
                    base_angle
                };
            },
        }

        if voice_index == 0 {
            self.two_voice_stereo_flipper = !self.two_voice_stereo_flipper;
        }
        angle * std::f32::consts::PI * sign
    }
}

// This is silly but it works and is somehow fast enough
fn check_inequality(
    a1: f32, b1: f32, a2: f32, b2: f32, a3: f32, b3: f32, 
    a4: f32, b4: f32, a5: f32, b5: f32, a6: f32, b6: f32, 
    a7: f32, b7: f32, a8: f32, b8: f32,
    a9: f32, b9: f32, a10: f32, b10: f32, a11: f32, b11: f32, 
    a12: f32, b12: f32, a13: f32, b13: f32, a14: f32, b14: f32, 
    a15: f32, b15: f32, a16: f32, b16: f32
) -> bool {
    a1 != b1 || a2 != b2 || a3 != b3 || a4 != b4 ||
    a5 != b5 || a6 != b6 || a7 != b7 || a8 != b8 ||
    a9 != b9 || a10 != b10 || a11 != b11 || a12 != b12 ||
    a13 != b13 || a14 != b14 || a15 != b15 || a16 != b16
}