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

Actuate - Synthesizer + Sampler/Granulizer by Ardura

#####################################
*/
#![allow(non_snake_case)]

use StateVariableFilter::ResonanceType;
use nih_plug_egui::{create_egui_editor, egui::{self, Color32, Rect, Rounding, RichText, FontId, Pos2}, EguiState};
use std::{sync::{Arc}, ops::RangeInclusive};
use nih_plug::{prelude::*};
use phf::phf_map;

// My Files
use audio_module::{AudioModuleType, AudioModule, Oscillator::{self, RetriggerStyle, SmoothStyle}};
use crate::audio_module::Oscillator::VoiceType;
mod audio_module;
mod StateVariableFilter;
mod ui_knob;
mod toggle_switch;

pub struct LoadedSample(Vec<Vec<f32>>);

// Plugin sizing
const WIDTH: u32 = 860;
const HEIGHT: u32 = 632;

// File Open Buffer Timer
const FILE_OPEN_BUFFER_MAX: u32 = 2000;

// GUI values to refer to
pub static GUI_VALS: phf::Map<&'static str, Color32> = phf_map! {
    "A_KNOB_OUTSIDE_COLOR" => Color32::from_rgb(85,180,168),
    "DARK_GREY_UI_COLOR" => Color32::from_rgb(49,53,71),
    "SYNTH_SOFT_BLUE" => Color32::from_rgb(142,166,201),
    "A_BACKGROUND_COLOR_TOP" => Color32::from_rgb(185,186,198),
    "SYNTH_BARS_PURPLE" => Color32::from_rgb(45,41,99),
    "SYNTH_MIDDLE_BLUE" => Color32::from_rgb(98,145,204),
    "FONT_COLOR" => Color32::from_rgb(10,103,210),
};

// Font
const FONT: nih_plug_egui::egui::FontId = FontId::monospace(14.0);
const SMALLER_FONT: nih_plug_egui::egui::FontId = FontId::monospace(11.0);

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
    filter_mod_smoother: Smoother<f32>,

    // File loading
    file_dialog: bool,
    file_open_buffer_timer: u32,
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
            filter_mod_smoother: Smoother::new(SmoothingStyle::Linear(300.0)),

            // File Loading
            file_dialog: false,
            file_open_buffer_timer: 0,
        }
    }
}

/// Plugin parameters struct
#[derive(Params)]
pub struct ActuateParams {
    #[persist = "editor-state"]
    editor_state: Arc<EguiState>,

    // Synth-level settings
    #[id = "Master Level"]
    pub master_level: FloatParam,

    #[id = "Max Voices"]
    pub voice_limit: IntParam,

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

    // Controls for when audio_module_1_type is Sampler/Granulizer
    #[id = "load_sample_1"]
    pub load_sample_1: BoolParam,

    #[id = "loop_sample_1"]
    pub loop_sample_1: BoolParam,

    #[id = "single_cycle_1"]
    pub single_cycle_1: BoolParam,

    // Controls for when audio_module_2_type is Sampler/Granulizer
    #[id = "load_sample_2"]
    pub load_sample_2: BoolParam,

    #[id = "loop_sample_2"]
    pub loop_sample_2: BoolParam,

    #[id = "single_cycle_2"]
    pub single_cycle_2: BoolParam,

    // Controls for when audio_module_3_type is Sampler/Granulizer
    #[id = "load_sample_3"]
    pub load_sample_3: BoolParam,

    #[id = "loop_sample_3"]
    pub loop_sample_3: BoolParam,

    #[id = "single_cycle_3"]
    pub single_cycle_3: BoolParam,

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

    #[id = "filter_env_peak"]
    pub filter_env_peak: FloatParam,

    #[id = "filter_env_decay"]
    pub filter_env_decay: FloatParam,

    #[id = "filter_env_curve"]
    pub filter_env_curve: EnumParam<Oscillator::SmoothStyle>,
}

