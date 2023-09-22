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

use std::{sync::Arc, collections::VecDeque, path::PathBuf, f32::consts::{SQRT_2}};
use nih_plug::{prelude::{Smoother, SmoothingStyle, ParamSetter, NoteEvent}, util, params::enums::Enum};
use nih_plug_egui::egui::{Ui, RichText};
use rand::Rng;
use rfd::FileDialog;
use pitch_shift::PitchShifter;
use serde::{Deserialize, Serialize};

// Audio module files
pub(crate) mod Oscillator;
use Oscillator::VoiceType;
use crate::{ActuateParams, ui_knob, GUI_VALS, toggle_switch, SMALLER_FONT, StateVariableFilter::ResonanceType};
use self::Oscillator::{RetriggerStyle, OscState, SmoothStyle};

// When you create a new audio module, you should add it here
#[derive(Enum, PartialEq, Clone, Copy, Serialize, Deserialize)]
pub enum AudioModuleType {
    Off,
    Osc,
    Sampler,
    Granulizer,
    Additive,
}

#[derive(Clone)]
struct VoiceVec {
    /// The identifier for this voice
    voices: VecDeque<SingleVoice>
}

// This is the information used to track voices and midi inputs
// I made single voice a struct so that we can add and remove via struct instead of running
// into any threading issues trying to modify different vecs in the same function rather than 1 struct
// Underscores are to get rid of the compiler warning thinking it's not used but it's stored for debugging or passed between structs
// and still functional.
#[derive(Clone)]
struct SingleVoice {
    /// The note's key/note, in `0..128`. Only used for the voice terminated event.
    note: u8,
    /// Velocity of our note
    _velocity: f32,
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
    // Final info for a note to work
    _detune: f32,
    _unison_detune_value: f32,
    frequency: f32,
    _attack_time: f32,
    _decay_time: f32,
    _release_time: f32,
    _retrigger: RetriggerStyle,
    _voice_type: Oscillator::VoiceType,

    // This is only used for unison detunes
    _angle: f32,

    // Sampler/Granulizer Pos
    sample_pos: usize,
    loop_it: bool,
}

#[derive(Clone)]
pub struct AudioModule {
    // Stored sample rate in case the audio module needs it
    sample_rate: f32,
    audio_module_type: AudioModuleType,

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

    ///////////////////////////////////////////////////////////

    // Stored params from main lib here on a per-module basis

    // Osc module knob storage
    pub osc_type: VoiceType,
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
}

// When you create a new audio module you need to add its default creation here as well
impl Default for AudioModule {
    fn default() -> Self {
        Self {
            // Audio modules will use these
            sample_rate: 44100.0,
            audio_module_type: AudioModuleType::Osc,

            // Granulizer/Sampler
            loaded_sample: vec![vec![0.0,0.0]],
            sample_lib: vec![vec![vec![0.0,0.0]]], //Vec<Vec<Vec<f32>>>
            loop_wavetable: false,
            single_cycle: false,
            restretch: true,
            prev_restretch: false,

            // Osc module knob storage
            osc_type: VoiceType::Sine,
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
            playing_voices: VoiceVec { voices: VecDeque::new() },
            unison_voices: VoiceVec { voices: VecDeque::new() },

            // Tracking stopping voices
            is_playing: false,
        }
    }
}

