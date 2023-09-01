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

Actuate - Synthesizer + Granulizer by Ardura

#####################################
*/
#![allow(non_snake_case)]

use StateVariableFilter::ResonanceType;
use nih_plug_egui::{create_egui_editor, egui::{self, Color32, Rect, Rounding, RichText, FontId, Pos2}, EguiState};
use rubato::Resampler;
use std::{sync::{Arc}, ops::RangeInclusive};
use nih_plug::{prelude::*};
use phf::phf_map;

// My Files
use audio_module::{AudioModuleType, AudioModule, Oscillator::{self, RetriggerStyle}};
use crate::audio_module::Oscillator::VoiceType;
mod audio_module;
mod StateVariableFilter;
mod ui_knob;

pub struct LoadedSample(Vec<Vec<f32>>);

/// The number of simultaneous voices for this synth.
const _NUM_VOICES: usize = 16;

// Plugin sizing
const WIDTH: u32 = 832;
const HEIGHT: u32 = 632;

// GUI values to refer to
pub static GUI_VALS: phf::Map<&'static str, Color32> = phf_map! {
    "A_KNOB_OUTSIDE_COLOR" => Color32::from_rgb(90,81,184),
    "DARK_GREY_UI_COLOR" => Color32::from_rgb(49,53,71),
    "A_BACKGROUND_COLOR_TOP" => Color32::from_rgb(185,186,198),
    "A_BACKGROUND_COLOR_BOTTOM" => Color32::from_rgb(60,60,68),
    "SYNTH_BARS_PURPLE" => Color32::from_rgb(45,41,99),
    "SYNTH_MIDDLE_BLUE" => Color32::from_rgb(98,145,204),
    "FONT_COLOR" => Color32::from_rgb(10,103,210),
};

// Font
const FONT: nih_plug_egui::egui::FontId = FontId::monospace(14.0);

pub struct Actuate {
    pub params: Arc<ActuateParams>,
    pub sample_rate: f32,
    
    // Modules
    audio_module_1: AudioModule,
    _audio_module_1_type: AudioModuleType,
    audio_module_2: AudioModule,
    _audio_module_2_type: AudioModuleType,
    audio_module_3: AudioModule,
    _audio_module_3_type: AudioModuleType,

    // Filter
    filter: StateVariableFilter::StateVariableFilter,
}

impl Default for Actuate {
    fn default() -> Self {
        Self {
            params: Arc::new(Default::default()),
            sample_rate: 44100.0,

            // Module 1
            audio_module_1: AudioModule::default(),
            _audio_module_1_type: AudioModuleType::Osc,
            audio_module_2: AudioModule::default(),
            _audio_module_2_type: AudioModuleType::Off,
            audio_module_3: AudioModule::default(),
            _audio_module_3_type: AudioModuleType::Off,

            // Filter
            filter: StateVariableFilter::StateVariableFilter::default(),
        }
    }
}

/// Plugin parameters struct
#[derive(Params)]
pub struct ActuateParams {
    #[persist = "editor-state"]
    editor_state: Arc<EguiState>,

    #[id = "Master Level"]
    pub master_level: FloatParam,

    // This audio module is what switches between functions for generators in the synth
    #[id = "audio_module_1_type"]
    pub _audio_module_1_type: EnumParam<AudioModuleType>,

    #[id = "audio_module_2_type"]
    pub _audio_module_2_type: EnumParam<AudioModuleType>,

    #[id = "audio_module_3_type"]
    pub _audio_module_3_type: EnumParam<AudioModuleType>,

    // Audio Module Gains
    #[id = "audio_module_1_level"]
    pub audio_module_1_level: FloatParam,

    #[id = "audio_module_2_level"]
    pub audio_module_2_level: FloatParam,
    
    #[id = "audio_module_3_level"]
    pub audio_module_3_level: FloatParam,

    // Controls for when audio_module_1_type is Osc
    #[id = "osc_1_type"]
    pub osc_1_type: EnumParam<VoiceType>,

    #[id = "osc_1_octave"]
    pub osc_1_octave: IntParam,

    #[id = "osc_1_semitones"]
    pub osc_1_semitones: IntParam,

    #[id = "osc_1_detune"]
    pub osc_1_detune: FloatParam,