impl Default for ActuateParams {
    fn default() -> Self {
        Self {
            editor_state: EguiState::from_size(WIDTH, HEIGHT),

            master_level: FloatParam::new("Master", 0.4, FloatRange::Linear { min: 0.0, max: 2.0 }).with_value_to_string(formatters::v2s_f32_percentage(0)).with_unit("%"),
            voice_limit: IntParam::new("Max Voices", 16, IntRange::Linear { min: 1, max: 32 }),

            _audio_module_1_type: EnumParam::new("Type", AudioModuleType::Osc),
            _audio_module_2_type: EnumParam::new("Type", AudioModuleType::Off),
            _audio_module_3_type: EnumParam::new("Type", AudioModuleType::Off),

            audio_module_1_level: FloatParam::new("Level", 0.5, FloatRange::Linear { min: 0.0, max: 1.0 }).with_value_to_string(formatters::v2s_f32_percentage(0)).with_unit("%"),
            audio_module_2_level: FloatParam::new("Level", 0.5, FloatRange::Linear { min: 0.0, max: 1.0 }).with_value_to_string(formatters::v2s_f32_percentage(0)).with_unit("%"),
            audio_module_3_level: FloatParam::new("Level", 0.5, FloatRange::Linear { min: 0.0, max: 1.0 }).with_value_to_string(formatters::v2s_f32_percentage(0)).with_unit("%"),

            // Oscillators
            ////////////////////////////////////////////////////////////////////////////////////
            osc_1_type: EnumParam::new("Wave", VoiceType::Sine),
            osc_1_octave: IntParam::new("Octave", 0, IntRange::Linear { min: -2, max: 2 }),
            osc_1_semitones: IntParam::new("Semitones", 0, IntRange::Linear { min: -11, max: 11 }),
            osc_1_detune: FloatParam::new("Detune", 0.0, FloatRange::Linear { min: -0.999, max: 0.999 }),
            osc_1_attack: FloatParam::new("Attack", 0.1, FloatRange::Skewed { min: 0.001, max: 999.9, factor: 0.5 }).with_value_to_string(format_nothing()),
            osc_1_decay: FloatParam::new("Decay", 0.1, FloatRange::Skewed { min: 0.001, max: 999.9, factor: 0.5 }).with_value_to_string(format_nothing()),
            osc_1_sustain: FloatParam::new("Sustain", 999.9, FloatRange::Linear { min: 0.001, max: 999.9 }).with_value_to_string(format_nothing()),
            osc_1_release: FloatParam::new("Release", 5.0, FloatRange::Skewed { min: 0.001, max: 999.9, factor: 0.5 }).with_value_to_string(format_nothing()),
            osc_1_mod_amount: FloatParam::new("Modifier", 0.0, FloatRange::Linear { min: 0.0, max: 1.0 }).with_value_to_string(formatters::v2s_f32_rounded(2)),
            osc_1_retrigger: EnumParam::new("Retrigger", RetriggerStyle::Free),
            osc_1_atk_curve: EnumParam::new("Atk Curve", Oscillator::SmoothStyle::Linear),
            osc_1_dec_curve: EnumParam::new("Dec Curve", Oscillator::SmoothStyle::Linear),
            osc_1_rel_curve: EnumParam::new("Rel Curve", Oscillator::SmoothStyle::Linear),

            osc_2_type: EnumParam::new("Wave", VoiceType::Sine),
            osc_2_octave: IntParam::new("Octave", 0, IntRange::Linear { min: -2, max: 2 }),
            osc_2_semitones: IntParam::new("Semitones", 0, IntRange::Linear { min: -11, max: 11 }),
            osc_2_detune: FloatParam::new("Detune", 0.0, FloatRange::Linear { min: -0.999, max: 0.999 }),
            osc_2_attack: FloatParam::new("Attack", 0.1, FloatRange::Skewed { min: 0.001, max: 999.9, factor: 0.5 }).with_value_to_string(format_nothing()),
            osc_2_decay: FloatParam::new("Decay", 0.1, FloatRange::Skewed { min: 0.001, max: 999.9, factor: 0.5 }).with_value_to_string(format_nothing()),
            osc_2_sustain: FloatParam::new("Sustain", 999.9, FloatRange::Linear { min: 0.001, max: 999.9 }).with_value_to_string(format_nothing()),
            osc_2_release: FloatParam::new("Release", 5.0, FloatRange::Skewed { min: 0.001, max: 999.9, factor: 0.5 }).with_value_to_string(format_nothing()),
            osc_2_mod_amount: FloatParam::new("Modifier", 0.0, FloatRange::Linear { min: 0.0, max: 1.0 }).with_value_to_string(formatters::v2s_f32_rounded(2)),
            osc_2_retrigger: EnumParam::new("Retrigger", RetriggerStyle::Free),
            osc_2_atk_curve: EnumParam::new("Atk Curve", Oscillator::SmoothStyle::Linear),
            osc_2_dec_curve: EnumParam::new("Dec Curve", Oscillator::SmoothStyle::Linear),
            osc_2_rel_curve: EnumParam::new("Rel Curve", Oscillator::SmoothStyle::Linear),

            osc_3_type: EnumParam::new("Wave", VoiceType::Sine),
            osc_3_octave: IntParam::new("Octave", 0, IntRange::Linear { min: -2, max: 2 }),
            osc_3_semitones: IntParam::new("Semitones", 0, IntRange::Linear { min: -11, max: 11 }),
            osc_3_detune: FloatParam::new("Detune", 0.0, FloatRange::Linear { min: -0.999, max: 0.999 }),
            osc_3_attack: FloatParam::new("Attack", 0.1, FloatRange::Skewed { min: 0.001, max: 999.9, factor: 0.5 }).with_value_to_string(format_nothing()),
            osc_3_decay: FloatParam::new("Decay", 0.1, FloatRange::Skewed { min: 0.001, max: 999.9, factor: 0.5 }).with_value_to_string(format_nothing()),
            osc_3_sustain: FloatParam::new("Sustain", 999.9, FloatRange::Linear { min: 0.001, max: 999.9 }).with_value_to_string(format_nothing()),
            osc_3_release: FloatParam::new("Release", 5.0, FloatRange::Skewed { min: 0.001, max: 999.9, factor: 0.5 }).with_value_to_string(format_nothing()),
            osc_3_mod_amount: FloatParam::new("Modifier", 0.0, FloatRange::Linear { min: 0.0, max: 1.0 }).with_value_to_string(formatters::v2s_f32_rounded(2)),
            osc_3_retrigger: EnumParam::new("Retrigger", RetriggerStyle::Free),
            osc_3_atk_curve: EnumParam::new("Atk Curve", Oscillator::SmoothStyle::Linear),
            osc_3_dec_curve: EnumParam::new("Dec Curve", Oscillator::SmoothStyle::Linear),
            osc_3_rel_curve: EnumParam::new("Rel Curve", Oscillator::SmoothStyle::Linear),

            // Granulizer/Sampler
            ////////////////////////////////////////////////////////////////////////////////////
            load_sample_1: BoolParam::new("Load Sample", false),
            load_sample_2: BoolParam::new("Load Sample", false),
            load_sample_3: BoolParam::new("Load Sample", false),
            loop_sample_1: BoolParam::new("Loop Sample", false),
            loop_sample_2: BoolParam::new("Loop Sample", false),
            loop_sample_3: BoolParam::new("Loop Sample", false),
            single_cycle_1: BoolParam::new("single cycle", false),
            single_cycle_2: BoolParam::new("single cycle", false),
            single_cycle_3: BoolParam::new("single cycle", false),

            // Filter
            ////////////////////////////////////////////////////////////////////////////////////
            filter_lp_amount: FloatParam::new("Low Pass", 1.0, FloatRange::Linear { min: 0.0, max: 1.0 }).with_value_to_string(formatters::v2s_f32_percentage(0)),
            filter_hp_amount: FloatParam::new("High Pass", 0.0, FloatRange::Linear { min: 0.0, max: 1.0 }).with_value_to_string(formatters::v2s_f32_percentage(0)),
            filter_bp_amount: FloatParam::new("Band Pass", 0.0, FloatRange::Linear { min: 0.0, max: 1.0 }).with_value_to_string(formatters::v2s_f32_percentage(0)),

            filter_wet: FloatParam::new("Filter Wet", 1.0, FloatRange::Linear { min: 0.0, max: 1.0 }).with_unit("%").with_value_to_string(formatters::v2s_f32_percentage(0)),
            filter_resonance: FloatParam::new("Q", 0.5, FloatRange::Linear { min: 0.2, max: 1.0 } ).with_unit("%").with_value_to_string(formatters::v2s_f32_percentage(0)),
            filter_res_type: EnumParam::new("Q Type", ResonanceType::Default),
            filter_cutoff: FloatParam::new("Frequency", 2000.0, FloatRange::Skewed { min: 20.0, max: 16000.0, factor: 0.5 }).with_value_to_string(formatters::v2s_f32_rounded(0)).with_unit("Hz"),

            filter_env_peak: FloatParam::new("Env Peak", 0.0, FloatRange::SymmetricalSkewed { min: -8000.0, max: 8000.0, factor: 0.6, center: 0.0 }).with_value_to_string(format_nothing()),
            filter_env_decay: FloatParam::new("Env Decay", 300.0, FloatRange::Skewed { min: 0.001, max: 999.9, factor: 0.2}).with_value_to_string(formatters::v2s_f32_rounded(2)),
            filter_env_curve: EnumParam::new("Curve",Oscillator::SmoothStyle::Linear),
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
                                RangeInclusive::new(0.0, (HEIGHT as f32)*0.72)), 
                            Rounding::from(16.0), *GUI_VALS.get("A_BACKGROUND_COLOR_TOP").unwrap());
                        ui.painter().rect_filled(
                            Rect::from_x_y_ranges(
                                RangeInclusive::new(0.0, WIDTH as f32), 
                                RangeInclusive::new((HEIGHT as f32)*0.72, HEIGHT as f32)), 
                            Rounding::from(16.0), *GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap());

