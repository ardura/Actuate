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

use nih_plug::{
    params::enums::Enum,
    prelude::{NoteEvent, ParamSetter, Smoother, SmoothingStyle},
    util,
};
use nih_plug_egui::egui::{RichText, Ui};
use pitch_shift::PitchShifter;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::{collections::VecDeque, f32::consts::SQRT_2, path::PathBuf, sync::Arc};

// Audio module files
pub(crate) mod Oscillator;
use self::Oscillator::{DeterministicWhiteNoiseGenerator, OscState, RetriggerStyle, SmoothStyle};
use crate::{
    toggle_switch, ui_knob, ActuateParams, CustomParamSlider, CustomVerticalSlider, GUI_VALS,
    SMALLER_FONT,
};
use CustomParamSlider::ParamSlider as HorizontalParamSlider;
use CustomVerticalSlider::ParamSlider as VerticalParamSlider;
use Oscillator::VoiceType;

// When you create a new audio module, you should add it here
#[derive(Enum, PartialEq, Clone, Copy, Serialize, Deserialize)]
pub enum AudioModuleType {
    Off,
    Osc,
    Sampler,
    Granulizer,
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

    // Additive variables
    pub add_partial0: f32,
    pub add_partial0_phase: f32,
    pub add_partial1: f32,
    pub add_partial1_phase: f32,
    pub add_partial2: f32,
    pub add_partial2_phase: f32,

    // Voice storage
    playing_voices: VoiceVec,
    unison_voices: VoiceVec,

    // Tracking stopping voices too
    is_playing: bool,

    // Noise variables
    noise_obj: Oscillator::DeterministicWhiteNoiseGenerator,
}