    #[id = "osc_1_mod_amount"]
    pub osc_1_mod_amount: FloatParam,

    #[id = "osc_1_attack"]
    pub osc_1_attack: FloatParam,

    #[id = "osc_1_decay"]
    pub osc_1_decay: FloatParam,

    #[id = "osc_1_sustain"]
    pub osc_1_sustain: FloatParam,

    #[id = "osc_1_release"]
    pub osc_1_release: FloatParam,

    #[id = "osc_1_retrigger"]
    pub osc_1_retrigger: EnumParam<RetriggerStyle>,

    #[id = "osc_1_atk_curve"]
    pub osc_1_atk_curve: EnumParam<Oscillator::SmoothStyle>,

    #[id = "osc_1_dec_curve"]
    pub osc_1_dec_curve: EnumParam<Oscillator::SmoothStyle>,

    #[id = "osc_1_rel_curve"]
    pub osc_1_rel_curve: EnumParam<Oscillator::SmoothStyle>,

    // Controls for when audio_module_2_type is Osc
    #[id = "osc_2_type"]
    pub osc_2_type: EnumParam<VoiceType>,

    #[id = "osc_2_octave"]
    pub osc_2_octave: IntParam,

    #[id = "osc_2_semitones"]
    pub osc_2_semitones: IntParam,

    #[id = "osc_2_detune"]
    pub osc_2_detune: FloatParam,

    #[id = "osc_2_mod_amount"]
    pub osc_2_mod_amount: FloatParam,

    #[id = "osc_2_attack"]
    pub osc_2_attack: FloatParam,

    #[id = "osc_2_decay"]
    pub osc_2_decay: FloatParam,

    #[id = "osc_2_sustain"]
    pub osc_2_sustain: FloatParam,

    #[id = "osc_2_release"]
    pub osc_2_release: FloatParam,

    #[id = "osc_2_retrigger"]
    pub osc_2_retrigger: EnumParam<RetriggerStyle>,

    #[id = "osc_2_atk_curve"]
    pub osc_2_atk_curve: EnumParam<Oscillator::SmoothStyle>,

    #[id = "osc_2_dec_curve"]
    pub osc_2_dec_curve: EnumParam<Oscillator::SmoothStyle>,

    #[id = "osc_2_rel_curve"]
    pub osc_2_rel_curve: EnumParam<Oscillator::SmoothStyle>,

    // Controls for when audio_module_3_type is Osc
    #[id = "osc_3_type"]
    pub osc_3_type: EnumParam<VoiceType>,

    #[id = "osc_3_octave"]
    pub osc_3_octave: IntParam,

    #[id = "osc_3_semitones"]
    pub osc_3_semitones: IntParam,

    #[id = "osc_3_detune"]
    pub osc_3_detune: FloatParam,

    #[id = "osc_3_mod_amount"]
    pub osc_3_mod_amount: FloatParam,

    #[id = "osc_3_attack"]
    pub osc_3_attack: FloatParam,

    #[id = "osc_3_decay"]
    pub osc_3_decay: FloatParam,

    #[id = "osc_3_sustain"]
    pub osc_3_sustain: FloatParam,

    #[id = "osc_3_release"]
    pub osc_3_release: FloatParam,

    #[id = "osc_3_retrigger"]
    pub osc_3_retrigger: EnumParam<RetriggerStyle>,

    #[id = "osc_3_atk_curve"]
    pub osc_3_atk_curve: EnumParam<Oscillator::SmoothStyle>,

    #[id = "osc_3_dec_curve"]
    pub osc_3_dec_curve: EnumParam<Oscillator::SmoothStyle>,

    #[id = "osc_3_rel_curve"]
    pub osc_3_rel_curve: EnumParam<Oscillator::SmoothStyle>,

    // Controls for when audio_module_1_type is Granulizer

    // Filter
    #[id = "filter_wet"]
    pub filter_wet: FloatParam,

    #[id = "filter_cutoff"]
    pub filter_cutoff: FloatParam,

    #[id = "filter_resonance"]
    pub filter_resonance: FloatParam,

    #[id = "filter_res_type"]
    pub filter_res_type: EnumParam<ResonanceType>,

    #[id = "filter_lp_amount"]
    pub filter_lp_amount: FloatParam,