                        ui.set_style(style_var);

                        ui.horizontal(|ui| {
                            // Synth Bars on left and right
                            let synth_bar_space = 32.0;
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
                                const KNOB_SIZE: f32 = 32.0;
                                const TEXT_SIZE: f32 = 13.0;
                                ui.horizontal(|ui|{
                                    ui.vertical(|ui|{
                                        ui.label(RichText::new("Generators")
                                            .font(FONT)
                                            .color(*GUI_VALS.get("FONT_COLOR").unwrap()))
                                            .on_hover_text("These are the audio modules that create sound on midi events");
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

                                        ui.horizontal(|ui|{
                                            let master_level_knob = ui_knob::ArcKnob::for_param(
                                                &params.master_level, 
                                                setter, 
                                                KNOB_SIZE + 8.0)
                                                .preset_style(ui_knob::KnobStyle::NewPresets1)
                                                .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                                                .set_line_color(*GUI_VALS.get("SYNTH_MIDDLE_BLUE").unwrap())
                                                .set_text_size(TEXT_SIZE);
                                            ui.add(master_level_knob);
        
                                            let voice_limit_knob = ui_knob::ArcKnob::for_param(
                                                &params.voice_limit, 
                                                setter, 
                                                KNOB_SIZE - 8.0)
                                                .preset_style(ui_knob::KnobStyle::NewPresets1)
                                                .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                                                .set_line_color(*GUI_VALS.get("SYNTH_MIDDLE_BLUE").unwrap())
                                                .set_text_size(TEXT_SIZE);
                                            ui.add(voice_limit_knob);
                                        });
    
                                        // Spacing under master knob to put filters in the right spot
                                        ui.add_space(KNOB_SIZE * 3.0 + 20.0);
                                    });
                                
                                    ui.separator();
                                    ui.vertical(|ui|{
                                        ui.label(RichText::new("Generator Controls")
                                            .font(SMALLER_FONT)
                                            .color(*GUI_VALS.get("FONT_COLOR").unwrap()))
                                            .on_hover_text("These are the controls for the active/selected generators");
                                        audio_module::AudioModule::draw_modules(ui, params.clone(), setter);
                                    });
                                });

                                ui.label("Filters");
                                
                                // Filter section
                                ui.vertical(|ui| {
                                    ui.horizontal(|ui| {
                                        let filter_wet_knob = ui_knob::ArcKnob::for_param(
                                            &params.filter_wet, 
                                            setter, 
                                            KNOB_SIZE)
                                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                                            .set_fill_color(*GUI_VALS.get("SYNTH_BARS_PURPLE").unwrap())
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
                                            .set_fill_color(*GUI_VALS.get("SYNTH_BARS_PURPLE").unwrap())
                                            .set_line_color(*GUI_VALS.get("SYNTH_MIDDLE_BLUE").unwrap())
                                            .set_text_size(TEXT_SIZE);
                                        ui.add(filter_resonance_knob);

                                        let filter_res_type_knob = ui_knob::ArcKnob::for_param(
                                            &params.filter_res_type, 
                                            setter, 
                                            KNOB_SIZE)
                                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                                            .set_fill_color(*GUI_VALS.get("SYNTH_BARS_PURPLE").unwrap())
                                            .set_line_color(*GUI_VALS.get("SYNTH_MIDDLE_BLUE").unwrap())
                                            .set_text_size(TEXT_SIZE);
                                        ui.add(filter_res_type_knob);

                                        let filter_hp_knob = ui_knob::ArcKnob::for_param(
                                            &params.filter_hp_amount, 
                                            setter, 
                                            KNOB_SIZE)
                                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                                            .set_fill_color(*GUI_VALS.get("SYNTH_BARS_PURPLE").unwrap())
                                            .set_line_color(*GUI_VALS.get("SYNTH_MIDDLE_BLUE").unwrap())
                                            .set_text_size(TEXT_SIZE);
                                        ui.add(filter_hp_knob);

                                        let filter_bp_knob = ui_knob::ArcKnob::for_param(
                                            &params.filter_bp_amount, 
                                            setter, 
                                            KNOB_SIZE)
                                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                                            .set_fill_color(*GUI_VALS.get("SYNTH_BARS_PURPLE").unwrap())
                                            .set_line_color(*GUI_VALS.get("SYNTH_MIDDLE_BLUE").unwrap())
                                            .set_text_size(TEXT_SIZE);
                                        ui.add(filter_bp_knob);

                                        let filter_lp_knob = ui_knob::ArcKnob::for_param(
                                            &params.filter_lp_amount, 
                                            setter, 
                                            KNOB_SIZE)
                                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                                            .set_fill_color(*GUI_VALS.get("SYNTH_BARS_PURPLE").unwrap())
                                            .set_line_color(*GUI_VALS.get("SYNTH_MIDDLE_BLUE").unwrap())
                                            .set_text_size(TEXT_SIZE);
                                        ui.add(filter_lp_knob);
                                    });
                                    ui.horizontal(|ui| {
                                        let filter_env_peak = ui_knob::ArcKnob::for_param(
                                            &params.filter_env_peak, 
                                            setter, 
                                            KNOB_SIZE)
                                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                                            .set_fill_color(*GUI_VALS.get("SYNTH_BARS_PURPLE").unwrap())
                                            .set_line_color(*GUI_VALS.get("SYNTH_MIDDLE_BLUE").unwrap())
                                            .set_text_size(TEXT_SIZE);
                                        ui.add(filter_env_peak);

                                        let filter_env_decay_knob = ui_knob::ArcKnob::for_param(
                                            &params.filter_env_decay, 
                                            setter, 
                                            KNOB_SIZE)
                                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                                            .set_fill_color(*GUI_VALS.get("SYNTH_BARS_PURPLE").unwrap())
                                            .set_line_color(*GUI_VALS.get("SYNTH_MIDDLE_BLUE").unwrap())
                                            .set_text_size(TEXT_SIZE);
                                        ui.add(filter_env_decay_knob);

                                        let filter_env_curve_knob = ui_knob::ArcKnob::for_param(
                                            &params.filter_env_curve, 
                                            setter, 
                                            KNOB_SIZE)
                                            .preset_style(ui_knob::KnobStyle::NewPresets1)
                                            .set_fill_color(*GUI_VALS.get("SYNTH_BARS_PURPLE").unwrap())
                                            .set_line_color(*GUI_VALS.get("SYNTH_MIDDLE_BLUE").unwrap())
                                            .set_text_size(TEXT_SIZE);
                                        ui.add(filter_env_curve_knob);
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
        ProcessStatus::Normal
    }
}

impl Actuate {
    // Send midi events to the audio modules and let them process them - also send params so they can access
    fn process_midi(&mut self, context: &mut impl ProcessContext<Self>, buffer: &mut Buffer) {
        for (sample_id, mut channel_samples) in buffer.iter_samples().enumerate() {
            // Get around post file loading breaking things with an arbitrary buffer
            if self.file_dialog {
                self.file_open_buffer_timer += 1;
                if self.file_open_buffer_timer > FILE_OPEN_BUFFER_MAX {
                    self.file_open_buffer_timer = 0;
                    self.file_dialog = false;
                }
            }

            // Reset our output buffer signal
            *channel_samples.get_mut(0).unwrap() = 0.0;
            *channel_samples.get_mut(1).unwrap() = 0.0;

            // This weird bit is to stop playing when going from play to stop
            // but also allowing playing of the synth while stopped
            // midi choke doesn't seem to be working in FL
            if !context.transport().playing && (
                self.audio_module_1.get_playing()
                || self.audio_module_2.get_playing()
                || self.audio_module_3.get_playing()
            ) {
                self.audio_module_1.set_playing(false);
                self.audio_module_2.set_playing(false);
                self.audio_module_3.set_playing(false);
                self.audio_module_1.clear_voices();
                self.audio_module_2.clear_voices();
                self.audio_module_3.clear_voices();
            }
            if context.transport().playing {
                self.audio_module_1.set_playing(true);
                self.audio_module_2.set_playing(true);
                self.audio_module_3.set_playing(true);
            }

            let midi_event: Option<NoteEvent<()>> = context.next_event();
            let sent_voice_max: usize = self.params.voice_limit.value() as usize;
            let mut wave1_l: f32 = 0.0;
            let mut wave2_l: f32 = 0.0;
            let mut wave3_l: f32 = 0.0;
            let mut wave1_r: f32 = 0.0;
            let mut wave2_r: f32 = 0.0;
            let mut wave3_r: f32 = 0.0;
            // These track if a new note happens to re-open the filter
            let mut reset_filter_controller1: bool = false;
            let mut reset_filter_controller2: bool = false;
            let mut reset_filter_controller3: bool = false;

            // Since File Dialog can be set by any of these we need to check each time
            if !self.file_dialog {
                (wave1_l, wave1_r, reset_filter_controller1) = self.audio_module_1.process_midi(sample_id, self.params.clone(), midi_event.clone(), 1, sent_voice_max, &mut self.file_dialog);
            }
            if !self.file_dialog {
                (wave2_l, wave2_r, reset_filter_controller2) = self.audio_module_2.process_midi(sample_id, self.params.clone(), midi_event.clone(), 2, sent_voice_max, &mut self.file_dialog);
            }
            if !self.file_dialog {
                (wave3_l, wave3_r, reset_filter_controller3) = self.audio_module_3.process_midi(sample_id, self.params.clone(), midi_event.clone(), 3, sent_voice_max, &mut self.file_dialog);
            }

            wave1_l *= self.params.audio_module_1_level.value();
            wave2_l *= self.params.audio_module_2_level.value();
            wave3_l *= self.params.audio_module_3_level.value();
            wave1_r *= self.params.audio_module_1_level.value();
            wave2_r *= self.params.audio_module_2_level.value();
            wave3_r *= self.params.audio_module_3_level.value();

            let mut left_output: f32 = wave1_l + wave2_l + wave3_l;
            let mut right_output: f32 = wave1_r + wave2_r + wave3_r;

            // Try to trigger our filter mods on note on! This is sequential/single because we just need a trigger at a point in time
            if reset_filter_controller1 || reset_filter_controller2 || reset_filter_controller3 {
                self.filter_mod_smoother = match self.params.filter_env_curve.value() {
                    SmoothStyle::Linear => Smoother::new(SmoothingStyle::Linear(self.params.filter_env_decay.value())),
                    SmoothStyle::Logarithmic => Smoother::new(SmoothingStyle::Logarithmic(self.params.filter_env_decay.value())),
                    SmoothStyle::Exponential => Smoother::new(SmoothingStyle::Exponential(self.params.filter_env_decay.value())),
                };
                // This makes our filter actuate point
                self.filter_mod_smoother.reset((self.params.filter_cutoff.value() + self.params.filter_env_peak.value()).clamp(20.0, 16000.0));
                
                // Set up the smoother for our filter movement
                self.filter_mod_smoother.set_target(self.sample_rate, self.params.filter_cutoff.value());
            }

            // Filtering before output
            self.filter.update(
               self.filter_mod_smoother.next(),
               self.params.filter_resonance.value(),
               self.sample_rate,
               self.params.filter_res_type.value(),
            );
            
            let low_l: f32;
            let band_l: f32;
            let high_l: f32;
            let low_r: f32;
            let band_r: f32;
            let high_r: f32;

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
            *channel_samples.get_mut(0).unwrap() = left_output * self.params.master_level.value();
            *channel_samples.get_mut(1).unwrap() = right_output * self.params.master_level.value();
        }
    }
}

impl ClapPlugin for Actuate {
    const CLAP_ID: &'static str = "com.ardura.actuate";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("Sampler + Synth");
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
