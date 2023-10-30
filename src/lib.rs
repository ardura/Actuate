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

use nih_plug_egui::{
    create_egui_editor,
    egui::{self, Align2, Color32, FontId, Pos2, Rect, RichText, Rounding, Vec2},
    EguiState,
};
use rfd::FileDialog;
use StateVariableFilter::ResonanceType;

use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use nih_plug::prelude::*;
use phf::phf_map;
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::{
    fs::File,
    io::Write,
    ops::RangeInclusive,
    sync::{
        atomic::{AtomicBool, AtomicU32, Ordering},
        Arc, Mutex,
    },
};

// My Files
use crate::audio_module::Oscillator::VoiceType;
use audio_module::{
    AudioModule, AudioModuleType,
    Oscillator::{self, OscState, RetriggerStyle, SmoothStyle},
};
use CustomParamSlider::ParamSlider as HorizontalParamSlider;
use CustomVerticalSlider::ParamSlider as VerticalParamSlider;
mod BoolButton;
mod CustomParamSlider;
mod CustomVerticalSlider;
mod LFOController;
mod StateVariableFilter;
mod audio_module;
mod toggle_switch;
mod ui_knob;

pub struct LoadedSample(Vec<Vec<f32>>);

// Plugin sizing
const WIDTH: u32 = 920;
const HEIGHT: u32 = 656;

const PRESET_BANK_SIZE: usize = 32;

// File Open Buffer Timer
const FILE_OPEN_BUFFER_MAX: u32 = 1;

// GUI values to refer to
pub static GUI_VALS: phf::Map<&'static str, Color32> = phf_map! {
    "A_KNOB_OUTSIDE_COLOR" => Color32::from_rgb(67,157,148),
    "DARK_GREY_UI_COLOR" => Color32::from_rgb(49,53,71),
    "LIGHT_GREY_UI_COLOR" => Color32::from_rgb(99,103,121),
    "LIGHTER_GREY_UI_COLOR" => Color32::from_rgb(149,153,171),
    "SYNTH_SOFT_BLUE" => Color32::from_rgb(142,166,201),
    "SYNTH_SOFT_BLUE2" => Color32::from_rgb(102,126,181),
    "A_BACKGROUND_COLOR_TOP" => Color32::from_rgb(185,186,198),
    "SYNTH_BARS_PURPLE" => Color32::from_rgb(45,41,99),
    "SYNTH_MIDDLE_BLUE" => Color32::from_rgb(98,145,204),
    "FONT_COLOR" => Color32::from_rgb(10,103,210),
};

#[derive(Debug, PartialEq)]
enum FilterSelect {
    Filter1,
    Filter2,
}

// Values for Audio Module Routing
#[derive(Enum, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub enum AMFilterRouting {
    Bypass,
    Filter1,
    Filter2,
    Both,
}

// Filter order routing
#[derive(Enum, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub enum FilterRouting {
    Parallel,
    Series12,
    Series21,
}

// Font
const FONT: nih_plug_egui::egui::FontId = FontId::monospace(14.0);
const LOADING_FONT: nih_plug_egui::egui::FontId = FontId::monospace(20.0);
const SMALLER_FONT: nih_plug_egui::egui::FontId = FontId::monospace(11.0);

#[derive(Serialize, Deserialize, Clone)]
struct ActuatePreset {
    // Modules 1
    ///////////////////////////////////////////////////////////
    mod1_audio_module_type: AudioModuleType,
    mod1_audio_module_level: f32,
    // Granulizer/Sampler
    mod1_loaded_sample: Vec<Vec<f32>>,
    mod1_sample_lib: Vec<Vec<Vec<f32>>>,
    mod1_loop_wavetable: bool,
    mod1_single_cycle: bool,
    mod1_restretch: bool,
    mod1_prev_restretch: bool,
    mod1_grain_hold: i32,
    mod1_grain_gap: i32,
    mod1_start_position: f32,
    mod1_end_position: f32,
    mod1_grain_crossfade: i32,

    // Osc module knob storage
    mod1_osc_type: VoiceType,
    mod1_osc_octave: i32,
    mod1_osc_semitones: i32,
    mod1_osc_detune: f32,
    mod1_osc_attack: f32,
    mod1_osc_decay: f32,
    mod1_osc_sustain: f32,
    mod1_osc_release: f32,
    mod1_osc_retrigger: RetriggerStyle,
    mod1_osc_atk_curve: SmoothStyle,
    mod1_osc_dec_curve: SmoothStyle,
    mod1_osc_rel_curve: SmoothStyle,
    mod1_osc_unison: i32,
    mod1_osc_unison_detune: f32,
    mod1_osc_stereo: f32,

    // Modules 2
    ///////////////////////////////////////////////////////////
    mod2_audio_module_type: AudioModuleType,
    mod2_audio_module_level: f32,
    // Granulizer/Sampler
    mod2_loaded_sample: Vec<Vec<f32>>,
    mod2_sample_lib: Vec<Vec<Vec<f32>>>,
    mod2_loop_wavetable: bool,
    mod2_single_cycle: bool,
    mod2_restretch: bool,
    mod2_prev_restretch: bool,
    mod2_grain_hold: i32,
    mod2_grain_gap: i32,
    mod2_start_position: f32,
    mod2_end_position: f32,
    mod2_grain_crossfade: i32,

    // Osc module knob storage
    mod2_osc_type: VoiceType,
    mod2_osc_octave: i32,
    mod2_osc_semitones: i32,
    mod2_osc_detune: f32,
    mod2_osc_attack: f32,
    mod2_osc_decay: f32,
    mod2_osc_sustain: f32,
    mod2_osc_release: f32,
    mod2_osc_retrigger: RetriggerStyle,
    mod2_osc_atk_curve: SmoothStyle,
    mod2_osc_dec_curve: SmoothStyle,
    mod2_osc_rel_curve: SmoothStyle,
    mod2_osc_unison: i32,
    mod2_osc_unison_detune: f32,
    mod2_osc_stereo: f32,

    // Modules 3
    ///////////////////////////////////////////////////////////
    mod3_audio_module_type: AudioModuleType,
    mod3_audio_module_level: f32,
    // Granulizer/Sampler
    mod3_loaded_sample: Vec<Vec<f32>>,
    mod3_sample_lib: Vec<Vec<Vec<f32>>>,
    mod3_loop_wavetable: bool,
    mod3_single_cycle: bool,
    mod3_restretch: bool,
    mod3_prev_restretch: bool,
    mod3_grain_hold: i32,
    mod3_grain_gap: i32,
    mod3_start_position: f32,
    mod3_end_position: f32,
    mod3_grain_crossfade: i32,

    // Osc module knob storage
    mod3_osc_type: VoiceType,
    mod3_osc_octave: i32,
    mod3_osc_semitones: i32,
    mod3_osc_detune: f32,
    mod3_osc_attack: f32,
    mod3_osc_decay: f32,
    mod3_osc_sustain: f32,
    mod3_osc_release: f32,
    mod3_osc_retrigger: RetriggerStyle,
    mod3_osc_atk_curve: SmoothStyle,
    mod3_osc_dec_curve: SmoothStyle,
    mod3_osc_rel_curve: SmoothStyle,
    mod3_osc_unison: i32,
    mod3_osc_unison_detune: f32,
    mod3_osc_stereo: f32,

    // Filters
    filter_wet: f32,
    filter_cutoff: f32,
    filter_resonance: f32,
    filter_res_type: ResonanceType,
    filter_lp_amount: f32,
    filter_hp_amount: f32,
    filter_bp_amount: f32,
    filter_env_peak: f32,
    filter_env_attack: f32,
    filter_env_decay: f32,
    filter_env_sustain: f32,
    filter_env_release: f32,
    filter_env_atk_curve: Oscillator::SmoothStyle,
    filter_env_dec_curve: Oscillator::SmoothStyle,
    filter_env_rel_curve: Oscillator::SmoothStyle,

    filter_wet_2: f32,
    filter_cutoff_2: f32,
    filter_resonance_2: f32,
    filter_res_type_2: ResonanceType,
    filter_lp_amount_2: f32,
    filter_hp_amount_2: f32,
    filter_bp_amount_2: f32,
    filter_env_peak_2: f32,
    filter_env_attack_2: f32,
    filter_env_decay_2: f32,
    filter_env_sustain_2: f32,
    filter_env_release_2: f32,
    filter_env_atk_curve_2: Oscillator::SmoothStyle,
    filter_env_dec_curve_2: Oscillator::SmoothStyle,
    filter_env_rel_curve_2: Oscillator::SmoothStyle,

    // LFOs
    lfo1_freq: f32,
    lfo1_retrigger: LFOController::LFORetrigger,
    lfo1_sync: bool,
    lfo1_snap: LFOController::LFOSnapValues,
    lfo1_waveform: LFOController::Waveform,
    lfo1_phase: f32,

    lfo2_freq: f32,
    lfo2_retrigger: LFOController::LFORetrigger,
    lfo2_sync: bool,
    lfo2_snap: LFOController::LFOSnapValues,
    lfo2_waveform: LFOController::Waveform,
    lfo2_phase: f32,

    lfo3_freq: f32,
    lfo3_retrigger: LFOController::LFORetrigger,
    lfo3_sync: bool,
    lfo3_snap: LFOController::LFOSnapValues,
    lfo3_waveform: LFOController::Waveform,
    lfo3_phase: f32,
}

#[derive(Clone)]
pub struct Actuate {
    pub params: Arc<ActuateParams>,
    pub sample_rate: f32,

    // Plugin control Arcs
    update_something: Arc<AtomicBool>,
    clear_voices: Arc<AtomicBool>,
    reload_entire_preset: Arc<AtomicBool>,
    file_dialog: Arc<AtomicBool>,
    file_open_buffer_timer: Arc<AtomicU32>,
    current_preset: Arc<AtomicU32>,

    update_current_preset: Arc<AtomicBool>,
    load_bank: Arc<AtomicBool>,
    save_bank: Arc<AtomicBool>,

    // Modules
    audio_module_1: Arc<Mutex<AudioModule>>,
    _audio_module_1_type: AudioModuleType,
    audio_module_2: Arc<Mutex<AudioModule>>,
    _audio_module_2_type: AudioModuleType,
    audio_module_3: Arc<Mutex<AudioModule>>,
    _audio_module_3_type: AudioModuleType,

    // Filters
    filter_l_1: StateVariableFilter::StateVariableFilter,
    filter_r_1: StateVariableFilter::StateVariableFilter,
    filter_state_1: OscState,
    filter_atk_smoother_1: Smoother<f32>,
    filter_dec_smoother_1: Smoother<f32>,
    filter_rel_smoother_1: Smoother<f32>,

    filter_l_2: StateVariableFilter::StateVariableFilter,
    filter_r_2: StateVariableFilter::StateVariableFilter,
    filter_state_2: OscState,
    filter_atk_smoother_2: Smoother<f32>,
    filter_dec_smoother_2: Smoother<f32>,
    filter_rel_smoother_2: Smoother<f32>,

    // LFOs!
    lfo_1: LFOController::LFOController,
    lfo_2: LFOController::LFOController,
    lfo_3: LFOController::LFOController,

    // Preset Lib Default
    preset_lib_name: String,
    preset_lib: Arc<Mutex<Vec<ActuatePreset>>>,

    // Used for DC Offset calculations
    dc_filter_l: StateVariableFilter::StateVariableFilter,
    dc_filter_r: StateVariableFilter::StateVariableFilter,
}

