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

use std::sync::Arc;

use nih_plug::{prelude::{Enum, Smoother, SmoothingStyle, ParamSetter, NoteEvent}, util};

// Audio module files
pub(crate) mod Oscillator;
use Oscillator::VoiceType;
use nih_plug_egui::egui::{Ui};

use crate::{ActuateParams, ui_knob, GUI_VALS};

use self::Oscillator::RetriggerStyle;

// When you create a new audio module, you should add it here
#[derive(Enum, PartialEq, Clone, Copy)]
pub enum AudioModuleType {
    Off,
    Osc,
    Granulizer,
}

#[derive(Clone)]
pub struct AudioModule {
    // Stored sample rate in case the audio module needs it
    sample_rate: f32,
    // The MIDI note ID of the active note triggered by MIDI
    midi_note_id: u8,
    // The frequency of the active note triggered by MIDI
    midi_note_freq: f32,
    audio_module_type: AudioModuleType,

    // Audio modules should go here as their struct
    osc: Oscillator::Oscillator,

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
    osc_atk_curve: Oscillator::SmoothStyle,
    osc_rel_curve: Oscillator::SmoothStyle,

    // Current gain for envelope
    audio_module_current_gain: Smoother<f32>
}

// When you create a new audio module you need to add its default creation here as well
impl Default for AudioModule {
    fn default() -> Self {
        Self {
            // Audio modules will use these
            sample_rate: 44100.0,
            midi_note_id: 0,
            midi_note_freq: 1.0,
            audio_module_type: AudioModuleType::Osc,

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
            osc_atk_curve: Oscillator::SmoothStyle::Linear,
            osc_rel_curve: Oscillator::SmoothStyle::Linear,

            // Osc module defaults
            osc: Oscillator::Oscillator { 
                sample_rate: 44100.0, 
                osc_type: VoiceType::Sine, 
                osc_attack: Smoother::new(SmoothingStyle::Linear(50.0)), 
                osc_release: Smoother::new(SmoothingStyle::Linear(50.0)), 
                prev_attack: 0.0, 
                prev_release: 0.0,
                attack_smoothing: Oscillator::SmoothStyle::Linear,
                prev_attack_smoothing: Oscillator::SmoothStyle::Linear,
                release_smoothing: Oscillator::SmoothStyle::Linear,
                prev_release_smoothing: Oscillator::SmoothStyle::Linear,
                osc_mod_amount: 0.0, 
                prev_note_phase_delta: 0.0, 
                phase: 0.0,
                osc_state: Oscillator::OscState::Off,
            },

            // Current Gain for envelopes
            audio_module_current_gain: Smoother::new(SmoothingStyle::Linear(0.0)), 
        }
    }
}

