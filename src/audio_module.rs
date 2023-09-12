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

use std::{sync::Arc, collections::VecDeque, path::PathBuf};

use nih_plug::{prelude::{Smoother, SmoothingStyle, ParamSetter, NoteEvent}, util, params::enums::Enum};

// Audio module files
pub(crate) mod Oscillator;
use Oscillator::VoiceType;
use nih_plug_egui::egui::Ui;
use rand::Rng;
use rfd::FileDialog;
use rubato::Resampler;
use crate::{ActuateParams, ui_knob, GUI_VALS, toggle_switch};
use self::Oscillator::{RetriggerStyle, OscState, SmoothStyle};

// When you create a new audio module, you should add it here
#[derive(Enum, PartialEq, Clone, Copy)]
pub enum AudioModuleType {
    Off,
    Osc,
    Granulizer,
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
    frequency: f32,
    _attack_time: f32,
    _decay_time: f32,
    _release_time: f32,
    _retrigger: RetriggerStyle,
    _voice_type: Oscillator::VoiceType,

    // Granulizer Pos
    sample_pos: usize,
}

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
    loaded_sample: Vec<Vec<f32>>,

    ///////////////////////////////////////////////////////////

    // Stored params from main lib here on a per-module basis

    // Osc module knob storage
    osc_type: VoiceType,
    osc_octave: i32,
    osc_semitones: i32,
    osc_detune: f32,
    osc_attack: f32,
    osc_decay: f32,
    osc_sustain: f32,
    osc_release: f32,
    osc_mod_amount: f32,
    osc_retrigger: RetriggerStyle,
    osc_atk_curve: SmoothStyle,
    osc_dec_curve: SmoothStyle,
    osc_rel_curve: SmoothStyle,

    // Voice storage
    playing_voices: VoiceVec,

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

            // Osc module knob storage
            osc_type: VoiceType::Sine,
            osc_octave: 0,
            osc_semitones: 0,
            osc_detune: 0.0,
            osc_attack: 0.01,
            osc_decay: 0.0,
            osc_sustain: 1.0,
            osc_release: 0.07,
            osc_mod_amount: 0.0,
            osc_retrigger: RetriggerStyle::Free,
            osc_atk_curve: SmoothStyle::Linear,
            osc_rel_curve: SmoothStyle::Linear,
            osc_dec_curve: SmoothStyle::Linear,

            // Voice storage
            playing_voices: VoiceVec { voices: VecDeque::new() },

            is_playing: false,
        }
    }
}