// When you create a new audio module you need to add its default creation here as well
#[allow(overflowing_literals)]
impl Default for AudioModule {
    fn default() -> Self {
        Self {
            // Audio modules will use these
            sample_rate: 44100.0,
            audio_module_type: AudioModuleType::Osc,
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

            // Additive variables
            add_partial0: 1.0,
            add_partial0_phase: 0.0,
            add_partial1: 0.0,
            add_partial1_phase: 0.0,
            add_partial2: 0.0,
            add_partial2_phase: 0.0,

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

        const VERT_BAR_HEIGHT: f32 = 106.0;
        let VERT_BAR_HEIGHT_SHORTENED: f32 = VERT_BAR_HEIGHT - ui.spacing().interact_size.y;
        const VERT_BAR_WIDTH: f32 = 14.0;
        const HCURVE_WIDTH: f32 = 120.0;
        const HCURVE_BWIDTH: f32 = 28.0;

        // This is ugly but I couldn't figure out a better architechture for egui and separating audio modules

        /////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
        // Spot One (1)
        /////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
        match params._audio_module_1_type.value() {
            AudioModuleType::Off => {
                // Blank space
                ui.label("Disabled");
            }
            AudioModuleType::Osc => {
                const KNOB_SIZE: f32 = 30.0;
                const TEXT_SIZE: f32 = 12.0;
                // Oscillator
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            let osc_1_type_knob =
                                ui_knob::ArcKnob::for_param(&params.osc_1_type, setter, KNOB_SIZE)
                                    .preset_style(ui_knob::KnobStyle::NewPresets1)
                                    .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                                    .set_line_color(*GUI_VALS.get("SYNTH_MIDDLE_BLUE").unwrap())
                                    .set_text_size(TEXT_SIZE);
                            ui.add(osc_1_type_knob);

                            let osc_1_retrigger_knob = ui_knob::ArcKnob::for_param(
                                &params.osc_1_retrigger,
                                setter,
                                KNOB_SIZE,
                            )
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .use_outline(true)
                            .set_text_size(TEXT_SIZE);
                            ui.add(osc_1_retrigger_knob);
                        });

                        ui.vertical(|ui| {
                            let osc_1_octave_knob = ui_knob::ArcKnob::for_param(
                                &params.osc_1_octave,
                                setter,
                                KNOB_SIZE,
                            )
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .use_outline(true)
                            .set_text_size(TEXT_SIZE);
                            ui.add(osc_1_octave_knob);

                            let osc_1_semitones_knob = ui_knob::ArcKnob::for_param(
                                &params.osc_1_semitones,
                                setter,
                                KNOB_SIZE,
                            )
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .use_outline(true)
                            .set_text_size(TEXT_SIZE);
                            ui.add(osc_1_semitones_knob);
                        });

                        ui.vertical(|ui| {
                            let osc_1_stereo_knob = ui_knob::ArcKnob::for_param(
                                &params.osc_1_stereo,
                                setter,
                                KNOB_SIZE,
                            )
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .use_outline(true)
                            .set_text_size(TEXT_SIZE);
                            ui.add(osc_1_stereo_knob);

                            let osc_1_unison_knob = ui_knob::ArcKnob::for_param(
                                &params.osc_1_unison,
                                setter,
                                KNOB_SIZE,
                            )
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .use_outline(true)
                            .set_text_size(TEXT_SIZE);
                            ui.add(osc_1_unison_knob);
                        });

                        ui.vertical(|ui| {
                            let osc_1_detune_knob = ui_knob::ArcKnob::for_param(
                                &params.osc_1_detune,
                                setter,
                                KNOB_SIZE,
                            )
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .use_outline(true)
                            .set_text_size(TEXT_SIZE);
                            ui.add(osc_1_detune_knob);

                            let osc_1_unison_detune_knob = ui_knob::ArcKnob::for_param(
                                &params.osc_1_unison_detune,
                                setter,
                                KNOB_SIZE,
                            )
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .use_outline(true)
                            .set_text_size(TEXT_SIZE);
                            ui.add(osc_1_unison_detune_knob);
                        });

                        // ADSR
                        ui.add(
                            VerticalParamSlider::for_param(&params.osc_1_attack, setter)
                                .with_width(VERT_BAR_WIDTH)
                                .with_height(VERT_BAR_HEIGHT)
                                .set_reversed(true)
                                .override_colors(
                                    *GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap(),
                                    *GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap(),
                                ),
                        );
                        ui.add(
                            VerticalParamSlider::for_param(&params.osc_1_decay, setter)
                                .with_width(VERT_BAR_WIDTH)
                                .with_height(VERT_BAR_HEIGHT)
                                .set_reversed(true)
                                .override_colors(
                                    *GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap(),
                                    *GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap(),
                                ),
                        );
                        ui.add(
                            VerticalParamSlider::for_param(&params.osc_1_sustain, setter)
                                .with_width(VERT_BAR_WIDTH)
                                .with_height(VERT_BAR_HEIGHT)
                                .set_reversed(true)
                                .override_colors(
                                    *GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap(),
                                    *GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap(),
                                ),
                        );
                        ui.add(
                            VerticalParamSlider::for_param(&params.osc_1_release, setter)
                                .with_width(VERT_BAR_WIDTH)
                                .with_height(VERT_BAR_HEIGHT)
                                .set_reversed(true)
                                .override_colors(
                                    *GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap(),
                                    *GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap(),
                                ),
                        );

                        // Curve sliders
                        ui.vertical(|ui| {
                            ui.add(
                                HorizontalParamSlider::for_param(&params.osc_1_atk_curve, setter)
                                    .with_width(HCURVE_BWIDTH)
                                    .set_left_sided_label(true)
                                    .set_label_width(HCURVE_WIDTH),
                            );
                            ui.add(
                                HorizontalParamSlider::for_param(&params.osc_1_dec_curve, setter)
                                    .with_width(HCURVE_BWIDTH)
                                    .set_left_sided_label(true)
                                    .set_label_width(HCURVE_WIDTH),
                            );
                            ui.add(
                                HorizontalParamSlider::for_param(&params.osc_1_rel_curve, setter)
                                    .with_width(HCURVE_BWIDTH)
                                    .set_left_sided_label(true)
                                    .set_label_width(HCURVE_WIDTH),
                            );
                        });
                    });
                });
            }
            AudioModuleType::Sampler => {
                const KNOB_SIZE: f32 = 25.0;
                const TEXT_SIZE: f32 = 11.0;
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Load Sample")
                        .font(SMALLER_FONT)
                        .color(*GUI_VALS.get("FONT_COLOR").unwrap()))
                        .on_hover_text("The longer the sample the longer the process!");
                        let switch_toggle = toggle_switch::ToggleSwitch::for_param(&params.load_sample_1, setter);
                        ui.add(switch_toggle);

                        ui.label(RichText::new("Resample")
                        .font(SMALLER_FONT)
                        .color(*GUI_VALS.get("FONT_COLOR").unwrap()))
                        .on_hover_text("Reload your sample after changing this! On: Resample, Off: Repitch");
                        let stretch_toggle = toggle_switch::ToggleSwitch::for_param(&params.restretch_1, setter);
                        ui.add(stretch_toggle);

                        ui.label(RichText::new("Looping")
                        .font(SMALLER_FONT)
                        .color(*GUI_VALS.get("FONT_COLOR").unwrap()))
                        .on_hover_text("To repeat your sample if MIDI key is held");
                        let loop_toggle = toggle_switch::ToggleSwitch::for_param(&params.loop_sample_1, setter);
                        ui.add(loop_toggle);

                        ui.label(RichText::new("Single Cycle")
                        .font(SMALLER_FONT)
                        .color(*GUI_VALS.get("FONT_COLOR").unwrap()))
                        .on_hover_text("Use this with Looping ON and Resample ON if you loaded a single cycle waveform");
                        let sc_toggle = toggle_switch::ToggleSwitch::for_param(&params.single_cycle_1, setter);
                        ui.add(sc_toggle);
                    });
                    ui.horizontal(|ui|{
                        ui.vertical(|ui| {
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
                        });
                        ui.vertical(|ui| {
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
                        });
                        ui.vertical(|ui|{
                            let start_position_1_knob = ui_knob::ArcKnob::for_param(
                                &params.start_position_1,
                                setter,
                                KNOB_SIZE)
                                .preset_style(ui_knob::KnobStyle::NewPresets1)
                                .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                                .set_line_color(*GUI_VALS.get("SYNTH_MIDDLE_BLUE").unwrap())
                                .set_text_size(TEXT_SIZE);
                            ui.add(start_position_1_knob);
                            let end_position_1_knob = ui_knob::ArcKnob::for_param(
                                &params.end_position_1,
                                setter,
                                KNOB_SIZE)
                                .preset_style(ui_knob::KnobStyle::NewPresets1)
                                .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                                .set_line_color(*GUI_VALS.get("SYNTH_MIDDLE_BLUE").unwrap())
                                .set_text_size(TEXT_SIZE);
                            ui.add(end_position_1_knob);
                        });
                        // ADSR
                        ui.add(VerticalParamSlider::for_param(&params.osc_1_attack, setter).with_width(VERT_BAR_WIDTH).with_height(VERT_BAR_HEIGHT_SHORTENED).set_reversed(true).override_colors(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap(), *GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap()));
                        ui.add(VerticalParamSlider::for_param(&params.osc_1_decay, setter).with_width(VERT_BAR_WIDTH).with_height(VERT_BAR_HEIGHT_SHORTENED).set_reversed(true).override_colors(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap(), *GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap()));
                        ui.add(VerticalParamSlider::for_param(&params.osc_1_sustain, setter).with_width(VERT_BAR_WIDTH).with_height(VERT_BAR_HEIGHT_SHORTENED).set_reversed(true).override_colors(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap(), *GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap()));
                        ui.add(VerticalParamSlider::for_param(&params.osc_1_release, setter).with_width(VERT_BAR_WIDTH).with_height(VERT_BAR_HEIGHT_SHORTENED).set_reversed(true).override_colors(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap(), *GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap()));

                        // Curve sliders
                        ui.vertical(|ui| {
                            ui.add(HorizontalParamSlider::for_param(&params.osc_1_atk_curve, setter).with_width(HCURVE_BWIDTH).set_left_sided_label(true).set_label_width(HCURVE_WIDTH));
                            ui.add(HorizontalParamSlider::for_param(&params.osc_1_dec_curve, setter).with_width(HCURVE_BWIDTH).set_left_sided_label(true).set_label_width(HCURVE_WIDTH));
                            ui.add(HorizontalParamSlider::for_param(&params.osc_1_rel_curve, setter).with_width(HCURVE_BWIDTH).set_left_sided_label(true).set_label_width(HCURVE_WIDTH));
                        });
                    });
                });
            }
            AudioModuleType::Granulizer => {
                const KNOB_SIZE: f32 = 25.0;
                const TEXT_SIZE: f32 = 11.0;
                // This fixes the granulizer release being longer than a grain itself
                if params.grain_hold_1.value() < params.grain_crossfade_1.value() {
                    setter.set_parameter(&params.grain_crossfade_1, params.grain_hold_1.value());
                }
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("Load Sample")
                                .font(SMALLER_FONT)
                                .color(*GUI_VALS.get("FONT_COLOR").unwrap()),
                        )
                        .on_hover_text("The longer the sample the longer the process!");
                        let switch_toggle =
                            toggle_switch::ToggleSwitch::for_param(&params.load_sample_1, setter);
                        ui.add(switch_toggle);

                        ui.label(
                            RichText::new("Looping")
                                .font(SMALLER_FONT)
                                .color(*GUI_VALS.get("FONT_COLOR").unwrap()),
                        )
                        .on_hover_text("To repeat your sample if MIDI key is held");
                        let loop_toggle =
                            toggle_switch::ToggleSwitch::for_param(&params.loop_sample_1, setter);
                        ui.add(loop_toggle);

                        ui.add_space(30.0);
                        ui.label(
                            RichText::new("Note: ADSR is per note, Shape is AR per grain")
                                .font(SMALLER_FONT)
                                .color(*GUI_VALS.get("FONT_COLOR").unwrap()),
                        )
                        .on_hover_text("ADSR is per note, Shape is AR per grain");
                    });
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            let osc_1_octave_knob = ui_knob::ArcKnob::for_param(
                                &params.osc_1_octave,
                                setter,
                                KNOB_SIZE,
                            )
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .use_outline(true)
                            .set_text_size(TEXT_SIZE);
                            ui.add(osc_1_octave_knob);

                            let osc_1_semitones_knob = ui_knob::ArcKnob::for_param(
                                &params.osc_1_semitones,
                                setter,
                                KNOB_SIZE,
                            )
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .use_outline(true)
                            .set_text_size(TEXT_SIZE);
                            ui.add(osc_1_semitones_knob);
                        });

                        ui.vertical(|ui| {
                            let osc_1_retrigger_knob = ui_knob::ArcKnob::for_param(
                                &params.osc_1_retrigger,
                                setter,
                                KNOB_SIZE,
                            )
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .use_outline(true)
                            .set_text_size(TEXT_SIZE);
                            ui.add(osc_1_retrigger_knob);

                            let grain_crossfade_1_knob = ui_knob::ArcKnob::for_param(
                                &params.grain_crossfade_1,
                                setter,
                                KNOB_SIZE,
                            )
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                            .set_line_color(*GUI_VALS.get("SYNTH_MIDDLE_BLUE").unwrap())
                            .set_text_size(TEXT_SIZE);
                            ui.add(grain_crossfade_1_knob);
                        });

                        ui.vertical(|ui| {
                            let grain_hold_1_knob = ui_knob::ArcKnob::for_param(
                                &params.grain_hold_1,
                                setter,
                                KNOB_SIZE,
                            )
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                            .set_line_color(*GUI_VALS.get("SYNTH_MIDDLE_BLUE").unwrap())
                            .set_text_size(TEXT_SIZE);
                            ui.add(grain_hold_1_knob);

                            let grain_gap_1_knob =
                                ui_knob::ArcKnob::for_param(&params.grain_gap_1, setter, KNOB_SIZE)
                                    .preset_style(ui_knob::KnobStyle::NewPresets1)
                                    .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                                    .set_line_color(*GUI_VALS.get("SYNTH_MIDDLE_BLUE").unwrap())
                                    .set_text_size(TEXT_SIZE);
                            ui.add(grain_gap_1_knob);
                        });

                        ui.vertical(|ui| {
                            let start_position_1_knob = ui_knob::ArcKnob::for_param(
                                &params.start_position_1,
                                setter,
                                KNOB_SIZE,
                            )
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                            .set_line_color(*GUI_VALS.get("SYNTH_MIDDLE_BLUE").unwrap())
                            .set_text_size(TEXT_SIZE);
                            ui.add(start_position_1_knob);

                            let end_position_1_knob = ui_knob::ArcKnob::for_param(
                                &params.end_position_1,
                                setter,
                                KNOB_SIZE,
                            )
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                            .set_line_color(*GUI_VALS.get("SYNTH_MIDDLE_BLUE").unwrap())
                            .set_text_size(TEXT_SIZE);
                            ui.add(end_position_1_knob);
                        });
                        // ADSR
                        ui.add(
                            VerticalParamSlider::for_param(&params.osc_1_attack, setter)
                                .with_width(VERT_BAR_WIDTH)
                                .with_height(VERT_BAR_HEIGHT_SHORTENED)
                                .set_reversed(true)
                                .override_colors(
                                    *GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap(),
                                    *GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap(),
                                ),
                        );
                        ui.add(
                            VerticalParamSlider::for_param(&params.osc_1_decay, setter)
                                .with_width(VERT_BAR_WIDTH)
                                .with_height(VERT_BAR_HEIGHT_SHORTENED)
                                .set_reversed(true)
                                .override_colors(
                                    *GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap(),
                                    *GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap(),
                                ),
                        );
                        ui.add(
                            VerticalParamSlider::for_param(&params.osc_1_sustain, setter)
                                .with_width(VERT_BAR_WIDTH)
                                .with_height(VERT_BAR_HEIGHT_SHORTENED)
                                .set_reversed(true)
                                .override_colors(
                                    *GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap(),
                                    *GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap(),
                                ),
                        );
                        ui.add(
                            VerticalParamSlider::for_param(&params.osc_1_release, setter)
                                .with_width(VERT_BAR_WIDTH)
                                .with_height(VERT_BAR_HEIGHT_SHORTENED)
                                .set_reversed(true)
                                .override_colors(
                                    *GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap(),
                                    *GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap(),
                                ),
                        );

                        // Curve sliders
                        ui.vertical(|ui| {
                            ui.add(
                                HorizontalParamSlider::for_param(&params.osc_1_atk_curve, setter)
                                    .with_width(HCURVE_BWIDTH)
                                    .set_left_sided_label(true)
                                    .set_label_width(HCURVE_WIDTH),
                            );
                            ui.add(
                                HorizontalParamSlider::for_param(&params.osc_1_dec_curve, setter)
                                    .with_width(HCURVE_BWIDTH)
                                    .set_left_sided_label(true)
                                    .set_label_width(HCURVE_WIDTH),
                            );
                            ui.add(
                                HorizontalParamSlider::for_param(&params.osc_1_rel_curve, setter)
                                    .with_width(HCURVE_BWIDTH)
                                    .set_left_sided_label(true)
                                    .set_label_width(HCURVE_WIDTH),
                            );
                        });
                    });
                });
            }
        }

        ui.separator();

        /////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
        // Spot Two (2)
        /////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

        match params._audio_module_2_type.value() {
            AudioModuleType::Off => {
                // Blank space
                ui.label("Disabled");
            }
            AudioModuleType::Osc => {
                const KNOB_SIZE: f32 = 30.0;
                const TEXT_SIZE: f32 = 12.0;
                // Oscillator
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            let osc_2_type_knob =
                                ui_knob::ArcKnob::for_param(&params.osc_2_type, setter, KNOB_SIZE)
                                    .preset_style(ui_knob::KnobStyle::NewPresets1)
                                    .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                                    .set_line_color(*GUI_VALS.get("SYNTH_MIDDLE_BLUE").unwrap())
                                    .set_text_size(TEXT_SIZE);
                            ui.add(osc_2_type_knob);

                            let osc_2_retrigger_knob = ui_knob::ArcKnob::for_param(
                                &params.osc_2_retrigger,
                                setter,
                                KNOB_SIZE,
                            )
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .use_outline(true)
                            .set_text_size(TEXT_SIZE);
                            ui.add(osc_2_retrigger_knob);
                        });

                        ui.vertical(|ui| {
                            let osc_2_octave_knob = ui_knob::ArcKnob::for_param(
                                &params.osc_2_octave,
                                setter,
                                KNOB_SIZE,
                            )
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .use_outline(true)
                            .set_text_size(TEXT_SIZE);
                            ui.add(osc_2_octave_knob);

                            let osc_2_semitones_knob = ui_knob::ArcKnob::for_param(
                                &params.osc_2_semitones,
                                setter,
                                KNOB_SIZE,
                            )
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .use_outline(true)
                            .set_text_size(TEXT_SIZE);
                            ui.add(osc_2_semitones_knob);
                        });

                        ui.vertical(|ui| {
                            let osc_2_stereo_knob = ui_knob::ArcKnob::for_param(
                                &params.osc_2_stereo,
                                setter,
                                KNOB_SIZE,
                            )
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .use_outline(true)
                            .set_text_size(TEXT_SIZE);
                            ui.add(osc_2_stereo_knob);

                            let osc_2_unison_knob = ui_knob::ArcKnob::for_param(
                                &params.osc_2_unison,
                                setter,
                                KNOB_SIZE,
                            )
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .use_outline(true)
                            .set_text_size(TEXT_SIZE);
                            ui.add(osc_2_unison_knob);
                        });

                        ui.vertical(|ui| {
                            let osc_2_detune_knob = ui_knob::ArcKnob::for_param(
                                &params.osc_2_detune,
                                setter,
                                KNOB_SIZE,
                            )
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .use_outline(true)
                            .set_text_size(TEXT_SIZE);
                            ui.add(osc_2_detune_knob);

                            let osc_2_unison_detune_knob = ui_knob::ArcKnob::for_param(
                                &params.osc_2_unison_detune,
                                setter,
                                KNOB_SIZE,
                            )
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .use_outline(true)
                            .set_text_size(TEXT_SIZE);
                            ui.add(osc_2_unison_detune_knob);
                        });

                        // ADSR
                        ui.add(
                            VerticalParamSlider::for_param(&params.osc_2_attack, setter)
                                .with_width(VERT_BAR_WIDTH)
                                .with_height(VERT_BAR_HEIGHT)
                                .set_reversed(true)
                                .override_colors(
                                    *GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap(),
                                    *GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap(),
                                ),
                        );
                        ui.add(
                            VerticalParamSlider::for_param(&params.osc_2_decay, setter)
                                .with_width(VERT_BAR_WIDTH)
                                .with_height(VERT_BAR_HEIGHT)
                                .set_reversed(true)
                                .override_colors(
                                    *GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap(),
                                    *GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap(),
                                ),
                        );
                        ui.add(
                            VerticalParamSlider::for_param(&params.osc_2_sustain, setter)
                                .with_width(VERT_BAR_WIDTH)
                                .with_height(VERT_BAR_HEIGHT)
                                .set_reversed(true)
                                .override_colors(
                                    *GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap(),
                                    *GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap(),
                                ),
                        );
                        ui.add(
                            VerticalParamSlider::for_param(&params.osc_2_release, setter)
                                .with_width(VERT_BAR_WIDTH)
                                .with_height(VERT_BAR_HEIGHT)
                                .set_reversed(true)
                                .override_colors(
                                    *GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap(),
                                    *GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap(),
                                ),
                        );

                        // Curve sliders
                        ui.vertical(|ui| {
                            ui.add(
                                HorizontalParamSlider::for_param(&params.osc_2_atk_curve, setter)
                                    .with_width(HCURVE_BWIDTH)
                                    .set_left_sided_label(true)
                                    .set_label_width(HCURVE_WIDTH),
                            );
                            ui.add(
                                HorizontalParamSlider::for_param(&params.osc_2_dec_curve, setter)
                                    .with_width(HCURVE_BWIDTH)
                                    .set_left_sided_label(true)
                                    .set_label_width(HCURVE_WIDTH),
                            );
                            ui.add(
                                HorizontalParamSlider::for_param(&params.osc_2_rel_curve, setter)
                                    .with_width(HCURVE_BWIDTH)
                                    .set_left_sided_label(true)
                                    .set_label_width(HCURVE_WIDTH),
                            );
                        });
                    });
                });
            }
            AudioModuleType::Sampler => {
                const KNOB_SIZE: f32 = 25.0;
                const TEXT_SIZE: f32 = 11.0;
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Load Sample")
                        .font(SMALLER_FONT)
                        .color(*GUI_VALS.get("FONT_COLOR").unwrap()))
                        .on_hover_text("The longer the sample the longer the process!");
                        let switch_toggle = toggle_switch::ToggleSwitch::for_param(&params.load_sample_2, setter);
                        ui.add(switch_toggle);

                        ui.label(RichText::new("Resample")
                        .font(SMALLER_FONT)
                        .color(*GUI_VALS.get("FONT_COLOR").unwrap()))
                        .on_hover_text("Reload your sample after changing this! On: Resample, Off: Repitch");
                        let stretch_toggle = toggle_switch::ToggleSwitch::for_param(&params.restretch_2, setter);
                        ui.add(stretch_toggle);

                        ui.label(RichText::new("Looping")
                        .font(SMALLER_FONT)
                        .color(*GUI_VALS.get("FONT_COLOR").unwrap()))
                        .on_hover_text("To repeat your sample if MIDI key is held");
                        let loop_toggle = toggle_switch::ToggleSwitch::for_param(&params.loop_sample_2, setter);
                        ui.add(loop_toggle);

                        ui.label(RichText::new("Single Cycle")
                        .font(SMALLER_FONT)
                        .color(*GUI_VALS.get("FONT_COLOR").unwrap()))
                        .on_hover_text("Use this with Looping ON and Resample ON if you loaded a single cycle waveform");
                        let sc_toggle = toggle_switch::ToggleSwitch::for_param(&params.single_cycle_2, setter);
                        ui.add(sc_toggle);
                    });
                    ui.horizontal(|ui|{
                        ui.vertical(|ui| {
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
                        });
                        ui.vertical(|ui| {
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
                        });
                        ui.vertical(|ui|{
                            let start_position_2_knob = ui_knob::ArcKnob::for_param(
                                &params.start_position_2,
                                setter,
                                KNOB_SIZE)
                                .preset_style(ui_knob::KnobStyle::NewPresets1)
                                .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                                .set_line_color(*GUI_VALS.get("SYNTH_MIDDLE_BLUE").unwrap())
                                .set_text_size(TEXT_SIZE);
                            ui.add(start_position_2_knob);
                            let end_position_2_knob = ui_knob::ArcKnob::for_param(
                                &params.end_position_2,
                                setter,
                                KNOB_SIZE)
                                .preset_style(ui_knob::KnobStyle::NewPresets1)
                                .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                                .set_line_color(*GUI_VALS.get("SYNTH_MIDDLE_BLUE").unwrap())
                                .set_text_size(TEXT_SIZE);
                            ui.add(end_position_2_knob);
                        });
                        // ADSR
                        ui.add(VerticalParamSlider::for_param(&params.osc_2_attack, setter).with_width(VERT_BAR_WIDTH).with_height(VERT_BAR_HEIGHT_SHORTENED).set_reversed(true).override_colors(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap(), *GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap()));
                        ui.add(VerticalParamSlider::for_param(&params.osc_2_decay, setter).with_width(VERT_BAR_WIDTH).with_height(VERT_BAR_HEIGHT_SHORTENED).set_reversed(true).override_colors(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap(), *GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap()));
                        ui.add(VerticalParamSlider::for_param(&params.osc_2_sustain, setter).with_width(VERT_BAR_WIDTH).with_height(VERT_BAR_HEIGHT_SHORTENED).set_reversed(true).override_colors(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap(), *GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap()));
                        ui.add(VerticalParamSlider::for_param(&params.osc_2_release, setter).with_width(VERT_BAR_WIDTH).with_height(VERT_BAR_HEIGHT_SHORTENED).set_reversed(true).override_colors(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap(), *GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap()));

                        // Curve sliders
                        ui.vertical(|ui| {
                            ui.add(HorizontalParamSlider::for_param(&params.osc_2_atk_curve, setter).with_width(HCURVE_BWIDTH).set_left_sided_label(true).set_label_width(HCURVE_WIDTH));
                            ui.add(HorizontalParamSlider::for_param(&params.osc_2_dec_curve, setter).with_width(HCURVE_BWIDTH).set_left_sided_label(true).set_label_width(HCURVE_WIDTH));
                            ui.add(HorizontalParamSlider::for_param(&params.osc_2_rel_curve, setter).with_width(HCURVE_BWIDTH).set_left_sided_label(true).set_label_width(HCURVE_WIDTH));
                        });
                    });
                });
            }
            AudioModuleType::Granulizer => {
                const KNOB_SIZE: f32 = 25.0;
                const TEXT_SIZE: f32 = 11.0;
                // This fixes the granulizer release being longer than a grain itself
                if params.grain_hold_2.value() < params.grain_crossfade_2.value() {
                    setter.set_parameter(&params.grain_crossfade_2, params.grain_hold_2.value());
                }
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("Load Sample")
                                .font(SMALLER_FONT)
                                .color(*GUI_VALS.get("FONT_COLOR").unwrap()),
                        )
                        .on_hover_text("The longer the sample the longer the process!");
                        let switch_toggle =
                            toggle_switch::ToggleSwitch::for_param(&params.load_sample_2, setter);
                        ui.add(switch_toggle);

                        ui.label(
                            RichText::new("Looping")
                                .font(SMALLER_FONT)
                                .color(*GUI_VALS.get("FONT_COLOR").unwrap()),
                        )
                        .on_hover_text("To repeat your sample if MIDI key is held");
                        let loop_toggle =
                            toggle_switch::ToggleSwitch::for_param(&params.loop_sample_2, setter);
                        ui.add(loop_toggle);

                        ui.add_space(30.0);
                        ui.label(
                            RichText::new("Note: ADSR is per note, Shape is AR per grain")
                                .font(SMALLER_FONT)
                                .color(*GUI_VALS.get("FONT_COLOR").unwrap()),
                        )
                        .on_hover_text("ADSR is per note, Shape is AR per grain");
                    });
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            let osc_2_octave_knob = ui_knob::ArcKnob::for_param(
                                &params.osc_2_octave,
                                setter,
                                KNOB_SIZE,
                            )
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .use_outline(true)
                            .set_text_size(TEXT_SIZE);
                            ui.add(osc_2_octave_knob);

                            let osc_2_semitones_knob = ui_knob::ArcKnob::for_param(
                                &params.osc_2_semitones,
                                setter,
                                KNOB_SIZE,
                            )
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .use_outline(true)
                            .set_text_size(TEXT_SIZE);
                            ui.add(osc_2_semitones_knob);
                        });

                        ui.vertical(|ui| {
                            let osc_2_retrigger_knob = ui_knob::ArcKnob::for_param(
                                &params.osc_2_retrigger,
                                setter,
                                KNOB_SIZE,
                            )
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .use_outline(true)
                            .set_text_size(TEXT_SIZE);
                            ui.add(osc_2_retrigger_knob);

                            let grain_crossfade_2_knob = ui_knob::ArcKnob::for_param(
                                &params.grain_crossfade_2,
                                setter,
                                KNOB_SIZE,
                            )
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                            .set_line_color(*GUI_VALS.get("SYNTH_MIDDLE_BLUE").unwrap())
                            .set_text_size(TEXT_SIZE);
                            ui.add(grain_crossfade_2_knob);
                        });

                        ui.vertical(|ui| {
                            let grain_hold_2_knob = ui_knob::ArcKnob::for_param(
                                &params.grain_hold_2,
                                setter,
                                KNOB_SIZE,
                            )
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                            .set_line_color(*GUI_VALS.get("SYNTH_MIDDLE_BLUE").unwrap())
                            .set_text_size(TEXT_SIZE);
                            ui.add(grain_hold_2_knob);

                            let grain_gap_2_knob =
                                ui_knob::ArcKnob::for_param(&params.grain_gap_2, setter, KNOB_SIZE)
                                    .preset_style(ui_knob::KnobStyle::NewPresets1)
                                    .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                                    .set_line_color(*GUI_VALS.get("SYNTH_MIDDLE_BLUE").unwrap())
                                    .set_text_size(TEXT_SIZE);
                            ui.add(grain_gap_2_knob);
                        });

                        ui.vertical(|ui| {
                            let start_position_2_knob = ui_knob::ArcKnob::for_param(
                                &params.start_position_2,
                                setter,
                                KNOB_SIZE,
                            )
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                            .set_line_color(*GUI_VALS.get("SYNTH_MIDDLE_BLUE").unwrap())
                            .set_text_size(TEXT_SIZE);
                            ui.add(start_position_2_knob);

                            let end_position_2_knob = ui_knob::ArcKnob::for_param(
                                &params.end_position_2,
                                setter,
                                KNOB_SIZE,
                            )
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                            .set_line_color(*GUI_VALS.get("SYNTH_MIDDLE_BLUE").unwrap())
                            .set_text_size(TEXT_SIZE);
                            ui.add(end_position_2_knob);
                        });
                        // ADSR
                        ui.add(
                            VerticalParamSlider::for_param(&params.osc_2_attack, setter)
                                .with_width(VERT_BAR_WIDTH)
                                .with_height(VERT_BAR_HEIGHT_SHORTENED)
                                .set_reversed(true)
                                .override_colors(
                                    *GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap(),
                                    *GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap(),
                                ),
                        );
                        ui.add(
                            VerticalParamSlider::for_param(&params.osc_2_decay, setter)
                                .with_width(VERT_BAR_WIDTH)
                                .with_height(VERT_BAR_HEIGHT_SHORTENED)
                                .set_reversed(true)
                                .override_colors(
                                    *GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap(),
                                    *GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap(),
                                ),
                        );
                        ui.add(
                            VerticalParamSlider::for_param(&params.osc_2_sustain, setter)
                                .with_width(VERT_BAR_WIDTH)
                                .with_height(VERT_BAR_HEIGHT_SHORTENED)
                                .set_reversed(true)
                                .override_colors(
                                    *GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap(),
                                    *GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap(),
                                ),
                        );
                        ui.add(
                            VerticalParamSlider::for_param(&params.osc_2_release, setter)
                                .with_width(VERT_BAR_WIDTH)
                                .with_height(VERT_BAR_HEIGHT_SHORTENED)
                                .set_reversed(true)
                                .override_colors(
                                    *GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap(),
                                    *GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap(),
                                ),
                        );

                        // Curve sliders
                        ui.vertical(|ui| {
                            ui.add(
                                HorizontalParamSlider::for_param(&params.osc_2_atk_curve, setter)
                                    .with_width(HCURVE_BWIDTH)
                                    .set_left_sided_label(true)
                                    .set_label_width(HCURVE_WIDTH),
                            );
                            ui.add(
                                HorizontalParamSlider::for_param(&params.osc_2_dec_curve, setter)
                                    .with_width(HCURVE_BWIDTH)
                                    .set_left_sided_label(true)
                                    .set_label_width(HCURVE_WIDTH),
                            );
                            ui.add(
                                HorizontalParamSlider::for_param(&params.osc_2_rel_curve, setter)
                                    .with_width(HCURVE_BWIDTH)
                                    .set_left_sided_label(true)
                                    .set_label_width(HCURVE_WIDTH),
                            );
                        });
                    });
                });
            }
        }

        ui.separator();

        /////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
        // Spot Three (3)
        /////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

        match params._audio_module_3_type.value() {
            AudioModuleType::Off => {
                // Blank space
                ui.label("Disabled");
            }
            AudioModuleType::Osc => {
                const KNOB_SIZE: f32 = 30.0;
                const TEXT_SIZE: f32 = 12.0;
                // Oscillator
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            let osc_3_type_knob =
                                ui_knob::ArcKnob::for_param(&params.osc_3_type, setter, KNOB_SIZE)
                                    .preset_style(ui_knob::KnobStyle::NewPresets1)
                                    .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                                    .set_line_color(*GUI_VALS.get("SYNTH_MIDDLE_BLUE").unwrap())
                                    .set_text_size(TEXT_SIZE);
                            ui.add(osc_3_type_knob);

                            let osc_3_retrigger_knob = ui_knob::ArcKnob::for_param(
                                &params.osc_3_retrigger,
                                setter,
                                KNOB_SIZE,
                            )
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .use_outline(true)
                            .set_text_size(TEXT_SIZE);
                            ui.add(osc_3_retrigger_knob);
                        });

                        ui.vertical(|ui| {
                            let osc_3_octave_knob = ui_knob::ArcKnob::for_param(
                                &params.osc_3_octave,
                                setter,
                                KNOB_SIZE,
                            )
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .use_outline(true)
                            .set_text_size(TEXT_SIZE);
                            ui.add(osc_3_octave_knob);

                            let osc_3_semitones_knob = ui_knob::ArcKnob::for_param(
                                &params.osc_3_semitones,
                                setter,
                                KNOB_SIZE,
                            )
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .use_outline(true)
                            .set_text_size(TEXT_SIZE);
                            ui.add(osc_3_semitones_knob);
                        });

                        ui.vertical(|ui| {
                            let osc_3_stereo_knob = ui_knob::ArcKnob::for_param(
                                &params.osc_3_stereo,
                                setter,
                                KNOB_SIZE,
                            )
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .use_outline(true)
                            .set_text_size(TEXT_SIZE);
                            ui.add(osc_3_stereo_knob);

                            let osc_3_unison_knob = ui_knob::ArcKnob::for_param(
                                &params.osc_3_unison,
                                setter,
                                KNOB_SIZE,
                            )
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .use_outline(true)
                            .set_text_size(TEXT_SIZE);
                            ui.add(osc_3_unison_knob);
                        });

                        ui.vertical(|ui| {
                            let osc_3_detune_knob = ui_knob::ArcKnob::for_param(
                                &params.osc_3_detune,
                                setter,
                                KNOB_SIZE,
                            )
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .use_outline(true)
                            .set_text_size(TEXT_SIZE);
                            ui.add(osc_3_detune_knob);

                            let osc_3_unison_detune_knob = ui_knob::ArcKnob::for_param(
                                &params.osc_3_unison_detune,
                                setter,
                                KNOB_SIZE,
                            )
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .use_outline(true)
                            .set_text_size(TEXT_SIZE);
                            ui.add(osc_3_unison_detune_knob);
                        });

                        // ADSR
                        ui.add(
                            VerticalParamSlider::for_param(&params.osc_3_attack, setter)
                                .with_width(VERT_BAR_WIDTH)
                                .with_height(VERT_BAR_HEIGHT)
                                .set_reversed(true)
                                .override_colors(
                                    *GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap(),
                                    *GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap(),
                                ),
                        );
                        ui.add(
                            VerticalParamSlider::for_param(&params.osc_3_decay, setter)
                                .with_width(VERT_BAR_WIDTH)
                                .with_height(VERT_BAR_HEIGHT)
                                .set_reversed(true)
                                .override_colors(
                                    *GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap(),
                                    *GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap(),
                                ),
                        );
                        ui.add(
                            VerticalParamSlider::for_param(&params.osc_3_sustain, setter)
                                .with_width(VERT_BAR_WIDTH)
                                .with_height(VERT_BAR_HEIGHT)
                                .set_reversed(true)
                                .override_colors(
                                    *GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap(),
                                    *GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap(),
                                ),
                        );
                        ui.add(
                            VerticalParamSlider::for_param(&params.osc_3_release, setter)
                                .with_width(VERT_BAR_WIDTH)
                                .with_height(VERT_BAR_HEIGHT)
                                .set_reversed(true)
                                .override_colors(
                                    *GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap(),
                                    *GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap(),
                                ),
                        );

                        // Curve sliders
                        ui.vertical(|ui| {
                            ui.add(
                                HorizontalParamSlider::for_param(&params.osc_3_atk_curve, setter)
                                    .with_width(HCURVE_BWIDTH)
                                    .set_left_sided_label(true)
                                    .set_label_width(HCURVE_WIDTH),
                            );
                            ui.add(
                                HorizontalParamSlider::for_param(&params.osc_3_dec_curve, setter)
                                    .with_width(HCURVE_BWIDTH)
                                    .set_left_sided_label(true)
                                    .set_label_width(HCURVE_WIDTH),
                            );
                            ui.add(
                                HorizontalParamSlider::for_param(&params.osc_3_rel_curve, setter)
                                    .with_width(HCURVE_BWIDTH)
                                    .set_left_sided_label(true)
                                    .set_label_width(HCURVE_WIDTH),
                            );
                        });
                    });
                });
            }
            AudioModuleType::Sampler => {
                const KNOB_SIZE: f32 = 25.0;
                const TEXT_SIZE: f32 = 11.0;
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Load Sample")
                        .font(SMALLER_FONT)
                        .color(*GUI_VALS.get("FONT_COLOR").unwrap()))
                        .on_hover_text("The longer the sample the longer the process!");
                        let switch_toggle = toggle_switch::ToggleSwitch::for_param(&params.load_sample_3, setter);
                        ui.add(switch_toggle);

                        ui.label(RichText::new("Resample")
                        .font(SMALLER_FONT)
                        .color(*GUI_VALS.get("FONT_COLOR").unwrap()))
                        .on_hover_text("Reload your sample after changing this! On: Resample, Off: Repitch");
                        let stretch_toggle = toggle_switch::ToggleSwitch::for_param(&params.restretch_3, setter);
                        ui.add(stretch_toggle);

                        ui.label(RichText::new("Looping")
                        .font(SMALLER_FONT)
                        .color(*GUI_VALS.get("FONT_COLOR").unwrap()))
                        .on_hover_text("To repeat your sample if MIDI key is held");
                        let loop_toggle = toggle_switch::ToggleSwitch::for_param(&params.loop_sample_3, setter);
                        ui.add(loop_toggle);

                        ui.label(RichText::new("Single Cycle")
                        .font(SMALLER_FONT)
                        .color(*GUI_VALS.get("FONT_COLOR").unwrap()))
                        .on_hover_text("Use this with Looping ON and Resample ON if you loaded a single cycle waveform");
                        let sc_toggle = toggle_switch::ToggleSwitch::for_param(&params.single_cycle_3, setter);
                        ui.add(sc_toggle);
                    });
                    ui.horizontal(|ui|{
                        ui.vertical(|ui| {
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
                        });
                        ui.vertical(|ui| {
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
                        });
                        ui.vertical(|ui|{
                            let start_position_3_knob = ui_knob::ArcKnob::for_param(
                                &params.start_position_3,
                                setter,
                                KNOB_SIZE)
                                .preset_style(ui_knob::KnobStyle::NewPresets1)
                                .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                                .set_line_color(*GUI_VALS.get("SYNTH_MIDDLE_BLUE").unwrap())
                                .set_text_size(TEXT_SIZE);
                            ui.add(start_position_3_knob);
                            let end_position_3_knob = ui_knob::ArcKnob::for_param(
                                &params.end_position_3,
                                setter,
                                KNOB_SIZE)
                                .preset_style(ui_knob::KnobStyle::NewPresets1)
                                .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                                .set_line_color(*GUI_VALS.get("SYNTH_MIDDLE_BLUE").unwrap())
                                .set_text_size(TEXT_SIZE);
                            ui.add(end_position_3_knob);
                        });
                        // ADSR
                        ui.add(VerticalParamSlider::for_param(&params.osc_3_attack, setter).with_width(VERT_BAR_WIDTH).with_height(VERT_BAR_HEIGHT_SHORTENED).set_reversed(true).override_colors(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap(), *GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap()));
                        ui.add(VerticalParamSlider::for_param(&params.osc_3_decay, setter).with_width(VERT_BAR_WIDTH).with_height(VERT_BAR_HEIGHT_SHORTENED).set_reversed(true).override_colors(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap(), *GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap()));
                        ui.add(VerticalParamSlider::for_param(&params.osc_3_sustain, setter).with_width(VERT_BAR_WIDTH).with_height(VERT_BAR_HEIGHT_SHORTENED).set_reversed(true).override_colors(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap(), *GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap()));
                        ui.add(VerticalParamSlider::for_param(&params.osc_3_release, setter).with_width(VERT_BAR_WIDTH).with_height(VERT_BAR_HEIGHT_SHORTENED).set_reversed(true).override_colors(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap(), *GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap()));

                        // Curve sliders
                        ui.vertical(|ui| {
                            ui.add(HorizontalParamSlider::for_param(&params.osc_3_atk_curve, setter).with_width(HCURVE_BWIDTH).set_left_sided_label(true).set_label_width(HCURVE_WIDTH));
                            ui.add(HorizontalParamSlider::for_param(&params.osc_3_dec_curve, setter).with_width(HCURVE_BWIDTH).set_left_sided_label(true).set_label_width(HCURVE_WIDTH));
                            ui.add(HorizontalParamSlider::for_param(&params.osc_3_rel_curve, setter).with_width(HCURVE_BWIDTH).set_left_sided_label(true).set_label_width(HCURVE_WIDTH));
                        });
                    });
                });
            }
            AudioModuleType::Granulizer => {
                const KNOB_SIZE: f32 = 25.0;
                const TEXT_SIZE: f32 = 11.0;
                // This fixes the granulizer release being longer than a grain itself
                if params.grain_hold_3.value() < params.grain_crossfade_3.value() {
                    setter.set_parameter(&params.grain_crossfade_3, params.grain_hold_3.value());
                }
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("Load Sample")
                                .font(SMALLER_FONT)
                                .color(*GUI_VALS.get("FONT_COLOR").unwrap()),
                        )
                        .on_hover_text("The longer the sample the longer the process!");
                        let switch_toggle =
                            toggle_switch::ToggleSwitch::for_param(&params.load_sample_3, setter);
                        ui.add(switch_toggle);

                        ui.label(
                            RichText::new("Looping")
                                .font(SMALLER_FONT)
                                .color(*GUI_VALS.get("FONT_COLOR").unwrap()),
                        )
                        .on_hover_text("To repeat your sample if MIDI key is held");
                        let loop_toggle =
                            toggle_switch::ToggleSwitch::for_param(&params.loop_sample_3, setter);
                        ui.add(loop_toggle);

                        ui.add_space(30.0);
                        ui.label(
                            RichText::new("Note: ADSR is per note, Shape is AR per grain")
                                .font(SMALLER_FONT)
                                .color(*GUI_VALS.get("FONT_COLOR").unwrap()),
                        )
                        .on_hover_text("ADSR is per note, Shape is AR per grain");
                    });
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            let osc_3_octave_knob = ui_knob::ArcKnob::for_param(
                                &params.osc_3_octave,
                                setter,
                                KNOB_SIZE,
                            )
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .use_outline(true)
                            .set_text_size(TEXT_SIZE);
                            ui.add(osc_3_octave_knob);

                            let osc_3_semitones_knob = ui_knob::ArcKnob::for_param(
                                &params.osc_3_semitones,
                                setter,
                                KNOB_SIZE,
                            )
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .use_outline(true)
                            .set_text_size(TEXT_SIZE);
                            ui.add(osc_3_semitones_knob);
                        });

                        ui.vertical(|ui| {
                            let osc_3_retrigger_knob = ui_knob::ArcKnob::for_param(
                                &params.osc_3_retrigger,
                                setter,
                                KNOB_SIZE,
                            )
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                            .use_outline(true)
                            .set_text_size(TEXT_SIZE);
                            ui.add(osc_3_retrigger_knob);

                            let grain_crossfade_3_knob = ui_knob::ArcKnob::for_param(
                                &params.grain_crossfade_3,
                                setter,
                                KNOB_SIZE,
                            )
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                            .set_line_color(*GUI_VALS.get("SYNTH_MIDDLE_BLUE").unwrap())
                            .set_text_size(TEXT_SIZE);
                            ui.add(grain_crossfade_3_knob);
                        });

                        ui.vertical(|ui| {
                            let grain_hold_3_knob = ui_knob::ArcKnob::for_param(
                                &params.grain_hold_3,
                                setter,
                                KNOB_SIZE,
                            )
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                            .set_line_color(*GUI_VALS.get("SYNTH_MIDDLE_BLUE").unwrap())
                            .set_text_size(TEXT_SIZE);
                            ui.add(grain_hold_3_knob);

                            let grain_gap_3_knob =
                                ui_knob::ArcKnob::for_param(&params.grain_gap_3, setter, KNOB_SIZE)
                                    .preset_style(ui_knob::KnobStyle::NewPresets1)
                                    .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                                    .set_line_color(*GUI_VALS.get("SYNTH_MIDDLE_BLUE").unwrap())
                                    .set_text_size(TEXT_SIZE);
                            ui.add(grain_gap_3_knob);
                        });

                        ui.vertical(|ui| {
                            let start_position_3_knob = ui_knob::ArcKnob::for_param(
                                &params.start_position_3,
                                setter,
                                KNOB_SIZE,
                            )
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                            .set_line_color(*GUI_VALS.get("SYNTH_MIDDLE_BLUE").unwrap())
                            .set_text_size(TEXT_SIZE);
                            ui.add(start_position_3_knob);

                            let end_position_3_knob = ui_knob::ArcKnob::for_param(
                                &params.end_position_3,
                                setter,
                                KNOB_SIZE,
                            )
                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                            .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                            .set_line_color(*GUI_VALS.get("SYNTH_MIDDLE_BLUE").unwrap())
                            .set_text_size(TEXT_SIZE);
                            ui.add(end_position_3_knob);
                        });
                        // ADSR
                        ui.add(
                            VerticalParamSlider::for_param(&params.osc_3_attack, setter)
                                .with_width(VERT_BAR_WIDTH)
                                .with_height(VERT_BAR_HEIGHT_SHORTENED)
                                .set_reversed(true)
                                .override_colors(
                                    *GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap(),
                                    *GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap(),
                                ),
                        );
                        ui.add(
                            VerticalParamSlider::for_param(&params.osc_3_decay, setter)
                                .with_width(VERT_BAR_WIDTH)
                                .with_height(VERT_BAR_HEIGHT_SHORTENED)
                                .set_reversed(true)
                                .override_colors(
                                    *GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap(),
                                    *GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap(),
                                ),
                        );
                        ui.add(
                            VerticalParamSlider::for_param(&params.osc_3_sustain, setter)
                                .with_width(VERT_BAR_WIDTH)
                                .with_height(VERT_BAR_HEIGHT_SHORTENED)
                                .set_reversed(true)
                                .override_colors(
                                    *GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap(),
                                    *GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap(),
                                ),
                        );
                        ui.add(
                            VerticalParamSlider::for_param(&params.osc_3_release, setter)
                                .with_width(VERT_BAR_WIDTH)
                                .with_height(VERT_BAR_HEIGHT_SHORTENED)
                                .set_reversed(true)
                                .override_colors(
                                    *GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap(),
                                    *GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap(),
                                ),
                        );

                        // Curve sliders
                        ui.vertical(|ui| {
                            ui.add(
                                HorizontalParamSlider::for_param(&params.osc_3_atk_curve, setter)
                                    .with_width(HCURVE_BWIDTH)
                                    .set_left_sided_label(true)
                                    .set_label_width(HCURVE_WIDTH),
                            );
                            ui.add(
                                HorizontalParamSlider::for_param(&params.osc_3_dec_curve, setter)
                                    .with_width(HCURVE_BWIDTH)
                                    .set_left_sided_label(true)
                                    .set_label_width(HCURVE_WIDTH),
                            );
                            ui.add(
                                HorizontalParamSlider::for_param(&params.osc_3_rel_curve, setter)
                                    .with_width(HCURVE_BWIDTH)
                                    .set_left_sided_label(true)
                                    .set_label_width(HCURVE_WIDTH),
                            );
                        });
                    });
                });
            }
        }
    }

    // Index proper params from knobs
    // This lets us have a copy for voices, and also track changes like restretch changing or ADR slopes
    pub fn consume_params(&mut self, params: Arc<ActuateParams>, voice_index: usize) {
        match voice_index {
            1 => {
                self.audio_module_type = params._audio_module_1_type.value();
                self.osc_type = params.osc_1_type.value();
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
            }
            2 => {
                self.audio_module_type = params._audio_module_2_type.value();
                self.osc_type = params.osc_2_type.value();
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
            }
            3 => {
                self.audio_module_type = params._audio_module_3_type.value();
                self.osc_type = params.osc_3_type.value();
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
            }
            _ => {}
        }
    }

    // I was looking at the PolyModSynth Example and decided on this
    // Handle the audio module midi events and regular pricessing
    // This is an INDIVIDUAL instance process unlike the GUI function
    // This sends back the OSC output + note on for filter to reset
    pub fn process(
        &mut self,
        sample_id: usize,
        event_passed: Option<NoteEvent<()>>,
        voice_max: usize,
    ) -> (f32, f32, bool) {
        // If the process is in here the file dialog is not open per lib.rs

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
                    NoteEvent::NoteOn {
                        mut note, velocity, ..
                    } => {
                        // Osc + generic stuff
                        note_on = true;
                        let mut new_phase: f32 = 0.0;

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
                        let base_note = note as f32 + self.osc_detune;
                        let detuned_note: f32 = util::f32_midi_note_to_freq(base_note);

                        // Reset the retrigger on Oscs
                        match self.osc_retrigger {
                            RetriggerStyle::Retrigger => {
                                // Start our phase back at 0
                                new_phase = 0.0;
                            }
                            RetriggerStyle::Random | RetriggerStyle::UniRandom => {
                                match self.audio_module_type {
                                    AudioModuleType::Osc => {
                                        // Get a random phase to use
                                        // Poly solution is to pass the phase to the struct
                                        // instead of the osc alone
                                        let mut rng = rand::thread_rng();
                                        new_phase = rng.gen_range(0.0..1.0);
                                    }
                                    AudioModuleType::Sampler => {
                                        let mut rng = rand::thread_rng();
                                        new_phase = rng.gen_range(
                                            0.0..self.sample_lib[note as usize][0].len() as f32,
                                        );
                                    }
                                    AudioModuleType::Granulizer => {
                                        let mut rng = rand::thread_rng();
                                        new_phase = rng.gen_range(
                                            0.0..self.sample_lib[note as usize][0].len() as f32,
                                        );
                                    }
                                    _ => {}
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
                                        base_note + detune_step * (unison_voice + 1) as f32,
                                    );
                                } else {
                                    unison_notes[unison_voice] = util::f32_midi_note_to_freq(
                                        base_note - detune_step * (unison_voice) as f32,
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
                        };

                        match attack_smoother.style {
                            SmoothingStyle::Logarithmic(_) => {
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
                                        return (0.0, 0.0, false);
                                    }
                                } else {
                                    // Nothing is in our sample library, skip attempting audio output
                                    return (0.0, 0.0, false);
                                }
                            }
                            _ => {
                                // These fields aren't used by Osc, Off, Or Additive
                                scaled_sample_pos = 0;
                                scaled_end_pos = 0;
                            }
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
                        };

                        // Add our voice struct to our voice tracking deque
                        self.playing_voices.voices.push_back(new_voice);

                        // Add unison voices to our voice tracking deque
                        if self.osc_unison > 1 && self.audio_module_type == AudioModuleType::Osc {
                            let unison_even_voices = if self.osc_unison % 2 == 0 {
                                self.osc_unison
                            } else {
                                self.osc_unison - 1
                            };
                            let mut unison_angles = vec![0.0; unison_even_voices as usize];
                            for i in 1..(unison_even_voices + 1) {
                                let voice_angle = self.calculate_panning(i - 1, self.osc_unison);
                                unison_angles[(i - 1) as usize] = voice_angle;
                            }

                            for unison_voice in 0..(self.osc_unison as usize - 1) {
                                let uni_phase = match self.osc_retrigger {
                                    RetriggerStyle::UniRandom => {
                                        let mut rng = rand::thread_rng();
                                        rng.gen_range(0.0..1.0)
                                    }
                                    _ => new_phase,
                                };

                                let new_unison_voice: SingleVoice = SingleVoice {
                                    note: note,
                                    _velocity: velocity,
                                    phase: uni_phase,
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
                                },
                            );

                            if self.osc_unison > 1 && self.audio_module_type == AudioModuleType::Osc
                            {
                                self.unison_voices.voices.resize(
                                    voice_max as usize,
                                    // Insert a dummy "Off" entry when resizing UP
                                    SingleVoice {
                                        note: 0,
                                        _velocity: 0.0,
                                        phase: new_phase,
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
                                    },
                                );
                            }
                        }

                        // Remove any off notes
                        for (i, voice) in self.playing_voices.voices.clone().iter().enumerate() {
                            if voice.state == OscState::Off {
                                self.playing_voices.voices.remove(i);
                            } else if voice.grain_state == GrainState::Releasing
                                && voice.grain_release.steps_left() == 0
                            {
                                self.playing_voices.voices.remove(i);
                            }
                        }
                        if self.audio_module_type == AudioModuleType::Osc {
                            for (i, unison_voice) in
                                self.unison_voices.voices.clone().iter().enumerate()
                            {
                                if unison_voice.state == OscState::Off {
                                    self.unison_voices.voices.remove(i);
                                }
                            }
                        }
                    }
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
                            -2 => shifted_note - 24,
                            -1 => shifted_note - 12,
                            0 => shifted_note,
                            1 => shifted_note + 12,
                            2 => shifted_note + 24,
                            _ => shifted_note,
                        };

                        if self.audio_module_type == AudioModuleType::Osc {
                            // Update the matching unison voices
                            for unison_voice in self.unison_voices.voices.iter_mut() {
                                if unison_voice.note == shifted_note
                                    && unison_voice.state != OscState::Releasing
                                {
                                    // Start our release level from our current gain on the voice
                                    unison_voice.osc_release.reset(unison_voice.amp_current);
                                    // Set our new release target to 0.0 so the note fades
                                    match unison_voice.osc_release.style {
                                        SmoothingStyle::Logarithmic(_) => {
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
                                    SmoothingStyle::Logarithmic(_) => {
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

        // This is a dummy entry
        let mut next_grain: SingleVoice = SingleVoice {
            note: 0,
            _velocity: 0.0,
            phase: 0.0,
            phase_delta: 0.0,
            state: OscState::Off,
            // These get cloned since smoother cannot be copied
            amp_current: 0.0,
            osc_attack: Smoother::new(SmoothingStyle::None),
            osc_decay: Smoother::new(SmoothingStyle::None),
            osc_release: Smoother::new(SmoothingStyle::None),
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
        };
        let mut new_grain: bool = false;

        // Second check for off notes before output to cut down on interating...with iterating
        for (i, voice) in self.playing_voices.voices.clone().iter().enumerate() {
            if voice.state == OscState::Off {
                self.playing_voices.voices.remove(i);
            } else if voice.grain_state == GrainState::Releasing
                && voice.grain_release.steps_left() == 0
            {
                self.playing_voices.voices.remove(i);
            }
        }
        if self.audio_module_type == AudioModuleType::Osc {
            for (i, unison_voice) in self.unison_voices.voices.clone().iter().enumerate() {
                if unison_voice.state == OscState::Off {
                    self.unison_voices.voices.remove(i);
                }
            }
        }

        ////////////////////////////////////////////////////////////
        // Update our voices before output
        ////////////////////////////////////////////////////////////
        for voice in self.playing_voices.voices.iter_mut() {
            if self.audio_module_type == AudioModuleType::Osc
                || self.audio_module_type == AudioModuleType::Sampler
            {
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
                        phase: voice.phase,
                        phase_delta: voice.phase_delta,
                        state: voice.state,
                        // These get cloned since smoother cannot be copied
                        amp_current: voice.amp_current,
                        osc_attack: voice.osc_attack.clone(),
                        osc_decay: voice.osc_decay.clone(),
                        osc_release: voice.osc_release.clone(),
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
            AudioModuleType::Osc | AudioModuleType::Sampler => {
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
            AudioModuleType::Osc => {
                let mut summed_voices_l: f32 = 0.0;
                let mut summed_voices_r: f32 = 0.0;
                let mut stereo_voices_l: f32 = 0.0;
                let mut stereo_voices_r: f32 = 0.0;
                let mut center_voices: f32 = 0.0;
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

                    voice.phase_delta = voice.frequency / self.sample_rate;
                    if self.audio_module_type == AudioModuleType::Osc {
                        center_voices += match self.osc_type {
                            VoiceType::Sine => {
                                Oscillator::get_sine(voice.phase) * temp_osc_gain_multiplier
                            }
                            VoiceType::Tri => {
                                Oscillator::get_tri(voice.phase) * temp_osc_gain_multiplier
                            }
                            VoiceType::Saw => {
                                Oscillator::get_saw(voice.phase) * temp_osc_gain_multiplier
                            }
                            VoiceType::RSaw => {
                                Oscillator::get_rsaw(voice.phase) * temp_osc_gain_multiplier
                            }
                            VoiceType::Ramp => {
                                Oscillator::get_ramp(voice.phase) * temp_osc_gain_multiplier
                            }
                            VoiceType::Square => {
                                Oscillator::get_square(voice.phase) * temp_osc_gain_multiplier
                            }
                            VoiceType::RSquare => {
                                Oscillator::get_rsquare(voice.phase) * temp_osc_gain_multiplier
                            }
                            VoiceType::Pulse => {
                                Oscillator::get_pulse(voice.phase) * temp_osc_gain_multiplier
                            }
                            VoiceType::Noise => {
                                self.noise_obj.generate_sample() * temp_osc_gain_multiplier
                            }
                        };
                    }
                    // This is the Additive scenario
                    else {
                        // Add up the partials
                        let mut partial_sum: f32 = 0.0;
                        partial_sum += Oscillator::calculate_fast_sine(Self::rescale_phase_added(
                            voice.phase,
                            self.add_partial0_phase,
                        )) * self.add_partial0
                            * temp_osc_gain_multiplier;
                        let mut temp_freq_double = voice.frequency * 2.0;
                        let mut temp_phase_delta = temp_freq_double / self.sample_rate;
                        partial_sum += Oscillator::calculate_fast_sine(Self::rescale_phase_added(
                            voice.phase,
                            temp_phase_delta + self.add_partial1_phase,
                        )) * self.add_partial1
                            * temp_osc_gain_multiplier;
                        temp_freq_double *= 2.0;
                        temp_phase_delta = temp_freq_double / self.sample_rate;
                        partial_sum += Oscillator::calculate_fast_sine(Self::rescale_phase_added(
                            voice.phase,
                            temp_phase_delta + self.add_partial2_phase,
                        )) * self.add_partial2
                            * temp_osc_gain_multiplier;
                        center_voices += partial_sum;
                    }
                }
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

                    unison_voice.phase_delta = unison_voice.frequency / self.sample_rate;

                    if self.osc_unison > 1 {
                        let mut temp_unison_voice: f32 = 0.0;
                        if self.audio_module_type == AudioModuleType::Osc {
                            temp_unison_voice = match self.osc_type {
                                VoiceType::Sine => {
                                    Oscillator::get_sine(unison_voice.phase)
                                        * temp_osc_gain_multiplier
                                }
                                VoiceType::Tri => {
                                    Oscillator::get_tri(unison_voice.phase)
                                        * temp_osc_gain_multiplier
                                }
                                VoiceType::Saw => {
                                    Oscillator::get_saw(unison_voice.phase)
                                        * temp_osc_gain_multiplier
                                }
                                VoiceType::RSaw => {
                                    Oscillator::get_rsaw(unison_voice.phase)
                                        * temp_osc_gain_multiplier
                                }
                                VoiceType::Ramp => {
                                    Oscillator::get_ramp(unison_voice.phase)
                                        * temp_osc_gain_multiplier
                                }
                                VoiceType::Square => {
                                    Oscillator::get_square(unison_voice.phase)
                                        * temp_osc_gain_multiplier
                                }
                                VoiceType::RSquare => {
                                    Oscillator::get_rsquare(unison_voice.phase)
                                        * temp_osc_gain_multiplier
                                }
                                VoiceType::Pulse => {
                                    Oscillator::get_pulse(unison_voice.phase)
                                        * temp_osc_gain_multiplier
                                }
                                VoiceType::Noise => {
                                    self.noise_obj.generate_sample() * temp_osc_gain_multiplier
                                }
                            };
                        } else {
                            temp_unison_voice +=
                                Oscillator::calculate_fast_sine(Self::rescale_phase_added(
                                    unison_voice.phase,
                                    self.add_partial0_phase,
                                )) * self.add_partial0
                                    * temp_osc_gain_multiplier;
                        }

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
                let width_coeff = self.osc_stereo * 0.5;
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
                                summed_voices_l +=
                                    NoteVector[0][voice.sample_pos] * temp_osc_gain_multiplier;
                                summed_voices_r +=
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
                (summed_voices_l, summed_voices_r)
            }
            AudioModuleType::Off => {
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

                shifter.shift_pitch(1, translated_i, loaded_left, &mut out_buffer_left);
                shifter.shift_pitch(1, translated_i, loaded_right, &mut out_buffer_right);

                let mut NoteVector = Vec::with_capacity(2);
                NoteVector.insert(0, out_buffer_left);
                NoteVector.insert(1, out_buffer_right);
                self.sample_lib.insert(i, NoteVector);
            }
        }
    }

    fn calculate_panning(&mut self, voice_index: i32, num_voices: i32) -> f32 {
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
                -0.25 * std::f32::consts::PI * sign // First voice panned left
            } else {
                if self.two_voice_stereo_flipper {
                    self.two_voice_stereo_flipper = false;
                } else {
                    self.two_voice_stereo_flipper = true;
                }
                0.25 * std::f32::consts::PI * sign // Second voice panned right
            };
        }

        // Handle the special case for 3 voices.
        if num_voices == 3 {
            return match voice_index {
                0 => {
                    if self.two_voice_stereo_flipper {
                        self.two_voice_stereo_flipper = false;
                    } else {
                        self.two_voice_stereo_flipper = true;
                    }
                    -0.25 * std::f32::consts::PI * sign
                } // First voice panned left
                1 => 0.0,                                // Second voice panned center
                2 => 0.25 * std::f32::consts::PI * sign, // Third voice panned right
                _ => 0.0,                                // Handle other cases gracefully
            };
        }

        // Calculate the pan angle for voices with index 0 and 1.
        let base_angle = ((voice_index / 2) as f32) / ((num_voices / 2) as f32 - 1.0) - 0.5;

        // Determine the final angle based on even or odd index.
        let angle = if voice_index % 2 == 0 {
            -base_angle
        } else {
            base_angle
        };
        if voice_index == 0 {
            if self.two_voice_stereo_flipper {
                self.two_voice_stereo_flipper = false;
            } else {
                self.two_voice_stereo_flipper = true;
            }
        }

        angle * std::f32::consts::PI * sign // Use full scale for other cases
    }

    fn rescale_phase_added(voice_phase: f32, additive_phase: f32) -> f32 {
        let temp = voice_phase + additive_phase;
        let phase = if temp > 1.0 { temp - 1.0 } else { temp };
        phase
    }
}