    #[id = "filter_hp_amount"]
    pub filter_hp_amount: FloatParam,

    #[id = "filter_bp_amount"]
    pub filter_bp_amount: FloatParam,
}

impl Default for ActuateParams {
    fn default() -> Self {
        Self {
            editor_state: EguiState::from_size(WIDTH, HEIGHT),
            master_level: FloatParam::new("Master", 0.7, FloatRange::Linear { min: 0.0, max: 2.0 }).with_value_to_string(formatters::v2s_f32_percentage(0)).with_unit("%"),

            _audio_module_1_type: EnumParam::new("Type", AudioModuleType::Osc),
            _audio_module_2_type: EnumParam::new("Type", AudioModuleType::Off),
            _audio_module_3_type: EnumParam::new("Type", AudioModuleType::Off),

            audio_module_1_level: FloatParam::new("Level", 1.0, FloatRange::Linear { min: 0.0, max: 1.0 }).with_value_to_string(formatters::v2s_f32_percentage(0)).with_unit("%"),
            audio_module_2_level: FloatParam::new("Level", 0.0, FloatRange::Linear { min: 0.0, max: 1.0 }).with_value_to_string(formatters::v2s_f32_percentage(0)).with_unit("%"),
            audio_module_3_level: FloatParam::new("Level", 0.0, FloatRange::Linear { min: 0.0, max: 1.0 }).with_value_to_string(formatters::v2s_f32_percentage(0)).with_unit("%"),

            // Oscillators
            ////////////////////////////////////////////////////////////////////////////////////
            osc_1_type: EnumParam::new("Wave", VoiceType::Sine),
            osc_1_octave: IntParam::new("Octave", 0, IntRange::Linear { min: -2, max: 2 }),
            osc_1_semitones: IntParam::new("Semitones", 0, IntRange::Linear { min: -11, max: 11 }),
            osc_1_detune: FloatParam::new("Detune", 0.0, FloatRange::Linear { min: -0.999, max: 0.999 }),
            osc_1_attack: FloatParam::new("Attack", 0.1, FloatRange::Skewed { min: 0.1, max: 999.9, factor: 0.5 }).with_value_to_string(format_nothing()),
            osc_1_decay: FloatParam::new("Decay", 0.1, FloatRange::Skewed { min: 0.1, max: 999.9, factor: 0.5 }).with_value_to_string(format_nothing()),
            osc_1_sustain: FloatParam::new("Sustain", 999.9, FloatRange::Linear { min: 0.1, max: 999.9 }).with_value_to_string(format_nothing()),
            osc_1_release: FloatParam::new("Release", 5.0, FloatRange::Skewed { min: 0.1, max: 999.9, factor: 0.5 }).with_value_to_string(format_nothing()),
            osc_1_mod_amount: FloatParam::new("Modifier", 0.0, FloatRange::Linear { min: 0.0, max: 1.0 }).with_value_to_string(formatters::v2s_f32_rounded(2)),
            osc_1_retrigger: EnumParam::new("Retrigger", RetriggerStyle::Free),
            osc_1_atk_curve: EnumParam::new("Atk Curve", Oscillator::SmoothStyle::Linear),
            osc_1_dec_curve: EnumParam::new("Dec Curve", Oscillator::SmoothStyle::Linear),
            osc_1_rel_curve: EnumParam::new("Rel Curve", Oscillator::SmoothStyle::Linear),

            osc_2_type: EnumParam::new("Wave", VoiceType::Sine),
            osc_2_octave: IntParam::new("Octave", 0, IntRange::Linear { min: -2, max: 2 }),
            osc_2_semitones: IntParam::new("Semitones", 0, IntRange::Linear { min: -11, max: 11 }),
            osc_2_detune: FloatParam::new("Detune", 0.0, FloatRange::Linear { min: -0.999, max: 0.999 }),
            osc_2_attack: FloatParam::new("Attack", 0.1, FloatRange::Skewed { min: 0.1, max: 999.9, factor: 0.5 }).with_value_to_string(format_nothing()),
            osc_2_decay: FloatParam::new("Decay", 0.1, FloatRange::Skewed { min: 0.1, max: 999.9, factor: 0.5 }).with_value_to_string(format_nothing()),
            osc_2_sustain: FloatParam::new("Sustain", 999.9, FloatRange::Linear { min: 0.1, max: 999.9 }).with_value_to_string(format_nothing()),
            osc_2_release: FloatParam::new("Release", 5.0, FloatRange::Skewed { min: 0.1, max: 999.9, factor: 0.5 }).with_value_to_string(format_nothing()),
            osc_2_mod_amount: FloatParam::new("Modifier", 0.0, FloatRange::Linear { min: 0.0, max: 1.0 }).with_value_to_string(formatters::v2s_f32_rounded(2)),
            osc_2_retrigger: EnumParam::new("Retrigger", RetriggerStyle::Free),
            osc_2_atk_curve: EnumParam::new("Atk Curve", Oscillator::SmoothStyle::Linear),
            osc_2_dec_curve: EnumParam::new("Dec Curve", Oscillator::SmoothStyle::Linear),
            osc_2_rel_curve: EnumParam::new("Rel Curve", Oscillator::SmoothStyle::Linear),

            osc_3_type: EnumParam::new("Wave", VoiceType::Sine),
            osc_3_octave: IntParam::new("Octave", 0, IntRange::Linear { min: -2, max: 2 }),
            osc_3_semitones: IntParam::new("Semitones", 0, IntRange::Linear { min: -11, max: 11 }),
            osc_3_detune: FloatParam::new("Detune", 0.0, FloatRange::Linear { min: -0.999, max: 0.999 }),
            osc_3_attack: FloatParam::new("Attack", 0.1, FloatRange::Skewed { min: 0.1, max: 999.9, factor: 0.5 }).with_value_to_string(format_nothing()),
            osc_3_decay: FloatParam::new("Decay", 0.1, FloatRange::Skewed { min: 0.1, max: 999.9, factor: 0.5 }).with_value_to_string(format_nothing()),
            osc_3_sustain: FloatParam::new("Sustain", 999.9, FloatRange::Linear { min: 0.1, max: 999.9 }).with_value_to_string(format_nothing()),
            osc_3_release: FloatParam::new("Release", 5.0, FloatRange::Skewed { min: 0.1, max: 999.9, factor: 0.5 }).with_value_to_string(format_nothing()),
            osc_3_mod_amount: FloatParam::new("Modifier", 0.0, FloatRange::Linear { min: 0.0, max: 1.0 }).with_value_to_string(formatters::v2s_f32_rounded(2)),
            osc_3_retrigger: EnumParam::new("Retrigger", RetriggerStyle::Free),
            osc_3_atk_curve: EnumParam::new("Atk Curve", Oscillator::SmoothStyle::Linear),
            osc_3_dec_curve: EnumParam::new("Dec Curve", Oscillator::SmoothStyle::Linear),
            osc_3_rel_curve: EnumParam::new("Rel Curve", Oscillator::SmoothStyle::Linear),

            // Filter
            ////////////////////////////////////////////////////////////////////////////////////
            filter_lp_amount: FloatParam::new("Low Pass", 1.0, FloatRange::Linear { min: 0.0, max: 1.0 }).with_value_to_string(formatters::v2s_f32_percentage(0)),
            filter_hp_amount: FloatParam::new("High Pass", 0.0, FloatRange::Linear { min: 0.0, max: 1.0 }).with_value_to_string(formatters::v2s_f32_percentage(0)),
            filter_bp_amount: FloatParam::new("Band Pass", 0.0, FloatRange::Linear { min: 0.0, max: 1.0 }).with_value_to_string(formatters::v2s_f32_percentage(0)),

            filter_wet: FloatParam::new("Filter Wet", 1.0, FloatRange::Linear { min: 0.0, max: 1.0 }).with_unit("%").with_value_to_string(formatters::v2s_f32_percentage(0)),
            filter_resonance: FloatParam::new("Resonance", 0.5, FloatRange::Linear { min: 0.2, max: 1.0 } ).with_unit("%").with_value_to_string(formatters::v2s_f32_percentage(0)),
            filter_res_type: EnumParam::new("Q Type", ResonanceType::Default),
            filter_cutoff: FloatParam::new("Frequency", 2000.0, FloatRange::Skewed { min: 20.0, max: 16000.0, factor: 0.5 }).with_value_to_string(formatters::v2s_f32_rounded(0)).with_unit("Hz"),
        }
    }
}