impl AudioModule {
    // Draw functions for each module type. This works at CRATE level on the params to draw all 3!!!
    // Passing the params here is not the nicest thing but we have to move things around to get past the threading stuff + egui's gui separation
    pub fn draw_modules(ui: &mut Ui, params: Arc<ActuateParams>, setter: &ParamSetter<'_>) {
        // Resetting these from the draw thread since setter is valid here - I recognize this is ugly/bad practice
        if params.load_sample_1.value() {
            setter.set_parameter(&params.load_sample_1, false);
        }
        if params.load_sample_2.value() {
            setter.set_parameter(&params.load_sample_2, false);
        }
        if params.load_sample_3.value() {
            setter.set_parameter(&params.load_sample_3, false);
        }

        // Prevent Speaker Destruction since setter is valid here - Resonance spikes and non clipped signal are deadly
        // I recognize this is ugly/bad practice
        match params.filter_res_type.value() {
            ResonanceType::Default => {}, // Do nothing
            _ => { 
                if params.filter_resonance.value() < 0.15 {
                    setter.set_parameter(&params.filter_resonance, 0.15);
                }
            }
        }

        const KNOB_SIZE: f32 = 30.0;
        const TEXT_SIZE: f32 = 12.0;

        // This is kind of ugly but I couldn't figure out a better architechture for egui and separating audio modules
        
        // Spot one
        match params._audio_module_1_type.value() {
            AudioModuleType::Off => {
                // Blank space
                ui.label("Disabled");
            },
            AudioModuleType::Osc => {
                // Oscillator
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        let osc_1_type_knob = ui_knob::ArcKnob::for_param(
                            &params.osc_1_type, 
                            setter, 
                            KNOB_SIZE)
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                            .set_line_color(*GUI_VALS.get("SYNTH_MIDDLE_BLUE").unwrap())
                            .set_text_size(TEXT_SIZE);
                        ui.add(osc_1_type_knob);

                        let osc_1_mod_knob = ui_knob::ArcKnob::for_param(
                            &params.osc_1_mod_amount, 
                            setter, 
                            KNOB_SIZE)
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("A_BACKGROUND_COLOR_TOP").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .use_outline(true)
                            .set_text_size(TEXT_SIZE);
                        ui.add(osc_1_mod_knob);

                        let osc_1_octave_knob = ui_knob::ArcKnob::for_param(
                            &params.osc_1_octave, 
                            setter, 
                            KNOB_SIZE)
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("A_BACKGROUND_COLOR_TOP").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .use_outline(true)
                            .set_text_size(TEXT_SIZE);
                        ui.add(osc_1_octave_knob);

                        let osc_1_unison_knob = ui_knob::ArcKnob::for_param(
                            &params.osc_1_unison, 
                            setter, 
                            KNOB_SIZE)
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("A_BACKGROUND_COLOR_TOP").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .use_outline(true)
                            .set_text_size(TEXT_SIZE);
                        ui.add(osc_1_unison_knob);

                        let osc_1_unison_detune_knob = ui_knob::ArcKnob::for_param(
                            &params.osc_1_unison_detune, 
                            setter, 
                            KNOB_SIZE)
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("A_BACKGROUND_COLOR_TOP").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .use_outline(true)
                            .set_text_size(TEXT_SIZE);
                        ui.add(osc_1_unison_detune_knob);

                        let osc_1_attack_knob = ui_knob::ArcKnob::for_param(
                            &params.osc_1_attack, 
                            setter, 
                            KNOB_SIZE)
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .set_text_size(TEXT_SIZE);
                        ui.add(osc_1_attack_knob);

                        let osc_1_decay_knob = ui_knob::ArcKnob::for_param(
                            &params.osc_1_decay, 
                            setter, 
                            KNOB_SIZE)
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .set_text_size(TEXT_SIZE);
                        ui.add(osc_1_decay_knob);

                        let osc_1_sustain_knob = ui_knob::ArcKnob::for_param(
                            &params.osc_1_sustain, 
                            setter, 
                            KNOB_SIZE)
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .set_text_size(TEXT_SIZE);
                        ui.add(osc_1_sustain_knob);

                        let osc_1_release_knob = ui_knob::ArcKnob::for_param(
                            &params.osc_1_release, 
                            setter, 
                            KNOB_SIZE)
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .set_text_size(TEXT_SIZE);
                        ui.add(osc_1_release_knob);
                    });
                    ui.horizontal(|ui| {
                        let osc_1_retrigger_knob = ui_knob::ArcKnob::for_param(
                            &params.osc_1_retrigger, 
                            setter, 
                            KNOB_SIZE)
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("A_BACKGROUND_COLOR_TOP").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .use_outline(true)
                            .set_text_size(TEXT_SIZE);
                        ui.add(osc_1_retrigger_knob);

                        let osc_1_semitones_knob = ui_knob::ArcKnob::for_param(
                            &params.osc_1_semitones, 
                            setter, 
                            KNOB_SIZE)
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("A_BACKGROUND_COLOR_TOP").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .use_outline(true)
                            .set_text_size(TEXT_SIZE);
                        ui.add(osc_1_semitones_knob);

                        let osc_1_detune_knob = ui_knob::ArcKnob::for_param(
                            &params.osc_1_detune, 
                            setter, 
                            KNOB_SIZE)
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("A_BACKGROUND_COLOR_TOP").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .use_outline(true)
                            .set_text_size(TEXT_SIZE);
                        ui.add(osc_1_detune_knob);

                        // Space
                        ui.add_space(KNOB_SIZE*2.0 + 16.0);

                        let osc_1_stereo_knob = ui_knob::ArcKnob::for_param(
                            &params.osc_1_stereo, 
                            setter, 
                            KNOB_SIZE)
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("A_BACKGROUND_COLOR_TOP").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .use_outline(true)
                            .set_text_size(TEXT_SIZE);
                        ui.add(osc_1_stereo_knob);
                       
                        let osc_1_atk_curve_knob = ui_knob::ArcKnob::for_param(
                            &params.osc_1_atk_curve, 
                            setter, 
                            KNOB_SIZE)
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .set_text_size(TEXT_SIZE);
                        ui.add(osc_1_atk_curve_knob);
                        
                        // Attack knob space
                        //ui.add_space(KNOB_SIZE*2.0 + 16.0);

                        let osc_1_dec_curve_knob = ui_knob::ArcKnob::for_param(
                            &params.osc_1_dec_curve, 
                            setter, 
                            KNOB_SIZE)
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .set_text_size(TEXT_SIZE);
                        ui.add(osc_1_dec_curve_knob);

                        // Sustain knob space
                        ui.add_space(KNOB_SIZE*2.0 + 16.0);
                        
                        let osc_1_rel_curve_knob = ui_knob::ArcKnob::for_param(
                            &params.osc_1_rel_curve, 
                            setter, 
                            KNOB_SIZE)
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .set_text_size(TEXT_SIZE);
                        ui.add(osc_1_rel_curve_knob);
                    });
                });
            },
            AudioModuleType::Sampler => {
                const KNOB_SIZE: f32 = 25.0;
                const TEXT_SIZE: f32 = 11.0;
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new("Load Sample")
                                    .font(SMALLER_FONT)
                                    .color(*GUI_VALS.get("FONT_COLOR").unwrap()))
                                    .on_hover_text("The longer the sample the longer the process!");
                                    let switch_toggle = toggle_switch::ToggleSwitch::for_param(&params.load_sample_1, setter);
                                    ui.add(switch_toggle);
                                });
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new("Resample")
                                    .font(SMALLER_FONT)
                                    .color(*GUI_VALS.get("FONT_COLOR").unwrap()))
                                    .on_hover_text("Reload your sample after changing this! On: Resample, Off: Repitch");
                                    let stretch_toggle = toggle_switch::ToggleSwitch::for_param(&params.restretch_1, setter);
                                    ui.add(stretch_toggle);
                                });
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new("Looping")
                                    .font(SMALLER_FONT)
                                    .color(*GUI_VALS.get("FONT_COLOR").unwrap()))
                                    .on_hover_text("To repeat your sample if MIDI key is held");
                                    let loop_toggle = toggle_switch::ToggleSwitch::for_param(&params.loop_sample_1, setter);
                                    ui.add(loop_toggle);
                                });
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new("Single Cycle")
                                    .font(SMALLER_FONT)
                                    .color(*GUI_VALS.get("FONT_COLOR").unwrap()))
                                    .on_hover_text("Use this with Looping ON and Resample ON if you loaded a single cycle waveform");
                                    let sc_toggle = toggle_switch::ToggleSwitch::for_param(&params.single_cycle_1, setter);
                                    ui.add(sc_toggle);
                                });
                            });

                            ui.horizontal(|ui| {
                                let osc_1_octave_knob = ui_knob::ArcKnob::for_param(
                                    &params.osc_1_octave, 
                                    setter, 
                                    KNOB_SIZE)
                                    .preset_style(ui_knob::KnobStyle::NewPresets1)
                                    .set_fill_color(*GUI_VALS.get("A_BACKGROUND_COLOR_TOP").unwrap())
                                    .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                                    .use_outline(true)
                                    .set_text_size(TEXT_SIZE);
                                ui.add(osc_1_octave_knob);

                                let osc_1_attack_knob = ui_knob::ArcKnob::for_param(
                                    &params.osc_1_attack, 
                                    setter, 
                                    KNOB_SIZE)
                                    .preset_style(ui_knob::KnobStyle::NewPresets1)
                                    .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                                    .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                                    .set_text_size(TEXT_SIZE);
                                ui.add(osc_1_attack_knob);

                                let osc_1_decay_knob = ui_knob::ArcKnob::for_param(
                                    &params.osc_1_decay, 
                                    setter, 
                                    KNOB_SIZE)
                                    .preset_style(ui_knob::KnobStyle::NewPresets1)
                                    .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                                    .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                                    .set_text_size(TEXT_SIZE);
                                ui.add(osc_1_decay_knob);

                                let osc_1_sustain_knob = ui_knob::ArcKnob::for_param(
                                    &params.osc_1_sustain, 
                                    setter, 
                                    KNOB_SIZE)
                                    .preset_style(ui_knob::KnobStyle::NewPresets1)
                                    .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                                    .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                                    .set_text_size(TEXT_SIZE);
                                ui.add(osc_1_sustain_knob);

                                let osc_1_release_knob = ui_knob::ArcKnob::for_param(
                                    &params.osc_1_release, 
                                    setter, 
                                    KNOB_SIZE)
                                    .preset_style(ui_knob::KnobStyle::NewPresets1)
                                    .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                                    .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                                    .set_text_size(TEXT_SIZE);
                                ui.add(osc_1_release_knob);
                            });
                            ui.horizontal(|ui| {
                                // knob space
                                //ui.add_space(KNOB_SIZE*4.0 + 4.0);

                                let osc_1_semitones_knob = ui_knob::ArcKnob::for_param(
                                    &params.osc_1_semitones, 
                                    setter, 
                                    KNOB_SIZE)
                                    .preset_style(ui_knob::KnobStyle::NewPresets1)
                                    .set_fill_color(*GUI_VALS.get("A_BACKGROUND_COLOR_TOP").unwrap())
                                    .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                                    .use_outline(true)
                                    .set_text_size(TEXT_SIZE);
                                ui.add(osc_1_semitones_knob);
                                
                                let osc_1_atk_curve_knob = ui_knob::ArcKnob::for_param(
                                    &params.osc_1_atk_curve, 
                                    setter, 
                                    KNOB_SIZE)
                                    .preset_style(ui_knob::KnobStyle::NewPresets1)
                                    .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                                    .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                                    .set_text_size(TEXT_SIZE);
                                ui.add(osc_1_atk_curve_knob);

                                let osc_1_dec_curve_knob = ui_knob::ArcKnob::for_param(
                                    &params.osc_1_dec_curve, 
                                    setter, 
                                    KNOB_SIZE)
                                    .preset_style(ui_knob::KnobStyle::NewPresets1)
                                    .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                                    .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                                    .set_text_size(TEXT_SIZE);
                                ui.add(osc_1_dec_curve_knob);

                                // Sustain knob space
                                ui.add_space(KNOB_SIZE*2.0 + 16.0);
                                
                                let osc_1_rel_curve_knob = ui_knob::ArcKnob::for_param(
                                    &params.osc_1_rel_curve, 
                                    setter, 
                                    KNOB_SIZE)
                                    .preset_style(ui_knob::KnobStyle::NewPresets1)
                                    .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                                    .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                                    .set_text_size(TEXT_SIZE);
                                ui.add(osc_1_rel_curve_knob);
                            });
                        });
                    });
                });
            }
            AudioModuleType::Granulizer => {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Coming soon")
                    .font(SMALLER_FONT)
                    .color(*GUI_VALS.get("FONT_COLOR").unwrap()))
                    .on_hover_text("owo");
                });
            },
            AudioModuleType::Additive => {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Coming soon")
                    .font(SMALLER_FONT)
                    .color(*GUI_VALS.get("FONT_COLOR").unwrap()))
                    .on_hover_text("owo");
                });
            },
        }

        ui.separator();

        // Spot two
        match params._audio_module_2_type.value() {
            AudioModuleType::Off => {
                // Blank space
                ui.label("Disabled");
            },
            AudioModuleType::Osc => {
                // Oscillator
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        let osc_2_type_knob = ui_knob::ArcKnob::for_param(
                            &params.osc_2_type, 
                            setter, 
                            KNOB_SIZE)
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                            .set_line_color(*GUI_VALS.get("SYNTH_MIDDLE_BLUE").unwrap())
                            .set_text_size(TEXT_SIZE);
                        ui.add(osc_2_type_knob);

                        let osc_2_mod_knob = ui_knob::ArcKnob::for_param(
                            &params.osc_2_mod_amount, 
                            setter, 
                            KNOB_SIZE)
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("SYNTH_SOFT_BLUE").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .use_outline(true)
                            .set_text_size(TEXT_SIZE);
                        ui.add(osc_2_mod_knob);

                        let osc_2_octave_knob = ui_knob::ArcKnob::for_param(
                            &params.osc_2_octave, 
                            setter, 
                            KNOB_SIZE)
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("SYNTH_SOFT_BLUE").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .use_outline(true)
                            .set_text_size(TEXT_SIZE);
                        ui.add(osc_2_octave_knob);

                        let osc_2_unison_knob = ui_knob::ArcKnob::for_param(
                            &params.osc_2_unison, 
                            setter, 
                            KNOB_SIZE)
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("A_BACKGROUND_COLOR_TOP").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .use_outline(true)
                            .set_text_size(TEXT_SIZE);
                        ui.add(osc_2_unison_knob);

                        let osc_2_unison_detune_knob = ui_knob::ArcKnob::for_param(
                            &params.osc_2_unison_detune, 
                            setter, 
                            KNOB_SIZE)
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("A_BACKGROUND_COLOR_TOP").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .use_outline(true)
                            .set_text_size(TEXT_SIZE);
                        ui.add(osc_2_unison_detune_knob);

                        let osc_2_attack_knob = ui_knob::ArcKnob::for_param(
                            &params.osc_2_attack, 
                            setter, 
                            KNOB_SIZE)
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .set_text_size(TEXT_SIZE);
                        ui.add(osc_2_attack_knob);

                        let osc_2_decay_knob = ui_knob::ArcKnob::for_param(
                            &params.osc_2_decay, 
                            setter, 
                            KNOB_SIZE)
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .set_text_size(TEXT_SIZE);
                        ui.add(osc_2_decay_knob);

                        let osc_2_sustain_knob = ui_knob::ArcKnob::for_param(
                            &params.osc_2_sustain, 
                            setter, 
                            KNOB_SIZE)
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .set_text_size(TEXT_SIZE);
                        ui.add(osc_2_sustain_knob);

                        let osc_2_release_knob = ui_knob::ArcKnob::for_param(
                            &params.osc_2_release, 
                            setter, 
                            KNOB_SIZE)
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .set_text_size(TEXT_SIZE);
                        ui.add(osc_2_release_knob);
                    });
                    ui.horizontal(|ui| {
                        let osc_2_retrigger_knob = ui_knob::ArcKnob::for_param(
                            &params.osc_2_retrigger, 
                            setter, 
                            KNOB_SIZE)
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("A_BACKGROUND_COLOR_TOP").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .use_outline(true)
                            .set_text_size(TEXT_SIZE);
                        ui.add(osc_2_retrigger_knob);

                        let osc_2_semitones_knob = ui_knob::ArcKnob::for_param(
                            &params.osc_2_semitones, 
                            setter, 
                            KNOB_SIZE)
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("A_BACKGROUND_COLOR_TOP").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .use_outline(true)
                            .set_text_size(TEXT_SIZE);
                        ui.add(osc_2_semitones_knob);

                        let osc_2_detune_knob = ui_knob::ArcKnob::for_param(
                            &params.osc_2_detune, 
                            setter, 
                            KNOB_SIZE)
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("A_BACKGROUND_COLOR_TOP").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .use_outline(true)
                            .set_text_size(TEXT_SIZE);
                        ui.add(osc_2_detune_knob);

                        // Space
                        ui.add_space(KNOB_SIZE*2.0 + 16.0);

                        let osc_2_stereo_knob = ui_knob::ArcKnob::for_param(
                            &params.osc_2_stereo, 
                            setter, 
                            KNOB_SIZE)
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("A_BACKGROUND_COLOR_TOP").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .use_outline(true)
                            .set_text_size(TEXT_SIZE);
                        ui.add(osc_2_stereo_knob);
                        
                        let osc_2_atk_curve_knob = ui_knob::ArcKnob::for_param(
                            &params.osc_2_atk_curve, 
                            setter, 
                            KNOB_SIZE)
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .set_text_size(TEXT_SIZE);
                        ui.add(osc_2_atk_curve_knob);
                        
                        // Attack knob space
                        //ui.add_space(KNOB_SIZE*2.0 + 16.0);

                        let osc_2_dec_curve_knob = ui_knob::ArcKnob::for_param(
                            &params.osc_2_dec_curve, 
                            setter, 
                            KNOB_SIZE)
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .set_text_size(TEXT_SIZE);
                        ui.add(osc_2_dec_curve_knob);

                        // Sustain knob space
                        ui.add_space(KNOB_SIZE*2.0 + 16.0);
                        
                        let osc_2_rel_curve_knob = ui_knob::ArcKnob::for_param(
                            &params.osc_2_rel_curve, 
                            setter, 
                            KNOB_SIZE)
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .set_text_size(TEXT_SIZE);
                        ui.add(osc_2_rel_curve_knob);
                    });
                });
            },
            AudioModuleType::Sampler => {
                const KNOB_SIZE: f32 = 25.0;
                const TEXT_SIZE: f32 = 11.0;
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new("Load Sample")
                                    .font(SMALLER_FONT)
                                    .color(*GUI_VALS.get("FONT_COLOR").unwrap()))
                                    .on_hover_text("The longer the sample the longer the process!");
                                    let switch_toggle = toggle_switch::ToggleSwitch::for_param(&params.load_sample_2, setter);
                                    ui.add(switch_toggle);
                                });
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new("Resample")
                                    .font(SMALLER_FONT)
                                    .color(*GUI_VALS.get("FONT_COLOR").unwrap()))
                                    .on_hover_text("Reload your sample after changing this! On: Resample, Off: Repitch");
                                    let stretch_toggle = toggle_switch::ToggleSwitch::for_param(&params.restretch_2, setter);
                                    ui.add(stretch_toggle);
                                });
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new("Looping")
                                    .font(SMALLER_FONT)
                                    .color(*GUI_VALS.get("FONT_COLOR").unwrap()))
                                    .on_hover_text("To repeat your sample if MIDI key is held");
                                    let loop_toggle = toggle_switch::ToggleSwitch::for_param(&params.loop_sample_2, setter);
                                    ui.add(loop_toggle);
                                });
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new("Single Cycle")
                                    .font(SMALLER_FONT)
                                    .color(*GUI_VALS.get("FONT_COLOR").unwrap()))
                                    .on_hover_text("Use this with Looping ON and Resample ON if you loaded a single cycle waveform");
                                    let sc_toggle = toggle_switch::ToggleSwitch::for_param(&params.single_cycle_2, setter);
                                    ui.add(sc_toggle);
                                });
                            });

                            ui.horizontal(|ui| {
                                let osc_2_octave_knob = ui_knob::ArcKnob::for_param(
                                    &params.osc_2_octave, 
                                    setter, 
                                    KNOB_SIZE)
                                    .preset_style(ui_knob::KnobStyle::NewPresets1)
                                    .set_fill_color(*GUI_VALS.get("A_BACKGROUND_COLOR_TOP").unwrap())
                                    .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                                    .use_outline(true)
                                    .set_text_size(TEXT_SIZE);
                                ui.add(osc_2_octave_knob);

                                let osc_2_attack_knob = ui_knob::ArcKnob::for_param(
                                    &params.osc_2_attack, 
                                    setter, 
                                    KNOB_SIZE)
                                    .preset_style(ui_knob::KnobStyle::NewPresets1)
                                    .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                                    .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                                    .set_text_size(TEXT_SIZE);
                                ui.add(osc_2_attack_knob);

                                let osc_2_decay_knob = ui_knob::ArcKnob::for_param(
                                    &params.osc_2_decay, 
                                    setter, 
                                    KNOB_SIZE)
                                    .preset_style(ui_knob::KnobStyle::NewPresets1)
                                    .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                                    .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                                    .set_text_size(TEXT_SIZE);
                                ui.add(osc_2_decay_knob);

                                let osc_2_sustain_knob = ui_knob::ArcKnob::for_param(
                                    &params.osc_2_sustain, 
                                    setter, 
                                    KNOB_SIZE)
                                    .preset_style(ui_knob::KnobStyle::NewPresets1)
                                    .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                                    .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                                    .set_text_size(TEXT_SIZE);
                                ui.add(osc_2_sustain_knob);

                                let osc_2_release_knob = ui_knob::ArcKnob::for_param(
                                    &params.osc_2_release, 
                                    setter, 
                                    KNOB_SIZE)
                                    .preset_style(ui_knob::KnobStyle::NewPresets1)
                                    .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                                    .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                                    .set_text_size(TEXT_SIZE);
                                ui.add(osc_2_release_knob);
                            });
                            ui.horizontal(|ui| {
                                // knob space
                                //ui.add_space(KNOB_SIZE*4.0 + 4.0);

                                let osc_2_semitones_knob = ui_knob::ArcKnob::for_param(
                                    &params.osc_2_semitones, 
                                    setter, 
                                    KNOB_SIZE)
                                    .preset_style(ui_knob::KnobStyle::NewPresets1)
                                    .set_fill_color(*GUI_VALS.get("A_BACKGROUND_COLOR_TOP").unwrap())
                                    .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                                    .use_outline(true)
                                    .set_text_size(TEXT_SIZE);
                                ui.add(osc_2_semitones_knob);
                                
                                let osc_2_atk_curve_knob = ui_knob::ArcKnob::for_param(
                                    &params.osc_2_atk_curve, 
                                    setter, 
                                    KNOB_SIZE)
                                    .preset_style(ui_knob::KnobStyle::NewPresets1)
                                    .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                                    .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                                    .set_text_size(TEXT_SIZE);
                                ui.add(osc_2_atk_curve_knob);

                                let osc_2_dec_curve_knob = ui_knob::ArcKnob::for_param(
                                    &params.osc_2_dec_curve, 
                                    setter, 
                                    KNOB_SIZE)
                                    .preset_style(ui_knob::KnobStyle::NewPresets1)
                                    .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                                    .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                                    .set_text_size(TEXT_SIZE);
                                ui.add(osc_2_dec_curve_knob);

                                // Sustain knob space
                                ui.add_space(KNOB_SIZE*2.0 + 16.0);
                                
                                let osc_2_rel_curve_knob = ui_knob::ArcKnob::for_param(
                                    &params.osc_2_rel_curve, 
                                    setter, 
                                    KNOB_SIZE)
                                    .preset_style(ui_knob::KnobStyle::NewPresets1)
                                    .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                                    .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                                    .set_text_size(TEXT_SIZE);
                                ui.add(osc_2_rel_curve_knob);
                            });
                        });
                    });
                });
            }
            AudioModuleType::Granulizer => {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Coming soon")
                    .font(SMALLER_FONT)
                    .color(*GUI_VALS.get("FONT_COLOR").unwrap()))
                    .on_hover_text("owo");
                });
            },
            AudioModuleType::Additive => {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Coming soon")
                    .font(SMALLER_FONT)
                    .color(*GUI_VALS.get("FONT_COLOR").unwrap()))
                    .on_hover_text("owo");
                });
            },
        }

        ui.separator();

        // Spot three
        match params._audio_module_3_type.value() {
            AudioModuleType::Off => {
                // Blank space
                ui.label("Disabled");
            },
            AudioModuleType::Osc => {
                // Oscillator
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        let osc_3_type_knob = ui_knob::ArcKnob::for_param(
                            &params.osc_3_type, 
                            setter, 
                            KNOB_SIZE)
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                            .set_line_color(*GUI_VALS.get("SYNTH_MIDDLE_BLUE").unwrap())
                            .set_text_size(TEXT_SIZE);
                        ui.add(osc_3_type_knob);

                        let osc_3_mod_knob = ui_knob::ArcKnob::for_param(
                            &params.osc_3_mod_amount, 
                            setter, 
                            KNOB_SIZE)
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("A_BACKGROUND_COLOR_TOP").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .use_outline(true)
                            .set_text_size(TEXT_SIZE);
                        ui.add(osc_3_mod_knob);

                        let osc_3_octave_knob = ui_knob::ArcKnob::for_param(
                            &params.osc_3_octave, 
                            setter, 
                            KNOB_SIZE)
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("A_BACKGROUND_COLOR_TOP").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .use_outline(true)
                            .set_text_size(TEXT_SIZE);
                        ui.add(osc_3_octave_knob);

                        let osc_3_unison_knob = ui_knob::ArcKnob::for_param(
                            &params.osc_3_unison, 
                            setter, 
                            KNOB_SIZE)
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("A_BACKGROUND_COLOR_TOP").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .use_outline(true)
                            .set_text_size(TEXT_SIZE);
                        ui.add(osc_3_unison_knob);

                        let osc_3_unison_detune_knob = ui_knob::ArcKnob::for_param(
                            &params.osc_3_unison_detune, 
                            setter, 
                            KNOB_SIZE)
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("A_BACKGROUND_COLOR_TOP").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .use_outline(true)
                            .set_text_size(TEXT_SIZE);
                        ui.add(osc_3_unison_detune_knob);

                        let osc_3_attack_knob = ui_knob::ArcKnob::for_param(
                            &params.osc_3_attack, 
                            setter, 
                            KNOB_SIZE)
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .set_text_size(TEXT_SIZE);
                        ui.add(osc_3_attack_knob);

                        let osc_3_decay_knob = ui_knob::ArcKnob::for_param(
                            &params.osc_3_decay, 
                            setter, 
                            KNOB_SIZE)
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .set_text_size(TEXT_SIZE);
                        ui.add(osc_3_decay_knob);

                        let osc_3_sustain_knob = ui_knob::ArcKnob::for_param(
                            &params.osc_3_sustain, 
                            setter, 
                            KNOB_SIZE)
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .set_text_size(TEXT_SIZE);
                        ui.add(osc_3_sustain_knob);

                        let osc_3_release_knob = ui_knob::ArcKnob::for_param(
                            &params.osc_3_release, 
                            setter, 
                            KNOB_SIZE)
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .set_text_size(TEXT_SIZE);
                        ui.add(osc_3_release_knob);
                    });
                    ui.horizontal(|ui| {
                        let osc_3_retrigger_knob = ui_knob::ArcKnob::for_param(
                            &params.osc_3_retrigger, 
                            setter, 
                            KNOB_SIZE)
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("A_BACKGROUND_COLOR_TOP").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .use_outline(true)
                            .set_text_size(TEXT_SIZE);
                        ui.add(osc_3_retrigger_knob);

                        let osc_3_semitones_knob = ui_knob::ArcKnob::for_param(
                            &params.osc_3_semitones, 
                            setter, 
                            KNOB_SIZE)
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("A_BACKGROUND_COLOR_TOP").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .use_outline(true)
                            .set_text_size(TEXT_SIZE);
                        ui.add(osc_3_semitones_knob);

                        let osc_3_detune_knob = ui_knob::ArcKnob::for_param(
                            &params.osc_3_detune, 
                            setter, 
                            KNOB_SIZE)
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("A_BACKGROUND_COLOR_TOP").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .use_outline(true)
                            .set_text_size(TEXT_SIZE);
                        ui.add(osc_3_detune_knob);

                        // Space
                        ui.add_space(KNOB_SIZE*2.0 + 16.0);

                        let osc_3_stereo_knob = ui_knob::ArcKnob::for_param(
                            &params.osc_3_stereo, 
                            setter, 
                            KNOB_SIZE)
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("A_BACKGROUND_COLOR_TOP").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .use_outline(true)
                            .set_text_size(TEXT_SIZE);
                        ui.add(osc_3_stereo_knob);

                        let osc_3_atk_curve_knob = ui_knob::ArcKnob::for_param(
                            &params.osc_3_atk_curve, 
                            setter, 
                            KNOB_SIZE)
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .set_text_size(TEXT_SIZE);
                        ui.add(osc_3_atk_curve_knob);
                        
                        // Attack knob space
                        //ui.add_space(KNOB_SIZE*2.0 + 16.0);

                        let osc_3_dec_curve_knob = ui_knob::ArcKnob::for_param(
                            &params.osc_3_dec_curve, 
                            setter, 
                            KNOB_SIZE)
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .set_text_size(TEXT_SIZE);
                        ui.add(osc_3_dec_curve_knob);

                        // Sustain knob space
                        ui.add_space(KNOB_SIZE*2.0 + 16.0);
                        
                        let osc_3_rel_curve_knob = ui_knob::ArcKnob::for_param(
                            &params.osc_3_rel_curve, 
                            setter, 
                            KNOB_SIZE)
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .set_text_size(TEXT_SIZE);
                        ui.add(osc_3_rel_curve_knob);
                    });
                });
            },
            AudioModuleType::Sampler => {
                const KNOB_SIZE: f32 = 25.0;
                const TEXT_SIZE: f32 = 11.0;
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new("Load Sample")
                                    .font(SMALLER_FONT)
                                    .color(*GUI_VALS.get("FONT_COLOR").unwrap()))
                                    .on_hover_text("The longer the sample the longer the process!");
                                    let switch_toggle = toggle_switch::ToggleSwitch::for_param(&params.load_sample_3, setter);
                                    ui.add(switch_toggle);
                                });
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new("Resample")
                                    .font(SMALLER_FONT)
                                    .color(*GUI_VALS.get("FONT_COLOR").unwrap()))
                                    .on_hover_text("Reload your sample after changing this! On: Resample, Off: Repitch");
                                    let stretch_toggle = toggle_switch::ToggleSwitch::for_param(&params.restretch_3, setter);
                                    ui.add(stretch_toggle);
                                });
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new("Looping")
                                    .font(SMALLER_FONT)
                                    .color(*GUI_VALS.get("FONT_COLOR").unwrap()))
                                    .on_hover_text("To repeat your sample if MIDI key is held");
                                    let loop_toggle = toggle_switch::ToggleSwitch::for_param(&params.loop_sample_3, setter);
                                    ui.add(loop_toggle);
                                });
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new("Single Cycle")
                                    .font(SMALLER_FONT)
                                    .color(*GUI_VALS.get("FONT_COLOR").unwrap()))
                                    .on_hover_text("Use this with Looping ON and Resample ON if you loaded a single cycle waveform");
                                    let sc_toggle = toggle_switch::ToggleSwitch::for_param(&params.single_cycle_3, setter);
                                    ui.add(sc_toggle);
                                });
                            });

                            ui.horizontal(|ui| {
                                let osc_3_octave_knob = ui_knob::ArcKnob::for_param(
                                    &params.osc_3_octave, 
                                    setter, 
                                    KNOB_SIZE)
                                    .preset_style(ui_knob::KnobStyle::NewPresets1)
                                    .set_fill_color(*GUI_VALS.get("A_BACKGROUND_COLOR_TOP").unwrap())
                                    .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                                    .use_outline(true)
                                    .set_text_size(TEXT_SIZE);
                                ui.add(osc_3_octave_knob);

                                let osc_3_attack_knob = ui_knob::ArcKnob::for_param(
                                    &params.osc_3_attack, 
                                    setter, 
                                    KNOB_SIZE)
                                    .preset_style(ui_knob::KnobStyle::NewPresets1)
                                    .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                                    .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                                    .set_text_size(TEXT_SIZE);
                                ui.add(osc_3_attack_knob);

                                let osc_3_decay_knob = ui_knob::ArcKnob::for_param(
                                    &params.osc_3_decay, 
                                    setter, 
                                    KNOB_SIZE)
                                    .preset_style(ui_knob::KnobStyle::NewPresets1)
                                    .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                                    .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                                    .set_text_size(TEXT_SIZE);
                                ui.add(osc_3_decay_knob);

                                let osc_3_sustain_knob = ui_knob::ArcKnob::for_param(
                                    &params.osc_3_sustain, 
                                    setter, 
                                    KNOB_SIZE)
                                    .preset_style(ui_knob::KnobStyle::NewPresets1)
                                    .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                                    .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                                    .set_text_size(TEXT_SIZE);
                                ui.add(osc_3_sustain_knob);

                                let osc_3_release_knob = ui_knob::ArcKnob::for_param(
                                    &params.osc_3_release, 
                                    setter, 
                                    KNOB_SIZE)
                                    .preset_style(ui_knob::KnobStyle::NewPresets1)
                                    .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                                    .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                                    .set_text_size(TEXT_SIZE);
                                ui.add(osc_3_release_knob);
                            });
                            ui.horizontal(|ui| {
                                // knob space
                                //ui.add_space(KNOB_SIZE*4.0 + 4.0);

                                let osc_3_semitones_knob = ui_knob::ArcKnob::for_param(
                                    &params.osc_3_semitones, 
                                    setter, 
                                    KNOB_SIZE)
                                    .preset_style(ui_knob::KnobStyle::NewPresets1)
                                    .set_fill_color(*GUI_VALS.get("A_BACKGROUND_COLOR_TOP").unwrap())
                                    .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                                    .use_outline(true)
                                    .set_text_size(TEXT_SIZE);
                                ui.add(osc_3_semitones_knob);
                                
                                let osc_3_atk_curve_knob = ui_knob::ArcKnob::for_param(
                                    &params.osc_3_atk_curve, 
                                    setter, 
                                    KNOB_SIZE)
                                    .preset_style(ui_knob::KnobStyle::NewPresets1)
                                    .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                                    .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                                    .set_text_size(TEXT_SIZE);
                                ui.add(osc_3_atk_curve_knob);

                                let osc_3_dec_curve_knob = ui_knob::ArcKnob::for_param(
                                    &params.osc_3_dec_curve, 
                                    setter, 
                                    KNOB_SIZE)
                                    .preset_style(ui_knob::KnobStyle::NewPresets1)
                                    .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                                    .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                                    .set_text_size(TEXT_SIZE);
                                ui.add(osc_3_dec_curve_knob);

                                // Sustain knob space
                                ui.add_space(KNOB_SIZE*2.0 + 16.0);
                                
                                let osc_3_rel_curve_knob = ui_knob::ArcKnob::for_param(
                                    &params.osc_3_rel_curve, 
                                    setter, 
                                    KNOB_SIZE)
                                    .preset_style(ui_knob::KnobStyle::NewPresets1)
                                    .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                                    .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                                    .set_text_size(TEXT_SIZE);
                                ui.add(osc_3_rel_curve_knob);
                            });
                        });
                    });
                });
            }
            AudioModuleType::Granulizer => {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Coming soon")
                    .font(SMALLER_FONT)
                    .color(*GUI_VALS.get("FONT_COLOR").unwrap()))
                    .on_hover_text("owo");
                });
            },
            AudioModuleType::Additive => {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Coming soon")
                    .font(SMALLER_FONT)
                    .color(*GUI_VALS.get("FONT_COLOR").unwrap()))
                    .on_hover_text("owo");
                });
            },
        }
    }

    // Index proper params from knobs
    // This lets us have a copy for voices, and also track changes like restretch changing or ADR slopes
    fn consume_params(&mut self, params: Arc<ActuateParams>, voice_index: usize) {
        match voice_index {
            1 => {
                self.audio_module_type = params._audio_module_1_type.value();
                self.osc_type = params.osc_1_type.value();
                self.osc_octave = params.osc_1_octave.value();
                self.osc_semitones = params.osc_1_semitones.value();
                self.osc_detune = params.osc_1_detune.value();
                self.osc_attack = params.osc_1_attack.value();
                self.osc_decay = params.osc_1_decay.value();
                self.osc_sustain = params.osc_1_sustain.value();
                self.osc_release = params.osc_1_release.value();
                self.osc_mod_amount = params.osc_1_mod_amount.value();
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
            },
            2 => {
                self.audio_module_type = params._audio_module_2_type.value();
                self.osc_type = params.osc_2_type.value();
                self.osc_octave = params.osc_2_octave.value();
                self.osc_semitones = params.osc_2_semitones.value();
                self.osc_detune = params.osc_2_detune.value();
                self.osc_attack = params.osc_2_attack.value();
                self.osc_decay = params.osc_2_decay.value();
                self.osc_sustain = params.osc_2_sustain.value();
                self.osc_release = params.osc_2_release.value();
                self.osc_mod_amount = params.osc_2_mod_amount.value();
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
            },
            3 => {
                self.audio_module_type = params._audio_module_3_type.value();
                self.osc_type = params.osc_3_type.value();
                self.osc_octave = params.osc_3_octave.value();
                self.osc_semitones = params.osc_3_semitones.value();
                self.osc_detune = params.osc_3_detune.value();
                self.osc_attack = params.osc_3_attack.value();
                self.osc_decay = params.osc_3_decay.value();
                self.osc_sustain = params.osc_3_sustain.value();
                self.osc_release = params.osc_3_release.value();
                self.osc_mod_amount = params.osc_3_mod_amount.value();
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
            },
            _ => {}
        }
    }

    // I was looking at the PolyModSynth Example and decided on this
    // Handle the audio module midi events and regular pricessing
    // This is an INDIVIDUAL instance process unlike the GUI function
    // This sends back the OSC output + note on for filter to reset
    pub fn process(&mut self, sample_id: usize, params: Arc<ActuateParams>, event_passed: Option<NoteEvent<()>>, voice_index: usize, voice_max: usize, file_open: &mut bool) -> (f32, f32, bool) {
        // If the process is in here the file dialog is not open per lib.rs

        // Get around the egui ui thread by using BoolParam changes :)
        if params.load_sample_1.value() && voice_index == 1 && !file_open.clone() {
            *file_open = true;
            let sample_file = FileDialog::new()
                        .add_filter("wav", &["wav"])
                        //.set_directory("/")
                        .pick_file();
            if Option::is_some(&sample_file) {
                self.load_new_sample(sample_file.unwrap());
            }
        }
        else if params.load_sample_2.value() && voice_index == 2  && !file_open.clone() {
            *file_open = true;
            let sample_file = FileDialog::new()
                .add_filter("wav", &["wav"])
                //.set_directory("/")
                .pick_file();
            if Option::is_some(&sample_file) {
                self.load_new_sample(sample_file.unwrap());
            }
        }
        else if params.load_sample_3.value() && voice_index == 3  && !file_open.clone() {
            *file_open = true;
            let sample_file = FileDialog::new()
                .add_filter("wav", &["wav"])
                //.set_directory("/")
                .pick_file();
            if Option::is_some(&sample_file) {
                self.load_new_sample(sample_file.unwrap());
            }
        }

        // Skip processing if our file dialog is open/running
        if *file_open {
            self.playing_voices.voices.clear();
            self.unison_voices.voices.clear();
            return (0.0, 0.0, false);
        }

        // Loader gets changed back from gui thread and triggers this
        if !params.load_sample_1.value() && !params.load_sample_1.value() && !params.load_sample_1.value() {
            *file_open = false;
        }

        // This function pulls our parameters for each audio module index
        self.consume_params(params, voice_index);

        // Midi events are processed here
        let mut note_on: bool = false;
        match event_passed {
            // The event was valid
            Some(mut event) => { 
                event = event_passed.unwrap();
                if event.timing() > sample_id as u32 {
                    return (0.0, 0.0, false);
                }
                match event {
                    ////////////////////////////////////////////////////////////
                    // MIDI EVENT NOTE ON
                    ////////////////////////////////////////////////////////////
                    NoteEvent::NoteOn { mut note, velocity , ..} => {
                        // Osc + generic stuff
                        note_on = true;
                        let mut new_phase: f32 = 0.0;

                        // Reset the retrigger on Oscs
                        match self.osc_retrigger {
                            RetriggerStyle::Retrigger => {
                                // Start our phase back at 0
                                new_phase = 0.0;
                            },
                            RetriggerStyle::Random => {
                                match self.audio_module_type {
                                    AudioModuleType::Osc => {
                                        // Get a random phase to use
                                        // Poly solution is to pass the phase to the struct
                                        // instead of the osc alone
                                        let mut rng = rand::thread_rng();
                                        new_phase = rng.gen_range(0.0..1.0);
                                    },
                                    AudioModuleType::Sampler => {
                                        let mut rng = rand::thread_rng();
                                        new_phase = rng.gen_range(0.0..self.loaded_sample[0].len() as f32);
                                    },
                                    _ => {},
                                }
                                
                            },
                            RetriggerStyle::Free => {
                                // Do nothing
                            }
                        }
                        // Sampler when single cycle needs this!!!
                        if self.single_cycle {
                            // 31 comes from comparing with 3xOsc position in MIDI notes
                            note += 31;
                        }
                        // Shift our note per octave
                        match self.osc_octave {
                            -2 => { note -= 24; },
                            -1 => { note -= 12; },
                            0 => { note -= 0; },
                            1 => { note += 12; },
                            2 => { note += 24; },
                            _ => {}
                        }
                        // Shift our note per semitones
                        note += self.osc_semitones as u8;
                        // Shift our note per detune
                        // I'm so glad nih-plug has this helper for f32 conversions!
                        let base_note = note as f32 + self.osc_detune;
                        let detuned_note: f32 = util::f32_midi_note_to_freq(base_note);

                        // Create an array of unison notes based off the param for how many unison voices we need
                        let mut unison_notes: Vec<f32> = vec![0.0; self.osc_unison as usize];
                        // If we have any unison voices
                        if self.osc_unison > 1 {
                            // Calculate the detune step amount per amount of voices
                            let detune_step = self.osc_unison_detune/self.osc_unison as f32;
                            for unison_voice in 0..(self.osc_unison as usize - 1) {
                                // Create the detuned notes around the base note
                                if unison_voice % 2 == 1 {
                                    unison_notes[unison_voice] = util::f32_midi_note_to_freq(base_note + detune_step*(unison_voice+1) as f32);
                                }
                                else {
                                    unison_notes[unison_voice] = util::f32_midi_note_to_freq(base_note - detune_step*(unison_voice) as f32);
                                }
                            }
                        }

                        let attack_smoother: Smoother<f32> = match self.osc_atk_curve {
                            SmoothStyle::Linear => Smoother::new(SmoothingStyle::Linear(self.osc_attack)),
                            SmoothStyle::Logarithmic => Smoother::new(SmoothingStyle::Logarithmic(self.osc_attack.clamp(0.0001, 999.9))),
                            SmoothStyle::Exponential => Smoother::new(SmoothingStyle::Exponential(self.osc_attack)),
                        };

                        let decay_smoother: Smoother<f32> = match self.osc_dec_curve {
                            SmoothStyle::Linear => Smoother::new(SmoothingStyle::Linear(self.osc_decay)),
                            SmoothStyle::Logarithmic => Smoother::new(SmoothingStyle::Logarithmic(self.osc_decay.clamp(0.0001, 999.9))),
                            SmoothStyle::Exponential => Smoother::new(SmoothingStyle::Exponential(self.osc_decay)),
                        };

                        let release_smoother: Smoother<f32> = match self.osc_rel_curve {
                            SmoothStyle::Linear => Smoother::new(SmoothingStyle::Linear(self.osc_release)),
                            SmoothStyle::Logarithmic => Smoother::new(SmoothingStyle::Logarithmic(self.osc_release.clamp(0.0001, 999.9))),
                            SmoothStyle::Exponential => Smoother::new(SmoothingStyle::Exponential(self.osc_release)),
                        };

                        match attack_smoother.style {
                            SmoothingStyle::Logarithmic(_) => { 
                                attack_smoother.reset(0.0001); 
                                attack_smoother.set_target(self.sample_rate, velocity.clamp(0.0001, 999.9));
                            },
                            _ => { 
                                attack_smoother.reset(0.0); 
                                attack_smoother.set_target(self.sample_rate, velocity);
                            },
                        }

                        // Osc Updates
                        let new_voice: SingleVoice = SingleVoice {
                            note: note,
                            _velocity: velocity,
                            phase: new_phase,
                            phase_delta: detuned_note / self.sample_rate,
                            state: OscState::Attacking,
                            // These get cloned since smoother cannot be copied
                            amp_current: 0.0,
                            osc_attack: attack_smoother.clone(),
                            osc_decay: decay_smoother.clone(),
                            osc_release: release_smoother.clone(),
                            _detune: self.osc_detune,
                            _unison_detune_value: self.osc_unison_detune,
                            frequency: detuned_note,
                            _attack_time: self.osc_attack,
                            _decay_time: self.osc_decay,
                            _release_time: self.osc_release,
                            _retrigger: self.osc_retrigger,
                            _voice_type: self.osc_type,
                            _angle: 0.0,
                            sample_pos: 0,
                            loop_it: self.loop_wavetable,
                        };

                        // Add our voice struct to our voice tracking deque
                        self.playing_voices.voices.push_front(new_voice);

                        // Add unison voices to our voice tracking deque
                        if self.osc_unison > 1 {
                            let unison_even_voices = if self.osc_unison % 2 == 0 { self.osc_unison } else { self.osc_unison - 1 };
                            let mut unison_angles = vec![0.0; unison_even_voices as usize];
                            for i in 1..(unison_even_voices+1) {
                                let voice_angle = AudioModule::calculate_panning(i - 1, self.osc_unison);
                                unison_angles[(i - 1) as usize] = voice_angle;
                            }
                            
                            for unison_voice in 0..(self.osc_unison as usize - 1) {
                                let new_unison_voice: SingleVoice = SingleVoice {
                                    note: note,
                                    _velocity: velocity,
                                    phase: new_phase,
                                    phase_delta: unison_notes[unison_voice] / self.sample_rate,
                                    state: OscState::Attacking,
                                    // These get cloned since smoother cannot be copied
                                    amp_current: 0.0,
                                    osc_attack: attack_smoother.clone(),
                                    osc_decay: decay_smoother.clone(),
                                    osc_release: release_smoother.clone(),
                                    _detune: self.osc_detune,
                                    _unison_detune_value: self.osc_unison_detune,
                                    frequency: unison_notes[unison_voice],
                                    //frequency: detuned_note,
                                    _attack_time: self.osc_attack,
                                    _decay_time: self.osc_decay,
                                    _release_time: self.osc_release,
                                    _retrigger: self.osc_retrigger,
                                    _voice_type: self.osc_type,
                                    _angle: unison_angles[unison_voice],
                                    sample_pos: 0,
                                    loop_it: self.loop_wavetable,
                                };
                                
                                self.unison_voices.voices.push_front(new_unison_voice);
                            }
                        }

                        // Remove the last voice when > voice_max
                        if self.playing_voices.voices.len() > voice_max {
                            self.playing_voices.voices.resize(voice_max, 
                                // Insert a dummy "Off" entry when resizing UP
                                 SingleVoice {
                                    note: 0,
                                    _velocity: 0.0,
                                    phase: 0.0,
                                    phase_delta: 0.0,
                                    state: OscState::Off,
                                    // These get cloned since smoother cannot be copied
                                    amp_current: 0.0,
                                    osc_attack: attack_smoother.clone(),
                                    osc_decay: decay_smoother.clone(),
                                    osc_release: release_smoother.clone(),
                                    _detune: 0.0,
                                    _unison_detune_value: 0.0,
                                    frequency: 0.0,
                                    _attack_time: self.osc_attack,
                                    _decay_time: self.osc_decay,
                                    _release_time: self.osc_release,
                                    _retrigger: self.osc_retrigger,
                                    _voice_type: self.osc_type,
                                    _angle: 0.0,
                                    sample_pos: 0,
                                    loop_it: self.loop_wavetable,
                                });

                            if self.osc_unison > 1 {
                                self.unison_voices.voices.resize(voice_max as usize, 
                                // Insert a dummy "Off" entry when resizing UP
                                SingleVoice {
                                    note: 0,
                                    _velocity: 0.0,
                                    phase: 0.0,
                                    phase_delta: 0.0,
                                    state: OscState::Off,
                                    // These get cloned since smoother cannot be copied
                                    amp_current: 0.0,
                                    osc_attack: attack_smoother.clone(),
                                    osc_decay: decay_smoother.clone(),
                                    osc_release: release_smoother.clone(),
                                    _detune: 0.0,
                                    _unison_detune_value: 0.0,
                                    frequency: 0.0,
                                    _attack_time: self.osc_attack,
                                    _decay_time: self.osc_decay,
                                    _release_time: self.osc_release,
                                    _retrigger: self.osc_retrigger,
                                    _voice_type: self.osc_type,
                                    _angle: 0.0,
                                    sample_pos: 0,
                                    loop_it: self.loop_wavetable,
                                });
                            }
                        }

                        // Remove any off notes
                        for (i, voice) in self.playing_voices.voices.clone().iter().enumerate() {
                            if voice.state == OscState::Off {
                                self.playing_voices.voices.remove(i);
                            }
                        }
                        for (i, unison_voice) in self.unison_voices.voices.clone().iter().enumerate() {
                            if unison_voice.state == OscState::Off {
                                self.unison_voices.voices.remove(i);
                            }
                        }
                    },
                    ////////////////////////////////////////////////////////////
                    // MIDI EVENT NOTE OFF
                    ////////////////////////////////////////////////////////////
                    NoteEvent::NoteOff { note, .. } => {
                        // Get voices on our note and not already releasing
                        // When a voice reaches 0.0 target on releasing

                        let mut shifted_note: u8 = note;

                        // Sampler when single cycle needs this!!!
                        if self.single_cycle {
                            // 31 comes from comparing with 3xOsc position in MIDI notes
                            shifted_note += 31;
                        }

                        shifted_note = match self.osc_octave {
                            -2 => { shifted_note - 24 },
                            -1 => { shifted_note - 12 },
                            0 => { shifted_note },
                            1 => { shifted_note + 12 },
                            2 => { shifted_note + 24 },
                            _ => { shifted_note }
                        };

                        // Update the matching unison voices
                        for unison_voice in self.unison_voices.voices.iter_mut() {
                            if unison_voice.note == shifted_note && unison_voice.state != OscState::Releasing {
                                // Start our release level from our current gain on the voice
                                unison_voice.osc_release.reset(unison_voice.amp_current);
                                                        // Set our new release target to 0.0 so the note fades
                                match unison_voice.osc_release.style {
                                    SmoothingStyle::Logarithmic(_) => { unison_voice.osc_release.set_target(self.sample_rate, 0.0001); },
                                    _ => { unison_voice.osc_release.set_target(self.sample_rate, 0.0); },
                                }
                                // Update our current amp
                                unison_voice.amp_current = unison_voice.osc_release.next();
                                // Update our voice state
                                unison_voice.state = OscState::Releasing;
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
                                    SmoothingStyle::Logarithmic(_) => { voice.osc_release.set_target(self.sample_rate, 0.0001); },
                                    _ => { voice.osc_release.set_target(self.sample_rate, 0.0); },
                                }
                                // Update our current amp
                                voice.amp_current = voice.osc_release.next();

                                // Update our base voice state to releasing
                                voice.state = OscState::Releasing;
                            }
                        }
                    },
                    // Stop event - doesn't seem to work from FL Studio but left in here
                    NoteEvent::Choke { .. } => { 
                        self.playing_voices.voices.clear();
                        self.unison_voices.voices.clear();
                    },
                    _ => (),
                }
            },
            // The event was invalid - do nothing
            None    => (),
        }

        ////////////////////////////////////////////////////////////
        // Update our voices before output
        ////////////////////////////////////////////////////////////
        for voice in self.playing_voices.voices.iter_mut() {
            // Move our phase outside of the midi events
            // I couldn't find much on how to model this so I based it off previous note phase
            voice.phase += voice.phase_delta;
            if voice.phase > 1.0 {
                voice.phase -= 1.0;
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
        }

        // Update our matching unison voices
        for unison_voice in self.unison_voices.voices.iter_mut() {
            // Move our phase outside of the midi events
            // I couldn't find much on how to model this so I based it off previous note phase
            unison_voice.phase += unison_voice.phase_delta;
            if unison_voice.phase > 1.0 {
                unison_voice.phase -= 1.0;
            }
            // Move from attack to decay if needed
            // Attack is over so use decay amount to reach sustain level - reusing current smoother
            if unison_voice.osc_attack.steps_left() == 0 && unison_voice.state == OscState::Attacking {
                unison_voice.state = OscState::Decaying;
                unison_voice.amp_current = unison_voice.osc_attack.next();
                // Now we will use decay smoother from here
                unison_voice.osc_decay.reset(unison_voice.amp_current);
                let sustain_scaled = self.osc_sustain / 999.9;
                unison_voice.osc_decay.set_target(self.sample_rate, sustain_scaled);
            }
            // Move from Decaying to Sustain hold
            if unison_voice.osc_decay.steps_left() == 0 && unison_voice.state == OscState::Decaying {
                unison_voice.state = OscState::Sustaining;
                let sustain_scaled = self.osc_sustain / 999.9;
                unison_voice.amp_current = sustain_scaled;
                unison_voice.osc_decay.set_target(self.sample_rate, sustain_scaled);
            }
            // End of release
            if unison_voice.state == OscState::Releasing && unison_voice.osc_release.steps_left() == 0 {
                unison_voice.state = OscState::Off;
            }
        }

        ////////////////////////////////////////////////////////////
        // Create output
        ////////////////////////////////////////////////////////////
        let output_signal_l: f32;
        let output_signal_r: f32;
        (output_signal_l, output_signal_r) = match self.audio_module_type {
            AudioModuleType::Osc => {
                let mut summed_voices_l: f32 = 0.0;
                let mut summed_voices_r: f32 = 0.0;
                let mut stereo_voices_l: f32 = 0.0;
                let mut stereo_voices_r: f32 = 0.0;
                let mut center_voices: f32 = 0.0;
                for voice in self.playing_voices.voices.iter_mut() {
                    // Get our current gain amount for use in match below
                    let temp_osc_gain_multiplier: f32 = match voice.state {
                        OscState::Attacking => { voice.osc_attack.next() },
                        OscState::Decaying => { voice.osc_decay.next() },
                        OscState::Sustaining => {  self.osc_sustain / 999.9 },
                        OscState::Releasing => { voice.osc_release.next() },
                        OscState::Off => 0.0,
                    };
                    voice.amp_current = temp_osc_gain_multiplier;
                        
                    voice.phase_delta = voice.frequency / self.sample_rate;
                    center_voices += match self.osc_type {
                        VoiceType::Sine  => Oscillator::calculate_sine(self.osc_mod_amount, voice.phase) * temp_osc_gain_multiplier,
                        VoiceType::Tri => Oscillator::calculate_tri(self.osc_mod_amount, voice.phase) * temp_osc_gain_multiplier,
                        VoiceType::Saw   => Oscillator::calculate_saw(self.osc_mod_amount, voice.phase) * temp_osc_gain_multiplier,
                        VoiceType::RSaw  => Oscillator::calculate_rsaw(self.osc_mod_amount, voice.phase) * temp_osc_gain_multiplier,
                        VoiceType::InSaw  => Oscillator::calculate_inward_saw(self.osc_mod_amount, voice.phase) * temp_osc_gain_multiplier,
                        VoiceType::Ramp => Oscillator::calculate_ramp(self.osc_mod_amount, voice.phase) * temp_osc_gain_multiplier,
                        VoiceType::Square => Oscillator::calculate_square(self.osc_mod_amount, voice.phase) * temp_osc_gain_multiplier,
                        VoiceType::RSquare => Oscillator::calculate_rounded_square(self.osc_mod_amount, voice.phase) * temp_osc_gain_multiplier,
                    };
                }
                // Stereo applies to unison voices
                for unison_voice in self.unison_voices.voices.iter_mut() {
                    // Get our current gain amount for use in match below
                    let temp_osc_gain_multiplier: f32 = match unison_voice.state {
                        OscState::Attacking => { unison_voice.osc_attack.next() },
                        OscState::Decaying => { unison_voice.osc_decay.next() },
                        OscState::Sustaining => {  self.osc_sustain / 999.9 },
                        OscState::Releasing => { unison_voice.osc_release.next() },
                        OscState::Off => 0.0,
                    };
                    unison_voice.amp_current = temp_osc_gain_multiplier;
                    
                    unison_voice.phase_delta = unison_voice.frequency / self.sample_rate;

                    if self.osc_unison > 1 {
                        let temp_unison_voice = match self.osc_type {
                            VoiceType::Sine  => Oscillator::calculate_sine(self.osc_mod_amount, unison_voice.phase) * temp_osc_gain_multiplier,
                            VoiceType::Tri => Oscillator::calculate_tri(self.osc_mod_amount, unison_voice.phase) * temp_osc_gain_multiplier,
                            VoiceType::Saw   => Oscillator::calculate_saw(self.osc_mod_amount, unison_voice.phase) * temp_osc_gain_multiplier,
                            VoiceType::RSaw  => Oscillator::calculate_rsaw(self.osc_mod_amount, unison_voice.phase) * temp_osc_gain_multiplier,
                            VoiceType::InSaw  => Oscillator::calculate_inward_saw(self.osc_mod_amount, unison_voice.phase) * temp_osc_gain_multiplier,
                            VoiceType::Ramp => Oscillator::calculate_ramp(self.osc_mod_amount, unison_voice.phase) * temp_osc_gain_multiplier,
                            VoiceType::Square => Oscillator::calculate_square(self.osc_mod_amount, unison_voice.phase) * temp_osc_gain_multiplier,
                            VoiceType::RSquare => Oscillator::calculate_rounded_square(self.osc_mod_amount, unison_voice.phase) * temp_osc_gain_multiplier,
                        };

                        // Create our stereo pan for unison

                        // Our angle comes back as radians
                        let pan = unison_voice._angle;

                        // Calculate the amplitudes for the panned voice using vector operations
                        let scale = SQRT_2 / 2.0;
                        let left_amp = scale * (pan.cos() + pan.sin()) * temp_unison_voice;
                        let right_amp = scale * (pan.cos() - pan.sin()) * temp_unison_voice;

                        // Add the voice to the sum of stereo voices
                        stereo_voices_l += left_amp;
                        stereo_voices_r += right_amp;
                    }
                }
                // Sum our voices for output
                summed_voices_l += center_voices;
                summed_voices_r += center_voices;
                summed_voices_l += stereo_voices_l;
                summed_voices_r += stereo_voices_r;

                
                // Stereo Spreading code
                let width_coeff = self.osc_stereo*0.5;
                let mid = (summed_voices_l + summed_voices_r)*0.5;
                let stereo = (summed_voices_r - summed_voices_l)*width_coeff;
                summed_voices_l = mid - stereo;
                summed_voices_r = mid + stereo;

                // Return output
                (summed_voices_l, summed_voices_r)
            },
            AudioModuleType::Sampler => {
                let mut summed_voices_l: f32 = 0.0;
                let mut summed_voices_r: f32 = 0.0;
                for voice in self.playing_voices.voices.iter_mut() {
                    // Get our current gain amount for use in match below
                    let temp_osc_gain_multiplier: f32 = match voice.state {
                        OscState::Attacking => { voice.osc_attack.next() },
                        OscState::Decaying => { voice.osc_decay.next() },
                        OscState::Sustaining => {  self.osc_sustain / 999.9 },
                        OscState::Releasing => { voice.osc_release.next() },
                        OscState::Off => 0.0,
                    };
                    voice.amp_current = temp_osc_gain_multiplier;

                    let usize_note = voice.note as usize;

                    // Use our Vec<midi note value<VectorOfChannels<VectorOfSamples>>>
                    // If our note is valid 0-127
                    if usize_note < self.sample_lib.len() {
                        // If our sample position is valid for our note
                        if voice.sample_pos < self.sample_lib[usize_note][0].len() {
                            // Get our channels of sample vectors
                            let NoteVector = &self.sample_lib[usize_note];
                            // We don't need to worry about mono/stereo here because it's been setup in load_new_sample()
                            summed_voices_l += NoteVector[0][voice.sample_pos] * temp_osc_gain_multiplier;
                            summed_voices_r += NoteVector[1][voice.sample_pos] * temp_osc_gain_multiplier;
                        }
                    }

                    // Sampler/Granulizer moves position
                    voice.sample_pos += 1;
                    if voice.loop_it && voice.sample_pos > self.sample_lib[usize_note][0].len(){
                        voice.sample_pos = 0;
                    }
                }
                (summed_voices_l,summed_voices_r)
            },
            AudioModuleType::Off => {
                // Do nothing, return 0.0
                (0.0, 0.0)
            }
            AudioModuleType::Granulizer => {
                // Do nothing, return 0.0
                (0.0, 0.0)
            },
            AudioModuleType::Additive => {
                // Do nothing, return 0.0
                (0.0, 0.0)
            },
        };

        // Send it back
        (output_signal_l, output_signal_r, note_on)
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

    fn load_new_sample(&mut self, path: PathBuf) {
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
                        .map(|s| util::db_to_gain(-36.0) * ((s.unwrap_or_default() as f32 * 256.0) / i8::MAX as f32))
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
                        .map(|s| util::db_to_gain(-36.0) * ((s.unwrap_or_default() as f32 * 256.0) / i16::MAX as f32))
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
                    new_samples[i].push(sample.clone());
                }
            }

            self.loaded_sample = new_samples;

            // Based off restretch vs non stretch use different algorithms
            // To generate a sample library
            self.regenerate_samples();

        };
    }

    // This method performs the sample recalculations when restretch is toggled
    fn regenerate_samples(&mut self) {
        // Compare our restretch change
        if self.restretch != self.prev_restretch {
            self.prev_restretch = self.restretch;
        }

        self.sample_lib.clear();

        if self.restretch {
            let middle_c:f32 = 256.0;
            // Generate our sample library from our sample
            for i in 0..127 {
                let target_pitch_factor = util::f32_midi_note_to_freq(i as f32)/middle_c;

                // Calculate the number of samples in the shifted frame
                let shifted_num_samples = (self.loaded_sample[0].len() as f32 / target_pitch_factor).round() as usize;

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
                        let interpolated_sample_l =
                            (1.0 - fractional_part) * self.loaded_sample[0][original_index]
                                + fractional_part * self.loaded_sample[0][original_index + 1];
                        if self.loaded_sample.len() > 1 {
                            interpolated_sample_r =
                            (1.0 - fractional_part) * self.loaded_sample[1][original_index]
                                + fractional_part * self.loaded_sample[1][original_index + 1];
                        }
                        else {
                            interpolated_sample_r = interpolated_sample_l;
                        }
                    
                        shifted_samples_l.push(interpolated_sample_l);
                        shifted_samples_r.push(interpolated_sample_r);
                    } 
                    else {
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
            let mut shifter = PitchShifter::new(20, self.sample_rate as usize);
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
                
                shifter.shift_pitch(8, translated_i, loaded_left, &mut out_buffer_left);
                shifter.shift_pitch(8, translated_i, loaded_right, &mut out_buffer_right);

                let mut NoteVector = Vec::with_capacity(2);
                NoteVector.insert(0, out_buffer_left);
                NoteVector.insert(1, out_buffer_right);
                self.sample_lib.insert(i, NoteVector);
            }
        }
    }

    fn calculate_panning(voice_index: i32, num_voices: i32) -> f32 {
        // Calculate the pan angle for the given voice index and total number of voices.
        // This uses equal-power panning.
    
        // Ensure the voice index is within bounds.
        let voice_index = voice_index.min(num_voices - 1);
    
        // Calculate the pan angle in radians.
        let angle = ((voice_index as f32) / (num_voices as f32 - 1.0) - 0.5) * std::f32::consts::PI;
    
        // We don't neeed degrees for our calculations
        // let degrees = angle * (180.0 / std::f32::consts::PI);
    
        // Return the pan angle in degrees.
        angle
    }
}