impl Default for Actuate {
    fn default() -> Self {
        // These are persistent fields to trigger updates like Diopser
        let update_something = Arc::new(AtomicBool::new(false));
        let clear_voices = Arc::new(AtomicBool::new(false));
        let reload_entire_preset = Arc::new(AtomicBool::new(false));
        let file_dialog = Arc::new(AtomicBool::new(false));
        let file_open_buffer_timer = Arc::new(AtomicU32::new(0));
        let current_preset = Arc::new(AtomicU32::new(0));

        let load_bank = Arc::new(AtomicBool::new(false));
        let save_bank = Arc::new(AtomicBool::new(false));
        let update_current_preset = Arc::new(AtomicBool::new(false));

        Self {
            params: Arc::new(ActuateParams::new(
                update_something.clone(),
                clear_voices.clone(),
                file_dialog.clone(),
                update_current_preset.clone(),
                load_bank.clone(),
                save_bank.clone(),
            )),
            sample_rate: 44100.0,

            // Plugin control ARCs
            update_something: update_something,
            clear_voices: clear_voices,
            reload_entire_preset: reload_entire_preset,
            file_dialog: file_dialog,
            file_open_buffer_timer: file_open_buffer_timer,
            current_preset: current_preset,

            load_bank: load_bank,
            save_bank: save_bank,
            update_current_preset: update_current_preset,

            // Module 1
            audio_module_1: Arc::new(Mutex::new(AudioModule::default())),
            _audio_module_1_type: AudioModuleType::Osc,
            audio_module_2: Arc::new(Mutex::new(AudioModule::default())),
            _audio_module_2_type: AudioModuleType::Off,
            audio_module_3: Arc::new(Mutex::new(AudioModule::default())),
            _audio_module_3_type: AudioModuleType::Off,

            // Filters
            filter_l_2: StateVariableFilter::StateVariableFilter::default().set_oversample(4),
            filter_r_2: StateVariableFilter::StateVariableFilter::default().set_oversample(4),
            filter_state_2: OscState::Off,
            filter_atk_smoother_2: Smoother::new(SmoothingStyle::Linear(300.0)),
            filter_dec_smoother_2: Smoother::new(SmoothingStyle::Linear(300.0)),
            filter_rel_smoother_2: Smoother::new(SmoothingStyle::Linear(300.0)),

            filter_l_1: StateVariableFilter::StateVariableFilter::default().set_oversample(4),
            filter_r_1: StateVariableFilter::StateVariableFilter::default().set_oversample(4),
            filter_state_1: OscState::Off,
            filter_atk_smoother_1: Smoother::new(SmoothingStyle::Linear(300.0)),
            filter_dec_smoother_1: Smoother::new(SmoothingStyle::Linear(300.0)),
            filter_rel_smoother_1: Smoother::new(SmoothingStyle::Linear(300.0)),

            //LFOs
            lfo_1: LFOController::LFOController::new(2.0, 1.0, LFOController::Waveform::Sine, 0.0),
            lfo_2: LFOController::LFOController::new(2.0, 1.0, LFOController::Waveform::Sine, 0.0),
            lfo_3: LFOController::LFOController::new(2.0, 1.0, LFOController::Waveform::Sine, 0.0),
            /*
            Note Type            Equation to Calculate Frequency (Hz)   Frequency at 120 BPM (Hz)
            -------------------------------------------------------------------------------------
            Whole Note           1 / (60 / 120)                        2.00 Hz
            Dotted Whole Note    1 / (60 / 120 * 1.5)                  1.33 Hz
            Half Note            1 / (60 / 120 / 2)                    4.00 Hz
            Dotted Half Note     1 / (60 / 120 / 2 * 1.5)              2.67 Hz
            Quarter Note         1 / (60 / 120 / 4)                    8.00 Hz
            Dotted Quarter Note  1 / (60 / 120 / 4 * 1.5)              5.33 Hz
            Eighth Note          1 / (60 / 120 / 8)                    16.00 Hz
            Dotted Eighth Note   1 / (60 / 120 / 8 * 1.5)              10.67 Hz
            Sixteenth Note       1 / (60 / 120 / 16)                   32.00 Hz
            Dotted Sixteenth Note 1 / (60 / 120 / 16 * 1.5)             21.33 Hz
            Triplet Whole Note   1 / (60 / 120 / 3)                    3.33 Hz
            Triplet Half Note    1 / (60 / 120 / 6)                    6.67 Hz
            Triplet Quarter Note 1 / (60 / 120 / 12)                   13.33 Hz
            Triplet Eighth Note  1 / (60 / 120 / 24)                   26.67 Hz
            Triplet Sixteenth Note 1 / (60 / 120 / 48)                 53.33 Hz
            */
            // Preset Library DEFAULT
            preset_lib_name: String::from("Default"),
            preset_lib: Arc::new(Mutex::new(vec![
                ActuatePreset {
                    mod1_audio_module_type: AudioModuleType::Osc,
                    mod1_audio_module_level: 1.0,
                    mod1_loaded_sample: vec![vec![0.0, 0.0]],
                    mod1_sample_lib: vec![vec![vec![0.0, 0.0]]],
                    mod1_loop_wavetable: false,
                    mod1_single_cycle: false,
                    mod1_restretch: true,
                    mod1_prev_restretch: false,
                    mod1_grain_hold: 200,
                    mod1_grain_gap: 200,
                    mod1_start_position: 0.0,
                    mod1_end_position: 1.0,
                    mod1_grain_crossfade: 50,
                    mod1_osc_type: VoiceType::Sine,
                    mod1_osc_octave: 0,
                    mod1_osc_semitones: 0,
                    mod1_osc_detune: 0.0,
                    mod1_osc_attack: 0.0001,
                    mod1_osc_decay: 0.0001,
                    mod1_osc_sustain: 999.9,
                    mod1_osc_release: 5.0,
                    mod1_osc_retrigger: RetriggerStyle::Retrigger,
                    mod1_osc_atk_curve: SmoothStyle::Linear,
                    mod1_osc_dec_curve: SmoothStyle::Linear,
                    mod1_osc_rel_curve: SmoothStyle::Linear,
                    mod1_osc_unison: 1,
                    mod1_osc_unison_detune: 0.0,
                    mod1_osc_stereo: 0.0,

                    mod2_audio_module_type: AudioModuleType::Off,
                    mod2_audio_module_level: 1.0,
                    mod2_loaded_sample: vec![vec![0.0, 0.0]],
                    mod2_sample_lib: vec![vec![vec![0.0, 0.0]]],
                    mod2_loop_wavetable: false,
                    mod2_single_cycle: false,
                    mod2_restretch: true,
                    mod2_prev_restretch: false,
                    mod2_grain_hold: 200,
                    mod2_grain_gap: 200,
                    mod2_start_position: 0.0,
                    mod2_end_position: 1.0,
                    mod2_grain_crossfade: 50,
                    mod2_osc_type: VoiceType::Sine,
                    mod2_osc_octave: 0,
                    mod2_osc_semitones: 0,
                    mod2_osc_detune: 0.0,
                    mod2_osc_attack: 0.0001,
                    mod2_osc_decay: 0.0001,
                    mod2_osc_sustain: 999.9,
                    mod2_osc_release: 5.0,
                    mod2_osc_retrigger: RetriggerStyle::Retrigger,
                    mod2_osc_atk_curve: SmoothStyle::Linear,
                    mod2_osc_dec_curve: SmoothStyle::Linear,
                    mod2_osc_rel_curve: SmoothStyle::Linear,
                    mod2_osc_unison: 1,
                    mod2_osc_unison_detune: 0.0,
                    mod2_osc_stereo: 0.0,

                    mod3_audio_module_type: AudioModuleType::Off,
                    mod3_audio_module_level: 1.0,
                    mod3_loaded_sample: vec![vec![0.0, 0.0]],
                    mod3_sample_lib: vec![vec![vec![0.0, 0.0]]],
                    mod3_loop_wavetable: false,
                    mod3_single_cycle: false,
                    mod3_restretch: true,
                    mod3_prev_restretch: false,
                    mod3_grain_hold: 200,
                    mod3_grain_gap: 200,
                    mod3_start_position: 0.0,
                    mod3_end_position: 1.0,
                    mod3_grain_crossfade: 50,
                    mod3_osc_type: VoiceType::Sine,
                    mod3_osc_octave: 0,
                    mod3_osc_semitones: 0,
                    mod3_osc_detune: 0.0,
                    mod3_osc_attack: 0.0001,
                    mod3_osc_decay: 0.0001,
                    mod3_osc_sustain: 999.9,
                    mod3_osc_release: 5.0,
                    mod3_osc_retrigger: RetriggerStyle::Retrigger,
                    mod3_osc_atk_curve: SmoothStyle::Linear,
                    mod3_osc_dec_curve: SmoothStyle::Linear,
                    mod3_osc_rel_curve: SmoothStyle::Linear,
                    mod3_osc_unison: 1,
                    mod3_osc_unison_detune: 0.0,
                    mod3_osc_stereo: 0.0,

                    filter_wet: 1.0,
                    filter_cutoff: 4000.0,
                    filter_resonance: 1.0,
                    filter_res_type: ResonanceType::Default,
                    filter_lp_amount: 1.0,
                    filter_hp_amount: 0.0,
                    filter_bp_amount: 0.0,
                    filter_env_peak: 0.0,
                    filter_env_attack: 0.0,
                    filter_env_decay: 250.0,
                    filter_env_sustain: 999.9,
                    filter_env_release: 100.0,
                    filter_env_atk_curve: SmoothStyle::Linear,
                    filter_env_dec_curve: SmoothStyle::Linear,
                    filter_env_rel_curve: SmoothStyle::Linear,

                    filter_wet_2: 1.0,
                    filter_cutoff_2: 4000.0,
                    filter_resonance_2: 1.0,
                    filter_res_type_2: ResonanceType::Default,
                    filter_lp_amount_2: 1.0,
                    filter_hp_amount_2: 0.0,
                    filter_bp_amount_2: 0.0,
                    filter_env_peak_2: 0.0,
                    filter_env_attack_2: 0.0,
                    filter_env_decay_2: 250.0,
                    filter_env_sustain_2: 999.9,
                    filter_env_release_2: 100.0,
                    filter_env_atk_curve_2: SmoothStyle::Linear,
                    filter_env_dec_curve_2: SmoothStyle::Linear,
                    filter_env_rel_curve_2: SmoothStyle::Linear,

                    // LFOs
                    lfo1_freq: 2.0,
                    lfo1_retrigger: LFOController::LFORetrigger::None,
                    lfo1_sync: true,
                    lfo1_snap: LFOController::LFOSnapValues::Half,
                    lfo1_waveform: LFOController::Waveform::Sine,
                    lfo1_phase: 0.0,

                    lfo2_freq: 2.0,
                    lfo2_retrigger: LFOController::LFORetrigger::None,
                    lfo2_sync: true,
                    lfo2_snap: LFOController::LFOSnapValues::Half,
                    lfo2_waveform: LFOController::Waveform::Sine,
                    lfo2_phase: 0.0,

                    lfo3_freq: 2.0,
                    lfo3_retrigger: LFOController::LFORetrigger::None,
                    lfo3_sync: true,
                    lfo3_snap: LFOController::LFOSnapValues::Half,
                    lfo3_waveform: LFOController::Waveform::Sine,
                    lfo3_phase: 0.0,
                };
                PRESET_BANK_SIZE
            ])),

            dc_filter_l: StateVariableFilter::StateVariableFilter::default().set_oversample(2),
            dc_filter_r: StateVariableFilter::StateVariableFilter::default().set_oversample(2),
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

    // Audio Module Filter Routing
    #[id = "audio_module_1_routing"]
    pub audio_module_1_routing: EnumParam<AMFilterRouting>,
    #[id = "audio_module_2_routing"]
    pub audio_module_2_routing: EnumParam<AMFilterRouting>,
    #[id = "audio_module_3_routing"]
    pub audio_module_3_routing: EnumParam<AMFilterRouting>,

    // Filter routing
    #[id = "filter_routing"]
    pub filter_routing: EnumParam<FilterRouting>,

    // Controls for when audio_module_1_type is Osc
    #[id = "osc_1_type"]
    pub osc_1_type: EnumParam<VoiceType>,
    #[id = "osc_1_octave"]
    pub osc_1_octave: IntParam,
    #[id = "osc_1_semitones"]
    pub osc_1_semitones: IntParam,
    #[id = "osc_1_detune"]
    pub osc_1_detune: FloatParam,
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
    #[id = "osc_1_unison"]
    pub osc_1_unison: IntParam,
    #[id = "osc_1_unison_detune"]
    pub osc_1_unison_detune: FloatParam,
    #[id = "osc_1_stereo"]
    pub osc_1_stereo: FloatParam,

    // Controls for when audio_module_2_type is Osc
    #[id = "osc_2_type"]
    pub osc_2_type: EnumParam<VoiceType>,
    #[id = "osc_2_octave"]
    pub osc_2_octave: IntParam,
    #[id = "osc_2_semitones"]
    pub osc_2_semitones: IntParam,
    #[id = "osc_2_detune"]
    pub osc_2_detune: FloatParam,
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
    #[id = "osc_2_unison"]
    pub osc_2_unison: IntParam,
    #[id = "osc_2_unison_detune"]
    pub osc_2_unison_detune: FloatParam,
    #[id = "osc_2_stereo"]
    pub osc_2_stereo: FloatParam,

    // Controls for when audio_module_3_type is Osc
    #[id = "osc_3_type"]
    pub osc_3_type: EnumParam<VoiceType>,
    #[id = "osc_3_octave"]
    pub osc_3_octave: IntParam,
    #[id = "osc_3_semitones"]
    pub osc_3_semitones: IntParam,
    #[id = "osc_3_detune"]
    pub osc_3_detune: FloatParam,
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

    #[id = "osc_3_unison"]
    pub osc_3_unison: IntParam,
    #[id = "osc_3_unison_detune"]
    pub osc_3_unison_detune: FloatParam,
    #[id = "osc_3_stereo"]
    pub osc_3_stereo: FloatParam,

    // Controls for when audio_module_1_type is Sampler/Granulizer
    #[id = "load_sample_1"]
    pub load_sample_1: BoolParam,
    #[id = "loop_sample_1"]
    pub loop_sample_1: BoolParam,
    #[id = "single_cycle_1"]
    pub single_cycle_1: BoolParam,
    #[id = "restretch_1"]
    pub restretch_1: BoolParam,
    #[id = "grain_hold_1"]
    grain_hold_1: IntParam,
    #[id = "grain_gap_1"]
    grain_gap_1: IntParam,
    #[id = "start_position_1"]
    start_position_1: FloatParam,
    #[id = "end_position_1"]
    end_position_1: FloatParam,
    #[id = "grain_crossfade_1"]
    grain_crossfade_1: IntParam,

    // Controls for when audio_module_2_type is Sampler/Granulizer
    #[id = "load_sample_2"]
    pub load_sample_2: BoolParam,
    #[id = "loop_sample_2"]
    pub loop_sample_2: BoolParam,
    #[id = "single_cycle_2"]
    pub single_cycle_2: BoolParam,
    #[id = "restretch_2"]
    pub restretch_2: BoolParam,
    #[id = "grain_hold_2"]
    grain_hold_2: IntParam,
    #[id = "grain_gap_2"]
    grain_gap_2: IntParam,
    #[id = "start_position_2"]
    start_position_2: FloatParam,
    #[id = "end_position_2"]
    end_position_2: FloatParam,
    #[id = "grain_crossfade_2"]
    grain_crossfade_2: IntParam,

    // Controls for when audio_module_3_type is Sampler/Granulizer
    #[id = "load_sample_3"]
    pub load_sample_3: BoolParam,
    #[id = "loop_sample_3"]
    pub loop_sample_3: BoolParam,
    #[id = "single_cycle_3"]
    pub single_cycle_3: BoolParam,
    #[id = "restretch_3"]
    pub restretch_3: BoolParam,
    #[id = "grain_hold_3"]
    grain_hold_3: IntParam,
    #[id = "grain_gap_3"]
    grain_gap_3: IntParam,
    #[id = "start_position_3"]
    start_position_3: FloatParam,
    #[id = "end_position_3"]
    end_position_3: FloatParam,
    #[id = "grain_crossfade_3"]
    grain_crossfade_3: IntParam,

    // Filters
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
    #[id = "filter_env_attack"]
    pub filter_env_attack: FloatParam,
    #[id = "filter_env_decay"]
    pub filter_env_decay: FloatParam,
    #[id = "filter_env_sustain"]
    pub filter_env_sustain: FloatParam,
    #[id = "filter_env_release"]
    pub filter_env_release: FloatParam,
    #[id = "filter_env_atk_curve"]
    pub filter_env_atk_curve: EnumParam<Oscillator::SmoothStyle>,
    #[id = "filter_env_dec_curve"]
    pub filter_env_dec_curve: EnumParam<Oscillator::SmoothStyle>,
    #[id = "filter_env_rel_curve"]
    pub filter_env_rel_curve: EnumParam<Oscillator::SmoothStyle>,

    #[id = "filter_wet_2"]
    pub filter_wet_2: FloatParam,
    #[id = "filter_cutoff_2"]
    pub filter_cutoff_2: FloatParam,
    #[id = "filter_resonance_2"]
    pub filter_resonance_2: FloatParam,
    #[id = "filter_res_type_2"]
    pub filter_res_type_2: EnumParam<ResonanceType>,
    #[id = "filter_lp_amount_2"]
    pub filter_lp_amount_2: FloatParam,
    #[id = "filter_hp_amount_2"]
    pub filter_hp_amount_2: FloatParam,
    #[id = "filter_bp_amount_2"]
    pub filter_bp_amount_2: FloatParam,
    #[id = "filter_env_peak_2"]
    pub filter_env_peak_2: FloatParam,
    #[id = "filter_env_attack_2"]
    pub filter_env_attack_2: FloatParam,
    #[id = "filter_env_decay_2"]
    pub filter_env_decay_2: FloatParam,
    #[id = "filter_env_sustain_2"]
    pub filter_env_sustain_2: FloatParam,
    #[id = "filter_env_release_2"]
    pub filter_env_release_2: FloatParam,
    #[id = "filter_env_atk_curve_2"]
    pub filter_env_atk_curve_2: EnumParam<Oscillator::SmoothStyle>,
    #[id = "filter_env_dec_curve_2"]
    pub filter_env_dec_curve_2: EnumParam<Oscillator::SmoothStyle>,
    #[id = "filter_env_rel_curve_2"]
    pub filter_env_rel_curve_2: EnumParam<Oscillator::SmoothStyle>,

    // LFOS
    #[id = "lfo1_Retrigger"]
    pub lfo1_retrigger: EnumParam<LFOController::LFORetrigger>,
    #[id = "lfo2_Retrigger"]
    pub lfo2_retrigger: EnumParam<LFOController::LFORetrigger>,
    #[id = "lfo3_Retrigger"]
    pub lfo3_retrigger: EnumParam<LFOController::LFORetrigger>,
    #[id = "lfo1_sync"]
    pub lfo1_sync: BoolParam,
    #[id = "lfo2_sync"]
    pub lfo2_sync: BoolParam,
    #[id = "lfo3_sync"]
    pub lfo3_sync: BoolParam,
    #[id = "lfo1_freq"]
    pub lfo1_freq: FloatParam,
    #[id = "lfo2_freq"]
    pub lfo2_freq: FloatParam,
    #[id = "lfo3_freq"]
    pub lfo3_freq: FloatParam,
    #[id = "lfo1_snap"]
    pub lfo1_snap: EnumParam<LFOController::LFOSnapValues>,
    #[id = "lfo2_snap"]
    pub lfo2_snap: EnumParam<LFOController::LFOSnapValues>,
    #[id = "lfo3_snap"]
    pub lfo3_snap: EnumParam<LFOController::LFOSnapValues>,
    #[id = "lfo1_waveform"]
    pub lfo1_waveform: EnumParam<LFOController::Waveform>,
    #[id = "lfo2_waveform"]
    pub lfo2_waveform: EnumParam<LFOController::Waveform>,
    #[id = "lfo3_waveform"]
    pub lfo3_waveform: EnumParam<LFOController::Waveform>,
    #[id = "lfo1_phase"]
    pub lfo1_phase: FloatParam,
    #[id = "lfo2_phase"]
    pub lfo2_phase: FloatParam,
    #[id = "lfo3_phase"]
    pub lfo3_phase: FloatParam,

    // UI Non-param Params
    #[id = "param_load_bank"]
    pub param_load_bank: BoolParam,
    #[id = "param_save_bank"]
    pub param_save_bank: BoolParam,
    #[id = "param_next_preset"]
    pub param_next_preset: BoolParam,
    #[id = "param_prev_preset"]
    pub param_prev_preset: BoolParam,
    #[id = "param_update_current_preset"]
    pub param_update_current_preset: BoolParam,

    // Not a param
    #[id = "loading"]
    pub loading: BoolParam,
}

impl ActuateParams {
    fn new(
        update_something: Arc<AtomicBool>,
        clear_voices: Arc<AtomicBool>,
        file_dialog: Arc<AtomicBool>,
        update_current_preset: Arc<AtomicBool>,
        load_bank: Arc<AtomicBool>,
        save_bank: Arc<AtomicBool>,
    ) -> Self {
        Self {
            editor_state: EguiState::from_size(WIDTH, HEIGHT),

            // Top Level objects
            ////////////////////////////////////////////////////////////////////////////////////
            master_level: FloatParam::new("Master", 1.0, FloatRange::Linear { min: 0.0, max: 2.0 })
                .with_value_to_string(formatters::v2s_f32_percentage(0))
                .with_unit("%"),
            voice_limit: IntParam::new("Max Voices", 64, IntRange::Linear { min: 1, max: 512 }),

            _audio_module_1_type: EnumParam::new("Type", AudioModuleType::Osc).with_callback({
                let clear_voices = clear_voices.clone();
                Arc::new(move |_| clear_voices.store(true, Ordering::Relaxed))
            }),
            _audio_module_2_type: EnumParam::new("Type", AudioModuleType::Off).with_callback({
                let clear_voices = clear_voices.clone();
                Arc::new(move |_| clear_voices.store(true, Ordering::Relaxed))
            }),
            _audio_module_3_type: EnumParam::new("Type", AudioModuleType::Off).with_callback({
                let clear_voices = clear_voices.clone();
                Arc::new(move |_| clear_voices.store(true, Ordering::Relaxed))
            }),

            audio_module_1_level: FloatParam::new(
                "Level",
                0.5,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_value_to_string(formatters::v2s_f32_percentage(0))
            .with_unit("%"),
            audio_module_2_level: FloatParam::new(
                "Level",
                0.5,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_value_to_string(formatters::v2s_f32_percentage(0))
            .with_unit("%"),
            audio_module_3_level: FloatParam::new(
                "Level",
                0.5,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_value_to_string(formatters::v2s_f32_percentage(0))
            .with_unit("%"),

            audio_module_1_routing: EnumParam::new("Routing", AMFilterRouting::Filter1),
            audio_module_2_routing: EnumParam::new("Routing", AMFilterRouting::Filter1),
            audio_module_3_routing: EnumParam::new("Routing", AMFilterRouting::Filter1),

            filter_routing: EnumParam::new("Filter Routing", FilterRouting::Parallel),

            // Oscillators
            ////////////////////////////////////////////////////////////////////////////////////
            osc_1_type: EnumParam::new("Wave", VoiceType::Sine).with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
            }),
            osc_1_octave: IntParam::new("Octave", 0, IntRange::Linear { min: -2, max: 2 })
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
                }),
            osc_1_semitones: IntParam::new("Semitones", 0, IntRange::Linear { min: -11, max: 11 })
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
                }),
            osc_1_detune: FloatParam::new(
                "Detune",
                0.0,
                FloatRange::Linear {
                    min: -0.999,
                    max: 0.999,
                },
            )
            .with_step_size(0.0001)
            .with_value_to_string(formatters::v2s_f32_rounded(4))
            .with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
            }),
            osc_1_attack: FloatParam::new(
                "Attack",
                0.0001,
                FloatRange::Skewed {
                    min: 0.0001,
                    max: 999.9,
                    factor: 0.5,
                },
            )
            .with_step_size(0.0001)
            .with_value_to_string(format_nothing())
            .with_unit("A")
            .with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
            }),
            osc_1_decay: FloatParam::new(
                "Decay",
                0.0001,
                FloatRange::Skewed {
                    min: 0.0001,
                    max: 999.9,
                    factor: 0.5,
                },
            )
            .with_step_size(0.0001)
            .with_value_to_string(format_nothing())
            .with_unit("D")
            .with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
            }),
            osc_1_sustain: FloatParam::new(
                "Sustain",
                999.9,
                FloatRange::Linear {
                    min: 0.0001,
                    max: 999.9,
                },
            )
            .with_step_size(0.0001)
            .with_value_to_string(format_nothing())
            .with_unit("S")
            .with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
            }),
            osc_1_release: FloatParam::new(
                "Release",
                5.0,
                FloatRange::Skewed {
                    min: 0.0001,
                    max: 999.9,
                    factor: 0.5,
                },
            )
            .with_step_size(0.0001)
            .with_value_to_string(format_nothing())
            .with_unit("R")
            .with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
            }),
            osc_1_retrigger: EnumParam::new("Retrigger", RetriggerStyle::Retrigger).with_callback(
                {
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
                },
            ),
            osc_1_atk_curve: EnumParam::new("Atk Curve", Oscillator::SmoothStyle::Linear)
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
                }),
            osc_1_dec_curve: EnumParam::new("Dec Curve", Oscillator::SmoothStyle::Linear)
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
                }),
            osc_1_rel_curve: EnumParam::new("Rel Curve", Oscillator::SmoothStyle::Linear)
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
                }),
            osc_1_unison: IntParam::new("Unison", 1, IntRange::Linear { min: 1, max: 9 })
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
                }),
            osc_1_unison_detune: FloatParam::new(
                "Uni Detune",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.0001)
            .with_value_to_string(formatters::v2s_f32_rounded(4))
            .with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
            }),
            osc_1_stereo: FloatParam::new("Stereo", 1.0, FloatRange::Linear { min: 0.0, max: 2.0 })
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
                }),

            osc_2_type: EnumParam::new("Wave", VoiceType::Sine).with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
            }),
            osc_2_octave: IntParam::new("Octave", 0, IntRange::Linear { min: -2, max: 2 })
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
                }),
            osc_2_semitones: IntParam::new("Semitones", 0, IntRange::Linear { min: -11, max: 11 })
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
                }),
            osc_2_detune: FloatParam::new(
                "Detune",
                0.0,
                FloatRange::Linear {
                    min: -0.999,
                    max: 0.999,
                },
            )
            .with_step_size(0.0001)
            .with_value_to_string(formatters::v2s_f32_rounded(4))
            .with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
            }),
            osc_2_attack: FloatParam::new(
                "Attack",
                0.0001,
                FloatRange::Skewed {
                    min: 0.0001,
                    max: 999.9,
                    factor: 0.5,
                },
            )
            .with_step_size(0.0001)
            .with_value_to_string(format_nothing())
            .with_unit("A")
            .with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
            }),
            osc_2_decay: FloatParam::new(
                "Decay",
                0.0001,
                FloatRange::Skewed {
                    min: 0.0001,
                    max: 999.9,
                    factor: 0.5,
                },
            )
            .with_step_size(0.0001)
            .with_value_to_string(format_nothing())
            .with_unit("D")
            .with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
            }),
            osc_2_sustain: FloatParam::new(
                "Sustain",
                999.9,
                FloatRange::Linear {
                    min: 0.0001,
                    max: 999.9,
                },
            )
            .with_step_size(0.0001)
            .with_value_to_string(format_nothing())
            .with_unit("S")
            .with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
            }),
            osc_2_release: FloatParam::new(
                "Release",
                5.0,
                FloatRange::Skewed {
                    min: 0.0001,
                    max: 999.9,
                    factor: 0.5,
                },
            )
            .with_step_size(0.0001)
            .with_value_to_string(format_nothing())
            .with_unit("R")
            .with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
            }),
            osc_2_retrigger: EnumParam::new("Retrigger", RetriggerStyle::Retrigger).with_callback(
                {
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
                },
            ),
            osc_2_atk_curve: EnumParam::new("Atk Curve", Oscillator::SmoothStyle::Linear)
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
                }),
            osc_2_dec_curve: EnumParam::new("Dec Curve", Oscillator::SmoothStyle::Linear)
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
                }),
            osc_2_rel_curve: EnumParam::new("Rel Curve", Oscillator::SmoothStyle::Linear)
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
                }),
            osc_2_unison: IntParam::new("Unison", 1, IntRange::Linear { min: 1, max: 9 })
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
                }),
            osc_2_unison_detune: FloatParam::new(
                "Uni Detune",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.0001)
            .with_value_to_string(formatters::v2s_f32_rounded(4))
            .with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
            }),
            osc_2_stereo: FloatParam::new("Stereo", 1.0, FloatRange::Linear { min: 0.0, max: 2.0 })
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
                }),

            osc_3_type: EnumParam::new("Wave", VoiceType::Sine).with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
            }),
            osc_3_octave: IntParam::new("Octave", 0, IntRange::Linear { min: -2, max: 2 })
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
                }),
            osc_3_semitones: IntParam::new("Semitones", 0, IntRange::Linear { min: -11, max: 11 })
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
                }),
            osc_3_detune: FloatParam::new(
                "Detune",
                0.0,
                FloatRange::Linear {
                    min: -0.999,
                    max: 0.999,
                },
            )
            .with_step_size(0.0001)
            .with_value_to_string(formatters::v2s_f32_rounded(4))
            .with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
            }),
            osc_3_attack: FloatParam::new(
                "Attack",
                0.0001,
                FloatRange::Skewed {
                    min: 0.0001,
                    max: 999.9,
                    factor: 0.5,
                },
            )
            .with_step_size(0.0001)
            .with_value_to_string(format_nothing())
            .with_unit("A")
            .with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
            }),
            osc_3_decay: FloatParam::new(
                "Decay",
                0.0001,
                FloatRange::Skewed {
                    min: 0.0001,
                    max: 999.9,
                    factor: 0.5,
                },
            )
            .with_step_size(0.0001)
            .with_value_to_string(format_nothing())
            .with_unit("D")
            .with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
            }),
            osc_3_sustain: FloatParam::new(
                "Sustain",
                999.9,
                FloatRange::Linear {
                    min: 0.0001,
                    max: 999.9,
                },
            )
            .with_step_size(0.0001)
            .with_value_to_string(format_nothing())
            .with_unit("S")
            .with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
            }),
            osc_3_release: FloatParam::new(
                "Release",
                5.0,
                FloatRange::Skewed {
                    min: 0.0001,
                    max: 999.9,
                    factor: 0.5,
                },
            )
            .with_step_size(0.0001)
            .with_value_to_string(format_nothing())
            .with_unit("R")
            .with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
            }),
            osc_3_retrigger: EnumParam::new("Retrigger", RetriggerStyle::Retrigger).with_callback(
                {
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
                },
            ),
            osc_3_atk_curve: EnumParam::new("Atk Curve", Oscillator::SmoothStyle::Linear)
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
                }),
            osc_3_dec_curve: EnumParam::new("Dec Curve", Oscillator::SmoothStyle::Linear)
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
                }),
            osc_3_rel_curve: EnumParam::new("Rel Curve", Oscillator::SmoothStyle::Linear)
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
                }),
            osc_3_unison: IntParam::new("Unison", 1, IntRange::Linear { min: 1, max: 9 })
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
                }),
            osc_3_unison_detune: FloatParam::new(
                "Uni Detune",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.0001)
            .with_value_to_string(formatters::v2s_f32_rounded(4))
            .with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
            }),
            osc_3_stereo: FloatParam::new("Stereo", 1.0, FloatRange::Linear { min: 0.0, max: 2.0 })
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
                }),

            // Granulizer/Sampler
            ////////////////////////////////////////////////////////////////////////////////////
            load_sample_1: BoolParam::new("Load Sample", false).with_callback({
                let file_dialog = file_dialog.clone();
                Arc::new(move |_| file_dialog.store(true, Ordering::Relaxed))
            }),
            load_sample_2: BoolParam::new("Load Sample", false).with_callback({
                let file_dialog = file_dialog.clone();
                Arc::new(move |_| file_dialog.store(true, Ordering::Relaxed))
            }),
            load_sample_3: BoolParam::new("Load Sample", false).with_callback({
                let file_dialog = file_dialog.clone();
                Arc::new(move |_| file_dialog.store(true, Ordering::Relaxed))
            }),
            // To loop the sampler/granulizer
            loop_sample_1: BoolParam::new("Loop Sample", false).with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
            }),
            loop_sample_2: BoolParam::new("Loop Sample", false).with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
            }),
            loop_sample_3: BoolParam::new("Loop Sample", false).with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
            }),
            // Sampler only - toggle single cycle mode
            single_cycle_1: BoolParam::new("Single Cycle", false).with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
            }),
            single_cycle_2: BoolParam::new("Single Cycle", false).with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
            }),
            single_cycle_3: BoolParam::new("Single Cycle", false).with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
            }),
            // Always true for granulizer/ can be off for sampler
            restretch_1: BoolParam::new("Load Stretch", true).with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
            }),
            restretch_2: BoolParam::new("Load Stretch", true).with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
            }),
            restretch_3: BoolParam::new("Load Stretch", true).with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
            }),
            // This is from 0 to 2000 samples
            grain_hold_1: IntParam::new("Grain Hold", 200, IntRange::Linear { min: 5, max: 22050 })
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
                }),
            grain_hold_2: IntParam::new("Grain Hold", 200, IntRange::Linear { min: 5, max: 22050 })
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
                }),
            grain_hold_3: IntParam::new("Grain Hold", 200, IntRange::Linear { min: 5, max: 22050 })
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
                }),
            grain_gap_1: IntParam::new("Grain Gap", 200, IntRange::Linear { min: 0, max: 22050 })
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
                }),
            grain_gap_2: IntParam::new("Grain Gap", 200, IntRange::Linear { min: 0, max: 22050 })
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
                }),
            grain_gap_3: IntParam::new("Grain Gap", 200, IntRange::Linear { min: 0, max: 22050 })
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
                }),
            // This is going to be in % since sample can be any size
            start_position_1: FloatParam::new(
                "Start Pos",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_unit("%")
            .with_value_to_string(formatters::v2s_f32_percentage(0))
            .with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
            }),
            start_position_2: FloatParam::new(
                "Start Pos",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_unit("%")
            .with_value_to_string(formatters::v2s_f32_percentage(0))
            .with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
            }),
            start_position_3: FloatParam::new(
                "Start Pos",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_unit("%")
            .with_value_to_string(formatters::v2s_f32_percentage(0))
            .with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
            }),
            end_position_1: FloatParam::new(
                "End Pos",
                1.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_unit("%")
            .with_value_to_string(formatters::v2s_f32_percentage(0))
            .with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
            }),
            end_position_2: FloatParam::new(
                "End Pos",
                1.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_unit("%")
            .with_value_to_string(formatters::v2s_f32_percentage(0))
            .with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
            }),
            end_position_3: FloatParam::new(
                "End Pos",
                1.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_unit("%")
            .with_value_to_string(formatters::v2s_f32_percentage(0))
            .with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
            }),
            // Grain Crossfade
            grain_crossfade_1: IntParam::new("Shape", 50, IntRange::Linear { min: 2, max: 2000 })
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
                }),
            grain_crossfade_2: IntParam::new("Shape", 50, IntRange::Linear { min: 2, max: 2000 })
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
                }),
            grain_crossfade_3: IntParam::new("Shape", 50, IntRange::Linear { min: 2, max: 2000 })
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
                }),

            // Filters
            ////////////////////////////////////////////////////////////////////////////////////
            filter_lp_amount: FloatParam::new(
                "Low Pass",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_value_to_string(formatters::v2s_f32_percentage(0))
            .with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
            }),
            filter_hp_amount: FloatParam::new(
                "High Pass",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_value_to_string(formatters::v2s_f32_percentage(0))
            .with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
            }),
            filter_bp_amount: FloatParam::new(
                "Band Pass",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_value_to_string(formatters::v2s_f32_percentage(0))
            .with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
            }),

            filter_wet: FloatParam::new(
                "Filter Wet",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_unit("%")
            .with_value_to_string(formatters::v2s_f32_percentage(0))
            .with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
            }),
            filter_resonance: FloatParam::new(
                "Bandwidth",
                1.0,
                FloatRange::Linear { min: 0.1, max: 1.0 },
            )
            .with_unit("%")
            .with_value_to_string(formatters::v2s_f32_percentage(0)),
            filter_res_type: EnumParam::new("Filter Type", ResonanceType::Default).with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
            }),
            filter_cutoff: FloatParam::new(
                "Frequency",
                16000.0,
                FloatRange::Skewed {
                    min: 20.0,
                    max: 16000.0,
                    factor: 0.5,
                },
            )
            .with_step_size(1.0)
            .with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
            }),

            filter_env_peak: FloatParam::new(
                "Env Peak",
                0.0,
                FloatRange::Linear {
                    min: -5000.0,
                    max: 5000.0,
                },
            )
            .with_step_size(1.0)
            .with_value_to_string(format_nothing())
            .with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
            }),
            filter_env_attack: FloatParam::new(
                "Env Attack",
                0.0001,
                FloatRange::Skewed {
                    min: 0.0001,
                    max: 999.9,
                    factor: 0.2,
                },
            )
            .with_value_to_string(format_nothing())
            .with_unit("A")
            .with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
            }),
            filter_env_decay: FloatParam::new(
                "Env Decay",
                0.0001,
                FloatRange::Skewed {
                    min: 0.0001,
                    max: 999.9,
                    factor: 0.2,
                },
            )
            .with_value_to_string(format_nothing())
            .with_unit("D")
            .with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
            }),
            filter_env_sustain: FloatParam::new(
                "Env Sustain",
                999.9,
                FloatRange::Skewed {
                    min: 0.0001,
                    max: 999.9,
                    factor: 0.2,
                },
            )
            .with_value_to_string(format_nothing())
            .with_unit("S")
            .with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
            }),
            filter_env_release: FloatParam::new(
                "Env Release",
                0.0001,
                FloatRange::Skewed {
                    min: 0.0001,
                    max: 999.9,
                    factor: 0.2,
                },
            )
            .with_value_to_string(format_nothing())
            .with_unit("R")
            .with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
            }),
            filter_env_atk_curve: EnumParam::new("Atk Curve", Oscillator::SmoothStyle::Linear)
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
                }),
            filter_env_dec_curve: EnumParam::new("Dec Curve", Oscillator::SmoothStyle::Linear)
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
                }),
            filter_env_rel_curve: EnumParam::new("Rel Curve", Oscillator::SmoothStyle::Linear)
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
                }),

            filter_lp_amount_2: FloatParam::new(
                "Low Pass",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_value_to_string(formatters::v2s_f32_percentage(0))
            .with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
            }),
            filter_hp_amount_2: FloatParam::new(
                "High Pass",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_value_to_string(formatters::v2s_f32_percentage(0))
            .with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
            }),
            filter_bp_amount_2: FloatParam::new(
                "Band Pass",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_value_to_string(formatters::v2s_f32_percentage(0))
            .with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
            }),

            filter_wet_2: FloatParam::new(
                "Filter Wet",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_unit("%")
            .with_value_to_string(formatters::v2s_f32_percentage(0))
            .with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
            }),
            filter_resonance_2: FloatParam::new(
                "Bandwidth",
                1.0,
                FloatRange::Linear { min: 0.1, max: 1.0 },
            )
            .with_unit("%")
            .with_value_to_string(formatters::v2s_f32_percentage(0)),
            filter_res_type_2: EnumParam::new("Filter Type", ResonanceType::Default).with_callback(
                {
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
                },
            ),
            filter_cutoff_2: FloatParam::new(
                "Frequency",
                16000.0,
                FloatRange::Skewed {
                    min: 20.0,
                    max: 16000.0,
                    factor: 0.5,
                },
            )
            .with_step_size(1.0)
            .with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
            }),

            filter_env_peak_2: FloatParam::new(
                "Env Peak",
                0.0,
                FloatRange::Linear {
                    min: -5000.0,
                    max: 5000.0,
                },
            )
            .with_step_size(1.0)
            .with_value_to_string(format_nothing())
            .with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
            }),
            filter_env_attack_2: FloatParam::new(
                "Env Attack",
                0.0001,
                FloatRange::Skewed {
                    min: 0.0001,
                    max: 999.9,
                    factor: 0.2,
                },
            )
            .with_value_to_string(format_nothing())
            .with_unit("A")
            .with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
            }),
            filter_env_decay_2: FloatParam::new(
                "Env Decay",
                0.0001,
                FloatRange::Skewed {
                    min: 0.0001,
                    max: 999.9,
                    factor: 0.2,
                },
            )
            .with_value_to_string(format_nothing())
            .with_unit("D")
            .with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
            }),
            filter_env_sustain_2: FloatParam::new(
                "Env Sustain",
                999.9,
                FloatRange::Skewed {
                    min: 0.0001,
                    max: 999.9,
                    factor: 0.2,
                },
            )
            .with_value_to_string(format_nothing())
            .with_unit("S")
            .with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
            }),
            filter_env_release_2: FloatParam::new(
                "Env Release",
                0.0001,
                FloatRange::Skewed {
                    min: 0.0001,
                    max: 999.9,
                    factor: 0.2,
                },
            )
            .with_value_to_string(format_nothing())
            .with_unit("R")
            .with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
            }),
            filter_env_atk_curve_2: EnumParam::new("Atk Curve", Oscillator::SmoothStyle::Linear)
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
                }),
            filter_env_dec_curve_2: EnumParam::new("Dec Curve", Oscillator::SmoothStyle::Linear)
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
                }),
            filter_env_rel_curve_2: EnumParam::new("Rel Curve", Oscillator::SmoothStyle::Linear)
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
                }),

            // LFOs
            ////////////////////////////////////////////////////////////////////////////////////
            lfo1_retrigger: EnumParam::new("LFO Retrigger", LFOController::LFORetrigger::None)
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
                }),
            lfo2_retrigger: EnumParam::new("LFO Retrigger", LFOController::LFORetrigger::None)
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
                }),
            lfo3_retrigger: EnumParam::new("LFO Retrigger", LFOController::LFORetrigger::None)
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
                }),
            lfo1_freq: FloatParam::new(
                "LFO1 Freq",
                4.62, // Defualt is half note at 138 bpm
                FloatRange::Skewed {
                    min: 1.0,
                    max: 160.0,
                    factor: 0.4,
                }, // Based on max bpm of 300
            )
            .with_value_to_string(formatters::v2s_f32_hz_then_khz(2))
            .with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
            }),
            lfo2_freq: FloatParam::new(
                "LFO2 Freq",
                4.62, // Defualt is half note at 138 bpm
                FloatRange::Skewed {
                    min: 1.0,
                    max: 160.0,
                    factor: 0.4,
                }, // Based on max bpm of 300
            )
            .with_value_to_string(formatters::v2s_f32_hz_then_khz(2))
            .with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
            }),
            lfo3_freq: FloatParam::new(
                "LFO3 Freq",
                4.62, // Defualt is half note at 138 bpm
                FloatRange::Skewed {
                    min: 1.0,
                    max: 160.0,
                    factor: 0.4,
                }, // Based on max bpm of 300
            )
            .with_value_to_string(formatters::v2s_f32_hz_then_khz(2))
            .with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
            }),
            lfo1_snap: EnumParam::new("LFO1 Snap", LFOController::LFOSnapValues::Half),
            lfo2_snap: EnumParam::new("LFO2 Snap", LFOController::LFOSnapValues::Half),
            lfo3_snap: EnumParam::new("LFO3 Snap", LFOController::LFOSnapValues::Half),
            lfo1_sync: BoolParam::new("LFO1 Sync", true),
            lfo2_sync: BoolParam::new("LFO2 Sync", true),
            lfo3_sync: BoolParam::new("LFO3 Sync", true),
            lfo1_waveform: EnumParam::new("LFO1 Waveform", LFOController::Waveform::Sine),
            lfo2_waveform: EnumParam::new("LFO2 Waveform", LFOController::Waveform::Sine),
            lfo3_waveform: EnumParam::new("LFO3 Waveform", LFOController::Waveform::Sine),
            lfo1_phase: FloatParam::new(
                "LFO1 Phase",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),
            lfo2_phase: FloatParam::new(
                "LFO1 Phase",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),
            lfo3_phase: FloatParam::new(
                "LFO1 Phase",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),

            // UI Non-Param Params
            ////////////////////////////////////////////////////////////////////////////////////
            param_load_bank: BoolParam::new("Load Bank", false).with_callback({
                let load_bank = load_bank.clone();
                Arc::new(move |_| load_bank.store(true, Ordering::Relaxed))
            }),
            param_save_bank: BoolParam::new("Save Bank", false).with_callback({
                let save_bank = save_bank.clone();
                Arc::new(move |_| save_bank.store(true, Ordering::Relaxed))
            }),

            // For some reason the callback doesn't work right here so I went by validating params for previous and next
            param_next_preset: BoolParam::new("->", false),
            param_prev_preset: BoolParam::new("<-", false),

            param_update_current_preset: BoolParam::new("Update Current Preset", false)
                .with_callback({
                    let update_current_preset = update_current_preset.clone();
                    Arc::new(move |_| update_current_preset.store(true, Ordering::Relaxed))
                }),

            // Not a param
            loading: BoolParam::new("loading_mod", false),
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
        let params: Arc<ActuateParams> = self.params.clone();
        let arc_preset = Arc::clone(&self.preset_lib); //Arc<Mutex<Vec<ActuatePreset>>>
        let clear_voices = Arc::clone(&self.clear_voices);
        let reload_entire_preset = Arc::clone(&self.reload_entire_preset);
        let current_preset = Arc::clone(&self.current_preset);
        let AM1 = Arc::clone(&self.audio_module_1);
        let AM2 = Arc::clone(&self.audio_module_2);
        let AM3 = Arc::clone(&self.audio_module_3);

        let update_current_preset = Arc::clone(&self.update_current_preset);
        let load_bank = Arc::clone(&self.load_bank);
        let save_bank = Arc::clone(&self.save_bank);

        let loading = Arc::clone(&self.file_dialog);
        let filter_select_outside = Arc::new(Mutex::new(FilterSelect::Filter1));

        // Do our GUI stuff
        create_egui_editor(
            self.params.editor_state.clone(),
            (),
            |_, _| {},
            move |egui_ctx, setter, _state| {
                egui::CentralPanel::default()
                    .show(egui_ctx, |ui| {
                        let current_preset_index = current_preset.load(Ordering::Relaxed);
                        let filter_select = filter_select_outside.clone();

                        // Reset our buttons
                        if params.param_next_preset.value() {
                            loading.store(true, Ordering::Relaxed);
                            setter.set_parameter(&params.loading, true);
                            if current_preset_index < 31 {
                                current_preset.store(current_preset_index + 1, Ordering::Relaxed);

                                setter.set_parameter(&params.param_next_preset, false);
                                clear_voices.store(true, Ordering::Relaxed);

                                // This is manually here to make sure it appears for long loads from different threads
                                // Create the loading popup here.
                                let screen_size = Rect::from_x_y_ranges(
                                RangeInclusive::new(0.0, WIDTH as f32),
                                RangeInclusive::new(0.0, HEIGHT as f32));
                                let popup_size = Vec2::new(400.0, 200.0);
                                let popup_pos = screen_size.center();

                                // Draw the loading popup content here.
                                ui.painter().rect_filled(Rect::from_center_size(Pos2 { x: popup_pos.x, y: popup_pos.y }, popup_size), 10.0, Color32::GRAY);
                                ui.painter().text(popup_pos, Align2::CENTER_CENTER, "Loading...", LOADING_FONT, Color32::BLACK);

                                // GUI thread misses this without this call here for some reason
                                Actuate::reload_entire_preset(setter, params.clone(), (current_preset_index + 1) as usize, arc_preset.clone(), AM1.clone(), AM2.clone(), AM3.clone());

                                // This is set for the process thread
                                reload_entire_preset.store(true, Ordering::Relaxed);
                            }
                            setter.set_parameter(&params.loading, false);
                        }
                        if params.param_prev_preset.value() {
                            loading.store(true, Ordering::Relaxed);
                            setter.set_parameter(&params.loading, true);
                            if current_preset_index > 0 {
                                current_preset.store(current_preset_index - 1, Ordering::Relaxed);

                                setter.set_parameter(&params.param_prev_preset, false);
                                clear_voices.store(true, Ordering::Relaxed);

                                // This is manually here to make sure it appears for long loads from different threads
                                // Create the loading popup here.
                                let screen_size = Rect::from_x_y_ranges(
                                    RangeInclusive::new(0.0, WIDTH as f32),
                                    RangeInclusive::new(0.0, HEIGHT as f32));
                                let popup_size = Vec2::new(400.0, 200.0);
                                let popup_pos = screen_size.center();

                                // Draw the loading popup content here.
                                ui.painter().rect_filled(Rect::from_center_size(Pos2 { x: popup_pos.x, y: popup_pos.y }, popup_size), 10.0, Color32::GRAY);
                                ui.painter().text(popup_pos, Align2::CENTER_CENTER, "Loading...", LOADING_FONT, Color32::BLACK);

                                // GUI thread misses this without this call here for some reason
                                Actuate::reload_entire_preset(setter, params.clone(), (current_preset_index - 1) as usize, arc_preset.clone(), AM1.clone(), AM2.clone(), AM3.clone());

                                // This is set for the process thread
                                reload_entire_preset.store(true, Ordering::Relaxed);
                            }
                            setter.set_parameter(&params.loading, false);
                        }
                        if load_bank.load(Ordering::Relaxed) {
                            setter.set_parameter(&params.loading, true);
                            reload_entire_preset.store(true, Ordering::Relaxed);
                            loading.store(true, Ordering::Relaxed);

                            // This is manually here to make sure it appears for long loads from different threads
                            // Create the loading popup here.
                            let screen_size = Rect::from_x_y_ranges(
                            RangeInclusive::new(0.0, WIDTH as f32),
                            RangeInclusive::new(0.0, HEIGHT as f32));
                            let popup_size = Vec2::new(400.0, 200.0);
                            let popup_pos = screen_size.center();

                            // Draw the loading popup content here.
                            ui.painter().rect_filled(Rect::from_center_size(Pos2 { x: popup_pos.x, y: popup_pos.y }, popup_size), 10.0, Color32::GRAY);
                            ui.painter().text(popup_pos, Align2::CENTER_CENTER, "Loading...", LOADING_FONT, Color32::BLACK);

                            Actuate::reload_entire_preset(setter, params.clone(), current_preset_index as usize, arc_preset.clone(), AM1.clone(), AM2.clone(), AM3.clone());
                            setter.set_parameter(&params.param_load_bank, false);
                            load_bank.store(false, Ordering::Relaxed);
                            reload_entire_preset.store(false, Ordering::Relaxed);
                            setter.set_parameter(&params.loading, false);
                        }
                        if save_bank.load(Ordering::Relaxed) {
                            setter.set_parameter(&params.param_save_bank, false);
                            save_bank.store(false, Ordering::Relaxed);
                        }
                        if update_current_preset.load(Ordering::Relaxed) || params.param_update_current_preset.value() {
                            setter.set_parameter(&params.param_update_current_preset, false);
                            update_current_preset.store(false, Ordering::Relaxed);
                        }

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

                            // Spacers for primary generator knobs
                            let generator_separator_length: f32 = 170.0;
                            ui.painter().rect_filled(
                                Rect::from_x_y_ranges(
                                    RangeInclusive::new(synth_bar_space + 4.0, synth_bar_space + generator_separator_length),
                                    RangeInclusive::new(192.0, 194.0)),
                                Rounding::none(),
                                *GUI_VALS.get("LIGHTER_GREY_UI_COLOR").unwrap()
                            );

                            // Spacers for primary generator knobs
                            ui.painter().rect_filled(
                                Rect::from_x_y_ranges(
                                    RangeInclusive::new(synth_bar_space + 4.0, synth_bar_space + generator_separator_length),
                                    RangeInclusive::new(328.0, 330.0)),
                                Rounding::none(),
                                *GUI_VALS.get("LIGHTER_GREY_UI_COLOR").unwrap()
                            );

                            ui.add_space(synth_bar_space);

                            // GUI Structure
                            ui.vertical(|ui| {
                                ui.horizontal(|ui|{
                                    ui.label(RichText::new("Actuate")
                                        .font(FONT)
                                        .color(*GUI_VALS.get("FONT_COLOR").unwrap()))
                                        .on_hover_text("by Ardura!");
                                    ui.add_space(20.0);
                                    ui.separator();
                                    ui.add(CustomParamSlider::ParamSlider::for_param(&params.master_level, setter)
                                        .slimmer(0.5)
                                        .set_left_sided_label(true)
                                        .set_label_width(70.0));
                                    ui.separator();
                                    ui.add(CustomParamSlider::ParamSlider::for_param(&params.voice_limit, setter)
                                        .slimmer(0.5)
                                        .set_left_sided_label(true)
                                        .set_label_width(84.0));
                                    ui.separator();
                                    ui.add(CustomParamSlider::ParamSlider::for_param(&params.filter_routing, setter)
                                        .slimmer(0.5)
                                        .set_left_sided_label(true)
                                        .set_label_width(120.0));
                                });
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
                                            let audio_module_1_filter_routing = ui_knob::ArcKnob::for_param(
                                                &params.audio_module_1_routing,
                                                setter,
                                                KNOB_SIZE)
                                                .preset_style(ui_knob::KnobStyle::NewPresets1)
                                                .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                                                .set_line_color(*GUI_VALS.get("SYNTH_MIDDLE_BLUE").unwrap())
                                                .set_text_size(TEXT_SIZE);
                                            ui.add(audio_module_1_filter_routing);
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
                                        ui.horizontal(|ui|{
                                            let audio_module_2_filter_routing = ui_knob::ArcKnob::for_param(
                                                &params.audio_module_2_routing,
                                                setter,
                                                KNOB_SIZE)
                                                .preset_style(ui_knob::KnobStyle::NewPresets1)
                                                .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                                                .set_line_color(*GUI_VALS.get("SYNTH_MIDDLE_BLUE").unwrap())
                                                .set_text_size(TEXT_SIZE);
                                            ui.add(audio_module_2_filter_routing);
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
                                            let audio_module_3_filter_routing = ui_knob::ArcKnob::for_param(
                                                &params.audio_module_3_routing,
                                                setter,
                                                KNOB_SIZE)
                                                .preset_style(ui_knob::KnobStyle::NewPresets1)
                                                .set_fill_color(*GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap())
                                                .set_line_color(*GUI_VALS.get("SYNTH_MIDDLE_BLUE").unwrap())
                                                .set_text_size(TEXT_SIZE);
                                            ui.add(audio_module_3_filter_routing);
                                        });
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
                                //ui.add_space(32.0);
                                ui.label("Filters");

                                // Filter section

                                ui.horizontal(|ui| {
                                    if *filter_select.lock().unwrap() == FilterSelect::Filter1 {
                                        ui.vertical(|ui|{
                                            let filter_wet_knob = ui_knob::ArcKnob::for_param(
                                                &params.filter_wet,
                                                setter,
                                                KNOB_SIZE)
                                                .preset_style(ui_knob::KnobStyle::NewPresets1)
                                                .set_fill_color(*GUI_VALS.get("SYNTH_BARS_PURPLE").unwrap())
                                                .set_line_color(*GUI_VALS.get("SYNTH_MIDDLE_BLUE").unwrap())
                                                .set_text_size(TEXT_SIZE);
                                            ui.add(filter_wet_knob);

                                            let filter_resonance_knob = ui_knob::ArcKnob::for_param(
                                                &params.filter_resonance,
                                                setter,
                                                KNOB_SIZE)
                                                .preset_style(ui_knob::KnobStyle::NewPresets1)
                                                .set_fill_color(*GUI_VALS.get("SYNTH_BARS_PURPLE").unwrap())
                                                .set_line_color(*GUI_VALS.get("SYNTH_MIDDLE_BLUE").unwrap())
                                                .set_text_size(TEXT_SIZE);
                                            ui.add(filter_resonance_knob);
                                        });
                                        ui.vertical(|ui|{
                                            let filter_cutoff_knob = ui_knob::ArcKnob::for_param(
                                                &params.filter_cutoff,
                                                setter,
                                                KNOB_SIZE)
                                                .preset_style(ui_knob::KnobStyle::NewPresets1)
                                                .set_fill_color(*GUI_VALS.get("SYNTH_BARS_PURPLE").unwrap())
                                                .set_line_color(*GUI_VALS.get("SYNTH_MIDDLE_BLUE").unwrap())
                                                .set_text_size(TEXT_SIZE);
                                            ui.add(filter_cutoff_knob);

                                            let filter_res_type_knob = ui_knob::ArcKnob::for_param(
                                                &params.filter_res_type,
                                                setter,
                                                KNOB_SIZE)
                                                .preset_style(ui_knob::KnobStyle::NewPresets1)
                                                .set_fill_color(*GUI_VALS.get("SYNTH_BARS_PURPLE").unwrap())
                                                .set_line_color(*GUI_VALS.get("SYNTH_MIDDLE_BLUE").unwrap())
                                                .set_text_size(TEXT_SIZE);
                                            ui.add(filter_res_type_knob);
                                        });
                                        ui.vertical(|ui|{
                                            let filter_hp_knob = ui_knob::ArcKnob::for_param(
                                                &params.filter_hp_amount,
                                                setter,
                                                KNOB_SIZE)
                                                .preset_style(ui_knob::KnobStyle::NewPresets1)
                                                .set_fill_color(*GUI_VALS.get("SYNTH_BARS_PURPLE").unwrap())
                                                .set_line_color(*GUI_VALS.get("SYNTH_MIDDLE_BLUE").unwrap())
                                                .set_text_size(TEXT_SIZE);
                                            ui.add(filter_hp_knob);
                                            let filter_env_peak = ui_knob::ArcKnob::for_param(
                                                &params.filter_env_peak,
                                                setter,
                                                KNOB_SIZE)
                                                .preset_style(ui_knob::KnobStyle::NewPresets1)
                                                .set_fill_color(*GUI_VALS.get("SYNTH_BARS_PURPLE").unwrap())
                                                .set_line_color(*GUI_VALS.get("SYNTH_MIDDLE_BLUE").unwrap())
                                                .set_text_size(TEXT_SIZE);
                                            ui.add(filter_env_peak);
                                        });
                                        ui.vertical(|ui| {
                                            let filter_lp_knob = ui_knob::ArcKnob::for_param(
                                                &params.filter_lp_amount,
                                                setter,
                                                KNOB_SIZE)
                                                .preset_style(ui_knob::KnobStyle::NewPresets1)
                                                .set_fill_color(*GUI_VALS.get("SYNTH_BARS_PURPLE").unwrap())
                                                .set_line_color(*GUI_VALS.get("SYNTH_MIDDLE_BLUE").unwrap())
                                                .set_text_size(TEXT_SIZE);
                                            ui.add(filter_lp_knob);
                                            let filter_bp_knob = ui_knob::ArcKnob::for_param(
                                                &params.filter_bp_amount,
                                                setter,
                                                KNOB_SIZE)
                                                .preset_style(ui_knob::KnobStyle::NewPresets1)
                                                .set_fill_color(*GUI_VALS.get("SYNTH_BARS_PURPLE").unwrap())
                                                .set_line_color(*GUI_VALS.get("SYNTH_MIDDLE_BLUE").unwrap())
                                                .set_text_size(TEXT_SIZE);
                                            ui.add(filter_bp_knob);
                                        });

                                        // Middle bottom light section
                                        ui.painter().rect_filled(
                                            Rect::from_x_y_ranges(
                                                RangeInclusive::new((WIDTH as f32)*0.35, (WIDTH as f32)*0.64),
                                                RangeInclusive::new((HEIGHT as f32)*0.73, (HEIGHT as f32) - 4.0)),
                                            Rounding::from(16.0),
                                            *GUI_VALS.get("SYNTH_SOFT_BLUE").unwrap()
                                        );
                                        ui.painter().rect_filled(
                                            Rect::from_x_y_ranges(
                                                RangeInclusive::new((WIDTH as f32)*0.40, (WIDTH as f32)*0.58),
                                                RangeInclusive::new((HEIGHT as f32) - 42.0, (HEIGHT as f32) - 20.0)),
                                            Rounding::from(8.0),
                                            *GUI_VALS.get("A_BACKGROUND_COLOR_TOP").unwrap()
                                        );
                                    } else {
                                        ui.vertical(|ui|{
                                            let filter_wet_knob = ui_knob::ArcKnob::for_param(
                                                &params.filter_wet_2,
                                                setter,
                                                KNOB_SIZE)
                                                .preset_style(ui_knob::KnobStyle::NewPresets1)
                                                .set_fill_color(*GUI_VALS.get("SYNTH_BARS_PURPLE").unwrap())
                                                .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                                                .set_text_size(TEXT_SIZE);
                                            ui.add(filter_wet_knob);

                                            let filter_resonance_knob = ui_knob::ArcKnob::for_param(
                                                &params.filter_resonance_2,
                                                setter,
                                                KNOB_SIZE)
                                                .preset_style(ui_knob::KnobStyle::NewPresets1)
                                                .set_fill_color(*GUI_VALS.get("SYNTH_BARS_PURPLE").unwrap())
                                                .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                                                .set_text_size(TEXT_SIZE);
                                            ui.add(filter_resonance_knob);
                                        });
                                        ui.vertical(|ui|{
                                            let filter_cutoff_knob = ui_knob::ArcKnob::for_param(
                                                &params.filter_cutoff_2,
                                                setter,
                                                KNOB_SIZE)
                                                .preset_style(ui_knob::KnobStyle::NewPresets1)
                                                .set_fill_color(*GUI_VALS.get("SYNTH_BARS_PURPLE").unwrap())
                                                .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                                                .set_text_size(TEXT_SIZE);
                                            ui.add(filter_cutoff_knob);

                                            let filter_res_type_knob = ui_knob::ArcKnob::for_param(
                                                &params.filter_res_type_2,
                                                setter,
                                                KNOB_SIZE)
                                                .preset_style(ui_knob::KnobStyle::NewPresets1)
                                                .set_fill_color(*GUI_VALS.get("SYNTH_BARS_PURPLE").unwrap())
                                                .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                                                .set_text_size(TEXT_SIZE);
                                            ui.add(filter_res_type_knob);
                                        });
                                        ui.vertical(|ui|{
                                            let filter_hp_knob = ui_knob::ArcKnob::for_param(
                                                &params.filter_hp_amount_2,
                                                setter,
                                                KNOB_SIZE)
                                                .preset_style(ui_knob::KnobStyle::NewPresets1)
                                                .set_fill_color(*GUI_VALS.get("SYNTH_BARS_PURPLE").unwrap())
                                                .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                                                .set_text_size(TEXT_SIZE);
                                            ui.add(filter_hp_knob);
                                            let filter_env_peak = ui_knob::ArcKnob::for_param(
                                                &params.filter_env_peak_2,
                                                setter,
                                                KNOB_SIZE)
                                                .preset_style(ui_knob::KnobStyle::NewPresets1)
                                                .set_fill_color(*GUI_VALS.get("SYNTH_BARS_PURPLE").unwrap())
                                                .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                                                .set_text_size(TEXT_SIZE);
                                            ui.add(filter_env_peak);
                                        });
                                        ui.vertical(|ui| {
                                            let filter_lp_knob = ui_knob::ArcKnob::for_param(
                                                &params.filter_lp_amount_2,
                                                setter,
                                                KNOB_SIZE)
                                                .preset_style(ui_knob::KnobStyle::NewPresets1)
                                                .set_fill_color(*GUI_VALS.get("SYNTH_BARS_PURPLE").unwrap())
                                                .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                                                .set_text_size(TEXT_SIZE);
                                            ui.add(filter_lp_knob);
                                            let filter_bp_knob = ui_knob::ArcKnob::for_param(
                                                &params.filter_bp_amount_2,
                                                setter,
                                                KNOB_SIZE)
                                                .preset_style(ui_knob::KnobStyle::NewPresets1)
                                                .set_fill_color(*GUI_VALS.get("SYNTH_BARS_PURPLE").unwrap())
                                                .set_line_color(*GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap())
                                                .set_text_size(TEXT_SIZE);
                                            ui.add(filter_bp_knob);
                                        });

                                        // Middle bottom light section
                                        ui.painter().rect_filled(
                                            Rect::from_x_y_ranges(
                                                RangeInclusive::new((WIDTH as f32)*0.35, (WIDTH as f32)*0.64),
                                                RangeInclusive::new((HEIGHT as f32)*0.73, (HEIGHT as f32) - 4.0)),
                                            Rounding::from(16.0),
                                            *GUI_VALS.get("SYNTH_SOFT_BLUE2").unwrap()
                                        );
                                        ui.painter().rect_filled(
                                            Rect::from_x_y_ranges(
                                                RangeInclusive::new((WIDTH as f32)*0.40, (WIDTH as f32)*0.58),
                                                RangeInclusive::new((HEIGHT as f32) - 42.0, (HEIGHT as f32) - 20.0)),
                                            Rounding::from(8.0),
                                            *GUI_VALS.get("A_BACKGROUND_COLOR_TOP").unwrap()
                                        );
                                    }

                                    ////////////////////////////////////////////////////////////
                                    // ADSR FOR FILTER
                                    const VERT_BAR_HEIGHT: f32 = 106.0;
                                    //let VERT_BAR_HEIGHT_SHORTENED: f32 = VERT_BAR_HEIGHT - ui.spacing().interact_size.y;
                                    const VERT_BAR_WIDTH: f32 = 14.0;
                                    const HCURVE_WIDTH: f32 = 120.0;
                                    const HCURVE_BWIDTH: f32 = 28.0;

                                    ui.vertical(|ui| {
                                        ui.horizontal(|ui| {
                                            if *filter_select.lock().unwrap() == FilterSelect::Filter1 {
                                                // ADSR
                                                ui.add(
                                                    VerticalParamSlider::for_param(&params.filter_env_attack, setter)
                                                        .with_width(VERT_BAR_WIDTH)
                                                        .with_height(VERT_BAR_HEIGHT)
                                                        .set_reversed(true)
                                                        .override_colors(
                                                            *GUI_VALS.get("SYNTH_BARS_PURPLE").unwrap(),
                                                            *GUI_VALS.get("SYNTH_MIDDLE_BLUE").unwrap(),
                                                        ),
                                                );
                                                ui.add(
                                                    VerticalParamSlider::for_param(&params.filter_env_decay, setter)
                                                        .with_width(VERT_BAR_WIDTH)
                                                        .with_height(VERT_BAR_HEIGHT)
                                                        .set_reversed(true)
                                                        .override_colors(
                                                            *GUI_VALS.get("SYNTH_BARS_PURPLE").unwrap(),
                                                            *GUI_VALS.get("SYNTH_MIDDLE_BLUE").unwrap(),
                                                        ),
                                                );
                                                ui.add(
                                                    VerticalParamSlider::for_param(&params.filter_env_sustain, setter)
                                                        .with_width(VERT_BAR_WIDTH)
                                                        .with_height(VERT_BAR_HEIGHT)
                                                        .set_reversed(true)
                                                        .override_colors(
                                                            *GUI_VALS.get("SYNTH_BARS_PURPLE").unwrap(),
                                                            *GUI_VALS.get("SYNTH_MIDDLE_BLUE").unwrap(),
                                                        ),
                                                );
                                                ui.add(
                                                    VerticalParamSlider::for_param(&params.filter_env_release, setter)
                                                        .with_width(VERT_BAR_WIDTH)
                                                        .with_height(VERT_BAR_HEIGHT)
                                                        .set_reversed(true)
                                                        .override_colors(
                                                            *GUI_VALS.get("SYNTH_BARS_PURPLE").unwrap(),
                                                            *GUI_VALS.get("SYNTH_MIDDLE_BLUE").unwrap(),
                                                        ),
                                                );
                                            } else {
                                                // ADSR
                                                ui.add(
                                                    VerticalParamSlider::for_param(&params.filter_env_attack_2, setter)
                                                        .with_width(VERT_BAR_WIDTH)
                                                        .with_height(VERT_BAR_HEIGHT)
                                                        .set_reversed(true)
                                                        .override_colors(
                                                            *GUI_VALS.get("SYNTH_BARS_PURPLE").unwrap(),
                                                            *GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap(),
                                                        ),
                                                );
                                                ui.add(
                                                    VerticalParamSlider::for_param(&params.filter_env_decay_2, setter)
                                                        .with_width(VERT_BAR_WIDTH)
                                                        .with_height(VERT_BAR_HEIGHT)
                                                        .set_reversed(true)
                                                        .override_colors(
                                                            *GUI_VALS.get("SYNTH_BARS_PURPLE").unwrap(),
                                                            *GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap(),
                                                        ),
                                                );
                                                ui.add(
                                                    VerticalParamSlider::for_param(&params.filter_env_sustain_2, setter)
                                                        .with_width(VERT_BAR_WIDTH)
                                                        .with_height(VERT_BAR_HEIGHT)
                                                        .set_reversed(true)
                                                        .override_colors(
                                                            *GUI_VALS.get("SYNTH_BARS_PURPLE").unwrap(),
                                                            *GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap(),
                                                        ),
                                                );
                                                ui.add(
                                                    VerticalParamSlider::for_param(&params.filter_env_release_2, setter)
                                                        .with_width(VERT_BAR_WIDTH)
                                                        .with_height(VERT_BAR_HEIGHT)
                                                        .set_reversed(true)
                                                        .override_colors(
                                                            *GUI_VALS.get("SYNTH_BARS_PURPLE").unwrap(),
                                                            *GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap(),
                                                        ),
                                                );
                                            }
                                        });
                                    });

                                    ui.horizontal(|ui| {
                                        // Curve sliders
                                        ui.vertical(|ui| {
                                            if *filter_select.lock().unwrap() == FilterSelect::Filter1 {
                                                ui.add(
                                                    HorizontalParamSlider::for_param(&params.filter_env_atk_curve, setter)
                                                        .with_width(HCURVE_BWIDTH)
                                                        .set_left_sided_label(true)
                                                        .set_label_width(HCURVE_WIDTH)
                                                        .override_colors(
                                                            *GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap(), 
                                                            *GUI_VALS.get("SYNTH_MIDDLE_BLUE").unwrap()),
                                                );
                                                ui.add(
                                                    HorizontalParamSlider::for_param(&params.filter_env_dec_curve, setter)
                                                        .with_width(HCURVE_BWIDTH)
                                                        .set_left_sided_label(true)
                                                        .set_label_width(HCURVE_WIDTH)
                                                        .override_colors(
                                                            *GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap(), 
                                                            *GUI_VALS.get("SYNTH_MIDDLE_BLUE").unwrap()),
                                                );
                                                ui.add(
                                                    HorizontalParamSlider::for_param(&params.filter_env_rel_curve, setter)
                                                        .with_width(HCURVE_BWIDTH)
                                                        .set_left_sided_label(true)
                                                        .set_label_width(HCURVE_WIDTH)
                                                        .override_colors(
                                                            *GUI_VALS.get("DARK_GREY_UI_COLOR").unwrap(), 
                                                            *GUI_VALS.get("SYNTH_MIDDLE_BLUE").unwrap()),
                                                );
                                            } else {
                                                ui.add(
                                                    HorizontalParamSlider::for_param(&params.filter_env_atk_curve_2, setter)
                                                        .with_width(HCURVE_BWIDTH)
                                                        .set_left_sided_label(true)
                                                        .set_label_width(HCURVE_WIDTH)
                                                        .override_colors(
                                                            *GUI_VALS.get("SYNTH_BARS_PURPLE").unwrap(), 
                                                            *GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap()),
                                                );
                                                ui.add(
                                                    HorizontalParamSlider::for_param(&params.filter_env_dec_curve_2, setter)
                                                        .with_width(HCURVE_BWIDTH)
                                                        .set_left_sided_label(true)
                                                        .set_label_width(HCURVE_WIDTH)
                                                        .override_colors(
                                                            *GUI_VALS.get("SYNTH_BARS_PURPLE").unwrap(), 
                                                            *GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap()),
                                                );
                                                ui.add(
                                                    HorizontalParamSlider::for_param(&params.filter_env_rel_curve_2, setter)
                                                        .with_width(HCURVE_BWIDTH)
                                                        .set_left_sided_label(true)
                                                        .set_label_width(HCURVE_WIDTH)
                                                        .override_colors(
                                                            *GUI_VALS.get("SYNTH_BARS_PURPLE").unwrap(), 
                                                            *GUI_VALS.get("A_KNOB_OUTSIDE_COLOR").unwrap()),
                                                );
                                            }
                                            ui.horizontal(|ui|{
                                                ui.horizontal(|ui| {
                                                    ui.selectable_value(&mut *filter_select.lock().unwrap(), FilterSelect::Filter1, RichText::new("Filter 1").color(Color32::BLACK));
                                                    ui.selectable_value(&mut *filter_select.lock().unwrap(), FilterSelect::Filter2, RichText::new("Filter 2").color(Color32::BLACK));
                                                });
                                            });
                                        });
                                    });

                                    // Move Presets over!
                                    ui.add_space(8.0);

                                    ui.painter().rect_filled(
                                        Rect::from_x_y_ranges(
                                            RangeInclusive::new((WIDTH as f32)*0.64, (WIDTH as f32) - (synth_bar_space + 4.0)),
                                            RangeInclusive::new((HEIGHT as f32)*0.73, (HEIGHT as f32) - 4.0)),
                                        Rounding::from(16.0),
                                        *GUI_VALS.get("SYNTH_BARS_PURPLE").unwrap()
                                    );
                                    // Preset Display
                                    ui.vertical(|ui|{
                                        ui.horizontal(|ui|{
                                            // I know this is wonky
                                            ui.add_space(120.0);
                                            ui.label(RichText::new("Preset")
                                                .background_color(*GUI_VALS.get("SYNTH_BARS_PURPLE").unwrap())
                                                .color(*GUI_VALS.get("SYNTH_MIDDLE_BLUE").unwrap())
                                                .size(16.0));
                                            ui.label(RichText::new(current_preset_index.to_string())
                                                .background_color(*GUI_VALS.get("SYNTH_BARS_PURPLE").unwrap())
                                                .color(*GUI_VALS.get("SYNTH_MIDDLE_BLUE").unwrap())
                                                .size(16.0));
                                        });
                                        ui.horizontal(|ui|{
                                            let prev_preset_button = BoolButton::BoolButton::for_param(&params.param_prev_preset, setter, 2.0, 2.0, FONT);
                                            ui.add(prev_preset_button);
                                            let update_current_preset = BoolButton::BoolButton::for_param(&params.param_update_current_preset, setter, 8.0, 2.0, SMALLER_FONT);
                                            ui.add(update_current_preset);
                                            let next_preset_button = BoolButton::BoolButton::for_param(&params.param_next_preset, setter, 2.0, 2.0, FONT);
                                            ui.add(next_preset_button);
                                        });
                                        ui.horizontal(|ui|{
                                            ui.add_space(68.0);
                                            let load_bank_button = BoolButton::BoolButton::for_param(&params.param_load_bank, setter, 3.5, 2.0, SMALLER_FONT);
                                            ui.add(load_bank_button);
                                            let save_bank_button = BoolButton::BoolButton::for_param(&params.param_save_bank, setter, 3.5, 2.0, SMALLER_FONT);
                                            ui.add(save_bank_button);
                                        })
                                    });
                                });
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

                        if params.loading.value() || loading.load(Ordering::Relaxed) {
                            // Create the loading popup here.
                            let screen_size = Rect::from_x_y_ranges(
                                RangeInclusive::new(0.0, WIDTH as f32),
                                RangeInclusive::new(0.0, HEIGHT as f32));
                            let popup_size = Vec2::new(400.0, 200.0);
                            let popup_pos = screen_size.center();

                            // Draw the loading popup content here.
                            ui.painter().rect_filled(Rect::from_center_size(Pos2 { x: popup_pos.x, y: popup_pos.y }, popup_size), 10.0, Color32::GRAY);
                            ui.painter().text(popup_pos, Align2::CENTER_CENTER, "Loading...", LOADING_FONT, Color32::BLACK);
                        }
                    });
            },
        )
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        self.sample_rate = buffer_config.sample_rate;

        return true;
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        // Clear any voices on change of module type (especially during play)
        if self.clear_voices.clone().load(Ordering::Relaxed) {
            self.audio_module_1.as_ref().lock().unwrap().clear_voices();
            self.audio_module_2.as_ref().lock().unwrap().clear_voices();
            self.audio_module_3.as_ref().lock().unwrap().clear_voices();

            self.clear_voices.store(false, Ordering::Relaxed);
            self.update_something.store(true, Ordering::Relaxed);
        }
        self.process_midi(context, buffer);
        ProcessStatus::Normal
    }

    const HARD_REALTIME_ONLY: bool = false;

    fn task_executor(&mut self) -> TaskExecutor<Self> {
        // In the default implementation we can simply ignore the value
        Box::new(|_| ())
    }

    fn filter_state(_state: &mut PluginState) {}

    fn reset(&mut self) {}

    fn deactivate(&mut self) {}
}