impl Plugin for Actuate {
    const NAME: &'static str = "Actuate";
    const VENDOR: &'static str = "Ardura";
    const URL: &'static str = "https://github.com/ardura";
    const EMAIL: &'static str = "azviscarra@gmail.com";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");
    const SAMPLE_ACCURATE_AUTOMATION: bool = true;
    const MIDI_INPUT: MidiConfig = MidiConfig::Basic;
    const MIDI_OUTPUT: MidiConfig = MidiConfig::Basic;

    type SysExMessage = ();
    type BackgroundTask = ();

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: None,
        main_output_channels: NonZeroU32::new(2),
        ..AudioIOLayout::const_default()
    }];

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    // This draws our GUI with egui library
    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        let params = self.params.clone();
        create_egui_editor(
            self.params.editor_state.clone(),
            (),
            |_, _| {},
            move |egui_ctx, setter, _state| {
                egui::CentralPanel::default()
                    .show(egui_ctx, |ui| {
                        // Change colors - there's probably a better way to do this
                        let mut style_var = ui.style_mut().clone();

                        // Assign default colors
                        style_var.visuals.widgets.inactive.bg_stroke.color = *GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap();
                        style_var.visuals.widgets.inactive.bg_fill = *GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap();
                        style_var.visuals.widgets.active.fg_stroke.color = *GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap();
                        style_var.visuals.widgets.active.bg_stroke.color = *GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap();
                        style_var.visuals.widgets.open.fg_stroke.color = *GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap();
                        style_var.visuals.widgets.open.bg_fill = *GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap();
                        // Lettering on param sliders
                        style_var.visuals.widgets.inactive.fg_stroke.color = *GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap();
                        // Background of the bar in param sliders
                        style_var.visuals.selection.bg_fill = *GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap();
                        style_var.visuals.selection.stroke.color = *GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap();
                        // Unfilled background of the bar
                        style_var.visuals.widgets.noninteractive.bg_fill = *GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap();

                        // Trying to draw background colors as rects
                        ui.painter().rect_filled(
                            Rect::from_x_y_ranges(
                                RangeInclusive::new(0.0, WIDTH as f32), 
                                RangeInclusive::new(0.0, (HEIGHT as f32)*0.7)), 
                            Rounding::from(16.0), *GUI_VALS.get("A_BACKGROUND_COLOR_TOP").unwrap());
                        ui.painter().rect_filled(
                            Rect::from_x_y_ranges(
                                RangeInclusive::new(0.0, WIDTH as f32), 
                                RangeInclusive::new((HEIGHT as f32)*0.7, HEIGHT as f32)), 
                            Rounding::from(16.0), *GUI_VALS.get("A_BACKGROUND_COLOR_BOTTOM").unwrap());

                        ui.set_style(style_var);

                        ui.horizontal(|ui| {
                            // Synth Bars on left and right
                            let synth_bar_space = 40.0;
                            ui.painter().rect_filled(
                                Rect::from_x_y_ranges(
                                    RangeInclusive::new(0.0, synth_bar_space), 
                                    RangeInclusive::new(0.0, HEIGHT as f32)),
                                Rounding::none(),
                                *GUI_VALS.get("SYNTH_BARS_PURPLE").unwrap()
                            );

                            ui.add_space(synth_bar_space);

                            // GUI Structure
                            ui.vertical(|ui| {
                                // Spacing :)
                                ui.label(RichText::new("Synth")
                                    .font(FONT)
                                    .color(*GUI_VALS.get("FONT_COLOR").unwrap()))
                                    .on_hover_text("by Ardura!");
                                ui.separator();
                                const KNOB_SIZE: f32 = 36.0;
                                const TEXT_SIZE: f32 = 13.0;
                                ui.horizontal(|ui|{
                                    ui.vertical(|ui|{
                                        ui.label("Generators");
                                        // Side knobs for types
                                        ui.horizontal(|ui|{
                                            let audio_module_1_knob = ui_knob::ArcKnob::for_param(
                                                &params._audio_module_1_type, 
                                                setter, 
                                                KNOB_SIZE)
                                                .preset_style(ui_knob::KnobStyle::NewPresets1)
                                                .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                                                .set_line_color(*GUI_VALS.get("SYNTH_MIDDLE_BLUE").unwrap())
                                                .set_text_size(TEXT_SIZE);
                                            ui.add(audio_module_1_knob);
                                            let audio_module_1_level_knob = ui_knob::ArcKnob::for_param(
                                                &params.audio_module_1_level, 
                                                setter, 
                                                KNOB_SIZE)
                                                .preset_style(ui_knob::KnobStyle::NewPresets1)
                                                .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                                                .set_line_color(*GUI_VALS.get("SYNTH_MIDDLE_BLUE").unwrap())
                                                .set_text_size(TEXT_SIZE);
                                            ui.add(audio_module_1_level_knob);
                                        });
                                        ui.horizontal(|ui|{
                                            let audio_module_2_knob = ui_knob::ArcKnob::for_param(
                                                &params._audio_module_2_type, 
                                                setter, 
                                                KNOB_SIZE)
                                                .preset_style(ui_knob::KnobStyle::NewPresets1)
                                                .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                                                .set_line_color(*GUI_VALS.get("SYNTH_MIDDLE_BLUE").unwrap())
                                                .set_text_size(TEXT_SIZE);
                                            ui.add(audio_module_2_knob);
                                            let audio_module_2_level_knob = ui_knob::ArcKnob::for_param(
                                                &params.audio_module_2_level, 
                                                setter, 
                                                KNOB_SIZE)
                                                .preset_style(ui_knob::KnobStyle::NewPresets1)
                                                .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                                                .set_line_color(*GUI_VALS.get("SYNTH_MIDDLE_BLUE").unwrap())
                                                .set_text_size(TEXT_SIZE);
                                            ui.add(audio_module_2_level_knob);
                                        });
                                        ui.horizontal(|ui| {
                                            let audio_module_3_knob = ui_knob::ArcKnob::for_param(
                                                &params._audio_module_3_type, 
                                                setter, 
                                                KNOB_SIZE)
                                                .preset_style(ui_knob::KnobStyle::NewPresets1)
                                                .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                                                .set_line_color(*GUI_VALS.get("SYNTH_MIDDLE_BLUE").unwrap())
                                                .set_text_size(TEXT_SIZE);
                                            ui.add(audio_module_3_knob);
                                            let audio_module_3_level_knob = ui_knob::ArcKnob::for_param(
                                                &params.audio_module_3_level, 
                                                setter, 
                                                KNOB_SIZE)
                                                .preset_style(ui_knob::KnobStyle::NewPresets1)
                                                .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                                                .set_line_color(*GUI_VALS.get("SYNTH_MIDDLE_BLUE").unwrap())
                                                .set_text_size(TEXT_SIZE);
                                            ui.add(audio_module_3_level_knob);
                                        });
                                    });
                                    ui.separator();
                                    ui.vertical(|ui|{
                                        ui.label("Generator Controls");
                                        audio_module::AudioModule::draw_modules(ui, params.clone(), setter);
                                    });
                                    ui.separator();
                                });
                                ui.separator();
                                ui.label("Filters");
                                // Filter section
                                
                                ui.vertical(|ui| {
                                    ui.horizontal(|ui| {
                                        let filter_wet_knob = ui_knob::ArcKnob::for_param(
                                            &params.filter_wet, 
                                            setter, 
                                            KNOB_SIZE)
                                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                                            .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                                            .set_text_size(TEXT_SIZE);
                                        ui.add(filter_wet_knob);

                                        let filter_cutoff_knob = ui_knob::ArcKnob::for_param(
                                            &params.filter_cutoff, 
                                            setter, 
                                            KNOB_SIZE)
                                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                                            .set_fill_color(*GUI_VALS.get("SYNTH_BARS_PURPLE").unwrap())
                                            .set_line_color(*GUI_VALS.get("SYNTH_MIDDLE_BLUE").unwrap())
                                            .set_text_size(TEXT_SIZE);
                                        ui.add(filter_cutoff_knob);

                                        let filter_resonance_knob = ui_knob::ArcKnob::for_param(
                                            &params.filter_resonance, 
                                            setter, 
                                            KNOB_SIZE)
                                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                                            .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                                            .set_text_size(TEXT_SIZE);
                                        ui.add(filter_resonance_knob);

                                        let filter_res_type_knob = ui_knob::ArcKnob::for_param(
                                            &params.filter_res_type, 
                                            setter, 
                                            KNOB_SIZE)
                                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                                            .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                                            .set_text_size(TEXT_SIZE);
                                        ui.add(filter_res_type_knob);

                                        let filter_hp_knob = ui_knob::ArcKnob::for_param(
                                            &params.filter_hp_amount, 
                                            setter, 
                                            KNOB_SIZE)
                                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                                            .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                                            .set_text_size(TEXT_SIZE);
                                        ui.add(filter_hp_knob);

                                        let filter_bp_knob = ui_knob::ArcKnob::for_param(
                                            &params.filter_bp_amount, 
                                            setter, 
                                            KNOB_SIZE)
                                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                                            .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                                            .set_text_size(TEXT_SIZE);
                                        ui.add(filter_bp_knob);

                                        let filter_lp_knob = ui_knob::ArcKnob::for_param(
                                            &params.filter_lp_amount, 
                                            setter, 
                                            KNOB_SIZE)
                                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                                            .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                                            .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                                            .set_text_size(TEXT_SIZE);
                                        ui.add(filter_lp_knob);
                                    });
                                });
                                
                                //nih_log!("{:?}",egui_ctx.input().raw);
                                //ui.label(RichText::new(format!("dropped_files: {}", egui_ctx.input().raw.dropped_files.len())).font(FONT).color(FONT_COLOR));
                                /*
                                // Dropped file logic
                                let mut samples_var: Vec<f32>;
                                // Collect dropped files:
                                let mut temp_dropped_files = egui_ctx.input().raw.dropped_files.clone();
                                if !temp_dropped_files.is_empty() {
                                    let dropped_file: Option<Arc<[u8]>> = temp_dropped_files.last().unwrap().bytes.clone();
                                    let file_bytes: Vec<u8> = dropped_file.unwrap().to_vec();
                                    let source_buffer = Cursor::new(file_bytes);
                                    // Attempting to use rodio decoder to read
                                    let decoder = rodio::Decoder::new(source_buffer).unwrap();
                                    samples_var = decoder.map(|sample| {
                                        let i16_sample = sample;
                                        i16_sample as f32 / i16::MAX as f32
                                    }).collect();
                                    //sample_vec = samples_var;
                                }
                                // Delete our dropped file from this buffer
                                temp_dropped_files.clear();
                                */
                            });

                            // Synth Bars on left and right
                            ui.painter().rect_filled(
                                Rect::from_x_y_ranges(
                                    RangeInclusive::new(WIDTH as f32 - synth_bar_space, WIDTH as f32), 
                                    RangeInclusive::new(0.0, HEIGHT as f32)),
                                Rounding::none(),
                                *GUI_VALS.get("SYNTH_BARS_PURPLE").unwrap()
                            );

                            // Screws for that vintage look
                            let screw_space = 16.0;
                            ui.painter().circle_filled(Pos2::new(screw_space,screw_space), 4.0, Color32::LIGHT_GRAY);
                            ui.painter().circle_filled(Pos2::new(screw_space,HEIGHT as f32 - screw_space), 4.0, Color32::LIGHT_GRAY);
                            ui.painter().circle_filled(Pos2::new(WIDTH as f32 - screw_space,screw_space), 4.0, Color32::LIGHT_GRAY);
                            ui.painter().circle_filled(Pos2::new(WIDTH as f32 - screw_space,HEIGHT as f32 - screw_space), 4.0, Color32::LIGHT_GRAY);
                        });
                    });
                }
            )
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        nih_log!("changed sample rate to {}", buffer_config.sample_rate);

        self.sample_rate = buffer_config.sample_rate;

        return true;
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        self.process_midi(context, buffer);

        //let mut amplitude = 0.0;

        /*
        for playing_sample in &mut self.playing_samples {
            match self.loaded_samples.get(&playing_sample.handle) {
                Some(loaded_sample) => {
                    for channel_samples in buffer.iter_samples() {
                        // channel_samples is [a, b, c]
                        for (channel_index, sample) in channel_samples.into_iter().enumerate() {
                            let s = loaded_sample
                                .0
                                .get(channel_index)
                                .unwrap_or(&vec![])
                                .get(playing_sample.position)
                                .unwrap_or(&0.0)
                                * playing_sample.gain;
                            *sample += s;
                            amplitude += s.abs();
                        }
                        playing_sample.position += 1;
                    }
                }
                None => {}
            }
        }
        */

        //amplitude /= buffer.samples() as f32 * buffer.channels() as f32;
        //self.visualizer.store(amplitude);

        // remove samples that are done playing
        //self.playing_samples
        //    .retain(|e| match self.loaded_samples.get(&e.handle) {
        //        Some(sample) => e.position < sample.0[0].len(),
        //        None => false,
        //    });

        ProcessStatus::Normal
    }
}