impl AudioModule {
    // Draw functions for each module type. This works at CRATE level on the params to draw all 3!!!
    pub fn draw_modules(ui: &mut Ui, params: Arc<ActuateParams>, setter: &ParamSetter<'_>) {
        const KNOB_SIZE: f32 = 30.0;
        const TEXT_SIZE: f32 = 12.0;
        const SPACER: f32 = 10.0;

        // This is kind of ugly but I couldn't figure out a better architechture for egui and separating audio modules

        ui.add_space(SPACER);
        
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
                            .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                            .set_line_color(*GUI_VALS.get("SYNTH_MIDDLE_BLUE").unwrap())
                            .set_text_size(TEXT_SIZE);
                        ui.add(osc_1_mod_knob);

                        let osc_1_octave_knob = ui_knob::ArcKnob::for_param(
                            &params.osc_1_octave, 
                            setter, 
                            KNOB_SIZE)
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                            .set_line_color(*GUI_VALS.get("SYNTH_MIDDLE_BLUE").unwrap())
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
                        let osc_1_retrigger_knob = ui_knob::ArcKnob::for_param(
                            &params.osc_1_retrigger, 
                            setter, 
                            KNOB_SIZE)
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .set_text_size(TEXT_SIZE);
                        ui.add(osc_1_retrigger_knob);

                        let osc_1_semitones_knob = ui_knob::ArcKnob::for_param(
                            &params.osc_1_semitones, 
                            setter, 
                            KNOB_SIZE)
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .set_text_size(TEXT_SIZE);
                        ui.add(osc_1_semitones_knob);

                        let osc_1_detune_knob = ui_knob::ArcKnob::for_param(
                            &params.osc_1_detune, 
                            setter, 
                            KNOB_SIZE)
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
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

                        // Decay knob space
                        ui.add_space(KNOB_SIZE*2.0 + 4.0);
                        // Sustain knob space
                        ui.add_space(KNOB_SIZE*2.0 + 4.0);
                        
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
                ui.label("In development!");
            }
        }

        ui.add_space(SPACER);

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
                            .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                            .set_line_color(*GUI_VALS.get("SYNTH_MIDDLE_BLUE").unwrap())
                            .set_text_size(TEXT_SIZE);
                        ui.add(osc_2_mod_knob);

                        let osc_2_octave_knob = ui_knob::ArcKnob::for_param(
                            &params.osc_2_octave, 
                            setter, 
                            KNOB_SIZE)
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                            .set_line_color(*GUI_VALS.get("SYNTH_MIDDLE_BLUE").unwrap())
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
                        let osc_2_retrigger_knob = ui_knob::ArcKnob::for_param(
                            &params.osc_2_retrigger, 
                            setter, 
                            KNOB_SIZE)
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .set_text_size(TEXT_SIZE);
                        ui.add(osc_2_retrigger_knob);

                        let osc_2_semitones_knob = ui_knob::ArcKnob::for_param(
                            &params.osc_2_semitones, 
                            setter, 
                            KNOB_SIZE)
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .set_text_size(TEXT_SIZE);
                        ui.add(osc_2_semitones_knob);

                        let osc_2_detune_knob = ui_knob::ArcKnob::for_param(
                            &params.osc_2_detune, 
                            setter, 
                            KNOB_SIZE)
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
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

                        // Decay knob space
                        ui.add_space(KNOB_SIZE*2.0 + 4.0);
                        // Sustain knob space
                        ui.add_space(KNOB_SIZE*2.0 + 4.0);
                        
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
            }
        }

        ui.add_space(SPACER);

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
                            .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                            .set_line_color(*GUI_VALS.get("SYNTH_MIDDLE_BLUE").unwrap())
                            .set_text_size(TEXT_SIZE);
                        ui.add(osc_3_mod_knob);

                        let osc_3_octave_knob = ui_knob::ArcKnob::for_param(
                            &params.osc_3_octave, 
                            setter, 
                            KNOB_SIZE)
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                            .set_line_color(*GUI_VALS.get("SYNTH_MIDDLE_BLUE").unwrap())
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
                        let osc_3_retrigger_knob = ui_knob::ArcKnob::for_param(
                            &params.osc_3_retrigger, 
                            setter, 
                            KNOB_SIZE)
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .set_text_size(TEXT_SIZE);
                        ui.add(osc_3_retrigger_knob);

                        let osc_3_semitones_knob = ui_knob::ArcKnob::for_param(
                            &params.osc_3_semitones, 
                            setter, 
                            KNOB_SIZE)
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .set_text_size(TEXT_SIZE);
                        ui.add(osc_3_semitones_knob);

                        let osc_3_detune_knob = ui_knob::ArcKnob::for_param(
                            &params.osc_3_detune, 
                            setter, 
                            KNOB_SIZE)
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
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

                        // Decay knob space
                        ui.add_space(KNOB_SIZE*2.0 + 4.0);
                        // Sustain knob space
                        ui.add_space(KNOB_SIZE*2.0 + 4.0);
                        
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
                ui.label("In development!");
            }
        }
    }

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
                self.osc_rel_curve = params.osc_3_rel_curve.value();
            },
            _ => {}
        }
    }

    // Handle the audio module midi events
    // This is an INDIVIDUAL instance process unlike the GUI function
    pub fn process_midi(&mut self, sample_id: usize, params: Arc<ActuateParams>, event_passed: Option<NoteEvent<()>>, voice_index: usize) -> f32 {
        self.consume_params(params, voice_index);
        // Update our envelopes if needed
        self.osc.check_update_attack(self.osc_attack, self.osc_atk_curve);
        self.osc.check_update_release(self.osc_release, self.osc_rel_curve);

        // Outputs from the modules
        let output_signal: f32;
        match event_passed {
            // The event was valid
            Some(mut event) => { 
                event = event_passed.unwrap();
                if event.timing() > sample_id as u32 {
                    return 0.0;
                }
                match event {
                    // Midi Calculation Code
                    NoteEvent::NoteOn { mut note, velocity, .. } => {
                        // Reset the retrigger on Oscs
                        match self.osc_retrigger {
                            RetriggerStyle::Retrigger => {
                                self.osc.reset_phase();
                            },
                            RetriggerStyle::Random => {
                                self.osc.set_random_phase();
                            },
                            RetriggerStyle::Free => {
                                // Do nothing
                            }
                        }
                        // Shift our note per octave
                        match self.osc_octave {
                            -2 => { note -= 24; },
                            -1 => { note -= 12; },
                            0 => {},
                            1 => { note += 12; },
                            2 => { note += 24; },
                            _ => {}
                        }
                        // Shift our note per semitones
                        note += self.osc_semitones as u8;
                        // Shift our note per detune
                        self.midi_note_id = note;
                        // I'm so glad nih-plug has this helper for f32 conversions!
                        self.midi_note_freq = util::f32_midi_note_to_freq(self.midi_note_id as f32 + self.osc_detune);
                        // Osc Updates
                        self.osc.reset_attack_smoother(0.0);
                        // Reset release for logic to know note is happening
                        self.osc.reset_release_smoother(0.0);
                        self.osc.set_attack_target(self.sample_rate, velocity);
                        self.audio_module_current_gain = self.osc.get_attack_smoother();
                        self.osc.set_osc_state(Oscillator::OscState::Attacking);
                    },
                    NoteEvent::NoteOff { note, .. } if note == self.midi_note_id => {
                        // This reset lets us fade from any max or other value to 0
                        self.osc.reset_release_smoother(self.audio_module_current_gain.next());
                        // Reset attack
                        self.osc.reset_attack_smoother(0.0);
                        self.osc.set_release_target(self.sample_rate, 0.0);
                        self.audio_module_current_gain = self.osc.get_release_smoother();
                        self.osc.set_osc_state(Oscillator::OscState::Releasing);
                    },
                    _ => (),
                }
            },
            // The event was invalid
            None    => (),
        }

        // Move our phase outside of the midi events
        // I couldn't find much on how to model this so I based it off previous note phase
        if self.osc_retrigger == RetriggerStyle::Free {
            self.osc.increment_phase();
        }

        // Attack is over so use decay amount to reach sustain level - reusing current smoother
        if  self.audio_module_current_gain.steps_left() == 0 && 
            self.osc.get_osc_state() == Oscillator::OscState::Attacking
        {
            self.osc.set_osc_state(Oscillator::OscState::Decaying);
            let temp_gain = self.audio_module_current_gain.next();
            self.audio_module_current_gain = Smoother::new(SmoothingStyle::Linear(self.osc_decay));
            self.audio_module_current_gain.reset(temp_gain);
            let sustain_scaled = self.osc_sustain / 999.9;
            self.audio_module_current_gain.set_target(self.sample_rate, sustain_scaled);
        }

        // Move from Decaying to Sustain hold
        if  self.audio_module_current_gain.steps_left() == 0 && 
            self.osc.get_osc_state() == Oscillator::OscState::Decaying
        {
            let sustain_scaled = self.osc_sustain / 999.9;
            self.audio_module_current_gain.set_target(self.sample_rate, sustain_scaled);
            self.osc.set_osc_state(Oscillator::OscState::Sustaining);
        }

        // End of release
        if  self.osc.get_osc_state() == Oscillator::OscState::Releasing &&
            self.audio_module_current_gain.steps_left() == 0
        {
            self.osc.set_osc_state(Oscillator::OscState::Off);
        }

        // Get our current gain amount for use in match below
        let temp_osc_1_gain_multiplier: f32 = self.audio_module_current_gain.next();

        // Generate our output signal!
        output_signal = match self.audio_module_type {
            AudioModuleType::Osc => {
                match self.osc_type {
                    VoiceType::Sine  => self.osc.calculate_sine(self.midi_note_freq, self.osc_mod_amount) * temp_osc_1_gain_multiplier,
                    VoiceType::Saw   => self.osc.calculate_saw(self.midi_note_freq, self.osc_mod_amount) * temp_osc_1_gain_multiplier,
                    VoiceType::RoundedSaw  => self.osc.calculate_rsaw(self.midi_note_freq, self.osc_mod_amount) * temp_osc_1_gain_multiplier,
                    VoiceType::InwardSaw  => self.osc.calculate_inward_saw(self.midi_note_freq, self.osc_mod_amount) * temp_osc_1_gain_multiplier,
                    VoiceType::DoubleExpSaw => self.osc.calculate_dub_exp_saw(self.midi_note_freq, self.osc_mod_amount) * temp_osc_1_gain_multiplier,
                    VoiceType::Ramp => self.osc.calculate_ramp(self.midi_note_freq, self.osc_mod_amount) * temp_osc_1_gain_multiplier,
                    VoiceType::Wave1 => self.osc.calculate_wave_1(self.midi_note_freq, self.osc_mod_amount) * temp_osc_1_gain_multiplier,
                }
            },
            AudioModuleType::Granulizer => {
                // TODO!
                0.0
            },
            AudioModuleType::Off => {
                // Do nothing, return 0.0
                0.0
            }
        };
        return output_signal
    }
}