impl Actuate {
    // Send midi events to the audio modules and let them process them - also send params so they can access
    fn process_midi(&mut self, context: &mut impl ProcessContext<Self>, buffer: &mut Buffer) {
        // Check if we're loading a file before process happens
        if self.params.load_sample_1.value() && self.file_dialog.load(Ordering::Relaxed) {
            self.file_dialog.store(true, Ordering::Relaxed);
            let sample_file = FileDialog::new()
                .add_filter("wav", &["wav"])
                //.set_directory("/")
                .pick_file();
            if Option::is_some(&sample_file) {
                self.audio_module_1
                    .as_ref()
                    .lock()
                    .unwrap()
                    .load_new_sample(sample_file.unwrap());
            }
        } else if self.params.load_sample_2.value() && self.file_dialog.load(Ordering::Relaxed) {
            self.file_dialog.store(true, Ordering::Relaxed);
            let sample_file = FileDialog::new()
                .add_filter("wav", &["wav"])
                //.set_directory("/")
                .pick_file();
            if Option::is_some(&sample_file) {
                self.audio_module_2
                    .as_ref()
                    .lock()
                    .unwrap()
                    .load_new_sample(sample_file.unwrap());
            }
        } else if self.params.load_sample_3.value() && self.file_dialog.load(Ordering::Relaxed) {
            self.file_dialog.store(true, Ordering::Relaxed);
            let sample_file = FileDialog::new()
                .add_filter("wav", &["wav"])
                //.set_directory("/")
                .pick_file();
            if Option::is_some(&sample_file) {
                self.audio_module_3
                    .as_ref()
                    .lock()
                    .unwrap()
                    .load_new_sample(sample_file.unwrap());
            }
        }

        for (sample_id, mut channel_samples) in buffer.iter_samples().enumerate() {
            // Get around post file loading breaking things with an arbitrary buffer
            if self.file_dialog.load(Ordering::Acquire) {
                self.file_open_buffer_timer.store(
                    self.file_open_buffer_timer.load(Ordering::Relaxed) + 1,
                    Ordering::Relaxed,
                );
                if self.file_open_buffer_timer.load(Ordering::Relaxed) > FILE_OPEN_BUFFER_MAX {
                    self.file_open_buffer_timer.store(0, Ordering::Relaxed);
                    self.file_dialog.store(false, Ordering::Release);
                }
            }

            // If the Load Bank button was pressed
            if self.load_bank.load(Ordering::Relaxed)
                && !self.file_dialog.load(Ordering::Relaxed)
                && self.file_open_buffer_timer.load(Ordering::Relaxed) == 0
                && !self.reload_entire_preset.load(Ordering::Acquire)
            {
                let AM1c = self.audio_module_1.clone();
                let AM2c = self.audio_module_2.clone();
                let AM3c = self.audio_module_3.clone();

                let mut AM1 = AM1c.lock().unwrap();
                let mut AM2 = AM2c.lock().unwrap();
                let mut AM3 = AM3c.lock().unwrap();

                // Load the preset bank
                self.file_dialog.store(true, Ordering::Relaxed);
                self.file_open_buffer_timer.store(1, Ordering::Relaxed);
                let unserialized: Vec<ActuatePreset>;
                (self.preset_lib_name, unserialized) = Actuate::load_preset_bank();
                self.load_bank.store(false, Ordering::Relaxed);

                let arc_lib: Arc<Mutex<Vec<ActuatePreset>>> = Arc::clone(&self.preset_lib);
                let mut locked_lib = arc_lib.lock().unwrap();

                // Load our items into our library from the unserialized save file
                for (item_index, item) in unserialized.iter().enumerate() {
                    // If our item exists then update it
                    if let Some(existing_item) = locked_lib.get_mut(item_index) {
                        *existing_item = item.clone();
                    } else {
                        // item_index is out of bounds in locked_lib
                        // These get dropped as the preset size should be the same all around
                    }
                }

                // Create missing samples on current preset
                AM1.regenerate_samples();
                AM2.regenerate_samples();
                AM3.regenerate_samples();

                // This is here again purposefully
                self.reload_entire_preset.store(true, Ordering::Release);
            }

            // If the save button has been pressed
            if self.save_bank.load(Ordering::Relaxed)
                && !self.file_dialog.load(Ordering::Relaxed)
                && self.file_open_buffer_timer.load(Ordering::Relaxed) == 0
            {
                self.file_dialog.store(true, Ordering::Relaxed);
                self.file_open_buffer_timer.store(1, Ordering::Relaxed);
                self.save_preset_bank();
                self.save_bank.store(false, Ordering::Relaxed);
            }

            // If the Update Current Preset button has been pressed
            if self.update_current_preset.load(Ordering::Relaxed)
                && !self.file_dialog.load(Ordering::Relaxed)
            {
                self.file_dialog.store(true, Ordering::Relaxed);
                self.file_open_buffer_timer.store(1, Ordering::Relaxed);
                self.update_current_preset();
                self.update_current_preset.store(false, Ordering::Relaxed);
            }

            // Prevent processing if our file dialog is open!!!
            if self.file_dialog.load(Ordering::Relaxed) {
                return;
            }

            // Processing
            /////////////////////////////////////////////////////////////////////////

            // Reset our output buffer signal
            *channel_samples.get_mut(0).unwrap() = 0.0;
            *channel_samples.get_mut(1).unwrap() = 0.0;

            // This weird bit is to stop playing when going from play to stop
            // but also allowing playing of the synth while stopped
            // midi choke doesn't seem to be working in FL
            if !context.transport().playing
                && (self.audio_module_1.clone().lock().unwrap().get_playing()
                    || self.audio_module_2.clone().lock().unwrap().get_playing()
                    || self.audio_module_3.clone().lock().unwrap().get_playing())
            {
                // Create clones here
                let AM1 = self.audio_module_1.clone();
                let AM2 = self.audio_module_1.clone();
                let AM3 = self.audio_module_1.clone();

                // For some reason this format works vs doing lock and storing it earlier
                AM1.lock().unwrap().set_playing(false);
                AM2.lock().unwrap().set_playing(false);
                AM3.lock().unwrap().set_playing(false);
                AM1.lock().unwrap().clear_voices();
                AM2.lock().unwrap().clear_voices();
                AM3.lock().unwrap().clear_voices();
            }
            if context.transport().playing {
                self.audio_module_1
                    .clone()
                    .lock()
                    .unwrap()
                    .set_playing(true);
                self.audio_module_2
                    .clone()
                    .lock()
                    .unwrap()
                    .set_playing(true);
                self.audio_module_3
                    .clone()
                    .lock()
                    .unwrap()
                    .set_playing(true);
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
            // These track if a note just finished to allow filter closing
            let mut note_off_filter_controller1: bool = false;
            let mut note_off_filter_controller2: bool = false;
            let mut note_off_filter_controller3: bool = false;

            // Trigger passing variables to the audio modules when the GUI input changes
            if self.update_something.load(Ordering::Relaxed) {
                self.audio_module_1
                    .clone()
                    .lock()
                    .unwrap()
                    .consume_params(self.params.clone(), 1);
                self.audio_module_2
                    .clone()
                    .lock()
                    .unwrap()
                    .consume_params(self.params.clone(), 2);
                self.audio_module_3
                    .clone()
                    .lock()
                    .unwrap()
                    .consume_params(self.params.clone(), 3);
                self.update_something.store(false, Ordering::Relaxed);
            }

            // Since File Dialog can be set by any of these we need to check each time
            if !self.file_dialog.load(Ordering::Relaxed)
                && self.params._audio_module_1_type.value() != AudioModuleType::Off
            {
                // We send our sample_id position, params, current midi event, module index, current voice max, and whether any params have changed
                (
                    wave1_l,
                    wave1_r,
                    reset_filter_controller1,
                    note_off_filter_controller1,
                ) = self.audio_module_1.clone().lock().unwrap().process(
                    sample_id,
                    midi_event.clone(),
                    sent_voice_max,
                );
                wave1_l *= self.params.audio_module_1_level.value();
                wave1_r *= self.params.audio_module_1_level.value();
            }
            if !self.file_dialog.load(Ordering::Relaxed)
                && self.params._audio_module_2_type.value() != AudioModuleType::Off
            {
                (
                    wave2_l,
                    wave2_r,
                    reset_filter_controller2,
                    note_off_filter_controller2,
                ) = self.audio_module_2.clone().lock().unwrap().process(
                    sample_id,
                    midi_event.clone(),
                    sent_voice_max,
                );
                wave2_l *= self.params.audio_module_2_level.value();
                wave2_r *= self.params.audio_module_2_level.value();
            }
            if !self.file_dialog.load(Ordering::Relaxed)
                && self.params._audio_module_3_type.value() != AudioModuleType::Off
            {
                (
                    wave3_l,
                    wave3_r,
                    reset_filter_controller3,
                    note_off_filter_controller3,
                ) = self.audio_module_3.clone().lock().unwrap().process(
                    sample_id,
                    midi_event.clone(),
                    sent_voice_max,
                );
                wave3_l *= self.params.audio_module_3_level.value();
                wave3_r *= self.params.audio_module_3_level.value();
            }

            // Define the outputs for filter routing or non-filter routing
            let mut left_output_filter1: f32 = 0.0;
            let mut right_output_filter1: f32 = 0.0;
            let mut left_output_filter2: f32 = 0.0;
            let mut right_output_filter2: f32 = 0.0;
            let mut left_output: f32 = 0.0;
            let mut right_output: f32 = 0.0;
            
            match self.params.audio_module_1_routing.value() {
                AMFilterRouting::Bypass => {
                    left_output += wave1_l;
                    right_output += wave1_r;
                },
                AMFilterRouting::Filter1 => {
                    left_output_filter1 += wave1_l;
                    right_output_filter1 += wave1_r;
                },
                AMFilterRouting::Filter2 => {
                    left_output_filter2 += wave1_l;
                    right_output_filter2 += wave1_r;
                }
                AMFilterRouting::Both => {
                    left_output_filter1 += wave1_l;
                    right_output_filter1 += wave1_r;
                    left_output_filter2 += wave1_l;
                    right_output_filter2 += wave1_r;
                }
            }
            #[allow(unused_assignments)]
            match self.params.audio_module_2_routing.value() {
                AMFilterRouting::Bypass => {
                    left_output += wave2_l;
                    right_output += wave2_r;
                },
                AMFilterRouting::Filter1 => {
                    left_output_filter1 += wave2_l;
                    right_output_filter1 += wave2_r;
                },
                AMFilterRouting::Filter2 => {
                    left_output_filter2 += wave2_l;
                    right_output_filter2 += wave2_r;
                }
                AMFilterRouting::Both => {
                    left_output_filter1 += wave2_l;
                    right_output_filter1 += wave2_r;
                    left_output_filter2 += wave2_l;
                    right_output_filter2 += wave2_r;
                }
            }
            #[allow(unused_assignments)]
            match self.params.audio_module_3_routing.value() {
                AMFilterRouting::Bypass => {
                    left_output += wave3_l;
                    right_output += wave3_r;
                },
                AMFilterRouting::Filter1 => {
                    left_output_filter1 += wave3_l;
                    right_output_filter1 += wave3_r;
                },
                AMFilterRouting::Filter2 => {
                    left_output_filter1 += wave3_l;
                    right_output_filter2 += wave3_r;
                }
                AMFilterRouting::Both => {
                    left_output_filter1 += wave3_l;
                    right_output_filter1 += wave3_r;
                    left_output_filter2 += wave3_l;
                    right_output_filter2 += wave3_r;
                }
            }

            // Filter 1 Processing
            ///////////////////////////////////////////////////////////////

            if self.params.filter_wet.value() > 0.0 && !self.file_dialog.load(Ordering::Relaxed) {
                // Filter state movement code
                //////////////////////////////////////////

                // If a note is ending and we should enter releasing
                if note_off_filter_controller1
                    || note_off_filter_controller2
                    || note_off_filter_controller3
                {
                    self.filter_state_1 = OscState::Releasing;

                    self.filter_rel_smoother_1 = match self.params.filter_env_rel_curve.value() {
                        SmoothStyle::Linear => Smoother::new(SmoothingStyle::Linear(
                            self.params.filter_env_release.value(),
                        )),
                        SmoothStyle::Logarithmic => Smoother::new(SmoothingStyle::Logarithmic(
                            self.params.filter_env_release.value(),
                        )),
                        SmoothStyle::Exponential => Smoother::new(SmoothingStyle::Exponential(
                            self.params.filter_env_release.value(),
                        )),
                    };

                    // Reset our filter release to be at sustain level to start
                    self.filter_rel_smoother_1.reset(
                        self.params.filter_cutoff.value()
                            * (self.params.filter_env_sustain.value() / 999.9),
                    );

                    // Move release to the cutoff to end
                    self.filter_rel_smoother_1
                        .set_target(self.sample_rate, self.params.filter_cutoff.value());
                }

                // Try to trigger our filter mods on note on! This is sequential/single because we just need a trigger at a point in time
                if reset_filter_controller1 || reset_filter_controller2 || reset_filter_controller3
                {
                    // Set our filter in attack state
                    self.filter_state_1 = OscState::Attacking;

                    // Consume our params for smoothing
                    self.filter_atk_smoother_1 = match self.params.filter_env_atk_curve.value() {
                        SmoothStyle::Linear => Smoother::new(SmoothingStyle::Linear(
                            self.params.filter_env_attack.value(),
                        )),
                        SmoothStyle::Logarithmic => Smoother::new(SmoothingStyle::Logarithmic(
                            self.params.filter_env_attack.value(),
                        )),
                        SmoothStyle::Exponential => Smoother::new(SmoothingStyle::Exponential(
                            self.params.filter_env_attack.value(),
                        )),
                    };

                    // Reset our attack to start from the filter cutoff
                    self.filter_atk_smoother_1
                        .reset(self.params.filter_cutoff.value());

                    // Since we're in attack state at the start of our note we need to setup the attack going to the env peak
                    self.filter_atk_smoother_1.set_target(
                        self.sample_rate,
                        (self.params.filter_cutoff.value() + self.params.filter_env_peak.value())
                            .clamp(20.0, 16000.0),
                    );
                }

                // If our attack has finished
                if self.filter_atk_smoother_1.steps_left() == 0
                    && self.filter_state_1 == OscState::Attacking
                {
                    self.filter_state_1 = OscState::Decaying;

                    self.filter_dec_smoother_1 = match self.params.filter_env_dec_curve.value() {
                        SmoothStyle::Linear => Smoother::new(SmoothingStyle::Linear(
                            self.params.filter_env_decay.value(),
                        )),
                        SmoothStyle::Logarithmic => Smoother::new(SmoothingStyle::Logarithmic(
                            self.params.filter_env_decay.value(),
                        )),
                        SmoothStyle::Exponential => Smoother::new(SmoothingStyle::Exponential(
                            self.params.filter_env_decay.value(),
                        )),
                    };

                    // This makes our filter decay start at env peak point
                    self.filter_dec_smoother_1.reset(
                        (self.params.filter_cutoff.value() + self.params.filter_env_peak.value())
                            .clamp(20.0, 16000.0),
                    );

                    // Set up the smoother for our filter movement to go from our decay point to our sustain point
                    self.filter_dec_smoother_1.set_target(
                        self.sample_rate,
                        self.params.filter_cutoff.value()
                            * (self.params.filter_env_sustain.value() / 999.9),
                    );
                }
                // If our decay has finished move to sustain state
                if self.filter_dec_smoother_1.steps_left() == 0
                    && self.filter_state_1 == OscState::Decaying
                {
                    self.filter_state_1 = OscState::Sustaining;
                }

                // use proper variable now that there are four filters and multiple states
                let next_filter_step = match self.filter_state_1 {
                    OscState::Attacking => self.filter_atk_smoother_1.next(),
                    OscState::Decaying | OscState::Sustaining => self.filter_dec_smoother_1.next(),
                    OscState::Releasing => self.filter_rel_smoother_1.next(),
                    // I don't expect this to be used
                    _ => self.params.filter_cutoff.value(),
                };

                // Filtering before output
                self.filter_l_1.update(
                    next_filter_step,
                    self.params.filter_resonance.value(),
                    self.sample_rate,
                    self.params.filter_res_type.value(),
                );
                self.filter_r_1.update(
                    next_filter_step,
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

                (low_l, band_l, high_l) = self.filter_l_1.process(left_output_filter1);
                (low_r, band_r, high_r) = self.filter_r_1.process(right_output_filter1);

                left_output += (low_l * self.params.filter_lp_amount.value()
                    + band_l * self.params.filter_bp_amount.value()
                    + high_l * self.params.filter_hp_amount.value())
                    * self.params.filter_wet.value()
                    + left_output * (1.0 - self.params.filter_wet.value());

                right_output += (low_r * self.params.filter_lp_amount.value()
                    + band_r * self.params.filter_bp_amount.value()
                    + high_r * self.params.filter_hp_amount.value())
                    * self.params.filter_wet.value()
                    + right_output * (1.0 - self.params.filter_wet.value());
            }

            // Filter 2 Processing
            ///////////////////////////////////////////////////////////////

            if self.params.filter_wet_2.value() > 0.0 && !self.file_dialog.load(Ordering::Relaxed) {
                // Filter state movement code
                //////////////////////////////////////////

                // If a note is ending and we should enter releasing
                if note_off_filter_controller1
                    || note_off_filter_controller2
                    || note_off_filter_controller3
                {
                    self.filter_state_2 = OscState::Releasing;

                    self.filter_rel_smoother_2 = match self.params.filter_env_rel_curve_2.value() {
                        SmoothStyle::Linear => Smoother::new(SmoothingStyle::Linear(
                            self.params.filter_env_release_2.value(),
                        )),
                        SmoothStyle::Logarithmic => Smoother::new(SmoothingStyle::Logarithmic(
                            self.params.filter_env_release_2.value(),
                        )),
                        SmoothStyle::Exponential => Smoother::new(SmoothingStyle::Exponential(
                            self.params.filter_env_release_2.value(),
                        )),
                    };

                    // Reset our filter release to be at sustain level to start
                    self.filter_rel_smoother_2.reset(
                        self.params.filter_cutoff_2.value()
                            * (self.params.filter_env_sustain_2.value() / 999.9),
                    );

                    // Move release to the cutoff to end
                    self.filter_rel_smoother_2
                        .set_target(self.sample_rate, self.params.filter_cutoff_2.value());
                }

                // Try to trigger our filter mods on note on! This is sequential/single because we just need a trigger at a point in time
                if reset_filter_controller1 || reset_filter_controller2 || reset_filter_controller3
                {
                    // Set our filter in attack state
                    self.filter_state_2 = OscState::Attacking;

                    // Consume our params for smoothing
                    self.filter_atk_smoother_2 = match self.params.filter_env_atk_curve_2.value() {
                        SmoothStyle::Linear => Smoother::new(SmoothingStyle::Linear(
                            self.params.filter_env_attack_2.value(),
                        )),
                        SmoothStyle::Logarithmic => Smoother::new(SmoothingStyle::Logarithmic(
                            self.params.filter_env_attack_2.value(),
                        )),
                        SmoothStyle::Exponential => Smoother::new(SmoothingStyle::Exponential(
                            self.params.filter_env_attack_2.value(),
                        )),
                    };

                    // Reset our attack to start from the filter cutoff
                    self.filter_atk_smoother_2
                        .reset(self.params.filter_cutoff_2.value());

                    // Since we're in attack state at the start of our note we need to setup the attack going to the env peak
                    self.filter_atk_smoother_2.set_target(
                        self.sample_rate,
                        (self.params.filter_cutoff_2.value()
                            + self.params.filter_env_peak_2.value())
                        .clamp(20.0, 16000.0),
                    );
                }

                // If our attack has finished
                if self.filter_atk_smoother_2.steps_left() == 0
                    && self.filter_state_2 == OscState::Attacking
                {
                    self.filter_state_2 = OscState::Decaying;

                    self.filter_dec_smoother_2 = match self.params.filter_env_dec_curve_2.value() {
                        SmoothStyle::Linear => Smoother::new(SmoothingStyle::Linear(
                            self.params.filter_env_decay_2.value(),
                        )),
                        SmoothStyle::Logarithmic => Smoother::new(SmoothingStyle::Logarithmic(
                            self.params.filter_env_decay_2.value(),
                        )),
                        SmoothStyle::Exponential => Smoother::new(SmoothingStyle::Exponential(
                            self.params.filter_env_decay_2.value(),
                        )),
                    };

                    // This makes our filter decay start at env peak point
                    self.filter_dec_smoother_2.reset(
                        (self.params.filter_cutoff_2.value()
                            + self.params.filter_env_peak_2.value())
                        .clamp(20.0, 16000.0),
                    );

                    // Set up the smoother for our filter movement to go from our decay point to our sustain point
                    self.filter_dec_smoother_2.set_target(
                        self.sample_rate,
                        self.params.filter_cutoff_2.value()
                            * (self.params.filter_env_sustain_2.value() / 999.9),
                    );
                }
                // If our decay has finished move to sustain state
                if self.filter_dec_smoother_2.steps_left() == 0
                    && self.filter_state_2 == OscState::Decaying
                {
                    self.filter_state_2 = OscState::Sustaining;
                }

                // use proper variable now that there are four filters and multiple states
                let next_filter_step = match self.filter_state_2 {
                    OscState::Attacking => self.filter_atk_smoother_2.next(),
                    OscState::Decaying | OscState::Sustaining => self.filter_dec_smoother_2.next(),
                    OscState::Releasing => self.filter_rel_smoother_2.next(),
                    // I don't expect this to be used
                    _ => self.params.filter_cutoff_2.value(),
                };

                // Filtering before output
                self.filter_l_2.update(
                    next_filter_step,
                    self.params.filter_resonance_2.value(),
                    self.sample_rate,
                    self.params.filter_res_type_2.value(),
                );
                self.filter_r_2.update(
                    next_filter_step,
                    self.params.filter_resonance_2.value(),
                    self.sample_rate,
                    self.params.filter_res_type_2.value(),
                );

                let low_l: f32;
                let band_l: f32;
                let high_l: f32;
                let low_r: f32;
                let band_r: f32;
                let high_r: f32;

                (low_l, band_l, high_l) = self.filter_l_2.process(left_output_filter2);
                (low_r, band_r, high_r) = self.filter_r_2.process(right_output_filter2);

                left_output += (low_l * self.params.filter_lp_amount_2.value()
                    + band_l * self.params.filter_bp_amount_2.value()
                    + high_l * self.params.filter_hp_amount_2.value())
                    * self.params.filter_wet_2.value()
                    + left_output * (1.0 - self.params.filter_wet_2.value());

                right_output += (low_r * self.params.filter_lp_amount_2.value()
                    + band_r * self.params.filter_bp_amount_2.value()
                    + high_r * self.params.filter_hp_amount_2.value())
                    * self.params.filter_wet_2.value()
                    + right_output * (1.0 - self.params.filter_wet_2.value());
            }

            // DC Offset Removal
            ////////////////////////////////////////////////////////////////////////////////////////

            if !self.file_dialog.load(Ordering::Relaxed) {
                // Remove DC Offsets with our SVF
                self.dc_filter_l
                    .update(20.0, 0.8, self.sample_rate, ResonanceType::Default);
                self.dc_filter_r
                    .update(20.0, 0.8, self.sample_rate, ResonanceType::Default);
                (_, _, left_output) = self.dc_filter_l.process(left_output);
                (_, _, right_output) = self.dc_filter_r.process(right_output);
            }

            // Final output to DAW
            ////////////////////////////////////////////////////////////////////////////////////////

            // Reassign our output signal
            *channel_samples.get_mut(0).unwrap() = left_output * self.params.master_level.value();
            *channel_samples.get_mut(1).unwrap() = right_output * self.params.master_level.value();
        }
    }

    // Load presets
    fn load_preset_bank() -> (String, Vec<ActuatePreset>) {
        let loading_bank = FileDialog::new()
            .add_filter("bin", &["bin"]) // Use the same filter as in save_preset_bank
            .pick_file();
        let return_name;

        if let Some(loading_bank) = loading_bank {
            return_name = loading_bank.to_str().unwrap_or("Invalid Path").to_string();

            // Read the compressed data from the file
            let mut compressed_data = Vec::new();
            if let Err(err) = std::fs::File::open(&return_name)
                .and_then(|mut file| file.read_to_end(&mut compressed_data))
            {
                eprintln!("Error reading compressed data from file: {}", err);
                return (err.to_string(), Vec::new());
            }

            // Decompress the data
            let decompressed_data = Self::decompress_bytes(&compressed_data);
            if let Err(err) = decompressed_data {
                eprintln!("Error decompressing data: {}", err);
                return (err.to_string(), Vec::new());
            }

            // Deserialize the MessagePack data
            let file_string_data = decompressed_data.unwrap();

            // Deserialize into ActuatePreset - return default empty lib if error
            let unserialized: Vec<ActuatePreset> = rmp_serde::from_slice(&file_string_data)
                .unwrap_or(vec![
                    ActuatePreset {
                        mod1_audio_module_type: AudioModuleType::Osc,
                        mod1_audio_module_level: 1.0,
                        mod1_loaded_sample: vec![vec![0.0, 0.0]],
                        mod1_sample_lib: vec![vec![vec![0.0, 0.0]]],
                        mod1_loop_wavetable: false,
                        mod1_single_cycle: false,
                        mod1_restretch: true,
                        mod1_prev_restretch: false,
                        mod1_grain_hold: 200,
                        mod1_grain_gap: 200,
                        mod1_start_position: 0.0,
                        mod1_end_position: 1.0,
                        mod1_grain_crossfade: 50,
                        mod1_osc_type: VoiceType::Sine,
                        mod1_osc_octave: 0,
                        mod1_osc_semitones: 0,
                        mod1_osc_detune: 0.0,
                        mod1_osc_attack: 0.0001,
                        mod1_osc_decay: 0.0001,
                        mod1_osc_sustain: 999.9,
                        mod1_osc_release: 5.0,
                        mod1_osc_retrigger: RetriggerStyle::Retrigger,
                        mod1_osc_atk_curve: SmoothStyle::Linear,
                        mod1_osc_dec_curve: SmoothStyle::Linear,
                        mod1_osc_rel_curve: SmoothStyle::Linear,
                        mod1_osc_unison: 1,
                        mod1_osc_unison_detune: 0.0,
                        mod1_osc_stereo: 0.0,

                        mod2_audio_module_type: AudioModuleType::Off,
                        mod2_audio_module_level: 1.0,
                        mod2_loaded_sample: vec![vec![0.0, 0.0]],
                        mod2_sample_lib: vec![vec![vec![0.0, 0.0]]],
                        mod2_loop_wavetable: false,
                        mod2_single_cycle: false,
                        mod2_restretch: true,
                        mod2_prev_restretch: false,
                        mod2_grain_hold: 200,
                        mod2_grain_gap: 200,
                        mod2_start_position: 0.0,
                        mod2_end_position: 1.0,
                        mod2_grain_crossfade: 50,
                        mod2_osc_type: VoiceType::Sine,
                        mod2_osc_octave: 0,
                        mod2_osc_semitones: 0,
                        mod2_osc_detune: 0.0,
                        mod2_osc_attack: 0.0001,
                        mod2_osc_decay: 0.0001,
                        mod2_osc_sustain: 999.9,
                        mod2_osc_release: 5.0,
                        mod2_osc_retrigger: RetriggerStyle::Retrigger,
                        mod2_osc_atk_curve: SmoothStyle::Linear,
                        mod2_osc_dec_curve: SmoothStyle::Linear,
                        mod2_osc_rel_curve: SmoothStyle::Linear,
                        mod2_osc_unison: 1,
                        mod2_osc_unison_detune: 0.0,
                        mod2_osc_stereo: 0.0,

                        mod3_audio_module_type: AudioModuleType::Off,
                        mod3_audio_module_level: 1.0,
                        mod3_loaded_sample: vec![vec![0.0, 0.0]],
                        mod3_sample_lib: vec![vec![vec![0.0, 0.0]]],
                        mod3_loop_wavetable: false,
                        mod3_single_cycle: false,
                        mod3_restretch: true,
                        mod3_prev_restretch: false,
                        mod3_grain_hold: 200,
                        mod3_grain_gap: 200,
                        mod3_start_position: 0.0,
                        mod3_end_position: 1.0,
                        mod3_grain_crossfade: 50,
                        mod3_osc_type: VoiceType::Sine,
                        mod3_osc_octave: 0,
                        mod3_osc_semitones: 0,
                        mod3_osc_detune: 0.0,
                        mod3_osc_attack: 0.0001,
                        mod3_osc_decay: 0.0001,
                        mod3_osc_sustain: 999.9,
                        mod3_osc_release: 5.0,
                        mod3_osc_retrigger: RetriggerStyle::Retrigger,
                        mod3_osc_atk_curve: SmoothStyle::Linear,
                        mod3_osc_dec_curve: SmoothStyle::Linear,
                        mod3_osc_rel_curve: SmoothStyle::Linear,
                        mod3_osc_unison: 1,
                        mod3_osc_unison_detune: 0.0,
                        mod3_osc_stereo: 0.0,

                        filter_wet: 1.0,
                        filter_cutoff: 4000.0,
                        filter_resonance: 1.0,
                        filter_res_type: ResonanceType::Default,
                        filter_lp_amount: 1.0,
                        filter_hp_amount: 0.0,
                        filter_bp_amount: 0.0,
                        filter_env_peak: 0.0,
                        filter_env_attack: 0.0001,
                        filter_env_decay: 250.0,
                        filter_env_sustain: 999.9,
                        filter_env_release: 0.0001,
                        filter_env_atk_curve: SmoothStyle::Linear,
                        filter_env_dec_curve: SmoothStyle::Linear,
                        filter_env_rel_curve: SmoothStyle::Linear,

                        filter_wet_2: 0.0,
                        filter_cutoff_2: 4000.0,
                        filter_resonance_2: 1.0,
                        filter_res_type_2: ResonanceType::Default,
                        filter_lp_amount_2: 1.0,
                        filter_hp_amount_2: 0.0,
                        filter_bp_amount_2: 0.0,
                        filter_env_peak_2: 0.0,
                        filter_env_attack_2: 0.0001,
                        filter_env_decay_2: 250.0,
                        filter_env_sustain_2: 999.9,
                        filter_env_release_2: 0.0001,
                        filter_env_atk_curve_2: SmoothStyle::Linear,
                        filter_env_dec_curve_2: SmoothStyle::Linear,
                        filter_env_rel_curve_2: SmoothStyle::Linear,

                        // LFOs
                        lfo1_freq: 2.0,
                        lfo1_retrigger: LFOController::LFORetrigger::None,
                        lfo1_sync: true,
                        lfo1_snap: LFOController::LFOSnapValues::Half,
                        lfo1_waveform: LFOController::Waveform::Sine,
                        lfo1_phase: 0.0,

                        lfo2_freq: 2.0,
                        lfo2_retrigger: LFOController::LFORetrigger::None,
                        lfo2_sync: true,
                        lfo2_snap: LFOController::LFOSnapValues::Half,
                        lfo2_waveform: LFOController::Waveform::Sine,
                        lfo2_phase: 0.0,

                        lfo3_freq: 2.0,
                        lfo3_retrigger: LFOController::LFORetrigger::None,
                        lfo3_sync: true,
                        lfo3_snap: LFOController::LFOSnapValues::Half,
                        lfo3_waveform: LFOController::Waveform::Sine,
                        lfo3_phase: 0.0,
                    };
                    PRESET_BANK_SIZE
                ]);

            return (return_name, unserialized);
        }
        return (String::from("Error"), Vec::new());
    }

    fn reload_entire_preset(
        setter: &ParamSetter,
        params: Arc<ActuateParams>,
        current_preset_index: usize,
        arc_preset: Arc<Mutex<Vec<ActuatePreset>>>,
        AMod1: Arc<Mutex<AudioModule>>,
        AMod2: Arc<Mutex<AudioModule>>,
        AMod3: Arc<Mutex<AudioModule>>,
    ) {
        // Create mutex locks for AudioModule
        let mut AMod1 = AMod1.as_ref().lock().unwrap();
        let mut AMod2 = AMod2.as_ref().lock().unwrap();
        let mut AMod3 = AMod3.as_ref().lock().unwrap();

        // Try to load preset into our params if possible
        let loaded_preset = &arc_preset.lock().unwrap()[current_preset_index as usize];
        setter.set_parameter(
            &params._audio_module_1_type,
            loaded_preset.mod1_audio_module_type,
        );
        setter.set_parameter(
            &params.audio_module_1_level,
            loaded_preset.mod1_audio_module_level,
        );
        setter.set_parameter(&params.loop_sample_1, loaded_preset.mod1_loop_wavetable);
        setter.set_parameter(&params.single_cycle_1, loaded_preset.mod1_single_cycle);
        setter.set_parameter(&params.restretch_1, loaded_preset.mod1_restretch);
        setter.set_parameter(&params.osc_1_type, loaded_preset.mod1_osc_type);
        setter.set_parameter(&params.osc_1_octave, loaded_preset.mod1_osc_octave);
        setter.set_parameter(&params.osc_1_semitones, loaded_preset.mod1_osc_semitones);
        setter.set_parameter(&params.osc_1_detune, loaded_preset.mod1_osc_detune);
        setter.set_parameter(&params.osc_1_attack, loaded_preset.mod1_osc_attack);
        setter.set_parameter(&params.osc_1_decay, loaded_preset.mod1_osc_decay);
        setter.set_parameter(&params.osc_1_sustain, loaded_preset.mod1_osc_sustain);
        setter.set_parameter(&params.osc_1_release, loaded_preset.mod1_osc_release);
        setter.set_parameter(&params.osc_1_retrigger, loaded_preset.mod1_osc_retrigger);
        setter.set_parameter(&params.osc_1_atk_curve, loaded_preset.mod1_osc_atk_curve);
        setter.set_parameter(&params.osc_1_dec_curve, loaded_preset.mod1_osc_dec_curve);
        setter.set_parameter(&params.osc_1_rel_curve, loaded_preset.mod1_osc_rel_curve);
        setter.set_parameter(&params.osc_1_unison, loaded_preset.mod1_osc_unison);
        setter.set_parameter(
            &params.osc_1_unison_detune,
            loaded_preset.mod1_osc_unison_detune,
        );
        setter.set_parameter(&params.osc_1_stereo, loaded_preset.mod1_osc_stereo);
        setter.set_parameter(&params.grain_gap_1, loaded_preset.mod1_grain_gap);
        setter.set_parameter(&params.grain_hold_1, loaded_preset.mod1_grain_hold);
        setter.set_parameter(
            &params.grain_crossfade_1,
            loaded_preset.mod1_grain_crossfade,
        );
        setter.set_parameter(&params.start_position_1, loaded_preset.mod1_start_position);
        setter.set_parameter(&params.end_position_1, loaded_preset.mod1_end_position);
        // loaded sample, sample_lib, and prev restretch are controlled differently
        setter.set_parameter(
            &params._audio_module_2_type,
            loaded_preset.mod2_audio_module_type,
        );
        setter.set_parameter(
            &params.audio_module_2_level,
            loaded_preset.mod2_audio_module_level,
        );
        setter.set_parameter(&params.loop_sample_2, loaded_preset.mod2_loop_wavetable);
        setter.set_parameter(&params.single_cycle_2, loaded_preset.mod2_single_cycle);
        setter.set_parameter(&params.restretch_2, loaded_preset.mod2_restretch);
        setter.set_parameter(&params.osc_2_type, loaded_preset.mod2_osc_type);
        setter.set_parameter(&params.osc_2_octave, loaded_preset.mod2_osc_octave);
        setter.set_parameter(&params.osc_2_semitones, loaded_preset.mod2_osc_semitones);
        setter.set_parameter(&params.osc_2_detune, loaded_preset.mod2_osc_detune);
        setter.set_parameter(&params.osc_2_attack, loaded_preset.mod2_osc_attack);
        setter.set_parameter(&params.osc_2_decay, loaded_preset.mod2_osc_decay);
        setter.set_parameter(&params.osc_2_sustain, loaded_preset.mod2_osc_sustain);
        setter.set_parameter(&params.osc_2_release, loaded_preset.mod2_osc_release);
        setter.set_parameter(&params.osc_2_retrigger, loaded_preset.mod2_osc_retrigger);
        setter.set_parameter(&params.osc_2_atk_curve, loaded_preset.mod2_osc_atk_curve);
        setter.set_parameter(&params.osc_2_dec_curve, loaded_preset.mod2_osc_dec_curve);
        setter.set_parameter(&params.osc_2_rel_curve, loaded_preset.mod2_osc_rel_curve);
        setter.set_parameter(&params.osc_2_unison, loaded_preset.mod2_osc_unison);
        setter.set_parameter(
            &params.osc_2_unison_detune,
            loaded_preset.mod2_osc_unison_detune,
        );
        setter.set_parameter(&params.osc_2_stereo, loaded_preset.mod2_osc_stereo);
        setter.set_parameter(&params.grain_gap_2, loaded_preset.mod2_grain_gap);
        setter.set_parameter(&params.grain_hold_2, loaded_preset.mod2_grain_hold);
        setter.set_parameter(
            &params.grain_crossfade_2,
            loaded_preset.mod2_grain_crossfade,
        );
        setter.set_parameter(&params.start_position_2, loaded_preset.mod2_start_position);
        setter.set_parameter(&params.end_position_2, loaded_preset.mod2_end_position);
        // loaded sample, sample_lib, and prev restretch are controlled differently
        setter.set_parameter(
            &params._audio_module_3_type,
            loaded_preset.mod3_audio_module_type,
        );
        setter.set_parameter(
            &params.audio_module_3_level,
            loaded_preset.mod3_audio_module_level,
        );
        setter.set_parameter(&params.loop_sample_3, loaded_preset.mod3_loop_wavetable);
        setter.set_parameter(&params.single_cycle_3, loaded_preset.mod3_single_cycle);
        setter.set_parameter(&params.restretch_3, loaded_preset.mod3_restretch);
        setter.set_parameter(&params.osc_3_type, loaded_preset.mod3_osc_type);
        setter.set_parameter(&params.osc_3_octave, loaded_preset.mod3_osc_octave);
        setter.set_parameter(&params.osc_3_semitones, loaded_preset.mod3_osc_semitones);
        setter.set_parameter(&params.osc_3_detune, loaded_preset.mod3_osc_detune);
        setter.set_parameter(&params.osc_3_attack, loaded_preset.mod3_osc_attack);
        setter.set_parameter(&params.osc_3_decay, loaded_preset.mod3_osc_decay);
        setter.set_parameter(&params.osc_3_sustain, loaded_preset.mod3_osc_sustain);
        setter.set_parameter(&params.osc_3_release, loaded_preset.mod3_osc_release);
        setter.set_parameter(&params.osc_3_retrigger, loaded_preset.mod3_osc_retrigger);
        setter.set_parameter(&params.osc_3_atk_curve, loaded_preset.mod3_osc_atk_curve);
        setter.set_parameter(&params.osc_3_dec_curve, loaded_preset.mod3_osc_dec_curve);
        setter.set_parameter(&params.osc_3_rel_curve, loaded_preset.mod3_osc_rel_curve);
        setter.set_parameter(&params.osc_3_unison, loaded_preset.mod3_osc_unison);
        setter.set_parameter(
            &params.osc_3_unison_detune,
            loaded_preset.mod3_osc_unison_detune,
        );
        setter.set_parameter(&params.osc_3_stereo, loaded_preset.mod3_osc_stereo);
        setter.set_parameter(&params.grain_gap_3, loaded_preset.mod3_grain_gap);
        setter.set_parameter(&params.grain_hold_3, loaded_preset.mod3_grain_hold);
        setter.set_parameter(
            &params.grain_crossfade_3,
            loaded_preset.mod3_grain_crossfade,
        );
        setter.set_parameter(&params.start_position_3, loaded_preset.mod3_start_position);
        setter.set_parameter(&params.end_position_3, loaded_preset.mod3_end_position);
        setter.set_parameter(&params.filter_wet, loaded_preset.filter_wet);
        setter.set_parameter(&params.filter_cutoff, loaded_preset.filter_cutoff);
        setter.set_parameter(&params.filter_resonance, loaded_preset.filter_resonance);
        setter.set_parameter(
            &params.filter_res_type,
            loaded_preset.filter_res_type.clone(),
        );
        setter.set_parameter(&params.filter_lp_amount, loaded_preset.filter_lp_amount);
        setter.set_parameter(&params.filter_hp_amount, loaded_preset.filter_hp_amount);
        setter.set_parameter(&params.filter_bp_amount, loaded_preset.filter_bp_amount);
        setter.set_parameter(&params.filter_env_peak, loaded_preset.filter_env_peak);
        setter.set_parameter(&params.filter_env_decay, loaded_preset.filter_env_decay);
        setter.set_parameter(
            &params.filter_env_atk_curve,
            loaded_preset.filter_env_atk_curve,
        );
        setter.set_parameter(
            &params.filter_env_dec_curve,
            loaded_preset.filter_env_dec_curve,
        );
        setter.set_parameter(
            &params.filter_env_rel_curve,
            loaded_preset.filter_env_rel_curve,
        );

        // Load the non-gui related preset stuff!
        /*
        setter: &ParamSetter,
        params: Arc<ActuateParams>,
        current_preset_index: usize,
        arc_preset: Arc<Mutex<Vec<ActuatePreset>>>,
        */
        AMod1.loaded_sample = loaded_preset.mod1_loaded_sample.clone();
        AMod1.sample_lib = loaded_preset.mod1_sample_lib.clone();
        AMod1.restretch = loaded_preset.mod1_restretch;

        AMod2.loaded_sample = loaded_preset.mod1_loaded_sample.clone();
        AMod2.sample_lib = loaded_preset.mod1_sample_lib.clone();
        AMod2.restretch = loaded_preset.mod1_restretch;

        AMod3.loaded_sample = loaded_preset.mod1_loaded_sample.clone();
        AMod3.sample_lib = loaded_preset.mod1_sample_lib.clone();
        AMod3.restretch = loaded_preset.mod1_restretch;

        // Note audio module type from the module is used here instead of from the main self type
        // This is because preset loading has changed it here first!
        AMod1.regenerate_samples();
        AMod2.regenerate_samples();
        AMod3.regenerate_samples();
    }

    fn save_preset_bank(&mut self) {
        let _updated_preset = self.update_current_preset();
        let saving_bank = FileDialog::new()
            .add_filter("bin", &["bin"]) // Use a binary format for audio data
            .set_file_name(&self.preset_lib_name)
            .save_file();

        if let Some(location) = saving_bank {
            // Create our new save file
            let file = File::create(location.clone());

            if let Ok(_file) = file {
                // Serialize our data to a binary format (MessagePack)
                let preset_store = Arc::clone(&self.preset_lib);
                let mut preset_lock = preset_store.lock().unwrap();

                // Clear out our generated notes and only keep the samples themselves
                for preset in preset_lock.iter_mut() {
                    preset.mod1_sample_lib.clear();
                    preset.mod2_sample_lib.clear();
                    preset.mod3_sample_lib.clear();
                }

                // Serialize to MessagePack bytes
                let serialized_data =
                    rmp_serde::to_vec::<&Vec<ActuatePreset>>(&preset_lock.as_ref());

                if let Err(err) = serialized_data {
                    eprintln!("Error serializing data: {}", err);
                    return;
                }

                // Compress the serialized data using different GzEncoder
                let compressed_data = Self::compress_bytes(&serialized_data.unwrap());

                // Now you can write the compressed data to the file
                if let Err(err) = std::fs::write(&location, &compressed_data) {
                    eprintln!("Error writing compressed data to file: {}", err);
                    return;
                }
            } else {
                eprintln!("Error creating file at location: {:?}", location);
            }
        }
        self.save_bank.store(false, Ordering::Relaxed);
    }

    // Functions to compress bytes and decompress using gz
    fn compress_bytes(data: &[u8]) -> Vec<u8> {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::best());
        encoder.write_all(data).unwrap();
        encoder.finish().unwrap()
    }

    fn decompress_bytes(compressed_data: &[u8]) -> Result<Vec<u8>, std::io::Error> {
        let mut decoder = GzDecoder::new(compressed_data);
        let mut decompressed_data = Vec::new();
        decoder.read_to_end(&mut decompressed_data)?;
        Ok(decompressed_data)
    }

    // Update our current preset
    fn update_current_preset(&mut self) {
        let arc_lib = Arc::clone(&self.preset_lib);
        let AM1c = self.audio_module_1.clone();
        let AM2c = self.audio_module_2.clone();
        let AM3c = self.audio_module_3.clone();

        let AM1 = AM1c.lock().unwrap();
        let AM2 = AM2c.lock().unwrap();
        let AM3 = AM3c.lock().unwrap();
        arc_lib.lock().unwrap()[self.current_preset.load(Ordering::Acquire) as usize] =
            ActuatePreset {
                // Modules 1
                ///////////////////////////////////////////////////////////
                mod1_audio_module_type: self.params._audio_module_1_type.value(),
                mod1_audio_module_level: self.params.audio_module_1_level.value(),
                // Granulizer/Sampler
                mod1_loaded_sample: AM1.loaded_sample.clone(),
                mod1_sample_lib: AM1.sample_lib.clone(),
                mod1_loop_wavetable: AM1.loop_wavetable,
                mod1_single_cycle: AM1.single_cycle,
                mod1_restretch: AM1.restretch,
                mod1_prev_restretch: AM1.prev_restretch,
                mod1_start_position: AM1.start_position,
                mod1_end_position: AM1._end_position,
                mod1_grain_crossfade: AM1.grain_crossfade,
                mod1_grain_gap: AM1.grain_gap,
                mod1_grain_hold: AM1.grain_hold,

                // Osc module knob storage
                mod1_osc_type: AM1.osc_type,
                mod1_osc_octave: AM1.osc_octave,
                mod1_osc_semitones: AM1.osc_semitones,
                mod1_osc_detune: AM1.osc_detune,
                mod1_osc_attack: AM1.osc_attack,
                mod1_osc_decay: AM1.osc_decay,
                mod1_osc_sustain: AM1.osc_sustain,
                mod1_osc_release: AM1.osc_release,
                mod1_osc_retrigger: AM1.osc_retrigger,
                mod1_osc_atk_curve: AM1.osc_atk_curve,
                mod1_osc_dec_curve: AM1.osc_dec_curve,
                mod1_osc_rel_curve: AM1.osc_rel_curve,
                mod1_osc_unison: AM1.osc_unison,
                mod1_osc_unison_detune: AM1.osc_unison_detune,
                mod1_osc_stereo: AM1.osc_stereo,

                // Modules 2
                ///////////////////////////////////////////////////////////
                mod2_audio_module_type: self.params._audio_module_2_type.value(),
                mod2_audio_module_level: self.params.audio_module_2_level.value(),
                // Granulizer/Sampler
                mod2_loaded_sample: AM2.loaded_sample.clone(),
                mod2_sample_lib: AM2.sample_lib.clone(),
                mod2_loop_wavetable: AM2.loop_wavetable,
                mod2_single_cycle: AM2.single_cycle,
                mod2_restretch: AM2.restretch,
                mod2_prev_restretch: AM2.prev_restretch,
                mod2_start_position: AM2.start_position,
                mod2_end_position: AM2._end_position,
                mod2_grain_crossfade: AM2.grain_crossfade,
                mod2_grain_gap: AM2.grain_gap,
                mod2_grain_hold: AM2.grain_hold,

                // Osc module knob storage
                mod2_osc_type: AM2.osc_type,
                mod2_osc_octave: AM2.osc_octave,
                mod2_osc_semitones: AM2.osc_semitones,
                mod2_osc_detune: AM2.osc_detune,
                mod2_osc_attack: AM2.osc_attack,
                mod2_osc_decay: AM2.osc_decay,
                mod2_osc_sustain: AM2.osc_sustain,
                mod2_osc_release: AM2.osc_release,
                mod2_osc_retrigger: AM2.osc_retrigger,
                mod2_osc_atk_curve: AM2.osc_atk_curve,
                mod2_osc_dec_curve: AM2.osc_dec_curve,
                mod2_osc_rel_curve: AM2.osc_rel_curve,
                mod2_osc_unison: AM2.osc_unison,
                mod2_osc_unison_detune: AM2.osc_unison_detune,
                mod2_osc_stereo: AM2.osc_stereo,

                // Modules 3
                ///////////////////////////////////////////////////////////
                mod3_audio_module_type: self.params._audio_module_3_type.value(),
                mod3_audio_module_level: self.params.audio_module_3_level.value(),
                // Granulizer/Sampler
                mod3_loaded_sample: AM3.loaded_sample.clone(),
                mod3_sample_lib: AM3.sample_lib.clone(),
                mod3_loop_wavetable: AM3.loop_wavetable,
                mod3_single_cycle: AM3.single_cycle,
                mod3_restretch: AM3.restretch,
                mod3_prev_restretch: AM3.prev_restretch,
                mod3_start_position: AM3.start_position,
                mod3_end_position: AM3._end_position,
                mod3_grain_crossfade: AM3.grain_crossfade,
                mod3_grain_gap: AM3.grain_gap,
                mod3_grain_hold: AM3.grain_hold,

                // Osc module knob storage
                mod3_osc_type: AM3.osc_type,
                mod3_osc_octave: AM3.osc_octave,
                mod3_osc_semitones: AM3.osc_semitones,
                mod3_osc_detune: AM3.osc_detune,
                mod3_osc_attack: AM3.osc_attack,
                mod3_osc_decay: AM3.osc_decay,
                mod3_osc_sustain: AM3.osc_sustain,
                mod3_osc_release: AM3.osc_release,
                mod3_osc_retrigger: AM3.osc_retrigger,
                mod3_osc_atk_curve: AM3.osc_atk_curve,
                mod3_osc_dec_curve: AM3.osc_dec_curve,
                mod3_osc_rel_curve: AM3.osc_rel_curve,
                mod3_osc_unison: AM3.osc_unison,
                mod3_osc_unison_detune: AM3.osc_unison_detune,
                mod3_osc_stereo: AM3.osc_stereo,

                // Filter storage - gotten from params
                filter_wet: self.params.filter_wet.value(),
                filter_cutoff: self.params.filter_cutoff.value(),
                filter_resonance: self.params.filter_resonance.value(),
                filter_res_type: self.params.filter_res_type.value(),
                filter_lp_amount: self.params.filter_lp_amount.value(),
                filter_hp_amount: self.params.filter_hp_amount.value(),
                filter_bp_amount: self.params.filter_bp_amount.value(),
                filter_env_peak: self.params.filter_env_peak.value(),
                filter_env_attack: self.params.filter_env_attack.value(),
                filter_env_decay: self.params.filter_env_decay.value(),
                filter_env_sustain: self.params.filter_env_sustain.value(),
                filter_env_release: self.params.filter_env_release.value(),
                filter_env_atk_curve: self.params.filter_env_atk_curve.value(),
                filter_env_dec_curve: self.params.filter_env_dec_curve.value(),
                filter_env_rel_curve: self.params.filter_env_rel_curve.value(),

                filter_wet_2: self.params.filter_wet_2.value(),
                filter_cutoff_2: self.params.filter_cutoff_2.value(),
                filter_resonance_2: self.params.filter_resonance_2.value(),
                filter_res_type_2: self.params.filter_res_type_2.value(),
                filter_lp_amount_2: self.params.filter_lp_amount_2.value(),
                filter_hp_amount_2: self.params.filter_hp_amount_2.value(),
                filter_bp_amount_2: self.params.filter_bp_amount_2.value(),
                filter_env_peak_2: self.params.filter_env_peak_2.value(),
                filter_env_attack_2: self.params.filter_env_attack_2.value(),
                filter_env_decay_2: self.params.filter_env_decay_2.value(),
                filter_env_sustain_2: self.params.filter_env_sustain_2.value(),
                filter_env_release_2: self.params.filter_env_release_2.value(),
                filter_env_atk_curve_2: self.params.filter_env_atk_curve_2.value(),
                filter_env_dec_curve_2: self.params.filter_env_dec_curve_2.value(),
                filter_env_rel_curve_2: self.params.filter_env_rel_curve_2.value(),

                lfo1_freq: self.params.lfo1_freq.value(),
                lfo1_retrigger: self.params.lfo1_retrigger.value(),
                lfo1_sync: self.params.lfo1_sync.value(),
                lfo1_snap: self.params.lfo1_snap.value(),
                lfo1_waveform: self.params.lfo1_waveform.value(),
                lfo1_phase: self.params.lfo1_phase.value(),

                lfo2_freq: self.params.lfo2_freq.value(),
                lfo2_retrigger: self.params.lfo2_retrigger.value(),
                lfo2_sync: self.params.lfo2_sync.value(),
                lfo2_snap: self.params.lfo2_snap.value(),
                lfo2_waveform: self.params.lfo2_waveform.value(),
                lfo2_phase: self.params.lfo2_phase.value(),

                lfo3_freq: self.params.lfo3_freq.value(),
                lfo3_retrigger: self.params.lfo3_retrigger.value(),
                lfo3_sync: self.params.lfo3_sync.value(),
                lfo3_snap: self.params.lfo3_snap.value(),
                lfo3_waveform: self.params.lfo3_waveform.value(),
                lfo3_phase: self.params.lfo3_phase.value(),
            };
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
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Instrument, Vst3SubCategory::Sampler];
}

nih_export_clap!(Actuate);
nih_export_vst3!(Actuate);

pub fn format_nothing() -> Arc<dyn Fn(f32) -> String + Send + Sync> {
    Arc::new(move |_| String::new())
}