#[allow(dead_code)]
fn resample(samples: LoadedSample, sample_rate_in: f32, sample_rate_out: f32) -> LoadedSample {
    let samples = samples.0;
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

            LoadedSample(waves_out)
        }
        Err(_) => LoadedSample(vec![]),
    }
}

impl Actuate {
    // Send midi events to the audio modules and let them process them - also send params so they can access
    fn process_midi(&mut self, context: &mut impl ProcessContext<Self>, buffer: &mut Buffer) {
        for (sample_id, mut channel_samples) in buffer.iter_samples().enumerate() {
            // Reset our output buffer signal
            *channel_samples.get_mut(0).unwrap() = 0.0;
            *channel_samples.get_mut(1).unwrap() = 0.0;

            let midi_event: Option<NoteEvent<()>> = context.next_event();
            let wave1: f32 = self.audio_module_1.process_midi(sample_id, self.params.clone(), midi_event.clone(), 1) * self.params.audio_module_1_level.value();
            let wave2: f32 = self.audio_module_2.process_midi(sample_id, self.params.clone(), midi_event.clone(), 2) * self.params.audio_module_2_level.value();
            let wave3: f32 = self.audio_module_3.process_midi(sample_id, self.params.clone(), midi_event.clone(), 3) * self.params.audio_module_3_level.value();

            let mut left_output = wave1 + wave2 + wave3;
            let mut right_output = wave1 + wave2 + wave3;

            // Filtering before output
            self.filter.update(
                self.params.filter_cutoff.value(),
                self.params.filter_resonance.value(),
                self.sample_rate,
                self.params.filter_res_type.value(),
            );
            
            let low_l;
            let band_l;
            let high_l;
            let low_r;
            let band_r;
            let high_r;

            (low_l, band_l, high_l) = self.filter.process(left_output);
            (low_r, band_r, high_r) = self.filter.process(right_output);

            left_output = (low_l*self.params.filter_lp_amount.value() 
                        + band_l*self.params.filter_bp_amount.value()
                        + high_l*self.params.filter_hp_amount.value())*self.params.filter_wet.value()
                        + left_output*(1.0-self.params.filter_wet.value());

            right_output = (low_r*self.params.filter_lp_amount.value() 
                        + band_r*self.params.filter_bp_amount.value()
                        + high_r*self.params.filter_hp_amount.value())*self.params.filter_wet.value()
                        + right_output*(1.0-self.params.filter_wet.value());

            // Reassign our output signal
            *channel_samples.get_mut(0).unwrap() = left_output;
            *channel_samples.get_mut(1).unwrap() = right_output;
        }
    }
}

impl ClapPlugin for Actuate {
    const CLAP_ID: &'static str = "com.ardura.actuate";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("Granulizer + Synth");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::AudioEffect,
        ClapFeature::Stereo,
        ClapFeature::Mono,
        ClapFeature::Utility,
    ];
}

impl Vst3Plugin for Actuate {
    const VST3_CLASS_ID: [u8; 16] = *b"ActuateArduraAAA";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[
        Vst3SubCategory::Instrument,
        Vst3SubCategory::Sampler,
    ];
}

nih_export_clap!(Actuate);
nih_export_vst3!(Actuate);

pub fn format_nothing() -> Arc<dyn Fn(f32) -> String + Send + Sync> {
    Arc::new(move | _ | {
        String::new()
    })
}