impl AudioModule {
    // Draw functions for each module type. This works at CRATE level on the params to draw all 3!!!
    // Passing the producers here is not the nicest thing but we have to move things around to get past the threading stuff + egui's gui separation
    pub fn draw_modules(ui: &mut Ui, params: Arc<ActuateParams>, setter: &ParamSetter<'_>) {
        // Resetting these from the draw thread since setter is valid here - ugly/bad practice
        if params.load_sample_1.value() {
            setter.set_parameter(&params.load_sample_1, false);
        }
        else if params.load_sample_2.value() {
            setter.set_parameter(&params.load_sample_2, false);
        }
        else if params.load_sample_3.value() {
            setter.set_parameter(&params.load_sample_3, false);
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

                        let osc_1_attack_knob = ui_knob::ArcKnob::for_param(
                            &params.osc_1_attack, 
                            setter, 
                            KNOB_SIZE)
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("A_BACKGROUND_COLOR_BOTTOM").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .set_text_size(TEXT_SIZE);
                        ui.add(osc_1_attack_knob);

                        let osc_1_decay_knob = ui_knob::ArcKnob::for_param(
                            &params.osc_1_decay, 
                            setter, 
                            KNOB_SIZE)
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("A_BACKGROUND_COLOR_BOTTOM").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .set_text_size(TEXT_SIZE);
                        ui.add(osc_1_decay_knob);

                        let osc_1_sustain_knob = ui_knob::ArcKnob::for_param(
                            &params.osc_1_sustain, 
                            setter, 
                            KNOB_SIZE)
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("A_BACKGROUND_COLOR_BOTTOM").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .set_text_size(TEXT_SIZE);
                        ui.add(osc_1_sustain_knob);

                        let osc_1_release_knob = ui_knob::ArcKnob::for_param(
                            &params.osc_1_release, 
                            setter, 
                            KNOB_SIZE)
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("A_BACKGROUND_COLOR_BOTTOM").unwrap())
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
            AudioModuleType::Granulizer => {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.label("Load Sample");
                            let switch_toggle = toggle_switch::ToggleSwitch::for_param(&params.load_sample_1, setter);
                            ui.add(switch_toggle);
                        });

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
                            .set_fill_color(*GUI_VALS.get("A_BACKGROUND_COLOR_BOTTOM").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .set_text_size(TEXT_SIZE);
                        ui.add(osc_1_attack_knob);

                        let osc_1_decay_knob = ui_knob::ArcKnob::for_param(
                            &params.osc_1_decay, 
                            setter, 
                            KNOB_SIZE)
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("A_BACKGROUND_COLOR_BOTTOM").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .set_text_size(TEXT_SIZE);
                        ui.add(osc_1_decay_knob);

                        let osc_1_sustain_knob = ui_knob::ArcKnob::for_param(
                            &params.osc_1_sustain, 
                            setter, 
                            KNOB_SIZE)
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("A_BACKGROUND_COLOR_BOTTOM").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .set_text_size(TEXT_SIZE);
                        ui.add(osc_1_sustain_knob);

                        let osc_1_release_knob = ui_knob::ArcKnob::for_param(
                            &params.osc_1_release, 
                            setter, 
                            KNOB_SIZE)
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("A_BACKGROUND_COLOR_BOTTOM").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .set_text_size(TEXT_SIZE);
                        ui.add(osc_1_release_knob);
                    });
                    ui.horizontal(|ui| {
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

                        // Attack knob space
                        //ui.add_space(KNOB_SIZE*2.0 + 16.0);
                       
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
            }
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
                            .set_fill_color(*GUI_VALS.get("A_BACKGROUND_COLOR_TOP").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .use_outline(true)
                            .set_text_size(TEXT_SIZE);
                        ui.add(osc_2_mod_knob);

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
                            .set_fill_color(*GUI_VALS.get("A_BACKGROUND_COLOR_BOTTOM").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .set_text_size(TEXT_SIZE);
                        ui.add(osc_2_attack_knob);

                        let osc_2_decay_knob = ui_knob::ArcKnob::for_param(
                            &params.osc_2_decay, 
                            setter, 
                            KNOB_SIZE)
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("A_BACKGROUND_COLOR_BOTTOM").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .set_text_size(TEXT_SIZE);
                        ui.add(osc_2_decay_knob);

                        let osc_2_sustain_knob = ui_knob::ArcKnob::for_param(
                            &params.osc_2_sustain, 
                            setter, 
                            KNOB_SIZE)
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("A_BACKGROUND_COLOR_BOTTOM").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .set_text_size(TEXT_SIZE);
                        ui.add(osc_2_sustain_knob);

                        let osc_2_release_knob = ui_knob::ArcKnob::for_param(
                            &params.osc_2_release, 
                            setter, 
                            KNOB_SIZE)
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("A_BACKGROUND_COLOR_BOTTOM").unwrap())
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
            AudioModuleType::Granulizer => {
                ui.label("In development!");
                ui.label("Load Sample");
                let switch_toggle = toggle_switch::ToggleSwitch::for_param(&params.load_sample_2, setter);
                ui.add(switch_toggle);
                //ui.add();

                /*
                if ui.add(Button::new("Load Sample")).clicked() {
                    let sample_file = FileDialog::new()
                        .add_filter("wav", &["wav"])
                        //.set_directory("/")
                        .pick_file();
                    
                    // Load our file if it exists
                    if Option::is_some(&sample_file) {
                        
                        ThreadMessage::LoadNewSample(sample_file.unwrap());
                    }
                } 
                */               
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

                        let osc_3_attack_knob = ui_knob::ArcKnob::for_param(
                            &params.osc_3_attack, 
                            setter, 
                            KNOB_SIZE)
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("A_BACKGROUND_COLOR_BOTTOM").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .set_text_size(TEXT_SIZE);
                        ui.add(osc_3_attack_knob);

                        let osc_3_decay_knob = ui_knob::ArcKnob::for_param(
                            &params.osc_3_decay, 
                            setter, 
                            KNOB_SIZE)
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("A_BACKGROUND_COLOR_BOTTOM").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .set_text_size(TEXT_SIZE);
                        ui.add(osc_3_decay_knob);

                        let osc_3_sustain_knob = ui_knob::ArcKnob::for_param(
                            &params.osc_3_sustain, 
                            setter, 
                            KNOB_SIZE)
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("A_BACKGROUND_COLOR_BOTTOM").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .set_text_size(TEXT_SIZE);
                        ui.add(osc_3_sustain_knob);

                        let osc_3_release_knob = ui_knob::ArcKnob::for_param(
                            &params.osc_3_release, 
                            setter, 
                            KNOB_SIZE)
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("A_BACKGROUND_COLOR_BOTTOM").unwrap())
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
            AudioModuleType::Granulizer => {
                ui.label("Load Sample");
                let switch_toggle = toggle_switch::ToggleSwitch::for_param(&params.load_sample_3, setter);
                ui.add(switch_toggle);
            }
        }
    }
    
    /*
    // Return state of type - reuses Oscillator states because it makes sense
    pub fn get_state(&mut self) -> OscState{
        match self.audio_module_type {
            AudioModuleType::Osc => {
                return self.osc.get_osc_state();
            },
            AudioModuleType::Granulizer => {
                // TODO
                return OscState::Off;
            },
            AudioModuleType::Off => {
                return OscState::Off;
            }
        }
    }
    */

    // Index proper params from knobs
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
            },
            _ => {}
        }
    }

    // I was looking at the PolyModSynth Example and decided on this
    // Handle the audio module midi events
    // This is an INDIVIDUAL instance process unlike the GUI function
    // This sends back the OSC output + note on for filter to reset
    pub fn process_midi(&mut self, sample_id: usize, params: Arc<ActuateParams>, event_passed: Option<NoteEvent<()>>, voice_index: usize, voice_max: usize, file_open: &mut bool) -> (f32, f32, bool) {
        // If the process is in here the file dialog is not open per lib.rs process_midi function

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
                                // Get a random phase to use
                                // Poly solution is to pass the phase to the struct
                                // instead of the osc alone
                                let mut rng = rand::thread_rng();
                                new_phase = rng.gen_range(0.0..1.0);
                            },
                            RetriggerStyle::Free => {
                                // Do nothing
                            }
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
                        let detuned_note = util::f32_midi_note_to_freq(note as f32 + self.osc_detune);

                        let attack_smoother: Smoother<f32> = match self.osc_atk_curve {
                            SmoothStyle::Linear => Smoother::new(SmoothingStyle::Linear(self.osc_attack)),
                            SmoothStyle::Logarithmic => Smoother::new(SmoothingStyle::Logarithmic(self.osc_attack.clamp(0.1, 999.9))),
                            SmoothStyle::Exponential => Smoother::new(SmoothingStyle::Exponential(self.osc_attack)),
                        };

                        let decay_smoother: Smoother<f32> = match self.osc_dec_curve {
                            SmoothStyle::Linear => Smoother::new(SmoothingStyle::Linear(self.osc_decay)),
                            SmoothStyle::Logarithmic => Smoother::new(SmoothingStyle::Logarithmic(self.osc_decay.clamp(0.1, 999.9))),
                            SmoothStyle::Exponential => Smoother::new(SmoothingStyle::Exponential(self.osc_decay)),
                        };

                        let release_smoother: Smoother<f32> = match self.osc_rel_curve {
                            SmoothStyle::Linear => Smoother::new(SmoothingStyle::Linear(self.osc_release)),
                            SmoothStyle::Logarithmic => Smoother::new(SmoothingStyle::Logarithmic(self.osc_release.clamp(0.1, 999.9))),
                            SmoothStyle::Exponential => Smoother::new(SmoothingStyle::Exponential(self.osc_release)),
                        };

                        match attack_smoother.style {
                            SmoothingStyle::Logarithmic(_) => { 
                                attack_smoother.reset(0.1); 
                                attack_smoother.set_target(self.sample_rate, velocity.clamp(1.0, 999.9));
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
                            frequency: detuned_note,
                            _attack_time: self.osc_attack,
                            _decay_time: self.osc_decay,
                            _release_time: self.osc_release,
                            _retrigger: self.osc_retrigger,
                            _voice_type: self.osc_type,
                            sample_pos: 0,
                        };

                        // Add our voice struct to our voice tracking deque
                        self.playing_voices.voices.push_front(new_voice);

                        // Remove the last voice when > 32
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
                                    frequency: 0.0,
                                    _attack_time: self.osc_attack,
                                    _decay_time: self.osc_decay,
                                    _release_time: self.osc_release,
                                    _retrigger: self.osc_retrigger,
                                    _voice_type: self.osc_type,
                                    sample_pos: 0,
                                });
                        }

                        // Remove any off notes
                        for (i, voice) in self.playing_voices.voices.clone().iter().enumerate() {
                            if voice.state == OscState::Off {
                                self.playing_voices.voices.remove(i);
                            }
                        }
                    },
                    ////////////////////////////////////////////////////////////
                    // MIDI EVENT NOTE OFF
                    ////////////////////////////////////////////////////////////
                    NoteEvent::NoteOff { note, .. } => {
                        // Iterate through our voice vecdeque to find the one to update
                        for voice in self.playing_voices.voices.iter_mut() {
                            // Get voices on our note and not already releasing
                            // When a voice reaches 0.0 target on releasing

                            let mut shifted_note: u8 = note;
                            shifted_note = match self.osc_octave {
                                -2 => { shifted_note - 24 },
                                -1 => { shifted_note - 12 },
                                0 => { shifted_note },
                                1 => { shifted_note + 12 },
                                2 => { shifted_note + 24 },
                                _ => { shifted_note }
                            };

                            if voice.note == shifted_note && voice.state != OscState::Releasing {
                                // Start our release level from our current gain on the voice
                                voice.osc_release.reset(voice.amp_current);
                                
                                // Set our new release target to 0.0 so the note fades
                                match voice.osc_release.style {
                                    SmoothingStyle::Logarithmic(_) => { voice.osc_release.set_target(self.sample_rate, 0.1); },
                                    _ => { voice.osc_release.set_target(self.sample_rate, 0.0); },
                                }
                                // Update our current amp
                                voice.amp_current = voice.osc_release.next();
                                // Update our voice state
                                voice.state = OscState::Releasing;
                            }
                        }
                    },
                    // Stop event - doesn't seem to work from FL Studio but left in here
                    NoteEvent::Choke { .. } => { self.playing_voices.voices.clear() },
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

        ////////////////////////////////////////////////////////////
        // Create output
        ////////////////////////////////////////////////////////////
        let output_signal_l: f32;
        let output_signal_r: f32;
        (output_signal_l, output_signal_r) = match self.audio_module_type {
            AudioModuleType::Osc => {
                let mut summed_voices: f32 = 0.0;
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
                    summed_voices += match self.osc_type {
                        VoiceType::Sine  => Oscillator::calculate_sine(self.osc_mod_amount, voice.phase) * temp_osc_gain_multiplier,
                        VoiceType::Tri => Oscillator::calculate_tri(self.osc_mod_amount, voice.phase) * temp_osc_gain_multiplier,
                        VoiceType::Saw   => Oscillator::calculate_saw(self.osc_mod_amount, voice.phase) * temp_osc_gain_multiplier,
                        VoiceType::RSaw  => Oscillator::calculate_rsaw(self.osc_mod_amount, voice.phase) * temp_osc_gain_multiplier,
                        VoiceType::InSaw  => Oscillator::calculate_inward_saw(self.osc_mod_amount, voice.phase) * temp_osc_gain_multiplier,
                        VoiceType::Ramp => Oscillator::calculate_ramp(self.osc_mod_amount, voice.phase) * temp_osc_gain_multiplier,
                        VoiceType::Square => Oscillator::calculate_square(self.osc_mod_amount, voice.phase) * temp_osc_gain_multiplier,
                        VoiceType::RSquare => Oscillator::calculate_rounded_square(self.osc_mod_amount, voice.phase) * temp_osc_gain_multiplier,
                    }
                }
                (summed_voices, summed_voices)
            },
            AudioModuleType::Granulizer => {
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
                        
                    //voice.phase_delta = voice.frequency / self.sample_rate;
                    if voice.sample_pos < self.loaded_sample[0].len() {
                        // If our sample is Stereo Channelled
                        if self.loaded_sample.len() == 2 {
                            summed_voices_l += self.loaded_sample[0][voice.sample_pos] * temp_osc_gain_multiplier;
                            summed_voices_r += self.loaded_sample[1][voice.sample_pos] * temp_osc_gain_multiplier;
                        } else {
                            // If our sample is mono we'll duplicate this
                            summed_voices_l += self.loaded_sample[0][voice.sample_pos] * temp_osc_gain_multiplier;
                            summed_voices_r += self.loaded_sample[0][voice.sample_pos] * temp_osc_gain_multiplier;
                        }
                    }

                    // Granulizer moves position
                    voice.sample_pos += 1;
                }
                (summed_voices_l,summed_voices_r)
            },
            AudioModuleType::Off => {
                // Do nothing, return 0.0
                (0.0, 0.0)
            }
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
    }

    fn load_new_sample(&mut self, path: PathBuf) {
        let reader = hound::WavReader::open(&path);
        if let Ok(mut reader) = reader {
            let spec = reader.spec();
            let inner_sample_rate = spec.sample_rate as f32;
            let channels = spec.channels as usize;

            let samples = match spec.sample_format {
                hound::SampleFormat::Int => reader
                    .samples::<i32>()
                    .map(|s| (s.unwrap_or_default() as f32 * 256.0) / i32::MAX as f32)
                    .collect::<Vec<f32>>(),
                hound::SampleFormat::Float => reader
                    .samples::<f32>()
                    .map(|s| s.unwrap_or_default())
                    .collect::<Vec<f32>>(),
            };

            // Uninterleave sample format to chunks for resampling
            let mut new_samples = vec![Vec::with_capacity(samples.len() / channels); channels];

            for sample_chunk in samples.chunks(channels) {
                // sample_chunk is a chunk like [a, b]
                for (i, sample) in sample_chunk.into_iter().enumerate() {
                    new_samples[i].push(sample.clone());
                }
            }

            self.loaded_sample = new_samples;
            
            // resample if needed
            //if inner_sample_rate != self.sample_rate {
                //samples[0] = Self::resample(samples[0], inner_sample_rate, self.sample_rate);
            //}

            //self.loaded_samples.insert(path.clone(), samples);
        };
    }

    fn resample(samples: Vec<Vec<f32>>, sample_rate_in: f32, sample_rate_out: f32) -> Vec<Vec<f32>> {
        let mut resampler = rubato::FftFixedIn::<f32>::new(
            sample_rate_in as usize,
            sample_rate_out as usize,
            samples[0].len(),
            8,
            samples.len(),
        )
        .unwrap();
    
        match resampler.process(&samples, None) {
            Ok(mut waves_out) => {
                // get the duration of leading silence introduced by FFT
                // https://github.com/HEnquist/rubato/blob/52cdc3eb8e2716f40bc9b444839bca067c310592/src/synchro.rs#L654
                let silence_len = resampler.output_delay();
    
                for channel in waves_out.iter_mut() {
                    channel.drain(..silence_len);
                    channel.shrink_to_fit();
                }
    
                waves_out
            }
            Err(_) => vec![],
        }
    }
}