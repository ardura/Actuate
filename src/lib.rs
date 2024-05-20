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
Version 1.2.7

#####################################

This is the first synth I've ever written and first large Rust project. Thanks for checking it out and have fun!

#####################################
*/

#![allow(non_snake_case)]
use actuate_enums::{AMFilterRouting, FilterAlgorithms, FilterRouting, ModulationDestination, ModulationSource, PitchRouting, PresetType, ReverbModel};
use actuate_structs::{ActuatePresetV126, ModulationStruct};
use flate2::{read::GzDecoder,write::GzEncoder,Compression};
use nih_plug::prelude::*;
use nih_plug_egui::{
    egui::{Color32, FontId},
    EguiState,
};
use std::{
    fs::File, io::{Read, Write}, path::PathBuf, sync::{
        atomic::{AtomicBool, AtomicU32, Ordering},
        Arc, Mutex,
    }
};

// My Files/crates
use audio_module::{
    AudioModule, AudioModuleType,
    Oscillator::{self, OscState, RetriggerStyle, SmoothStyle, VoiceType},
    frequency_modulation,
};
use fx::{
    abass::a_bass_saturation, 
    aw_galactic_reverb::GalacticReverb,
    simple_space_reverb::SimpleSpaceReverb,
    biquad_filters::{self, FilterType}, 
    buffermodulator::BufferModulator, 
    compressor::Compressor, 
    delay::{Delay, DelaySnapValues, DelayType}, 
    flanger::StereoFlanger, 
    limiter::StereoLimiter, 
    phaser::StereoPhaser, 
    reverb::StereoReverb, 
    saturation::{Saturation, SaturationType}, 
    ArduraFilter::{self, ResponseType}, 
    StateVariableFilter::{ResonanceType, StateVariableFilter}, 
    VCFilter::ResponseType as VCResponseType
};

use old_preset_structs::{
    load_unserialized_old, 
    load_unserialized_v114,
    load_unserialized_v122,
    load_unserialized_v123,
    load_unserialized_v125,
    ActuatePresetV123,
    ActuatePresetV125
};

mod actuate_gui;
mod actuate_enums;
mod actuate_structs;
mod CustomWidgets;
mod LFOController;
mod audio_module;
mod fx;
mod old_preset_structs;

// Plugin sizing
const WIDTH: u32 = 920;
const HEIGHT: u32 = 656;

// Until we have a real preset editor and browser it's better to keep the preset bank smaller
//const OLD_PRESET_BANK_SIZE: usize = 32;
const PRESET_BANK_SIZE: usize = 128;

// File Open Buffer Timer - fixes sync issues from load/save to the gui
const FILE_OPEN_BUFFER_MAX: u32 = 1;

// GUI values to refer to
pub const TEAL_GREEN: Color32 = Color32::from_rgb(61, 178, 166);
pub const DARKEST_BOTTOM_UI_COLOR: Color32 = Color32::from_rgb(27, 27, 27);
pub const DARKER_GREY_UI_COLOR: Color32 = Color32::from_rgb(34, 34, 34);
pub const DARK_GREY_UI_COLOR: Color32 = Color32::from_rgb(42, 42, 42);
pub const MEDIUM_GREY_UI_COLOR: Color32 = Color32::from_rgb(52, 52, 52);
pub const LIGHTER_GREY_UI_COLOR: Color32 = Color32::from_rgb(69, 69, 69);
pub const A_BACKGROUND_COLOR_TOP: Color32 = Color32::from_rgb(38, 38, 38);
pub const YELLOW_MUSTARD: Color32 = Color32::from_rgb(172, 131, 25);
pub const FONT_COLOR: Color32 = Color32::from_rgb(248, 248, 248);

// Fonts
const FONT: nih_plug_egui::egui::FontId = FontId::proportional(12.0);
const LOADING_FONT: nih_plug_egui::egui::FontId = FontId::proportional(20.0);
const SMALLER_FONT: nih_plug_egui::egui::FontId = FontId::proportional(11.0);

// This is the struct of the actual plugin object that tracks everything
//#[derive(Clone)]
pub struct Actuate {
    pub params: Arc<ActuateParams>,
    pub sample_rate: f32,

    // Plugin control Arcs
    update_something: Arc<AtomicBool>,
    clear_voices: Arc<AtomicBool>,
    reload_entire_preset: Arc<AtomicBool>,
    file_dialog: Arc<AtomicBool>,
    file_open_buffer_timer: Arc<AtomicU32>,
    browsing_presets: Arc<AtomicBool>,
    current_preset: Arc<AtomicU32>,
    update_current_preset: Arc<AtomicBool>,

    current_note_on_velocity: Arc<AtomicF32>,

    // Managing resample logic
    prev_restretch_1: Arc<AtomicBool>,
    prev_restretch_2: Arc<AtomicBool>,
    prev_restretch_3: Arc<AtomicBool>,

    // Modules
    audio_module_1: Arc<Mutex<AudioModule>>,
    audio_module_2: Arc<Mutex<AudioModule>>,
    audio_module_3: Arc<Mutex<AudioModule>>,

    // SVF Filters
    filter_l_1: StateVariableFilter,
    filter_r_1: StateVariableFilter,
    // TILT Filters
    tilt_filter_l_1: ArduraFilter::ArduraFilter,
    tilt_filter_r_1: ArduraFilter::ArduraFilter,
    // VCF Filters
    vcf_filter_l_1: fx::VCFilter::VCFilter,
    vcf_filter_r_1: fx::VCFilter::VCFilter,
    // Filter state variables
    filter_state_1: OscState,
    filter_atk_smoother_1: Smoother<f32>,
    filter_dec_smoother_1: Smoother<f32>,
    filter_rel_smoother_1: Smoother<f32>,

    // SVF Filters
    filter_l_2: StateVariableFilter,
    filter_r_2: StateVariableFilter,
    // TILT Filters
    tilt_filter_l_2: ArduraFilter::ArduraFilter,
    tilt_filter_r_2: ArduraFilter::ArduraFilter,
    // VCF Filters
    vcf_filter_l_2: fx::VCFilter::VCFilter,
    vcf_filter_r_2: fx::VCFilter::VCFilter,
    // Filter state variables
    filter_state_2: OscState,
    filter_atk_smoother_2: Smoother<f32>,
    filter_dec_smoother_2: Smoother<f32>,
    filter_rel_smoother_2: Smoother<f32>,

    // LFOs!
    lfo_1: LFOController::LFOController,
    lfo_2: LFOController::LFOController,
    lfo_3: LFOController::LFOController,

    // Modulation overrides for preset loading
    mod_override_source_1: Arc<Mutex<ModulationSource>>,
    mod_override_source_2: Arc<Mutex<ModulationSource>>,
    mod_override_source_3: Arc<Mutex<ModulationSource>>,
    mod_override_source_4: Arc<Mutex<ModulationSource>>,
    mod_override_dest_1: Arc<Mutex<ModulationDestination>>,
    mod_override_dest_2: Arc<Mutex<ModulationDestination>>,
    mod_override_dest_3: Arc<Mutex<ModulationDestination>>,
    mod_override_dest_4: Arc<Mutex<ModulationDestination>>,
    preset_category_override: Arc<Mutex<PresetType>>,

    // Preset Lib Default
    preset_lib_name: Arc<Mutex<String>>,
    preset_name: Arc<Mutex<String>>,
    preset_info: Arc<Mutex<String>>,
    preset_category: Arc<Mutex<PresetType>>,
    preset_lib: Arc<Mutex<Vec<ActuatePresetV126>>>,

    // Used for DC Offset calculations
    dc_filter_l: StateVariableFilter,
    dc_filter_r: StateVariableFilter,

    fm_state: OscState,
    fm_atk_smoother_1: Smoother<f32>,
    fm_dec_smoother_1: Smoother<f32>,
    fm_rel_smoother_1: Smoother<f32>,
    fm_atk_smoother_2: Smoother<f32>,
    fm_dec_smoother_2: Smoother<f32>,
    fm_rel_smoother_2: Smoother<f32>,
    fm_atk_smoother_3: Smoother<f32>,
    fm_dec_smoother_3: Smoother<f32>,
    fm_rel_smoother_3: Smoother<f32>,

    // EQ Structs
    // I'm not using the Interleaved ones since in Interleaf
    // People thought the quirks of interleaving were bugs
    bands: Arc<Mutex<[biquad_filters::Biquad; 3]>>,

    // Compressor
    compressor: Compressor,

    // Saturation
    saturator: Saturation,

    // Delay
    delay: Delay,

    // Reverb
    reverb: [StereoReverb; 8],
    galactic_reverb: GalacticReverb,
    simple_space: [SimpleSpaceReverb;4],

    // Phaser
    phaser: StereoPhaser,

    // Buffer Modulation
    buffermod: BufferModulator,

    // Flanger
    flanger: StereoFlanger,

    // Limiter
    limiter: StereoLimiter,

    // Preset browser stuff
    filter_acid: Arc<AtomicBool>,
    filter_analog: Arc<AtomicBool>,
    filter_bright: Arc<AtomicBool>,
    filter_chord: Arc<AtomicBool>,
    filter_crisp: Arc<AtomicBool>,
    filter_deep: Arc<AtomicBool>,
    filter_delicate: Arc<AtomicBool>,
    filter_hard: Arc<AtomicBool>,
    filter_harsh: Arc<AtomicBool>,
    filter_lush: Arc<AtomicBool>,
    filter_mellow: Arc<AtomicBool>,
    filter_resonant: Arc<AtomicBool>,
    filter_rich: Arc<AtomicBool>,
    filter_sharp: Arc<AtomicBool>,
    filter_silky: Arc<AtomicBool>,
    filter_smooth: Arc<AtomicBool>,
    filter_soft: Arc<AtomicBool>,
    filter_stab: Arc<AtomicBool>,
    filter_warm: Arc<AtomicBool>,
}

impl Default for Actuate {
    fn default() -> Self {
        // These are persistent fields to trigger updates like Diopser
        let update_something = Arc::new(AtomicBool::new(false));
        let clear_voices = Arc::new(AtomicBool::new(false));
        let reload_entire_preset = Arc::new(AtomicBool::new(false));
        let file_dialog = Arc::new(AtomicBool::new(false));
        let file_open_buffer_timer = Arc::new(AtomicU32::new(0));
        let browsing_presets = Arc::new(AtomicBool::new(false));
        let current_preset = Arc::new(AtomicU32::new(0));
        let update_current_preset = Arc::new(AtomicBool::new(false));

        Self {
            params: Arc::new(ActuateParams::new(
                update_something.clone(),
                clear_voices.clone(),
                file_dialog.clone(),
                update_current_preset.clone(),
            )),
            sample_rate: 44100.0,

            // Plugin control ARCs
            update_something: update_something,
            clear_voices: clear_voices,
            reload_entire_preset: reload_entire_preset,
            file_dialog: file_dialog,
            file_open_buffer_timer: file_open_buffer_timer,
            browsing_presets: browsing_presets,
            current_preset: current_preset,
            update_current_preset: update_current_preset,

            current_note_on_velocity: Arc::new(AtomicF32::new(0.0)),

            prev_restretch_1: Arc::new(AtomicBool::new(false)),
            prev_restretch_2: Arc::new(AtomicBool::new(false)),
            prev_restretch_3: Arc::new(AtomicBool::new(false)),

            // Module 1
            audio_module_1: Arc::new(Mutex::new(AudioModule::default())),
            audio_module_2: Arc::new(Mutex::new(AudioModule::default())),
            audio_module_3: Arc::new(Mutex::new(AudioModule::default())),

            // SVF Filters
            filter_l_2: StateVariableFilter::default().set_oversample(4),
            filter_r_2: StateVariableFilter::default().set_oversample(4),
            // TILT Filters
            tilt_filter_l_2: ArduraFilter::ArduraFilter::new(
                44100.0,
                20000.0,
                1.0,
                ResponseType::Lowpass,
            ),
            tilt_filter_r_2: ArduraFilter::ArduraFilter::new(
                44100.0,
                20000.0,
                1.0,
                ResponseType::Lowpass,
            ),
            // VCF Filters
            vcf_filter_l_1: fx::VCFilter::VCFilter::new(),
            vcf_filter_r_1: fx::VCFilter::VCFilter::new(),

            filter_state_2: OscState::Off,
            filter_atk_smoother_2: Smoother::new(SmoothingStyle::Linear(300.0)),
            filter_dec_smoother_2: Smoother::new(SmoothingStyle::Linear(300.0)),
            filter_rel_smoother_2: Smoother::new(SmoothingStyle::Linear(300.0)),

            // SVF Filters
            filter_l_1: StateVariableFilter::default().set_oversample(4),
            filter_r_1: StateVariableFilter::default().set_oversample(4),
            // TILT Filters
            tilt_filter_l_1: ArduraFilter::ArduraFilter::new(
                44100.0,
                20000.0,
                1.0,
                ResponseType::Lowpass,
            ),
            tilt_filter_r_1: ArduraFilter::ArduraFilter::new(
                44100.0,
                20000.0,
                1.0,
                ResponseType::Lowpass,
            ),
            // VCF Filters
            vcf_filter_l_2: fx::VCFilter::VCFilter::new(),
            vcf_filter_r_2: fx::VCFilter::VCFilter::new(),

            filter_state_1: OscState::Off,
            filter_atk_smoother_1: Smoother::new(SmoothingStyle::Linear(300.0)),
            filter_dec_smoother_1: Smoother::new(SmoothingStyle::Linear(300.0)),
            filter_rel_smoother_1: Smoother::new(SmoothingStyle::Linear(300.0)),

            //LFOs
            lfo_1: LFOController::LFOController::new(2.0, 1.0, LFOController::Waveform::Sine, 0.0),
            lfo_2: LFOController::LFOController::new(2.0, 1.0, LFOController::Waveform::Sine, 0.0),
            lfo_3: LFOController::LFOController::new(2.0, 1.0, LFOController::Waveform::Sine, 0.0),

            // Modulation Overrides
            mod_override_source_1: Arc::new(Mutex::new(ModulationSource::UnsetModulation)),
            mod_override_source_2: Arc::new(Mutex::new(ModulationSource::UnsetModulation)),
            mod_override_source_3: Arc::new(Mutex::new(ModulationSource::UnsetModulation)),
            mod_override_source_4: Arc::new(Mutex::new(ModulationSource::UnsetModulation)),
            mod_override_dest_1: Arc::new(Mutex::new(ModulationDestination::UnsetModulation)),
            mod_override_dest_2: Arc::new(Mutex::new(ModulationDestination::UnsetModulation)),
            mod_override_dest_3: Arc::new(Mutex::new(ModulationDestination::UnsetModulation)),
            mod_override_dest_4: Arc::new(Mutex::new(ModulationDestination::UnsetModulation)),
            preset_category_override: Arc::new(Mutex::new(PresetType::Select)),

            // Preset Library DEFAULT
            preset_lib_name: Arc::new(Mutex::new(String::from("Default"))),
            preset_name: Arc::new(Mutex::new(String::new())),
            preset_info: Arc::new(Mutex::new(String::new())),
            preset_category: Arc::new(Mutex::new(PresetType::Select)),
            preset_lib: Arc::new(Mutex::new(vec![
                DEFAULT_PRESET.clone();
                PRESET_BANK_SIZE
            ])),

            fm_state: OscState::Off,
            fm_atk_smoother_1: Smoother::new(SmoothingStyle::Linear(300.0)),
            fm_dec_smoother_1: Smoother::new(SmoothingStyle::Linear(300.0)),
            fm_rel_smoother_1: Smoother::new(SmoothingStyle::Linear(300.0)),
            fm_atk_smoother_2: Smoother::new(SmoothingStyle::Linear(300.0)),
            fm_dec_smoother_2: Smoother::new(SmoothingStyle::Linear(300.0)),
            fm_rel_smoother_2: Smoother::new(SmoothingStyle::Linear(300.0)),
            fm_atk_smoother_3: Smoother::new(SmoothingStyle::Linear(300.0)),
            fm_dec_smoother_3: Smoother::new(SmoothingStyle::Linear(300.0)),
            fm_rel_smoother_3: Smoother::new(SmoothingStyle::Linear(300.0)),

            dc_filter_l: StateVariableFilter::default().set_oversample(2),
            dc_filter_r: StateVariableFilter::default().set_oversample(2),

            // EQ Structs
            bands: Arc::new(Mutex::new([
                biquad_filters::Biquad::new(44100.0, 800.0, 0.0, 0.93, FilterType::LowShelf),
                biquad_filters::Biquad::new(44100.0, 3000.0, 0.0, 0.93, FilterType::Peak),
                biquad_filters::Biquad::new(44100.0, 10000.0, 0.0, 0.93, FilterType::HighShelf),
            ])),

            // Compressor
            compressor: Compressor::new(44100.0, 0.5, 0.5, 0.5, 0.5),

            // Saturation
            saturator: Saturation::new(),

            // Delay
            delay: Delay::new(44100.0, 138.0, DelaySnapValues::Quarter, 0.5),

            // Reverb
            reverb: [
                StereoReverb::new(44100.0, 0.5, 0.5),
                StereoReverb::new(44100.0, 0.5, 0.5),
                StereoReverb::new(44100.0, 0.5, 0.5),
                StereoReverb::new(44100.0, 0.5, 0.5),
                StereoReverb::new(44100.0, 0.5, 0.5),
                StereoReverb::new(44100.0, 0.5, 0.5),
                StereoReverb::new(44100.0, 0.5, 0.5),
                StereoReverb::new(44100.0, 0.5, 0.5),
            ],
            galactic_reverb: GalacticReverb::new(44100.0, 1.0, 0.76, 0.5),
            simple_space: [
                SimpleSpaceReverb::new(44100.0, 1.0, 0.76, 0.5),
                SimpleSpaceReverb::new(44100.0, 1.0, 0.76, 0.5),
                SimpleSpaceReverb::new(44100.0, 1.0, 0.76, 0.5),
                SimpleSpaceReverb::new(44100.0, 1.0, 0.76, 0.5),
            ],

            // Buffer Modulator
            buffermod: BufferModulator::new(44100.0, 0.5, 10.0),

            // Flanger initialized to use delay range of 50, for 100 samples
            flanger: StereoFlanger::new(44100.0, 0.5, 0.5, 10.0, 0.5, 20),

            // Phaser
            phaser: StereoPhaser::new(),

            // Limiter
            limiter: StereoLimiter::new(0.5, 0.5),

            // Preset browser stuff
            filter_acid: Arc::new(AtomicBool::new(false)),
            filter_analog: Arc::new(AtomicBool::new(false)),
            filter_bright: Arc::new(AtomicBool::new(false)),
            filter_chord: Arc::new(AtomicBool::new(false)),
            filter_crisp: Arc::new(AtomicBool::new(false)),
            filter_deep: Arc::new(AtomicBool::new(false)),
            filter_delicate: Arc::new(AtomicBool::new(false)),
            filter_hard: Arc::new(AtomicBool::new(false)),
            filter_harsh: Arc::new(AtomicBool::new(false)),
            filter_lush: Arc::new(AtomicBool::new(false)),
            filter_mellow: Arc::new(AtomicBool::new(false)),
            filter_resonant: Arc::new(AtomicBool::new(false)),
            filter_rich: Arc::new(AtomicBool::new(false)),
            filter_sharp: Arc::new(AtomicBool::new(false)),
            filter_silky: Arc::new(AtomicBool::new(false)),
            filter_smooth: Arc::new(AtomicBool::new(false)),
            filter_soft: Arc::new(AtomicBool::new(false)),
            filter_stab: Arc::new(AtomicBool::new(false)),
            filter_warm: Arc::new(AtomicBool::new(false)),
        }
    }
}

/// Plugin parameters struct
#[derive(Params)]
pub struct ActuateParams {
    #[persist = "editor-state"]
    editor_state: Arc<EguiState>,
    #[persist = "AM1_Sample"]
    am1_sample: Mutex<Vec<Vec<f32>>>,
    #[persist = "AM2_Sample"]
    am2_sample: Mutex<Vec<Vec<f32>>>,
    #[persist = "AM3_Sample"]
    am3_sample: Mutex<Vec<Vec<f32>>>,

    // Synth-level settings
    #[id = "Master Level"]
    pub master_level: FloatParam,
    #[id = "Max Voices"]
    pub voice_limit: IntParam,

    // This audio module is what switches between functions for generators in the synth
    #[id = "audio_module_1_type"]
    pub audio_module_1_type: EnumParam<AudioModuleType>,
    #[id = "audio_module_2_type"]
    pub audio_module_2_type: EnumParam<AudioModuleType>,
    #[id = "audio_module_3_type"]
    pub audio_module_3_type: EnumParam<AudioModuleType>,

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
    #[id = "filter_cutoff_link"]
    pub filter_cutoff_link: BoolParam,

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
    #[id = "filter_alg_type"]
    pub filter_alg_type: EnumParam<FilterAlgorithms>,
    #[id = "tilt_filter_type"]
    pub tilt_filter_type: EnumParam<ResponseType>,
    #[id = "vcf_filter_type"]
    pub vcf_filter_type: EnumParam<VCResponseType>,

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
    #[id = "filter_alg_type_2"]
    pub filter_alg_type_2: EnumParam<FilterAlgorithms>,
    #[id = "tilt_filter_type_2"]
    pub tilt_filter_type_2: EnumParam<ResponseType>,
    #[id = "vcf_filter_type_2"]
    pub vcf_filter_type_2: EnumParam<VCResponseType>,

    // Pitch Envelope
    #[id = "pitch_enable"]
    pub pitch_enable: BoolParam,
    #[id = "pitch_routing"]
    pub pitch_routing: EnumParam<PitchRouting>,
    #[id = "pitch_env_peak"]
    pub pitch_env_peak: FloatParam,
    #[id = "pitch_env_attack"]
    pub pitch_env_attack: FloatParam,
    #[id = "pitch_env_decay"]
    pub pitch_env_decay: FloatParam,
    #[id = "pitch_env_sustain"]
    pub pitch_env_sustain: FloatParam,
    #[id = "pitch_env_release"]
    pub pitch_env_release: FloatParam,
    #[id = "pitch_env_atk_curve"]
    pub pitch_env_atk_curve: EnumParam<Oscillator::SmoothStyle>,
    #[id = "pitch_env_dec_curve"]
    pub pitch_env_dec_curve: EnumParam<Oscillator::SmoothStyle>,
    #[id = "pitch_env_rel_curve"]
    pub pitch_env_rel_curve: EnumParam<Oscillator::SmoothStyle>,

    #[id = "pitch_enable_2"]
    pub pitch_enable_2: BoolParam,
    #[id = "pitch_routing_2"]
    pub pitch_routing_2: EnumParam<PitchRouting>,
    #[id = "pitch_env_peak_2"]
    pub pitch_env_peak_2: FloatParam,
    #[id = "pitch_env_attack_2"]
    pub pitch_env_attack_2: FloatParam,
    #[id = "pitch_env_decay_2"]
    pub pitch_env_decay_2: FloatParam,
    #[id = "pitch_env_sustain_2"]
    pub pitch_env_sustain_2: FloatParam,
    #[id = "pitch_env_release_2"]
    pub pitch_env_release_2: FloatParam,
    #[id = "pitch_env_atk_curve_2"]
    pub pitch_env_atk_curve_2: EnumParam<Oscillator::SmoothStyle>,
    #[id = "pitch_env_dec_curve_2"]
    pub pitch_env_dec_curve_2: EnumParam<Oscillator::SmoothStyle>,
    #[id = "pitch_env_rel_curve_2"]
    pub pitch_env_rel_curve_2: EnumParam<Oscillator::SmoothStyle>,

    // LFOS
    #[id = "lfo1_enable"]
    pub lfo1_enable: BoolParam,
    #[id = "lfo2_enable"]
    pub lfo2_enable: BoolParam,
    #[id = "lfo3_enable"]
    pub lfo3_enable: BoolParam,
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

    // Mod knobs
    #[id = "mod_amount_knob_1"]
    pub mod_amount_knob_1: FloatParam,
    #[id = "mod_amount_knob_2"]
    pub mod_amount_knob_2: FloatParam,
    #[id = "mod_amount_knob_3"]
    pub mod_amount_knob_3: FloatParam,
    #[id = "mod_amount_knob_4"]
    pub mod_amount_knob_4: FloatParam,
    #[id = "mod_source_1"]
    pub mod_source_1: EnumParam<ModulationSource>,
    #[id = "mod_source_2"]
    pub mod_source_2: EnumParam<ModulationSource>,
    #[id = "mod_source_3"]
    pub mod_source_3: EnumParam<ModulationSource>,
    #[id = "mod_source_4"]
    pub mod_source_4: EnumParam<ModulationSource>,
    #[id = "mod_destination_1"]
    pub mod_destination_1: EnumParam<ModulationDestination>,
    #[id = "mod_destination_2"]
    pub mod_destination_2: EnumParam<ModulationDestination>,
    #[id = "mod_destination_3"]
    pub mod_destination_3: EnumParam<ModulationDestination>,
    #[id = "mod_destination_4"]
    pub mod_destination_4: EnumParam<ModulationDestination>,

    // EQ Params
    #[id = "pre_use_eq"]
    pub pre_use_eq: BoolParam,

    #[id = "pre_low_freq"]
    pub pre_low_freq: FloatParam,
    #[id = "pre_mid_freq"]
    pub pre_mid_freq: FloatParam,
    #[id = "pre_high_freq"]
    pub pre_high_freq: FloatParam,

    #[id = "pre_low_gain"]
    pub pre_low_gain: FloatParam,
    #[id = "pre_mid_gain"]
    pub pre_mid_gain: FloatParam,
    #[id = "pre_high_gain"]
    pub pre_high_gain: FloatParam,

    // FX
    #[id = "use_fx"]
    pub use_fx: BoolParam,

    #[id = "use_compressor"]
    pub use_compressor: BoolParam,
    #[id = "comp_amt"]
    pub comp_amt: FloatParam,
    #[id = "comp_atk"]
    pub comp_atk: FloatParam,
    #[id = "comp_rel"]
    pub comp_rel: FloatParam,
    #[id = "comp_drive"]
    pub comp_drive: FloatParam,

    #[id = "use_abass"]
    pub use_abass: BoolParam,
    #[id = "abass_amount"]
    pub abass_amount: FloatParam,

    #[id = "use_saturation"]
    pub use_saturation: BoolParam,
    #[id = "sat_amt"]
    pub sat_amt: FloatParam,
    #[id = "sat_type"]
    pub sat_type: EnumParam<SaturationType>,

    #[id = "use_delay"]
    pub use_delay: BoolParam,
    #[id = "delay_amount"]
    pub delay_amount: FloatParam,
    #[id = "delay_time"]
    pub delay_time: EnumParam<DelaySnapValues>,
    #[id = "delay_decay"]
    pub delay_decay: FloatParam,
    #[id = "delay_type"]
    pub delay_type: EnumParam<DelayType>,

    #[id = "use_reverb"]
    pub use_reverb: BoolParam,
    #[id = "reverb_model"]
    pub reverb_model: EnumParam<ReverbModel>,
    #[id = "reverb_amount"]
    pub reverb_amount: FloatParam,
    #[id = "reverb_size"]
    pub reverb_size: FloatParam,
    #[id = "reverb_feedback"]
    pub reverb_feedback: FloatParam,

    #[id = "use_phaser"]
    pub use_phaser: BoolParam,
    #[id = "phaser_amount"]
    pub phaser_amount: FloatParam,
    #[id = "phaser_depth"]
    pub phaser_depth: FloatParam,
    #[id = "phaser_rate"]
    pub phaser_rate: FloatParam,
    #[id = "phaser_feedback"]
    pub phaser_feedback: FloatParam,

    #[id = "use_buffermod"]
    pub use_buffermod: BoolParam,
    #[id = "buffermod_amount"]
    pub buffermod_amount: FloatParam,
    #[id = "buffermod_depth"]
    pub buffermod_depth: FloatParam,
    #[id = "buffermod_rate"]
    pub buffermod_rate: FloatParam,
    #[id = "buffermod_spread"]
    pub buffermod_spread: FloatParam,
    #[id = "buffermod_timing"]
    pub buffermod_timing: FloatParam,

    #[id = "use_flanger"]
    pub use_flanger: BoolParam,
    #[id = "flanger_amount"]
    pub flanger_amount: FloatParam,
    #[id = "flanger_depth"]
    pub flanger_depth: FloatParam,
    #[id = "flanger_rate"]
    pub flanger_rate: FloatParam,
    #[id = "flanger_feedback"]
    pub flanger_feedback: FloatParam,

    #[id = "use_limiter"]
    pub use_limiter: BoolParam,
    #[id = "limiter_threshold"]
    pub limiter_threshold: FloatParam,
    #[id = "limiter_knee"]
    pub limiter_knee: FloatParam,

    // FM
    #[id = "fm_one_to_two"]
    pub fm_one_to_two: FloatParam,
    #[id = "fm_one_to_three"]
    pub fm_one_to_three: FloatParam,
    #[id = "fm_two_to_three"]
    pub fm_two_to_three: FloatParam,
    #[id = "fm_cycles"]
    pub fm_cycles: IntParam,
    #[id = "fm_attack"]
    pub fm_attack: FloatParam,
    #[id = "fm_decay"]
    pub fm_decay: FloatParam,
    #[id = "fm_sustain"]
    pub fm_sustain: FloatParam,
    #[id = "fm_release"]
    pub fm_release: FloatParam,
    #[id = "fm_attack_curve"]
    pub fm_attack_curve: EnumParam<Oscillator::SmoothStyle>,
    #[id = "fm_decay_curve"]
    pub fm_decay_curve: EnumParam<Oscillator::SmoothStyle>,
    #[id = "fm_release_curve"]
    pub fm_release_curve: EnumParam<Oscillator::SmoothStyle>,

    // UI Non-param Params
    #[id = "param_load_bank"]
    pub param_load_bank: BoolParam,
    #[id = "param_save_bank"]
    pub param_save_bank: BoolParam,
    #[id = "param_import_preset"]
    pub param_import_preset: BoolParam,
    #[id = "param_export_preset"]
    pub param_export_preset: BoolParam,
    #[id = "param_next_preset"]
    pub param_next_preset: BoolParam,
    #[id = "param_prev_preset"]
    pub param_prev_preset: BoolParam,
    #[id = "param_update_current_preset"]
    pub param_update_current_preset: BoolParam,

    #[id = "preset_category"]
    pub preset_category: EnumParam<PresetType>,
    #[id = "tag_acid"]
    pub tag_acid: BoolParam,
    #[id = "tag_analog"]
    pub tag_analog: BoolParam,
    #[id = "tag_bright"]
    pub tag_bright: BoolParam,
    #[id = "tag_chord"]
    pub tag_chord: BoolParam,
    #[id = "tag_crisp"]
    pub tag_crisp: BoolParam,
    #[id = "tag_deep"]
    pub tag_deep: BoolParam,
    #[id = "tag_delicate"]
    pub tag_delicate: BoolParam,
    #[id = "tag_hard"]
    pub tag_hard: BoolParam,
    #[id = "tag_harsh"]
    pub tag_harsh: BoolParam,
    #[id = "tag_lush"]
    pub tag_lush: BoolParam,
    #[id = "tag_mellow"]
    pub tag_mellow: BoolParam,
    #[id = "tag_resonant"]
    pub tag_resonant: BoolParam,
    #[id = "tag_rich"]
    pub tag_rich: BoolParam,
    #[id = "tag_sharp"]
    pub tag_sharp: BoolParam,
    #[id = "tag_silky"]
    pub tag_silky: BoolParam,
    #[id = "tag_smooth"]
    pub tag_smooth: BoolParam,
    #[id = "tag_soft"]
    pub tag_soft: BoolParam,
    #[id = "tag_stab"]
    pub tag_stab: BoolParam,
    #[id = "tag_warm"]
    pub tag_warm: BoolParam,

    // Not a param
    #[id = "loading"]
    pub loading: BoolParam,
}

// This is where parameters are established and defined as well as the callbacks to share gui/audio process info
impl ActuateParams {
    fn new(
        update_something: Arc<AtomicBool>,
        clear_voices: Arc<AtomicBool>,
        file_dialog: Arc<AtomicBool>,
        update_current_preset: Arc<AtomicBool>,
    ) -> Self {
        Self {
            editor_state: EguiState::from_size(WIDTH, HEIGHT),
            am1_sample: Mutex::new(vec![vec![0.0, 0.0]]),
            am2_sample: Mutex::new(vec![vec![0.0, 0.0]]),
            am3_sample: Mutex::new(vec![vec![0.0, 0.0]]),

            // Top Level objects
            ////////////////////////////////////////////////////////////////////////////////////
            master_level: FloatParam::new("Master", 1.0, FloatRange::Linear { min: 0.0, max: 2.0 })
                .with_value_to_string(formatters::v2s_f32_percentage(0))
                .with_unit("%"),
            voice_limit: IntParam::new("Max Voices", 64, IntRange::Linear { min: 1, max: 512 }),

            audio_module_1_type: EnumParam::new("Type", AudioModuleType::Osc).with_callback({
                let clear_voices = clear_voices.clone();
                Arc::new(move |_| clear_voices.store(true, Ordering::SeqCst))
            }),
            audio_module_2_type: EnumParam::new("Type", AudioModuleType::Off).with_callback({
                let clear_voices = clear_voices.clone();
                Arc::new(move |_| clear_voices.store(true, Ordering::SeqCst))
            }),
            audio_module_3_type: EnumParam::new("Type", AudioModuleType::Off).with_callback({
                let clear_voices = clear_voices.clone();
                Arc::new(move |_| clear_voices.store(true, Ordering::SeqCst))
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
                Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
            }),
            osc_1_octave: IntParam::new("Octave", 0, IntRange::Linear { min: -2, max: 2 })
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
                }),
            osc_1_semitones: IntParam::new("Semi", 0, IntRange::Linear { min: -11, max: 11 })
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
                }),
            osc_1_detune: FloatParam::new(
                "Fine",
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
                Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
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
                Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
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
                Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
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
                Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
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
                Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
            }),
            osc_1_retrigger: EnumParam::new("Retrig", RetriggerStyle::Retrigger).with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
            }),
            osc_1_atk_curve: EnumParam::new("Atk Curve", Oscillator::SmoothStyle::Linear)
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
                }),
            osc_1_dec_curve: EnumParam::new("Dec Curve", Oscillator::SmoothStyle::Linear)
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
                }),
            osc_1_rel_curve: EnumParam::new("Rel Curve", Oscillator::SmoothStyle::Linear)
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
                }),
            osc_1_unison: IntParam::new("Unison", 1, IntRange::Linear { min: 1, max: 9 })
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
                }),
            osc_1_unison_detune: FloatParam::new(
                "UDetune",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.0001)
            .with_value_to_string(formatters::v2s_f32_rounded(4))
            .with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
            }),
            osc_1_stereo: FloatParam::new("Stereo", 1.0, FloatRange::Linear { min: 0.0, max: 2.0 })
                .with_value_to_string(formatters::v2s_f32_rounded(2))
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
                }),

            osc_2_type: EnumParam::new("Wave", VoiceType::Sine).with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
            }),
            osc_2_octave: IntParam::new("Octave", 0, IntRange::Linear { min: -2, max: 2 })
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
                }),
            osc_2_semitones: IntParam::new("Semi", 0, IntRange::Linear { min: -11, max: 11 })
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
                }),
            osc_2_detune: FloatParam::new(
                "Fine",
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
                Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
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
                Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
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
                Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
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
                Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
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
                Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
            }),
            osc_2_retrigger: EnumParam::new("Retrig", RetriggerStyle::Retrigger).with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
            }),
            osc_2_atk_curve: EnumParam::new("Atk Curve", Oscillator::SmoothStyle::Linear)
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
                }),
            osc_2_dec_curve: EnumParam::new("Dec Curve", Oscillator::SmoothStyle::Linear)
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
                }),
            osc_2_rel_curve: EnumParam::new("Rel Curve", Oscillator::SmoothStyle::Linear)
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
                }),
            osc_2_unison: IntParam::new("Unison", 1, IntRange::Linear { min: 1, max: 9 })
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
                }),
            osc_2_unison_detune: FloatParam::new(
                "UDetune",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.0001)
            .with_value_to_string(formatters::v2s_f32_rounded(4))
            .with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
            }),
            osc_2_stereo: FloatParam::new("Stereo", 1.0, FloatRange::Linear { min: 0.0, max: 2.0 })
                .with_value_to_string(formatters::v2s_f32_rounded(2))
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
                }),

            osc_3_type: EnumParam::new("Wave", VoiceType::Sine).with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
            }),
            osc_3_octave: IntParam::new("Octave", 0, IntRange::Linear { min: -2, max: 2 })
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
                }),
            osc_3_semitones: IntParam::new("Semi", 0, IntRange::Linear { min: -11, max: 11 })
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
                }),
            osc_3_detune: FloatParam::new(
                "Fine",
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
                Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
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
                Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
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
                Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
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
                Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
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
                Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
            }),
            osc_3_retrigger: EnumParam::new("Retrig", RetriggerStyle::Retrigger).with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
            }),
            osc_3_atk_curve: EnumParam::new("Atk Curve", Oscillator::SmoothStyle::Linear)
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
                }),
            osc_3_dec_curve: EnumParam::new("Dec Curve", Oscillator::SmoothStyle::Linear)
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
                }),
            osc_3_rel_curve: EnumParam::new("Rel Curve", Oscillator::SmoothStyle::Linear)
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
                }),
            osc_3_unison: IntParam::new("Unison", 1, IntRange::Linear { min: 1, max: 9 })
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
                }),
            osc_3_unison_detune: FloatParam::new(
                "UDetune",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.0001)
            .with_value_to_string(formatters::v2s_f32_rounded(4))
            .with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
            }),
            osc_3_stereo: FloatParam::new("Stereo", 1.0, FloatRange::Linear { min: 0.0, max: 2.0 })
                .with_value_to_string(formatters::v2s_f32_rounded(2))
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
                }),

            // Granulizer/Sampler
            ////////////////////////////////////////////////////////////////////////////////////
            load_sample_1: BoolParam::new("Load Sample", false)
                .with_callback({
                    let file_dialog = file_dialog.clone();
                    Arc::new(move |_| file_dialog.store(true, Ordering::SeqCst))
                })
                .hide(),
            load_sample_2: BoolParam::new("Load Sample", false)
                .with_callback({
                    let file_dialog = file_dialog.clone();
                    Arc::new(move |_| file_dialog.store(true, Ordering::SeqCst))
                })
                .hide(),
            load_sample_3: BoolParam::new("Load Sample", false)
                .with_callback({
                    let file_dialog = file_dialog.clone();
                    Arc::new(move |_| file_dialog.store(true, Ordering::SeqCst))
                })
                .hide(),
            // To loop the sampler/granulizer
            loop_sample_1: BoolParam::new("Loop Sample", false).with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
            }),
            loop_sample_2: BoolParam::new("Loop Sample", false).with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
            }),
            loop_sample_3: BoolParam::new("Loop Sample", false).with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
            }),
            // Sampler only - toggle single cycle mode
            single_cycle_1: BoolParam::new("Single Cycle", false).with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
            }),
            single_cycle_2: BoolParam::new("Single Cycle", false).with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
            }),
            single_cycle_3: BoolParam::new("Single Cycle", false).with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
            }),
            // Always true for granulizer/ can be off for sampler
            restretch_1: BoolParam::new("Resample", true).with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
            }),
            restretch_2: BoolParam::new("Resample", true).with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
            }),
            restretch_3: BoolParam::new("Resample", true).with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
            }),
            // This is from 0 to 2000 samples
            grain_hold_1: IntParam::new("Hold", 200, IntRange::Linear { min: 5, max: 22050 })
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
                }),
            grain_hold_2: IntParam::new("Hold", 200, IntRange::Linear { min: 5, max: 22050 })
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
                }),
            grain_hold_3: IntParam::new("Hold", 200, IntRange::Linear { min: 5, max: 22050 })
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
                }),
            grain_gap_1: IntParam::new("Gap", 200, IntRange::Linear { min: 0, max: 22050 })
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
                }),
            grain_gap_2: IntParam::new("Gap", 200, IntRange::Linear { min: 0, max: 22050 })
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
                }),
            grain_gap_3: IntParam::new("Gap", 200, IntRange::Linear { min: 0, max: 22050 })
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
                }),
            // This is going to be in % since sample can be any size
            start_position_1: FloatParam::new(
                "Start",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_unit("%")
            .with_value_to_string(formatters::v2s_f32_percentage(0))
            .with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
            }),
            start_position_2: FloatParam::new(
                "Start",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_unit("%")
            .with_value_to_string(formatters::v2s_f32_percentage(0))
            .with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
            }),
            start_position_3: FloatParam::new(
                "Start",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_unit("%")
            .with_value_to_string(formatters::v2s_f32_percentage(0))
            .with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
            }),
            end_position_1: FloatParam::new("End", 1.0, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_unit("%")
                .with_value_to_string(formatters::v2s_f32_percentage(0))
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
                }),
            end_position_2: FloatParam::new("End", 1.0, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_unit("%")
                .with_value_to_string(formatters::v2s_f32_percentage(0))
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
                }),
            end_position_3: FloatParam::new("End", 1.0, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_unit("%")
                .with_value_to_string(formatters::v2s_f32_percentage(0))
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
                }),
            // Grain Crossfade
            grain_crossfade_1: IntParam::new("Shape", 50, IntRange::Linear { min: 2, max: 2000 })
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
                }),
            grain_crossfade_2: IntParam::new("Shape", 50, IntRange::Linear { min: 2, max: 2000 })
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
                }),
            grain_crossfade_3: IntParam::new("Shape", 50, IntRange::Linear { min: 2, max: 2000 })
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
                }),

            // Filters
            ////////////////////////////////////////////////////////////////////////////////////
            filter_lp_amount: FloatParam::new(
                "LPF",
                1.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_value_to_string(formatters::v2s_f32_percentage(0))
            .with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
            }),
            filter_hp_amount: FloatParam::new(
                "HPF",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_value_to_string(formatters::v2s_f32_percentage(0))
            .with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
            }),
            filter_bp_amount: FloatParam::new(
                "BPF",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_value_to_string(formatters::v2s_f32_percentage(0))
            .with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
            }),

            filter_wet: FloatParam::new("Filter", 1.0, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_unit("%")
                .with_value_to_string(formatters::v2s_f32_percentage(0))
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
                }),
            filter_resonance: FloatParam::new(
                "Res",
                1.0,
                FloatRange::Reversed(&FloatRange::Linear { min: 0.1, max: 1.0 }),
            )
            .with_unit("%")
            .with_value_to_string(formatters::v2s_f32_percentage(0)),
            filter_res_type: EnumParam::new("Res Type", ResonanceType::Default).with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
            }),
            filter_cutoff: FloatParam::new(
                "Cutoff",
                20000.0,
                FloatRange::Skewed {
                    min: 20.0,
                    max: 20000.0,
                    factor: 0.5,
                },
            )
            .with_step_size(1.0)
            .with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
            }),
            filter_alg_type: EnumParam::new("Filter Alg", FilterAlgorithms::SVF),
            tilt_filter_type: EnumParam::new("Filter Type", ResponseType::Lowpass),
            vcf_filter_type: EnumParam::new("Filter Type", VCResponseType::Lowpass),

            filter_env_peak: FloatParam::new(
                "Env Mod",
                0.0,
                FloatRange::Linear {
                    min: -14980.0,
                    max: 14980.0,
                },
            )
            .with_step_size(1.0)
            .with_value_to_string(format_nothing())
            .with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
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
                Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
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
                Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
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
                Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
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
                Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
            }),
            filter_env_atk_curve: EnumParam::new("Atk Curve", Oscillator::SmoothStyle::Linear)
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
                }),
            filter_env_dec_curve: EnumParam::new("Dec Curve", Oscillator::SmoothStyle::Linear)
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
                }),
            filter_env_rel_curve: EnumParam::new("Rel Curve", Oscillator::SmoothStyle::Linear)
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
                }),

            filter_lp_amount_2: FloatParam::new(
                "LPF",
                1.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_value_to_string(formatters::v2s_f32_percentage(0))
            .with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
            }),
            filter_hp_amount_2: FloatParam::new(
                "HPF",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_value_to_string(formatters::v2s_f32_percentage(0))
            .with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
            }),
            filter_bp_amount_2: FloatParam::new(
                "BPF",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_value_to_string(formatters::v2s_f32_percentage(0))
            .with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
            }),

            filter_wet_2: FloatParam::new("Filter", 1.0, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_unit("%")
                .with_value_to_string(formatters::v2s_f32_percentage(0))
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
                }),
            filter_resonance_2: FloatParam::new(
                "Res",
                1.0,
                FloatRange::Reversed(&FloatRange::Linear { min: 0.1, max: 1.0 }),
            )
            .with_unit("%")
            .with_value_to_string(formatters::v2s_f32_percentage(0)),
            filter_res_type_2: EnumParam::new("Res Type", ResonanceType::Default).with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
            }),
            filter_cutoff_2: FloatParam::new(
                "Cutoff",
                20000.0,
                FloatRange::Skewed {
                    min: 20.0,
                    max: 20000.0,
                    factor: 0.5,
                },
            )
            .with_step_size(1.0)
            .with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
            }),
            filter_alg_type_2: EnumParam::new("Filter Alg", FilterAlgorithms::SVF),
            tilt_filter_type_2: EnumParam::new("Filter Type", ResponseType::Lowpass),
            vcf_filter_type_2: EnumParam::new("Filter Type", VCResponseType::Lowpass),

            filter_env_peak_2: FloatParam::new(
                "Env Mod",
                0.0,
                FloatRange::Linear {
                    min: -14980.0,
                    max: 14980.0,
                },
            )
            .with_step_size(1.0)
            .with_value_to_string(format_nothing())
            .with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
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
                Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
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
                Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
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
                Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
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
                Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
            }),
            filter_env_atk_curve_2: EnumParam::new("Atk Curve", Oscillator::SmoothStyle::Linear)
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
                }),
            filter_env_dec_curve_2: EnumParam::new("Dec Curve", Oscillator::SmoothStyle::Linear)
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
                }),
            filter_env_rel_curve_2: EnumParam::new("Rel Curve", Oscillator::SmoothStyle::Linear)
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
                }),

            filter_cutoff_link: BoolParam::new("Filter Cutoffs Linked", false),

            // Pitch Envelope
            ////////////////////////////////////////////////////////////////////////////////////
            pitch_env_peak: FloatParam::new(
                "Pitch Env",
                0.0,
                FloatRange::Linear {
                    min: -144.0,
                    max: 144.0,
                },
            )
            //.with_step_size(1.0)
            //.with_value_to_string(format_nothing())
            .with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
            }),
            pitch_env_attack: FloatParam::new(
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
                Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
            }),
            pitch_env_decay: FloatParam::new(
                "Env Decay",
                300.0,
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
                Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
            }),
            pitch_env_sustain: FloatParam::new(
                "Env Sustain",
                0.0001,
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
                Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
            }),
            pitch_env_release: FloatParam::new(
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
                Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
            }),
            pitch_env_atk_curve: EnumParam::new("Atk Curve", Oscillator::SmoothStyle::Linear)
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
                }),
            pitch_env_dec_curve: EnumParam::new("Dec Curve", Oscillator::SmoothStyle::Linear)
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
                }),
            pitch_env_rel_curve: EnumParam::new("Rel Curve", Oscillator::SmoothStyle::Linear)
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
                }),
            pitch_enable: BoolParam::new("Pitch Enable", false),
            pitch_routing: EnumParam::new("Routing", PitchRouting::Osc1).with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
            }),

            pitch_env_peak_2: FloatParam::new(
                "Pitch Env",
                0.0,
                FloatRange::Linear {
                    min: -144.0,
                    max: 144.0,
                },
            )
            //.with_step_size(1.0)
            //.with_value_to_string(format_nothing())
            .with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
            }),
            pitch_env_attack_2: FloatParam::new(
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
                Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
            }),
            pitch_env_decay_2: FloatParam::new(
                "Env Decay",
                300.0,
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
                Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
            }),
            pitch_env_sustain_2: FloatParam::new(
                "Env Sustain",
                0.0001,
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
                Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
            }),
            pitch_env_release_2: FloatParam::new(
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
                Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
            }),
            pitch_env_atk_curve_2: EnumParam::new("Atk Curve", Oscillator::SmoothStyle::Linear)
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
                }),
            pitch_env_dec_curve_2: EnumParam::new("Dec Curve", Oscillator::SmoothStyle::Linear)
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
                }),
            pitch_env_rel_curve_2: EnumParam::new("Rel Curve", Oscillator::SmoothStyle::Linear)
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
                }),
            pitch_enable_2: BoolParam::new("Pitch Enable", false),
            pitch_routing_2: EnumParam::new("Routing", PitchRouting::Osc1).with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
            }),

            // LFOs
            ////////////////////////////////////////////////////////////////////////////////////
            lfo1_enable: BoolParam::new("LFO 1 Enable", false),
            lfo2_enable: BoolParam::new("LFO 2 Enable", false),
            lfo3_enable: BoolParam::new("LFO 3 Enable", false),
            lfo1_retrigger: EnumParam::new("LFO Retrigger", LFOController::LFORetrigger::None)
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
                }),
            lfo2_retrigger: EnumParam::new("LFO Retrigger", LFOController::LFORetrigger::None)
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
                }),
            lfo3_retrigger: EnumParam::new("LFO Retrigger", LFOController::LFORetrigger::None)
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
                }),
            lfo1_freq: FloatParam::new(
                "LFO1 Freq",
                4.62, // Defualt is half note at 138 bpm
                FloatRange::Skewed {
                    min: 1.0,
                    max: 9600.0,
                    factor: 0.3,
                }, // Based on max bpm of 300 w/ 32nd notes
            )
            .with_value_to_string(formatters::v2s_f32_hz_then_khz(2))
            .with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
            }),
            lfo2_freq: FloatParam::new(
                "LFO2 Freq",
                4.62, // Defualt is half note at 138 bpm
                FloatRange::Skewed {
                    min: 1.0,
                    max: 9600.0,
                    factor: 0.3,
                }, // Based on max bpm of 300 w/ 32nd notes
            )
            .with_value_to_string(formatters::v2s_f32_hz_then_khz(2))
            .with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
            }),
            lfo3_freq: FloatParam::new(
                "LFO3 Freq",
                4.62, // Defualt is half note at 138 bpm
                FloatRange::Skewed {
                    min: 1.0,
                    max: 9600.0,
                    factor: 0.3,
                }, // Based on max bpm of 300 w/ 32nd notes
            )
            .with_value_to_string(formatters::v2s_f32_hz_then_khz(2))
            .with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
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
                "LFO2 Phase",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),
            lfo3_phase: FloatParam::new(
                "LFO3 Phase",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),

            // Modulators
            ////////////////////////////////////////////////////////////////////////////////////
            mod_amount_knob_1: FloatParam::new(
                "Mod Amt 1",
                0.0,
                FloatRange::Linear {
                    min: -1.0,
                    max: 1.0,
                },
            )
            .with_value_to_string(format_nothing()),
            mod_amount_knob_2: FloatParam::new(
                "Mod Amt 2",
                0.0,
                FloatRange::Linear {
                    min: -1.0,
                    max: 1.0,
                },
            )
            .with_value_to_string(format_nothing()),
            mod_amount_knob_3: FloatParam::new(
                "Mod Amt 3",
                0.0,
                FloatRange::Linear {
                    min: -1.0,
                    max: 1.0,
                },
            )
            .with_value_to_string(format_nothing()),
            mod_amount_knob_4: FloatParam::new(
                "Mod Amt 4",
                0.0,
                FloatRange::Linear {
                    min: -1.0,
                    max: 1.0,
                },
            )
            .with_value_to_string(format_nothing()),
            mod_source_1: EnumParam::new("Source 1", ModulationSource::None),
            mod_source_2: EnumParam::new("Source 2", ModulationSource::None),
            mod_source_3: EnumParam::new("Source 3", ModulationSource::None),
            mod_source_4: EnumParam::new("Source 4", ModulationSource::None),
            mod_destination_1: EnumParam::new("Dest 1", ModulationDestination::None),
            mod_destination_2: EnumParam::new("Dest 2", ModulationDestination::None),
            mod_destination_3: EnumParam::new("Dest 3", ModulationDestination::None),
            mod_destination_4: EnumParam::new("Dest 4", ModulationDestination::None),

            // EQ
            pre_use_eq: BoolParam::new("EQ", false),
            pre_low_freq: FloatParam::new(
                "Low",
                800.0,
                FloatRange::Linear {
                    min: 100.0,
                    max: 2000.0,
                },
            )
            .with_step_size(1.0)
            .with_smoother(SmoothingStyle::Linear(5.0))
            .with_value_to_string(formatters::v2s_f32_hz_then_khz(0)),
            pre_mid_freq: FloatParam::new(
                "Mid",
                3000.0,
                FloatRange::Linear {
                    min: 1000.0,
                    max: 8000.0,
                },
            )
            .with_step_size(1.0)
            .with_smoother(SmoothingStyle::Linear(5.0))
            .with_value_to_string(formatters::v2s_f32_hz_then_khz(0)),
            pre_high_freq: FloatParam::new(
                "High",
                10000.0,
                FloatRange::Linear {
                    min: 3000.0,
                    max: 20000.0,
                },
            )
            .with_step_size(1.0)
            .with_smoother(SmoothingStyle::Linear(5.0))
            .with_value_to_string(formatters::v2s_f32_hz_then_khz(0)),
            pre_low_gain: FloatParam::new(
                "Low Gain",
                0.0,
                FloatRange::Linear {
                    min: -12.0,
                    max: 12.0,
                },
            )
            .with_value_to_string(formatters::v2s_f32_rounded(1)),
            pre_mid_gain: FloatParam::new(
                "Mid Gain",
                0.0,
                FloatRange::Linear {
                    min: -12.0,
                    max: 12.0,
                },
            )
            .with_value_to_string(formatters::v2s_f32_rounded(1)),
            pre_high_gain: FloatParam::new(
                "High Gain",
                0.0,
                FloatRange::Linear {
                    min: -12.0,
                    max: 12.0,
                },
            )
            .with_value_to_string(formatters::v2s_f32_rounded(1)),

            // fx
            use_fx: BoolParam::new("Use FX", true),

            use_compressor: BoolParam::new("Compressor", false),
            comp_amt: FloatParam::new("Amount", 0.3, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_value_to_string(formatters::v2s_f32_rounded(2)),
            comp_atk: FloatParam::new("Attack", 0.8, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_value_to_string(formatters::v2s_f32_rounded(2)),
            comp_rel: FloatParam::new("Release", 0.3, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_value_to_string(formatters::v2s_f32_rounded(2)),
            comp_drive: FloatParam::new("Drive", 0.3, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_value_to_string(formatters::v2s_f32_rounded(2)),

            use_abass: BoolParam::new("ABass", false),
            abass_amount: FloatParam::new(
                "Amount",
                0.000668,
                FloatRange::Skewed {
                    min: 0.0,
                    max: 1.0,
                    factor: 0.2,
                },
            )
            .with_value_to_string(formatters::v2s_f32_rounded(5)),

            use_saturation: BoolParam::new("Saturation", false),
            sat_amt: FloatParam::new("Amount", 0.0, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_value_to_string(formatters::v2s_f32_rounded(2)),
            sat_type: EnumParam::new("Type", SaturationType::Tape),

            use_delay: BoolParam::new("Delay", false),
            delay_amount: FloatParam::new("Amount", 0.5, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_value_to_string(formatters::v2s_f32_rounded(2)),
            delay_time: EnumParam::new("Time", DelaySnapValues::Quarter),
            delay_decay: FloatParam::new(
                "Decay",
                0.5,
                FloatRange::Linear {
                    min: 0.001,
                    max: 1.0,
                },
            )
            .with_value_to_string(formatters::v2s_f32_rounded(2)),
            delay_type: EnumParam::new("Type", DelayType::Stereo),

            use_reverb: BoolParam::new("Reverb", false),
            reverb_model: EnumParam::new("Model", ReverbModel::Default),
            reverb_amount: FloatParam::new(
                "Amount",
                0.85,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_value_to_string(formatters::v2s_f32_rounded(2)),
            reverb_size: FloatParam::new(
                "Size",
                1.0,
                FloatRange::Linear {
                    min: 0.001,
                    max: 2.0,
                },
            )
            .with_value_to_string(formatters::v2s_f32_rounded(2)),
            reverb_feedback: FloatParam::new(
                "Feedback",
                0.28,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_value_to_string(formatters::v2s_f32_rounded(2)),

            use_phaser: BoolParam::new("Phaser", false),
            phaser_amount: FloatParam::new(
                "Amount",
                0.5,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_value_to_string(formatters::v2s_f32_rounded(2)),
            phaser_depth: FloatParam::new("Depth", 1.0, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_value_to_string(formatters::v2s_f32_rounded(2)),
            phaser_rate: FloatParam::new(
                "Rate",
                1.0,
                FloatRange::Linear {
                    min: 0.001,
                    max: 16.0,
                },
            )
            .with_step_size(0.1)
            .with_value_to_string(formatters::v2s_f32_rounded(2)),
            phaser_feedback: FloatParam::new(
                "Feedback",
                0.5,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_value_to_string(formatters::v2s_f32_rounded(2)),

            use_buffermod: BoolParam::new("Buffer Modulator", false),
            buffermod_amount: FloatParam::new(
                "Amount",
                0.5,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_value_to_string(formatters::v2s_f32_rounded(2)),
            buffermod_depth: FloatParam::new(
                "Depth",
                1.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_value_to_string(formatters::v2s_f32_rounded(2)),
            buffermod_spread: FloatParam::new(
                "Spread",
                0.0,
                FloatRange::Skewed {
                    min: 0.0,
                    max: 1.0,
                    factor: 0.5,
                },
            )
            .with_step_size(0.001)
            .with_value_to_string(formatters::v2s_f32_rounded(3)),
            buffermod_rate: FloatParam::new(
                "Rate",
                0.01,
                FloatRange::Skewed {
                    min: 0.01,
                    max: 3.0,
                    factor: 0.5,
                },
            )
            .with_step_size(0.001)
            .with_value_to_string(formatters::v2s_f32_rounded(3)),
            buffermod_timing: FloatParam::new(
                "Buffer",
                620.0,
                FloatRange::Skewed {
                    min: 1.0,
                    max: 2700.0,
                    factor: 0.5,
                },
            )
            .with_step_size(1.0)
            .with_value_to_string(formatters::v2s_f32_rounded(0)),

            use_flanger: BoolParam::new("Flanger", false),
            flanger_amount: FloatParam::new(
                "Amount",
                0.5,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_value_to_string(formatters::v2s_f32_rounded(2)),
            flanger_depth: FloatParam::new("Depth", 0.5, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_value_to_string(formatters::v2s_f32_rounded(2)),
            flanger_rate: FloatParam::new(
                "Rate",
                5.0,
                FloatRange::Linear {
                    min: 0.001,
                    max: 24.0,
                },
            )
            .with_step_size(0.01)
            .with_value_to_string(formatters::v2s_f32_rounded(2)),
            flanger_feedback: FloatParam::new(
                "Feedback",
                0.5,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_value_to_string(formatters::v2s_f32_rounded(2)),

            use_limiter: BoolParam::new("Limiter", false),
            limiter_threshold: FloatParam::new(
                "Threshold",
                0.5,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_value_to_string(formatters::v2s_f32_rounded(2)),
            limiter_knee: FloatParam::new("Knee", 0.5, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_value_to_string(formatters::v2s_f32_rounded(2)),
            
            // FM
            fm_one_to_two: FloatParam::new("FM 1 to 2", 0.0, FloatRange::Skewed { min: 0.0, max: 20.0, factor: 0.3 })
                .with_value_to_string(formatters::v2s_f32_rounded(5)),
            
            fm_one_to_three: FloatParam::new("FM 1 to 3", 0.0, FloatRange::Skewed { min: 0.0, max: 20.0, factor: 0.3 })
                .with_value_to_string(formatters::v2s_f32_rounded(5)),
            
            fm_two_to_three: FloatParam::new("FM 2 to 3", 0.0, FloatRange::Skewed { min: 0.0, max: 20.0, factor: 0.3 })
                .with_value_to_string(formatters::v2s_f32_rounded(5)),
            fm_cycles: IntParam::new("Cycles", 1, IntRange::Linear { min: 1, max: 3 }),
            fm_attack: FloatParam::new(
                    "FM Attack",
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
                    Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
                }),
            fm_decay: FloatParam::new(
                    "FM Decay",
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
                    Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
                }),
            fm_sustain: FloatParam::new(
                    "FM Sustain",
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
                    Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
                }),
            fm_release: FloatParam::new(
                    "FM Release",
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
                    Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
                }),
            fm_attack_curve: EnumParam::new("Atk Curve", Oscillator::SmoothStyle::Linear)
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
                }),
            fm_decay_curve: EnumParam::new("Dec Curve", Oscillator::SmoothStyle::Linear)
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
                }),
            fm_release_curve: EnumParam::new("Rel Curve", Oscillator::SmoothStyle::Linear)
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::SeqCst))
                }),

            // UI Non-Param Params are dummy params for my buttons
            ////////////////////////////////////////////////////////////////////////////////////
            param_load_bank: BoolParam::new("Load Bank", false).hide(),
            param_save_bank: BoolParam::new("Save Bank", false).hide(),
            param_import_preset: BoolParam::new("Import Preset", false).hide(),
            param_export_preset: BoolParam::new("Export Preset", false).hide(),
            preset_category: EnumParam::new("Type", PresetType::Select).hide(),
            tag_acid: BoolParam::new("Acid", false).hide(),
            tag_analog: BoolParam::new("Analog", false).hide(),
            tag_bright: BoolParam::new("Bright", false).hide(),
            tag_chord: BoolParam::new("Chord", false).hide(),
            tag_crisp: BoolParam::new("Crisp", false).hide(),
            tag_deep: BoolParam::new("Deep", false).hide(),
            tag_delicate: BoolParam::new("Delicate", false).hide(),
            tag_hard: BoolParam::new("Hard", false).hide(),
            tag_harsh: BoolParam::new("Harsh", false).hide(),
            tag_lush: BoolParam::new("Lush", false).hide(),
            tag_mellow: BoolParam::new("Mellow", false).hide(),
            tag_resonant: BoolParam::new("Resonant", false).hide(),
            tag_rich: BoolParam::new("Rich", false).hide(),
            tag_sharp: BoolParam::new("Sharp", false).hide(),
            tag_silky: BoolParam::new("Silky", false).hide(),
            tag_smooth: BoolParam::new("Smooth", false).hide(),
            tag_soft: BoolParam::new("Soft", false).hide(),
            tag_stab: BoolParam::new("Stab", false).hide(),
            tag_warm: BoolParam::new("Warm", false).hide(),

            // For some reason the callback doesn't work right here so I went by validating params for previous and next
            param_next_preset: BoolParam::new("->", false).hide(),
            param_prev_preset: BoolParam::new("<-", false).hide(),

            param_update_current_preset: BoolParam::new("Update Preset", false)
                .with_callback({
                    let update_current_preset = update_current_preset.clone();
                    Arc::new(move |_| update_current_preset.store(true, Ordering::SeqCst))
                })
                .hide(),

            // Not a param
            loading: BoolParam::new("loading_mod", false).hide(),
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
        actuate_gui::make_actuate_gui(self, _async_executor)
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

    // Main processing thread that happens before the midi processing per-sample
    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        // Clear any voices on change of module type (especially during play)
        // This fixes panics and other broken things attempting to play during preset change/load
        if self.clear_voices.clone().load(Ordering::SeqCst) {
            self.audio_module_1.lock().unwrap().clear_voices();
            self.audio_module_2.lock().unwrap().clear_voices();
            self.audio_module_3.lock().unwrap().clear_voices();

            self.clear_voices.store(false, Ordering::SeqCst);
            self.update_something.store(true, Ordering::SeqCst);
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
        let mut lfo_1_current: f32 = -2.0;
        let mut lfo_2_current: f32 = -2.0;
        let mut lfo_3_current: f32 = -2.0;

        // Update our LFOs per each sample
        /////////////////////////////////////////////////////////////////////////////////////////////
        let bpm = context.transport().tempo.unwrap_or(1.0) as f32;
        if bpm == 1.0 {
            // This means we are not getting proper tempo so LFO can't sync
            return;
        }
        if self.params.lfo1_enable.value() {
            // Update LFO Frequency
            if self.params.lfo1_sync.value() {
                let divisor = match self.params.lfo1_snap.value() {
                    LFOController::LFOSnapValues::Quad => 16.0,
                    LFOController::LFOSnapValues::QuadD => 16.0 * 1.5,
                    LFOController::LFOSnapValues::QuadT => 16.0 / 3.0,
                    LFOController::LFOSnapValues::Double => 8.0,
                    LFOController::LFOSnapValues::DoubleD => 8.0 * 1.5,
                    LFOController::LFOSnapValues::DoubleT => 8.0 / 3.0,
                    LFOController::LFOSnapValues::Whole => 4.0,
                    LFOController::LFOSnapValues::WholeD => 4.0 * 1.5,
                    LFOController::LFOSnapValues::WholeT => 4.0 / 3.0,
                    LFOController::LFOSnapValues::Half => 2.0,
                    LFOController::LFOSnapValues::HalfD => 2.0 * 1.5,
                    LFOController::LFOSnapValues::HalfT => 2.0 / 3.0,
                    LFOController::LFOSnapValues::Quarter => 1.0,
                    LFOController::LFOSnapValues::QuarterD => 1.0 * 1.5,
                    LFOController::LFOSnapValues::QuarterT => 1.0 / 3.0,
                    LFOController::LFOSnapValues::Eighth => 0.5,
                    LFOController::LFOSnapValues::EighthD => 0.5 * 1.5,
                    LFOController::LFOSnapValues::EighthT => 0.5 / 3.0,
                    LFOController::LFOSnapValues::Sixteen => 0.25,
                    LFOController::LFOSnapValues::SixteenD => 0.25 * 1.5,
                    LFOController::LFOSnapValues::SixteenT => 0.25 / 3.0,
                    LFOController::LFOSnapValues::ThirtySecond => 0.125,
                    LFOController::LFOSnapValues::ThirtySecondD => 0.125 * 1.5,
                    LFOController::LFOSnapValues::ThirtySecondT => 0.125 / 3.0,
                };
                let freq_snap = (bpm / divisor) / 60.0;
                if self.params.lfo1_freq.value() != freq_snap {
                    self.lfo_1.set_frequency(freq_snap);
                }
            } else {
                if self.params.lfo1_freq.value() != self.lfo_1.get_frequency() {
                    self.lfo_1.set_frequency(self.params.lfo1_freq.value());
                }
            }

            // Update LFO Waveform
            if self.params.lfo1_waveform.value() != self.lfo_1.get_waveform() {
                self.lfo_1.set_waveform(self.params.lfo1_waveform.value());
            }
        }
        if self.params.lfo2_enable.value() {
            // Update LFO Frequency
            if self.params.lfo2_sync.value() {
                let divisor = match self.params.lfo2_snap.value() {
                    LFOController::LFOSnapValues::Quad => 16.0,
                    LFOController::LFOSnapValues::QuadD => 16.0 * 1.5,
                    LFOController::LFOSnapValues::QuadT => 16.0 / 3.0,
                    LFOController::LFOSnapValues::Double => 8.0,
                    LFOController::LFOSnapValues::DoubleD => 8.0 * 1.5,
                    LFOController::LFOSnapValues::DoubleT => 8.0 / 3.0,
                    LFOController::LFOSnapValues::Whole => 4.0,
                    LFOController::LFOSnapValues::WholeD => 4.0 * 1.5,
                    LFOController::LFOSnapValues::WholeT => 4.0 / 3.0,
                    LFOController::LFOSnapValues::Half => 2.0,
                    LFOController::LFOSnapValues::HalfD => 2.0 * 1.5,
                    LFOController::LFOSnapValues::HalfT => 2.0 / 3.0,
                    LFOController::LFOSnapValues::Quarter => 1.0,
                    LFOController::LFOSnapValues::QuarterD => 1.0 * 1.5,
                    LFOController::LFOSnapValues::QuarterT => 1.0 / 3.0,
                    LFOController::LFOSnapValues::Eighth => 0.5,
                    LFOController::LFOSnapValues::EighthD => 0.5 * 1.5,
                    LFOController::LFOSnapValues::EighthT => 0.5 / 3.0,
                    LFOController::LFOSnapValues::Sixteen => 0.25,
                    LFOController::LFOSnapValues::SixteenD => 0.25 * 1.5,
                    LFOController::LFOSnapValues::SixteenT => 0.25 / 3.0,
                    LFOController::LFOSnapValues::ThirtySecond => 0.125,
                    LFOController::LFOSnapValues::ThirtySecondD => 0.125 * 1.5,
                    LFOController::LFOSnapValues::ThirtySecondT => 0.125 / 3.0,
                };
                let freq_snap = (bpm / divisor) / 60.0;
                if self.params.lfo2_freq.value() != freq_snap {
                    self.lfo_2.set_frequency(freq_snap);
                }
            } else {
                if self.params.lfo2_freq.value() != self.lfo_2.get_frequency() {
                    self.lfo_2.set_frequency(self.params.lfo2_freq.value());
                }
            }

            // Update LFO Waveform
            if self.params.lfo2_waveform.value() != self.lfo_2.get_waveform() {
                self.lfo_2.set_waveform(self.params.lfo2_waveform.value());
            }
        }
        if self.params.lfo3_enable.value() {
            // Update LFO Frequency
            if self.params.lfo3_sync.value() {
                let divisor = match self.params.lfo3_snap.value() {
                    LFOController::LFOSnapValues::Quad => 16.0,
                    LFOController::LFOSnapValues::QuadD => 16.0 * 1.5,
                    LFOController::LFOSnapValues::QuadT => 16.0 / 3.0,
                    LFOController::LFOSnapValues::Double => 8.0,
                    LFOController::LFOSnapValues::DoubleD => 8.0 * 1.5,
                    LFOController::LFOSnapValues::DoubleT => 8.0 / 3.0,
                    LFOController::LFOSnapValues::Whole => 4.0,
                    LFOController::LFOSnapValues::WholeD => 4.0 * 1.5,
                    LFOController::LFOSnapValues::WholeT => 4.0 / 3.0,
                    LFOController::LFOSnapValues::Half => 2.0,
                    LFOController::LFOSnapValues::HalfD => 2.0 * 1.5,
                    LFOController::LFOSnapValues::HalfT => 2.0 / 3.0,
                    LFOController::LFOSnapValues::Quarter => 1.0,
                    LFOController::LFOSnapValues::QuarterD => 1.0 * 1.5,
                    LFOController::LFOSnapValues::QuarterT => 1.0 / 3.0,
                    LFOController::LFOSnapValues::Eighth => 0.5,
                    LFOController::LFOSnapValues::EighthD => 0.5 * 1.5,
                    LFOController::LFOSnapValues::EighthT => 0.5 / 3.0,
                    LFOController::LFOSnapValues::Sixteen => 0.25,
                    LFOController::LFOSnapValues::SixteenD => 0.25 * 1.5,
                    LFOController::LFOSnapValues::SixteenT => 0.25 / 3.0,
                    LFOController::LFOSnapValues::ThirtySecond => 0.125,
                    LFOController::LFOSnapValues::ThirtySecondD => 0.125 * 1.5,
                    LFOController::LFOSnapValues::ThirtySecondT => 0.125 / 3.0,
                };
                let freq_snap = (bpm / divisor) / 60.0;
                if self.params.lfo3_freq.value() != freq_snap {
                    self.lfo_3.set_frequency(freq_snap);
                }
            } else {
                if self.params.lfo3_freq.value() != self.lfo_3.get_frequency() {
                    self.lfo_3.set_frequency(self.params.lfo3_freq.value());
                }
            }

            // Update LFO Waveform
            if self.params.lfo3_waveform.value() != self.lfo_3.get_waveform() {
                self.lfo_3.set_waveform(self.params.lfo3_waveform.value());
            }
        }

        for (sample_id, mut channel_samples) in buffer.iter_samples().enumerate() {
            // Get around post file loading breaking things with an arbitrary buffer
            if self.file_dialog.load(Ordering::Acquire) {
                self.file_open_buffer_timer.store(
                    self.file_open_buffer_timer.load(Ordering::SeqCst) + 1,
                    Ordering::SeqCst,
                );
                if self.file_open_buffer_timer.load(Ordering::SeqCst) > FILE_OPEN_BUFFER_MAX {
                    self.file_open_buffer_timer.store(0, Ordering::SeqCst);
                    self.file_dialog.store(false, Ordering::SeqCst); //Changed from Release
                }
            }

            // If the Update Current Preset button has been pressed
            if self.update_current_preset.load(Ordering::SeqCst)
                && !self.file_dialog.load(Ordering::SeqCst)
            {
                self.file_dialog.store(true, Ordering::SeqCst);
                self.file_open_buffer_timer.store(1, Ordering::SeqCst);
                self.update_current_preset();
                self.update_current_preset.store(false, Ordering::SeqCst);

                // Save persistent sample data
                let am1_lock = self.audio_module_1.lock().unwrap();
                let am2_lock = self.audio_module_2.lock().unwrap();
                let am3_lock = self.audio_module_3.lock().unwrap();
                match am1_lock.audio_module_type {
                    AudioModuleType::Sampler | AudioModuleType::Granulizer => {
                        *self.params.am1_sample.lock().unwrap() = am1_lock.loaded_sample.clone();
                    },
                    _ => {},
                }
                match am2_lock.audio_module_type {
                    AudioModuleType::Sampler | AudioModuleType::Granulizer => {
                        *self.params.am2_sample.lock().unwrap() = am2_lock.loaded_sample.clone();
                    },
                    _ => {},
                }
                match am3_lock.audio_module_type {
                    AudioModuleType::Sampler | AudioModuleType::Granulizer => {
                        *self.params.am3_sample.lock().unwrap() = am3_lock.loaded_sample.clone();
                    },
                    _ => {},
                }
            }

            // Prevent processing if our file dialog is open!!!
            if self.file_dialog.load(Ordering::SeqCst) {
                return;
            }

            // Processing
            /////////////////////////////////////////////////////////////////////////////////////////////////

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
                let AM2 = self.audio_module_2.clone();
                let AM3 = self.audio_module_3.clone();

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
            if self.update_something.load(Ordering::SeqCst) {
                self.audio_module_1
                    .lock()
                    .unwrap()
                    .consume_params(self.params.clone(), 1);
                self.audio_module_2
                    .lock()
                    .unwrap()
                    .consume_params(self.params.clone(), 2);
                self.audio_module_3
                    .lock()
                    .unwrap()
                    .consume_params(self.params.clone(), 3);
                // Fix Auto restretch/repitch behavior
                if self.prev_restretch_1.load(Ordering::SeqCst) != self.params.restretch_1.value() {
                    self.prev_restretch_1.store(self.params.restretch_1.value(), Ordering::SeqCst);
                    self.audio_module_1.lock().unwrap().regenerate_samples();
                }
                if self.prev_restretch_2.load(Ordering::SeqCst) != self.params.restretch_2.value() {
                    self.prev_restretch_2.store(self.params.restretch_2.value(), Ordering::SeqCst);
                    
                    self.audio_module_2.lock().unwrap().regenerate_samples();
                }
                if self.prev_restretch_3.load(Ordering::SeqCst) != self.params.restretch_3.value() {
                    self.prev_restretch_3.store(self.params.restretch_3.value(), Ordering::SeqCst);
                    self.audio_module_3.lock().unwrap().regenerate_samples();
                }

                self.update_something.store(false, Ordering::SeqCst);
            }

            // Modulations
            /////////////////////////////////////////////////////////////////////////////////////////////////
            let mod_value_1: f32;
            let mod_value_2: f32;
            let mod_value_3: f32;
            let mod_value_4: f32;

            // If no modulations this = -2.0
            mod_value_1 = match self.params.mod_source_1.value() {
                ModulationSource::None | ModulationSource::UnsetModulation => -2.0,
                ModulationSource::LFO1 => lfo_1_current * self.params.mod_amount_knob_1.value(),
                ModulationSource::LFO2 => lfo_2_current * self.params.mod_amount_knob_1.value(),
                ModulationSource::LFO3 => lfo_3_current * self.params.mod_amount_knob_1.value(),
                ModulationSource::Velocity => {
                    // This is to allow invalid midi events to not break this logic since we only want NoteOn
                    match midi_event.clone().unwrap_or(NoteEvent::Choke {
                        timing: 0_u32,
                        voice_id: Some(0_i32),
                        channel: 0_u8,
                        note: 0_u8,
                    }) {
                        NoteEvent::NoteOn {
                            velocity,
                            timing: _,
                            voice_id: _,
                            channel: _,
                            note: _,
                        } => {
                            // Store velocity on new note happening
                            let vel = (velocity * self.params.mod_amount_knob_1.value().abs())
                                .clamp(0.0, 1.0);
                            if velocity != -1.0 {
                                self.current_note_on_velocity.store(vel, Ordering::SeqCst);
                            }
                            vel
                        }
                        _ => -2.0,
                    }
                }
            };

            mod_value_2 = match self.params.mod_source_2.value() {
                ModulationSource::None | ModulationSource::UnsetModulation => -2.0,
                ModulationSource::LFO1 => lfo_1_current * self.params.mod_amount_knob_2.value(),
                ModulationSource::LFO2 => lfo_2_current * self.params.mod_amount_knob_2.value(),
                ModulationSource::LFO3 => lfo_3_current * self.params.mod_amount_knob_2.value(),
                ModulationSource::Velocity => {
                    match midi_event.clone().unwrap_or(NoteEvent::Choke {
                        timing: 0_u32,
                        voice_id: Some(0_i32),
                        channel: 0_u8,
                        note: 0_u8,
                    }) {
                        NoteEvent::NoteOn {
                            velocity,
                            timing: _,
                            voice_id: _,
                            channel: _,
                            note: _,
                        } => {
                            if velocity != -1.0 {
                                self.current_note_on_velocity
                                    .store(velocity, Ordering::SeqCst);
                            }
                            (velocity * self.params.mod_amount_knob_2.value().abs()).clamp(0.0, 1.0)
                        }
                        _ => -2.0,
                    }
                }
            };

            mod_value_3 = match self.params.mod_source_3.value() {
                ModulationSource::None | ModulationSource::UnsetModulation => -2.0,
                ModulationSource::LFO1 => lfo_1_current * self.params.mod_amount_knob_3.value(),
                ModulationSource::LFO2 => lfo_2_current * self.params.mod_amount_knob_3.value(),
                ModulationSource::LFO3 => lfo_3_current * self.params.mod_amount_knob_3.value(),
                ModulationSource::Velocity => {
                    match midi_event.clone().unwrap_or(NoteEvent::Choke {
                        timing: 0_u32,
                        voice_id: Some(0_i32),
                        channel: 0_u8,
                        note: 0_u8,
                    }) {
                        NoteEvent::NoteOn {
                            velocity,
                            timing: _,
                            voice_id: _,
                            channel: _,
                            note: _,
                        } => {
                            if velocity != -1.0 {
                                self.current_note_on_velocity
                                    .store(velocity, Ordering::SeqCst);
                            }
                            (velocity * self.params.mod_amount_knob_3.value().abs()).clamp(0.0, 1.0)
                        }
                        _ => -2.0,
                    }
                }
            };

            mod_value_4 = match self.params.mod_source_4.value() {
                ModulationSource::None | ModulationSource::UnsetModulation => -2.0,
                ModulationSource::LFO1 => lfo_1_current * self.params.mod_amount_knob_4.value(),
                ModulationSource::LFO2 => lfo_2_current * self.params.mod_amount_knob_4.value(),
                ModulationSource::LFO3 => lfo_3_current * self.params.mod_amount_knob_4.value(),
                ModulationSource::Velocity => {
                    match midi_event.clone().unwrap_or(NoteEvent::Choke {
                        timing: 0_u32,
                        voice_id: Some(0_i32),
                        channel: 0_u8,
                        note: 0_u8,
                    }) {
                        NoteEvent::NoteOn {
                            velocity,
                            timing: _,
                            voice_id: _,
                            channel: _,
                            note: _,
                        } => {
                            if velocity != -1.0 {
                                self.current_note_on_velocity
                                    .store(velocity, Ordering::SeqCst);
                            }
                            (velocity * self.params.mod_amount_knob_4.value().abs()).clamp(0.0, 1.0)
                        }
                        _ => -2.0,
                    }
                }
            };

            let mut temp_mod_cutoff_1_source_1: f32 = 0.0;
            let mut temp_mod_cutoff_1_source_2: f32 = 0.0;
            let mut temp_mod_cutoff_1_source_3: f32 = 0.0;
            let mut temp_mod_cutoff_1_source_4: f32 = 0.0;
            let mut temp_mod_cutoff_2_source_1: f32 = 0.0;
            let mut temp_mod_cutoff_2_source_2: f32 = 0.0;
            let mut temp_mod_cutoff_2_source_3: f32 = 0.0;
            let mut temp_mod_cutoff_2_source_4: f32 = 0.0;
            let mut temp_mod_resonance_1_source_1: f32 = 0.0;
            let mut temp_mod_resonance_1_source_2: f32 = 0.0;
            let mut temp_mod_resonance_1_source_3: f32 = 0.0;
            let mut temp_mod_resonance_1_source_4: f32 = 0.0;
            let mut temp_mod_resonance_2_source_1: f32 = 0.0;
            let mut temp_mod_resonance_2_source_2: f32 = 0.0;
            let mut temp_mod_resonance_2_source_3: f32 = 0.0;
            let mut temp_mod_resonance_2_source_4: f32 = 0.0;
            let mut temp_mod_detune_1: f32 = 0.0;
            let mut temp_mod_detune_2: f32 = 0.0;
            let mut temp_mod_detune_3: f32 = 0.0;
            let mut temp_mod_uni_detune_1: f32 = 0.0;
            let mut temp_mod_uni_detune_2: f32 = 0.0;
            let mut temp_mod_uni_detune_3: f32 = 0.0;
            // These are used for velocity to detune linkages
            let mut temp_mod_vel_sum: f32 = 0.0;
            let mut temp_mod_uni_vel_sum: f32 = 0.0;
            let mut temp_mod_gain_1: f32 = -2.0;
            let mut temp_mod_gain_2: f32 = -2.0;
            let mut temp_mod_gain_3: f32 = -2.0;
            let mut temp_mod_lfo_gain_1: f32 = 1.0;
            let mut temp_mod_lfo_gain_2: f32 = 1.0;
            let mut temp_mod_lfo_gain_3: f32 = 1.0;
            // Modulation structs to pass things
            let modulations_1: ModulationStruct;
            let modulations_2: ModulationStruct;
            let modulations_3: ModulationStruct;
            let modulations_4: ModulationStruct;

            // In this modulation section the velocity stuff is all weird since we need to pass velocity mod
            // But this happens before we process the note values hence storing/passing it

            // This is outside for held notes on specific source -> destinations
            // This would happen when mod_value_X == 2.0 as a result - hence using the Atomic for velocity

            if self.params.mod_source_1.value() == ModulationSource::Velocity {
                match self.params.mod_destination_1.value() {
                    ModulationDestination::Cutoff_1 => {
                        temp_mod_cutoff_1_source_1 +=
                            8000.0 * self.current_note_on_velocity.load(Ordering::SeqCst);
                    }
                    ModulationDestination::Cutoff_2 => {
                        temp_mod_cutoff_2_source_1 +=
                            8000.0 * self.current_note_on_velocity.load(Ordering::SeqCst);
                    }
                    ModulationDestination::All_Gain => {
                        if self.params.mod_source_1.value() == ModulationSource::Velocity {
                            let vel = self.current_note_on_velocity.load(Ordering::SeqCst);
                            temp_mod_gain_1 = vel;
                            temp_mod_gain_2 = vel;
                            temp_mod_gain_3 = vel;
                        }
                    }
                    ModulationDestination::Osc1_Gain => {
                        if self.params.mod_source_1.value() == ModulationSource::Velocity {
                            temp_mod_gain_1 = self.current_note_on_velocity.load(Ordering::SeqCst);
                        }
                    }
                    ModulationDestination::Osc2_Gain => {
                        if self.params.mod_source_1.value() == ModulationSource::Velocity {
                            temp_mod_gain_2 = self.current_note_on_velocity.load(Ordering::SeqCst);
                        }
                    }
                    ModulationDestination::Osc3_Gain => {
                        if self.params.mod_source_1.value() == ModulationSource::Velocity {
                            temp_mod_gain_3 = self.current_note_on_velocity.load(Ordering::SeqCst);
                        }
                    }
                    ModulationDestination::Resonance_1 => {
                        if self.params.mod_source_1.value() == ModulationSource::Velocity {
                            temp_mod_resonance_1_source_1 +=
                                self.current_note_on_velocity.load(Ordering::SeqCst);
                        }
                    }
                    ModulationDestination::Resonance_2 => {
                        if self.params.mod_source_1.value() == ModulationSource::Velocity {
                            temp_mod_resonance_2_source_1 +=
                                self.current_note_on_velocity.load(Ordering::SeqCst);
                        }
                    }
                    _ => {}
                }
            }
            if self.params.mod_source_2.value() == ModulationSource::Velocity {
                match self.params.mod_destination_2.value() {
                    ModulationDestination::Cutoff_1 => {
                        temp_mod_cutoff_1_source_2 +=
                            8000.0 * self.current_note_on_velocity.load(Ordering::SeqCst);
                    }
                    ModulationDestination::Cutoff_2 => {
                        temp_mod_cutoff_2_source_2 +=
                            8000.0 * self.current_note_on_velocity.load(Ordering::SeqCst);
                    }
                    ModulationDestination::All_Gain => {
                        if self.params.mod_source_2.value() == ModulationSource::Velocity {
                            let vel = self.current_note_on_velocity.load(Ordering::SeqCst);
                            temp_mod_gain_1 = vel;
                            temp_mod_gain_2 = vel;
                            temp_mod_gain_3 = vel;
                        }
                    }
                    ModulationDestination::Osc1_Gain => {
                        if self.params.mod_source_2.value() == ModulationSource::Velocity {
                            temp_mod_gain_1 = self.current_note_on_velocity.load(Ordering::SeqCst);
                        }
                    }
                    ModulationDestination::Osc2_Gain => {
                        if self.params.mod_source_2.value() == ModulationSource::Velocity {
                            temp_mod_gain_2 = self.current_note_on_velocity.load(Ordering::SeqCst);
                        }
                    }
                    ModulationDestination::Osc3_Gain => {
                        if self.params.mod_source_2.value() == ModulationSource::Velocity {
                            temp_mod_gain_3 = self.current_note_on_velocity.load(Ordering::SeqCst);
                        }
                    }
                    ModulationDestination::Resonance_1 => {
                        if self.params.mod_source_2.value() == ModulationSource::Velocity {
                            temp_mod_resonance_1_source_2 -=
                                self.current_note_on_velocity.load(Ordering::SeqCst);
                        }
                    }
                    ModulationDestination::Resonance_2 => {
                        if self.params.mod_source_2.value() == ModulationSource::Velocity {
                            temp_mod_resonance_2_source_2 -=
                                self.current_note_on_velocity.load(Ordering::SeqCst);
                        }
                    }
                    _ => {}
                }
            }
            if self.params.mod_source_3.value() == ModulationSource::Velocity {
                match self.params.mod_destination_3.value() {
                    ModulationDestination::Cutoff_1 => {
                        temp_mod_cutoff_1_source_3 +=
                            8000.0 * self.current_note_on_velocity.load(Ordering::SeqCst);
                    }
                    ModulationDestination::Cutoff_2 => {
                        temp_mod_cutoff_2_source_3 +=
                            8000.0 * self.current_note_on_velocity.load(Ordering::SeqCst);
                    }
                    ModulationDestination::All_Gain => {
                        if self.params.mod_source_3.value() == ModulationSource::Velocity {
                            let vel = self.current_note_on_velocity.load(Ordering::SeqCst);
                            temp_mod_gain_1 = vel;
                            temp_mod_gain_2 = vel;
                            temp_mod_gain_3 = vel;
                        }
                    }
                    ModulationDestination::Osc1_Gain => {
                        if self.params.mod_source_3.value() == ModulationSource::Velocity {
                            temp_mod_gain_1 = self.current_note_on_velocity.load(Ordering::SeqCst);
                        }
                    }
                    ModulationDestination::Osc2_Gain => {
                        if self.params.mod_source_3.value() == ModulationSource::Velocity {
                            temp_mod_gain_2 = self.current_note_on_velocity.load(Ordering::SeqCst);
                        }
                    }
                    ModulationDestination::Osc3_Gain => {
                        if self.params.mod_source_3.value() == ModulationSource::Velocity {
                            temp_mod_gain_3 = self.current_note_on_velocity.load(Ordering::SeqCst);
                        }
                    }
                    ModulationDestination::Resonance_1 => {
                        if self.params.mod_source_3.value() == ModulationSource::Velocity {
                            temp_mod_resonance_1_source_3 +=
                                self.current_note_on_velocity.load(Ordering::SeqCst);
                        }
                    }
                    ModulationDestination::Resonance_2 => {
                        if self.params.mod_source_3.value() == ModulationSource::Velocity {
                            temp_mod_resonance_2_source_3 +=
                                self.current_note_on_velocity.load(Ordering::SeqCst);
                        }
                    }
                    _ => {}
                }
            }
            if self.params.mod_source_4.value() == ModulationSource::Velocity {
                match self.params.mod_destination_4.value() {
                    ModulationDestination::Cutoff_1 => {
                        temp_mod_cutoff_1_source_4 +=
                            8000.0 * self.current_note_on_velocity.load(Ordering::SeqCst);
                    }
                    ModulationDestination::Cutoff_2 => {
                        temp_mod_cutoff_2_source_4 +=
                            8000.0 * self.current_note_on_velocity.load(Ordering::SeqCst);
                    }
                    ModulationDestination::All_Gain => {
                        if self.params.mod_source_4.value() == ModulationSource::Velocity {
                            let vel = self.current_note_on_velocity.load(Ordering::SeqCst);
                            temp_mod_gain_1 = vel;
                            temp_mod_gain_2 = vel;
                            temp_mod_gain_3 = vel;
                        }
                    }
                    ModulationDestination::Osc1_Gain => {
                        if self.params.mod_source_4.value() == ModulationSource::Velocity {
                            temp_mod_gain_1 = self.current_note_on_velocity.load(Ordering::SeqCst);
                        }
                    }
                    ModulationDestination::Osc2_Gain => {
                        if self.params.mod_source_4.value() == ModulationSource::Velocity {
                            temp_mod_gain_2 = self.current_note_on_velocity.load(Ordering::SeqCst);
                        }
                    }
                    ModulationDestination::Osc3_Gain => {
                        if self.params.mod_source_4.value() == ModulationSource::Velocity {
                            temp_mod_gain_3 = self.current_note_on_velocity.load(Ordering::SeqCst);
                        }
                    }
                    ModulationDestination::Resonance_1 => {
                        if self.params.mod_source_4.value() == ModulationSource::Velocity {
                            temp_mod_resonance_1_source_4 +=
                                self.current_note_on_velocity.load(Ordering::SeqCst);
                        }
                    }
                    ModulationDestination::Resonance_2 => {
                        if self.params.mod_source_4.value() == ModulationSource::Velocity {
                            temp_mod_resonance_2_source_4 +=
                                self.current_note_on_velocity.load(Ordering::SeqCst);
                        }
                    }
                    _ => {}
                }
            }

            ///////////////////////////////////////////////////////////////
            // If mod_value is not -2.0 we are in Note ON event or an LFO
            if mod_value_1 != -2.0 {
                match self.params.mod_destination_1.value() {
                    ModulationDestination::None | ModulationDestination::UnsetModulation => {}
                    ModulationDestination::Cutoff_1 => {
                        if self.params.mod_source_1.value() == ModulationSource::Velocity {
                            // I don't think this gets reached in Velocity case because of mod_value_X
                            temp_mod_cutoff_1_source_1 +=
                                20000.0 * self.current_note_on_velocity.load(Ordering::SeqCst);
                        } else {
                            temp_mod_cutoff_1_source_1 += 20000.0 * mod_value_1;
                        }
                    }
                    ModulationDestination::Cutoff_2 => {
                        if self.params.mod_source_1.value() == ModulationSource::Velocity {
                            temp_mod_cutoff_2_source_1 +=
                                20000.0 * self.current_note_on_velocity.load(Ordering::SeqCst);
                        } else {
                            temp_mod_cutoff_2_source_1 += 20000.0 * mod_value_1;
                        }
                    }
                    ModulationDestination::Resonance_1 => {
                        if self.params.mod_source_1.value() == ModulationSource::Velocity {
                            temp_mod_resonance_1_source_1 -=
                                self.current_note_on_velocity.load(Ordering::SeqCst);
                        } else {
                            temp_mod_resonance_1_source_1 -= mod_value_1;
                        }
                    }
                    ModulationDestination::Resonance_2 => {
                        if self.params.mod_source_1.value() == ModulationSource::Velocity {
                            temp_mod_resonance_2_source_1 -=
                                self.current_note_on_velocity.load(Ordering::SeqCst);
                        } else {
                            temp_mod_resonance_2_source_1 -= mod_value_1;
                        }
                    }
                    ModulationDestination::All_Detune => {
                        if self.params.mod_source_1.value() == ModulationSource::Velocity {
                            temp_mod_vel_sum += mod_value_1;
                        }
                        temp_mod_detune_1 += mod_value_1;
                        temp_mod_detune_2 += mod_value_1;
                        temp_mod_detune_3 += mod_value_1;
                    }
                    ModulationDestination::Osc1Detune => {
                        if self.params.mod_source_1.value() == ModulationSource::Velocity {
                            temp_mod_vel_sum += mod_value_1;
                        }
                        temp_mod_detune_1 += mod_value_1;
                    }
                    ModulationDestination::Osc2Detune => {
                        if self.params.mod_source_1.value() == ModulationSource::Velocity {
                            temp_mod_vel_sum += mod_value_1;
                        }
                        temp_mod_detune_2 += mod_value_1;
                    }
                    ModulationDestination::Osc3Detune => {
                        if self.params.mod_source_1.value() == ModulationSource::Velocity {
                            temp_mod_vel_sum += mod_value_1;
                        }
                        temp_mod_detune_3 += mod_value_1;
                    }
                    ModulationDestination::All_UniDetune => {
                        if self.params.mod_source_1.value() == ModulationSource::Velocity {
                            temp_mod_vel_sum += mod_value_1;
                        }
                        temp_mod_uni_detune_1 += mod_value_1;
                        temp_mod_uni_detune_2 += mod_value_1;
                        temp_mod_uni_detune_3 += mod_value_1;
                    }
                    ModulationDestination::Osc1UniDetune => {
                        if self.params.mod_source_1.value() == ModulationSource::Velocity {
                            temp_mod_uni_vel_sum += mod_value_1;
                        }
                        temp_mod_uni_detune_1 += mod_value_1;
                    }
                    ModulationDestination::Osc2UniDetune => {
                        if self.params.mod_source_1.value() == ModulationSource::Velocity {
                            temp_mod_uni_vel_sum += mod_value_1;
                        }
                        temp_mod_uni_detune_2 += mod_value_1;
                    }
                    ModulationDestination::Osc3UniDetune => {
                        if self.params.mod_source_1.value() == ModulationSource::Velocity {
                            temp_mod_uni_vel_sum += mod_value_1;
                        }
                        temp_mod_uni_detune_3 += mod_value_1;
                    }
                    ModulationDestination::All_Gain => {
                        if self.params.mod_source_1.value() == ModulationSource::Velocity {
                            let vel = self.current_note_on_velocity.load(Ordering::SeqCst);
                            temp_mod_gain_1 = vel;
                            temp_mod_gain_2 = vel;
                            temp_mod_gain_3 = vel;
                        } else {
                            temp_mod_lfo_gain_1 = mod_value_1;
                            temp_mod_lfo_gain_2 = mod_value_1;
                            temp_mod_lfo_gain_3 = mod_value_1;
                        }
                    }
                    ModulationDestination::Osc1_Gain => {
                        if self.params.mod_source_1.value() == ModulationSource::Velocity {
                            temp_mod_gain_1 = self.current_note_on_velocity.load(Ordering::SeqCst);
                        } else {
                            temp_mod_lfo_gain_1 = mod_value_1;
                        }
                    }
                    ModulationDestination::Osc2_Gain => {
                        if self.params.mod_source_1.value() == ModulationSource::Velocity {
                            temp_mod_gain_2 = self.current_note_on_velocity.load(Ordering::SeqCst);
                        } else {
                            temp_mod_lfo_gain_2 = mod_value_1;
                        }
                    }
                    ModulationDestination::Osc3_Gain => {
                        if self.params.mod_source_1.value() == ModulationSource::Velocity {
                            temp_mod_gain_3 = self.current_note_on_velocity.load(Ordering::SeqCst);
                        } else {
                            temp_mod_lfo_gain_3 = mod_value_1;
                        }
                    }
                }
            }
            if mod_value_2 != -2.0 {
                match self.params.mod_destination_2.value() {
                    ModulationDestination::None | ModulationDestination::UnsetModulation => {}
                    ModulationDestination::Cutoff_1 => {
                        if self.params.mod_source_2.value() == ModulationSource::Velocity {
                            temp_mod_cutoff_1_source_2 +=
                                20000.0 * self.current_note_on_velocity.load(Ordering::SeqCst);
                        } else {
                            temp_mod_cutoff_1_source_2 += 20000.0 * mod_value_2;
                        }
                    }
                    ModulationDestination::Cutoff_2 => {
                        if self.params.mod_source_2.value() == ModulationSource::Velocity {
                            temp_mod_cutoff_2_source_2 +=
                                20000.0 * self.current_note_on_velocity.load(Ordering::SeqCst);
                        } else {
                            temp_mod_cutoff_2_source_2 += 20000.0 * mod_value_2;
                        }
                    }
                    ModulationDestination::Resonance_1 => {
                        if self.params.mod_source_2.value() == ModulationSource::Velocity {
                            temp_mod_resonance_1_source_2 -=
                                self.current_note_on_velocity.load(Ordering::SeqCst);
                        } else {
                            temp_mod_resonance_1_source_2 -= mod_value_2;
                        }
                    }
                    ModulationDestination::Resonance_2 => {
                        if self.params.mod_source_2.value() == ModulationSource::Velocity {
                            temp_mod_resonance_2_source_2 -=
                                self.current_note_on_velocity.load(Ordering::SeqCst);
                        } else {
                            temp_mod_resonance_2_source_2 -= mod_value_2;
                        }
                    }
                    ModulationDestination::All_Detune => {
                        if self.params.mod_source_2.value() == ModulationSource::Velocity {
                            temp_mod_vel_sum += mod_value_2;
                        }
                        temp_mod_detune_1 += mod_value_2;
                        temp_mod_detune_2 += mod_value_2;
                        temp_mod_detune_3 += mod_value_2;
                    }
                    ModulationDestination::Osc1Detune => {
                        if self.params.mod_source_2.value() == ModulationSource::Velocity {
                            temp_mod_vel_sum += mod_value_2;
                        }
                        temp_mod_detune_1 += mod_value_2;
                    }
                    ModulationDestination::Osc2Detune => {
                        if self.params.mod_source_2.value() == ModulationSource::Velocity {
                            temp_mod_vel_sum += mod_value_2;
                        }
                        temp_mod_detune_2 += mod_value_2;
                    }
                    ModulationDestination::Osc3Detune => {
                        if self.params.mod_source_2.value() == ModulationSource::Velocity {
                            temp_mod_vel_sum += mod_value_2;
                        }
                        temp_mod_detune_3 += mod_value_2;
                    }
                    ModulationDestination::All_UniDetune => {
                        if self.params.mod_source_2.value() == ModulationSource::Velocity {
                            temp_mod_vel_sum += mod_value_2;
                        }
                        temp_mod_uni_detune_1 += mod_value_2;
                        temp_mod_uni_detune_2 += mod_value_2;
                        temp_mod_uni_detune_3 += mod_value_2;
                    }
                    ModulationDestination::Osc1UniDetune => {
                        if self.params.mod_source_2.value() == ModulationSource::Velocity {
                            temp_mod_uni_vel_sum += mod_value_2;
                        }
                        temp_mod_uni_detune_1 += mod_value_2;
                    }
                    ModulationDestination::Osc2UniDetune => {
                        if self.params.mod_source_2.value() == ModulationSource::Velocity {
                            temp_mod_uni_vel_sum += mod_value_2;
                        }
                        temp_mod_uni_detune_2 += mod_value_2;
                    }
                    ModulationDestination::Osc3UniDetune => {
                        if self.params.mod_source_2.value() == ModulationSource::Velocity {
                            temp_mod_uni_vel_sum += mod_value_2;
                        }
                        temp_mod_uni_detune_3 += mod_value_2;
                    }
                    ModulationDestination::All_Gain => {
                        if self.params.mod_source_2.value() == ModulationSource::Velocity {
                            let vel = self.current_note_on_velocity.load(Ordering::SeqCst);
                            temp_mod_gain_1 = vel;
                            temp_mod_gain_2 = vel;
                            temp_mod_gain_3 = vel;
                        } else {
                            temp_mod_lfo_gain_1 = mod_value_2;
                            temp_mod_lfo_gain_2 = mod_value_2;
                            temp_mod_lfo_gain_3 = mod_value_2;
                        }
                    }
                    ModulationDestination::Osc1_Gain => {
                        if self.params.mod_source_2.value() == ModulationSource::Velocity {
                            temp_mod_gain_1 = self.current_note_on_velocity.load(Ordering::SeqCst);
                        } else {
                            temp_mod_lfo_gain_1 = mod_value_2;
                        }
                    }
                    ModulationDestination::Osc2_Gain => {
                        if self.params.mod_source_2.value() == ModulationSource::Velocity {
                            temp_mod_gain_2 = self.current_note_on_velocity.load(Ordering::SeqCst);
                        } else {
                            temp_mod_lfo_gain_2 = mod_value_2;
                        }
                    }
                    ModulationDestination::Osc3_Gain => {
                        if self.params.mod_source_2.value() == ModulationSource::Velocity {
                            temp_mod_gain_3 = self.current_note_on_velocity.load(Ordering::SeqCst);
                        } else {
                            temp_mod_lfo_gain_3 = mod_value_2;
                        }
                    }
                }
            }
            if mod_value_3 != -2.0 {
                match self.params.mod_destination_3.value() {
                    ModulationDestination::None | ModulationDestination::UnsetModulation => {}
                    ModulationDestination::Cutoff_1 => {
                        if self.params.mod_source_3.value() == ModulationSource::Velocity {
                            temp_mod_cutoff_1_source_3 +=
                                20000.0 * self.current_note_on_velocity.load(Ordering::SeqCst);
                        } else {
                            temp_mod_cutoff_1_source_3 += 20000.0 * mod_value_3;
                        }
                    }
                    ModulationDestination::Cutoff_2 => {
                        if self.params.mod_source_3.value() == ModulationSource::Velocity {
                            temp_mod_cutoff_2_source_3 +=
                                20000.0 * self.current_note_on_velocity.load(Ordering::SeqCst);
                        } else {
                            temp_mod_cutoff_2_source_3 += 20000.0 * mod_value_3;
                        }
                    }
                    ModulationDestination::Resonance_1 => {
                        if self.params.mod_source_3.value() == ModulationSource::Velocity {
                            temp_mod_resonance_1_source_3 -=
                                self.current_note_on_velocity.load(Ordering::SeqCst);
                        } else {
                            temp_mod_resonance_1_source_3 -= mod_value_3;
                        }
                    }
                    ModulationDestination::Resonance_2 => {
                        if self.params.mod_source_3.value() == ModulationSource::Velocity {
                            temp_mod_resonance_2_source_3 -=
                                self.current_note_on_velocity.load(Ordering::SeqCst);
                        } else {
                            temp_mod_resonance_2_source_3 -= mod_value_3;
                        }
                    }
                    ModulationDestination::All_Detune => {
                        if self.params.mod_source_3.value() == ModulationSource::Velocity {
                            temp_mod_vel_sum += mod_value_3;
                        }
                        temp_mod_detune_1 += mod_value_3;
                        temp_mod_detune_2 += mod_value_3;
                        temp_mod_detune_3 += mod_value_3;
                    }
                    ModulationDestination::Osc1Detune => {
                        if self.params.mod_source_3.value() == ModulationSource::Velocity {
                            temp_mod_vel_sum += mod_value_3;
                        }
                        temp_mod_detune_1 += mod_value_3;
                    }
                    ModulationDestination::Osc2Detune => {
                        if self.params.mod_source_3.value() == ModulationSource::Velocity {
                            temp_mod_vel_sum += mod_value_3;
                        }
                        temp_mod_detune_2 += mod_value_3;
                    }
                    ModulationDestination::Osc3Detune => {
                        if self.params.mod_source_3.value() == ModulationSource::Velocity {
                            temp_mod_vel_sum += mod_value_3;
                        }
                        temp_mod_detune_3 += mod_value_3;
                    }
                    ModulationDestination::All_UniDetune => {
                        if self.params.mod_source_3.value() == ModulationSource::Velocity {
                            temp_mod_vel_sum += mod_value_3;
                        }
                        temp_mod_uni_detune_1 += mod_value_3;
                        temp_mod_uni_detune_2 += mod_value_3;
                        temp_mod_uni_detune_3 += mod_value_3;
                    }
                    ModulationDestination::Osc1UniDetune => {
                        if self.params.mod_source_3.value() == ModulationSource::Velocity {
                            temp_mod_uni_vel_sum += mod_value_3;
                        }
                        temp_mod_uni_detune_1 += mod_value_3;
                    }
                    ModulationDestination::Osc2UniDetune => {
                        if self.params.mod_source_3.value() == ModulationSource::Velocity {
                            temp_mod_uni_vel_sum += mod_value_3;
                        }
                        temp_mod_uni_detune_2 += mod_value_3;
                    }
                    ModulationDestination::Osc3UniDetune => {
                        if self.params.mod_source_3.value() == ModulationSource::Velocity {
                            temp_mod_uni_vel_sum += mod_value_3;
                        }
                        temp_mod_uni_detune_3 += mod_value_3;
                    }
                    ModulationDestination::All_Gain => {
                        if self.params.mod_source_3.value() == ModulationSource::Velocity {
                            let vel = self.current_note_on_velocity.load(Ordering::SeqCst);
                            temp_mod_gain_1 = vel;
                            temp_mod_gain_2 = vel;
                            temp_mod_gain_3 = vel;
                        } else {
                            temp_mod_lfo_gain_1 = mod_value_3;
                            temp_mod_lfo_gain_2 = mod_value_3;
                            temp_mod_lfo_gain_3 = mod_value_3;
                        }
                    }
                    ModulationDestination::Osc1_Gain => {
                        if self.params.mod_source_3.value() == ModulationSource::Velocity {
                            temp_mod_gain_1 = self.current_note_on_velocity.load(Ordering::SeqCst);
                        } else {
                            temp_mod_lfo_gain_1 = mod_value_3;
                        }
                    }
                    ModulationDestination::Osc2_Gain => {
                        if self.params.mod_source_3.value() == ModulationSource::Velocity {
                            temp_mod_gain_2 = self.current_note_on_velocity.load(Ordering::SeqCst);
                        } else {
                            temp_mod_lfo_gain_2 = mod_value_3;
                        }
                    }
                    ModulationDestination::Osc3_Gain => {
                        if self.params.mod_source_3.value() == ModulationSource::Velocity {
                            temp_mod_gain_3 = self.current_note_on_velocity.load(Ordering::SeqCst);
                        } else {
                            temp_mod_lfo_gain_3 = mod_value_3;
                        }
                    }
                }
            }
            if mod_value_4 != -2.0 {
                match self.params.mod_destination_4.value() {
                    ModulationDestination::None | ModulationDestination::UnsetModulation => {}
                    ModulationDestination::Cutoff_1 => {
                        if self.params.mod_source_4.value() == ModulationSource::Velocity {
                            temp_mod_cutoff_1_source_4 +=
                                20000.0 * self.current_note_on_velocity.load(Ordering::SeqCst);
                        } else {
                            temp_mod_cutoff_1_source_4 += 20000.0 * mod_value_4;
                        }
                    }
                    ModulationDestination::Cutoff_2 => {
                        if self.params.mod_source_4.value() == ModulationSource::Velocity {
                            temp_mod_cutoff_2_source_4 +=
                                20000.0 * self.current_note_on_velocity.load(Ordering::SeqCst);
                        } else {
                            temp_mod_cutoff_2_source_4 += 20000.0 * mod_value_4;
                        }
                    }
                    ModulationDestination::Resonance_1 => {
                        if self.params.mod_source_4.value() == ModulationSource::Velocity {
                            temp_mod_resonance_1_source_4 -=
                                self.current_note_on_velocity.load(Ordering::SeqCst);
                        } else {
                            temp_mod_resonance_1_source_4 -= mod_value_4;
                        }
                    }
                    ModulationDestination::Resonance_2 => {
                        if self.params.mod_source_4.value() == ModulationSource::Velocity {
                            temp_mod_resonance_2_source_4 -=
                                self.current_note_on_velocity.load(Ordering::SeqCst);
                        } else {
                            temp_mod_resonance_2_source_4 -= mod_value_4;
                        }
                    }
                    ModulationDestination::All_Detune => {
                        if self.params.mod_source_4.value() == ModulationSource::Velocity {
                            temp_mod_vel_sum += mod_value_4;
                        }
                        temp_mod_detune_1 += mod_value_4;
                        temp_mod_detune_2 += mod_value_4;
                        temp_mod_detune_3 += mod_value_4;
                    }
                    ModulationDestination::Osc1Detune => {
                        if self.params.mod_source_4.value() == ModulationSource::Velocity {
                            temp_mod_vel_sum += mod_value_4;
                        }
                        temp_mod_detune_1 += mod_value_4;
                    }
                    ModulationDestination::Osc2Detune => {
                        if self.params.mod_source_4.value() == ModulationSource::Velocity {
                            temp_mod_vel_sum += mod_value_4;
                        }
                        temp_mod_detune_2 += mod_value_4;
                    }
                    ModulationDestination::Osc3Detune => {
                        if self.params.mod_source_4.value() == ModulationSource::Velocity {
                            temp_mod_vel_sum += mod_value_4;
                        }
                        temp_mod_detune_3 += mod_value_4;
                    }
                    ModulationDestination::All_UniDetune => {
                        if self.params.mod_source_4.value() == ModulationSource::Velocity {
                            temp_mod_vel_sum += mod_value_4;
                        }
                        temp_mod_uni_detune_1 += mod_value_4;
                        temp_mod_uni_detune_2 += mod_value_4;
                        temp_mod_uni_detune_3 += mod_value_4;
                    }
                    ModulationDestination::Osc1UniDetune => {
                        if self.params.mod_source_4.value() == ModulationSource::Velocity {
                            temp_mod_uni_vel_sum += mod_value_4;
                        }
                        temp_mod_uni_detune_1 += mod_value_4;
                    }
                    ModulationDestination::Osc2UniDetune => {
                        if self.params.mod_source_4.value() == ModulationSource::Velocity {
                            temp_mod_uni_vel_sum += mod_value_4;
                        }
                        temp_mod_uni_detune_2 += mod_value_4;
                    }
                    ModulationDestination::Osc3UniDetune => {
                        if self.params.mod_source_4.value() == ModulationSource::Velocity {
                            temp_mod_uni_vel_sum += mod_value_4;
                        }
                        temp_mod_uni_detune_3 += mod_value_4;
                    }
                    ModulationDestination::All_Gain => {
                        if self.params.mod_source_4.value() == ModulationSource::Velocity {
                            let vel = self.current_note_on_velocity.load(Ordering::SeqCst);
                            temp_mod_gain_1 = vel;
                            temp_mod_gain_2 = vel;
                            temp_mod_gain_3 = vel;
                        } else {
                            temp_mod_lfo_gain_1 = mod_value_4;
                            temp_mod_lfo_gain_2 = mod_value_4;
                            temp_mod_lfo_gain_3 = mod_value_4;
                        }
                    }
                    ModulationDestination::Osc1_Gain => {
                        if self.params.mod_source_4.value() == ModulationSource::Velocity {
                            temp_mod_gain_1 = self.current_note_on_velocity.load(Ordering::SeqCst);
                        } else {
                            temp_mod_lfo_gain_1 = mod_value_4;
                        }
                    }
                    ModulationDestination::Osc2_Gain => {
                        if self.params.mod_source_4.value() == ModulationSource::Velocity {
                            temp_mod_gain_2 = self.current_note_on_velocity.load(Ordering::SeqCst);
                        } else {
                            temp_mod_lfo_gain_2 = mod_value_4;
                        }
                    }
                    ModulationDestination::Osc3_Gain => {
                        if self.params.mod_source_4.value() == ModulationSource::Velocity {
                            temp_mod_gain_3 = self.current_note_on_velocity.load(Ordering::SeqCst);
                        } else {
                            temp_mod_lfo_gain_3 = mod_value_4;
                        }
                    }
                }
            }

            // I think this makes sense to split into structs so each modulation path has its own easily debuggable chain
            modulations_1 = ModulationStruct {
                temp_mod_cutoff_1: temp_mod_cutoff_1_source_1,
                temp_mod_cutoff_2: temp_mod_cutoff_2_source_1,
                temp_mod_resonance_1: temp_mod_resonance_1_source_1,
                temp_mod_resonance_2: temp_mod_resonance_2_source_1,
                temp_mod_detune_1: temp_mod_detune_1,
                temp_mod_detune_2: temp_mod_detune_2,
                temp_mod_detune_3: temp_mod_detune_3,
                temp_mod_uni_detune_1: temp_mod_uni_detune_1,
                temp_mod_uni_detune_2: temp_mod_uni_detune_2,
                temp_mod_uni_detune_3: temp_mod_uni_detune_3,
                temp_mod_vel_sum: temp_mod_vel_sum,
            };
            modulations_2 = ModulationStruct {
                temp_mod_cutoff_1: temp_mod_cutoff_1_source_2,
                temp_mod_cutoff_2: temp_mod_cutoff_2_source_2,
                temp_mod_resonance_1: temp_mod_resonance_1_source_2,
                temp_mod_resonance_2: temp_mod_resonance_2_source_2,
                temp_mod_detune_1: temp_mod_detune_1,
                temp_mod_detune_2: temp_mod_detune_2,
                temp_mod_detune_3: temp_mod_detune_3,
                temp_mod_uni_detune_1: temp_mod_uni_detune_1,
                temp_mod_uni_detune_2: temp_mod_uni_detune_2,
                temp_mod_uni_detune_3: temp_mod_uni_detune_3,
                temp_mod_vel_sum: temp_mod_vel_sum,
            };
            modulations_3 = ModulationStruct {
                temp_mod_cutoff_1: temp_mod_cutoff_1_source_3,
                temp_mod_cutoff_2: temp_mod_cutoff_2_source_3,
                temp_mod_resonance_1: temp_mod_resonance_1_source_3,
                temp_mod_resonance_2: temp_mod_resonance_2_source_3,
                temp_mod_detune_1: temp_mod_detune_1,
                temp_mod_detune_2: temp_mod_detune_2,
                temp_mod_detune_3: temp_mod_detune_3,
                temp_mod_uni_detune_1: temp_mod_uni_detune_1,
                temp_mod_uni_detune_2: temp_mod_uni_detune_2,
                temp_mod_uni_detune_3: temp_mod_uni_detune_3,
                temp_mod_vel_sum: temp_mod_vel_sum,
            };
            modulations_4 = ModulationStruct {
                temp_mod_cutoff_1: temp_mod_cutoff_1_source_4,
                temp_mod_cutoff_2: temp_mod_cutoff_2_source_4,
                temp_mod_resonance_1: temp_mod_resonance_1_source_4,
                temp_mod_resonance_2: temp_mod_resonance_2_source_4,
                temp_mod_detune_1: temp_mod_detune_1,
                temp_mod_detune_2: temp_mod_detune_2,
                temp_mod_detune_3: temp_mod_detune_3,
                temp_mod_uni_detune_1: temp_mod_uni_detune_1,
                temp_mod_uni_detune_2: temp_mod_uni_detune_2,
                temp_mod_uni_detune_3: temp_mod_uni_detune_3,
                temp_mod_vel_sum: temp_mod_vel_sum,
            };

            // Audio Module Processing of Audio kicks off here
            /////////////////////////////////////////////////////////////////////////////////////////////////

            let mut fm_wave_1: f32 = 0.0;
            let mut fm_wave_2: f32 = 0.0;
            // Since File Dialog can be set by any of these we need to check each time
            if !self.file_dialog.load(Ordering::SeqCst)
                && self.params.audio_module_1_type.value() != AudioModuleType::Off
            {
                // We send our sample_id position, params, current midi event, module index, current voice max, and whether any params have changed
                (
                    wave1_l,
                    wave1_r,
                    reset_filter_controller1,
                    note_off_filter_controller1,
                ) = self.audio_module_1.lock().unwrap().process(
                    sample_id,
                    midi_event.clone(),
                    sent_voice_max,
                    modulations_1.temp_mod_detune_1
                        + modulations_2.temp_mod_detune_1
                        + modulations_3.temp_mod_detune_1
                        + modulations_4.temp_mod_detune_1,
                    modulations_1.temp_mod_uni_detune_1
                        + modulations_2.temp_mod_uni_detune_1
                        + modulations_3.temp_mod_uni_detune_1
                        + modulations_4.temp_mod_uni_detune_1,
                    temp_mod_vel_sum,
                    temp_mod_uni_vel_sum,
                    temp_mod_gain_1,
                    temp_mod_lfo_gain_1,
                );
                // Sum to MONO
                fm_wave_1 = (wave1_l + wave1_r)/2.0;
                // I know this isn't a perfect 3rd, but 0.01 is acceptable headroom
                wave1_l *= self.params.audio_module_1_level.value() * 0.33;
                wave1_r *= self.params.audio_module_1_level.value() * 0.33;
            }
            if !self.file_dialog.load(Ordering::SeqCst)
                && self.params.audio_module_2_type.value() != AudioModuleType::Off
            {
                (
                    wave2_l,
                    wave2_r,
                    reset_filter_controller2,
                    note_off_filter_controller2,
                ) = self.audio_module_2.lock().unwrap().process(
                    sample_id,
                    midi_event.clone(),
                    sent_voice_max,
                    modulations_1.temp_mod_detune_2
                        + modulations_2.temp_mod_detune_2
                        + modulations_3.temp_mod_detune_2
                        + modulations_4.temp_mod_detune_2,
                    modulations_1.temp_mod_uni_detune_2
                        + modulations_2.temp_mod_uni_detune_2
                        + modulations_3.temp_mod_uni_detune_2
                        + modulations_4.temp_mod_uni_detune_2,
                    temp_mod_vel_sum,
                    temp_mod_uni_vel_sum,
                    temp_mod_gain_2,
                    temp_mod_lfo_gain_2,
                );
                // Sum to MONO
                fm_wave_2 = (wave2_l + wave2_r)/2.0;
                // I know this isn't a perfect 3rd, but 0.01 is acceptable headroom
                wave2_l *= self.params.audio_module_2_level.value() * 0.33;
                wave2_r *= self.params.audio_module_2_level.value() * 0.33;
            }
            if !self.file_dialog.load(Ordering::SeqCst)
                && self.params.audio_module_3_type.value() != AudioModuleType::Off
            {
                (
                    wave3_l,
                    wave3_r,
                    reset_filter_controller3,
                    note_off_filter_controller3,
                ) = self.audio_module_3.lock().unwrap().process(
                    sample_id,
                    midi_event.clone(),
                    sent_voice_max,
                    modulations_1.temp_mod_detune_3
                        + modulations_2.temp_mod_detune_3
                        + modulations_3.temp_mod_detune_3
                        + modulations_4.temp_mod_detune_3,
                    modulations_1.temp_mod_uni_detune_3
                        + modulations_2.temp_mod_uni_detune_3
                        + modulations_3.temp_mod_uni_detune_3
                        + modulations_4.temp_mod_uni_detune_3,
                    temp_mod_vel_sum,
                    temp_mod_uni_vel_sum,
                    temp_mod_gain_3,
                    temp_mod_lfo_gain_3,
                );
                // I know this isn't a perfect 3rd, but 0.01 is acceptable headroom
                wave3_l *= self.params.audio_module_3_level.value() * 0.33;
                wave3_r *= self.params.audio_module_3_level.value() * 0.33;
            }

            // FM Calculations
            let one_to_two = self.params.fm_one_to_two.value();
            let one_to_three = self.params.fm_one_to_three.value();
            let two_to_three = self.params.fm_two_to_three.value();

            // If a note is ending and we should enter releasing
            if note_off_filter_controller1
                || note_off_filter_controller2
                || note_off_filter_controller3
            {
                self.fm_state = OscState::Releasing;
                self.fm_rel_smoother_1 = match self.params.fm_release_curve.value() {
                    SmoothStyle::Linear => Smoother::new(SmoothingStyle::Linear(
                        self.params.fm_release.value(),
                    )),
                    SmoothStyle::Logarithmic => Smoother::new(SmoothingStyle::Logarithmic(
                        self.params.fm_release.value(),
                    )),
                    SmoothStyle::Exponential => Smoother::new(SmoothingStyle::Exponential(
                        self.params.fm_release.value(),
                    )),
                    SmoothStyle::LogSteep => Smoother::new(SmoothingStyle::LogSteep(
                        self.params.fm_release.value(),
                    )),
                };
                self.fm_rel_smoother_2 = self.fm_rel_smoother_1.clone();
                self.fm_rel_smoother_3 = self.fm_rel_smoother_1.clone();
                // Reset our filter release to be at sustain level to start
                self.fm_rel_smoother_1.reset(
                    self.params.fm_one_to_two.value() * (self.params.fm_sustain.value() / 999.9),
                );
                self.fm_rel_smoother_2.reset(
                    self.params.fm_one_to_three.value() * (self.params.fm_sustain.value() / 999.9),
                );
                self.fm_rel_smoother_3.reset(
                    self.params.fm_two_to_three.value() * (self.params.fm_sustain.value() / 999.9),
                );
                // Move release to the cutoff to end
                self.fm_rel_smoother_1
                    .set_target(self.sample_rate, self.params.fm_one_to_two.value());
                self.fm_rel_smoother_2
                    .set_target(self.sample_rate, self.params.fm_one_to_three.value());
                self.fm_rel_smoother_3
                    .set_target(self.sample_rate, self.params.fm_two_to_three.value());
            }
            // Try to trigger our filter mods on note on! This is sequential/single because we just need a trigger at a point in time
            if reset_filter_controller1 || reset_filter_controller2 || reset_filter_controller3 {
                // Set our filter in attack state
                self.fm_state = OscState::Attacking;
                // Consume our params for smoothing
                self.fm_atk_smoother_1 = match self.params.fm_attack_curve.value() {
                    SmoothStyle::Linear => Smoother::new(SmoothingStyle::Linear(
                        self.params.fm_attack.value(),
                    )),
                    SmoothStyle::Logarithmic => Smoother::new(SmoothingStyle::Logarithmic(
                        self.params.fm_attack.value(),
                    )),
                    SmoothStyle::Exponential => Smoother::new(SmoothingStyle::Exponential(
                        self.params.fm_attack.value(),
                    )),
                    SmoothStyle::LogSteep => Smoother::new(SmoothingStyle::LogSteep(
                        self.params.fm_attack.value(),
                    )),
                };
                self.fm_atk_smoother_2 = self.fm_atk_smoother_1.clone();
                self.fm_atk_smoother_3 = self.fm_atk_smoother_1.clone();
                // Reset our attack to start from 0.0
                if self.params.fm_attack_curve.value() == SmoothStyle::Linear {
                    self.fm_atk_smoother_1.reset(0.0);
                    self.fm_atk_smoother_2.reset(0.0);
                    self.fm_atk_smoother_3.reset(0.0);
                } else {
                    self.fm_atk_smoother_1.reset(0.0001);
                    self.fm_atk_smoother_2.reset(0.0001);
                    self.fm_atk_smoother_3.reset(0.0001);
                }
                // Since we're in attack state at the start of our note we need to setup the attack going to the env peak
                self.fm_atk_smoother_1.set_target(
                    self.sample_rate, self.params.fm_one_to_two.value()
                );
                self.fm_atk_smoother_2.set_target(
                    self.sample_rate, self.params.fm_one_to_three.value()
                );
                self.fm_atk_smoother_3.set_target(
                    self.sample_rate, self.params.fm_two_to_three.value()
                );
            }
            // If our attack has finished
            if self.fm_atk_smoother_1.steps_left() == 0
                && self.fm_state == OscState::Attacking
            {
                self.fm_state = OscState::Decaying;
                self.fm_dec_smoother_1 = match self.params.fm_decay_curve.value() {
                    SmoothStyle::Linear => Smoother::new(SmoothingStyle::Linear(
                        self.params.fm_decay.value()
                    )),
                    SmoothStyle::Logarithmic => Smoother::new(SmoothingStyle::Logarithmic(
                        self.params.fm_decay.value(),
                    )),
                    SmoothStyle::Exponential => Smoother::new(SmoothingStyle::Exponential(
                        self.params.fm_decay.value(),
                    )),
                    SmoothStyle::LogSteep => Smoother::new(SmoothingStyle::LogSteep(
                        self.params.fm_decay.value(),
                    )),
                };
                self.fm_dec_smoother_2 = self.fm_dec_smoother_1.clone();
                self.fm_dec_smoother_3 = self.fm_dec_smoother_1.clone();
                // This makes our fm decay start at env peak point
                self.fm_dec_smoother_1.reset(self.params.fm_one_to_two.value());
                self.fm_dec_smoother_2.reset(self.params.fm_one_to_three.value());
                self.fm_dec_smoother_3.reset(self.params.fm_two_to_three.value());
                // Set up the smoother for our filter movement to go from our decay point to our sustain point
                self.fm_dec_smoother_1.set_target(
                    self.sample_rate,
                    self.params.fm_sustain.value() / 999.9,
                );
                self.fm_dec_smoother_2.set_target(
                    self.sample_rate,
                    self.params.fm_sustain.value() / 999.9,
                );
                self.fm_dec_smoother_3.set_target(
                    self.sample_rate,
                    self.params.fm_sustain.value() / 999.9,
                );
            }
            // If our decay has finished move to sustain state
            if self.fm_dec_smoother_1.steps_left() == 0
                && self.fm_state == OscState::Decaying
            {
                self.fm_state = OscState::Sustaining;
            }
            let next_fm_step_1 = match self.fm_state {
                OscState::Attacking => {
                    self.fm_atk_smoother_1.next()
                },
                OscState::Decaying | OscState::Sustaining => {
                    self.fm_dec_smoother_1.next()
                },
                OscState::Releasing => {
                    self.fm_rel_smoother_1.next()
                },
                OscState::Off => {0.0},
            };
            let next_fm_step_2 = match self.fm_state {
                OscState::Attacking => {
                    self.fm_atk_smoother_2.next()
                },
                OscState::Decaying | OscState::Sustaining => {
                    self.fm_dec_smoother_2.next()
                },
                OscState::Releasing => {
                    self.fm_rel_smoother_2.next()
                },
                OscState::Off => {0.0},
            };
            let next_fm_step_3 = match self.fm_state {
                OscState::Attacking => {
                    self.fm_atk_smoother_3.next()
                },
                OscState::Decaying | OscState::Sustaining => {
                    self.fm_dec_smoother_3.next()
                },
                OscState::Releasing => {
                    self.fm_rel_smoother_3.next()
                },
                OscState::Off => {0.0},
            };
            let current_cycles = self.params.fm_cycles.value();
            if one_to_two > 0.0 {
                match current_cycles {
                    1 => {
                        wave2_l = frequency_modulation::frequency_modulation(fm_wave_1, wave2_l, next_fm_step_1);
                        wave2_r = frequency_modulation::frequency_modulation(fm_wave_1, wave2_r, next_fm_step_1);
                    },
                    2 => {
                        wave2_l = frequency_modulation::double_modulation(fm_wave_1, wave2_l, next_fm_step_1);
                        wave2_r = frequency_modulation::double_modulation(fm_wave_1, wave2_r, next_fm_step_1);
                    },
                    3 => {
                        wave2_l = frequency_modulation::triple_modulation(fm_wave_1, wave2_l, next_fm_step_1);
                        wave2_r = frequency_modulation::triple_modulation(fm_wave_1, wave2_r, next_fm_step_1);
                    },
                    _ => {}
                }
            }
            if one_to_three > 0.0 {
                match current_cycles {
                    1 => {
                        wave3_l = frequency_modulation::frequency_modulation(fm_wave_1, wave3_l, next_fm_step_2);
                        wave3_r = frequency_modulation::frequency_modulation(fm_wave_1, wave3_r, next_fm_step_2);
                    },
                    2 => {
                        wave3_l = frequency_modulation::double_modulation(fm_wave_1, wave3_l, next_fm_step_2);
                        wave3_r = frequency_modulation::double_modulation(fm_wave_1, wave3_r, next_fm_step_2);
                    },
                    3 => {
                        wave3_l = frequency_modulation::triple_modulation(fm_wave_1, wave3_l, next_fm_step_2);
                        wave3_r = frequency_modulation::triple_modulation(fm_wave_1, wave3_r, next_fm_step_2);
                    },
                    _ => {}
                }
            }
            if two_to_three > 0.0 {
                match current_cycles {
                    1 => {
                        wave3_l = frequency_modulation::frequency_modulation(fm_wave_2, wave3_l, next_fm_step_3);
                        wave3_r = frequency_modulation::frequency_modulation(fm_wave_2, wave3_r, next_fm_step_3);
                    },
                    2 => {
                        wave3_l = frequency_modulation::double_modulation(fm_wave_2, wave3_l, next_fm_step_3);
                        wave3_r = frequency_modulation::double_modulation(fm_wave_2, wave3_r, next_fm_step_3);
                    },
                    3 => {
                        wave3_l = frequency_modulation::triple_modulation(fm_wave_2, wave3_l, next_fm_step_3);
                        wave3_r = frequency_modulation::triple_modulation(fm_wave_2, wave3_r, next_fm_step_3);
                    },
                    _ => {}
                }
            }

            /////////////////////////////////////////////////////////////////////////////////////////////////
            // Audio Module Processing over

            // If a new note has happened we should reset the phase of our LFO if sync enabled
            if reset_filter_controller1 || reset_filter_controller2 || reset_filter_controller3 {
                if self.params.lfo1_sync.value() {
                    self.lfo_1.set_phase(self.params.lfo1_phase.value());
                }
                if self.params.lfo2_sync.value() {
                    self.lfo_2.set_phase(self.params.lfo2_phase.value());
                }
                if self.params.lfo3_sync.value() {
                    self.lfo_3.set_phase(self.params.lfo3_phase.value());
                }
            }

            // Get our new LFO values
            if self.params.lfo1_enable.value() {
                lfo_1_current = self.lfo_1.next_sample(self.sample_rate);
            }
            if self.params.lfo2_enable.value() {
                lfo_2_current = self.lfo_2.next_sample(self.sample_rate);
            }
            if self.params.lfo3_enable.value() {
                lfo_3_current = self.lfo_3.next_sample(self.sample_rate);
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
                },
                AMFilterRouting::Both => {
                    left_output_filter1 += wave1_l;
                    right_output_filter1 += wave1_r;
                    left_output_filter2 += wave1_l;
                    right_output_filter2 += wave1_r;
                },
            }
            //#[allow(unused_assignments)]
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
                },
                AMFilterRouting::Both => {
                    left_output_filter1 += wave2_l;
                    right_output_filter1 += wave2_r;
                    left_output_filter2 += wave2_l;
                    right_output_filter2 += wave2_r;
                },
            }
            //#[allow(unused_assignments)]
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
                    left_output_filter2 += wave3_l;
                    right_output_filter2 += wave3_r;
                },
                AMFilterRouting::Both => {
                    left_output_filter1 += wave3_l;
                    right_output_filter1 += wave3_r;
                    left_output_filter2 += wave3_l;
                    right_output_filter2 += wave3_r;
                },
            }

            let mut filter1_processed_l: f32 = 0.0;
            let mut filter1_processed_r: f32 = 0.0;
            let mut filter2_processed_l: f32 = 0.0;
            let mut filter2_processed_r: f32 = 0.0;

            // I ended up doing a passthrough/sum of modulations so they can stack if that's what user desires
            // without breaking things when they are unset
            match self.params.filter_routing.value() {
                FilterRouting::Parallel => {
                    self.filter_process_1(
                        note_off_filter_controller1,
                        note_off_filter_controller2,
                        note_off_filter_controller3,
                        reset_filter_controller1,
                        reset_filter_controller2,
                        reset_filter_controller3,
                        left_output_filter1,
                        right_output_filter1,
                        &mut filter1_processed_l,
                        &mut filter1_processed_r,
                        modulations_1.temp_mod_cutoff_1
                            + modulations_2.temp_mod_cutoff_1
                            + modulations_3.temp_mod_cutoff_1
                            + modulations_4.temp_mod_cutoff_1,
                        //+ vel_cutoff_1,
                        modulations_1.temp_mod_resonance_1
                            + modulations_2.temp_mod_resonance_1
                            + modulations_3.temp_mod_resonance_1
                            + modulations_4.temp_mod_resonance_1,
                        //+ vel_resonance_1,
                    );
                    self.filter_process_2(
                        note_off_filter_controller1,
                        note_off_filter_controller2,
                        note_off_filter_controller3,
                        reset_filter_controller1,
                        reset_filter_controller2,
                        reset_filter_controller3,
                        left_output_filter2,
                        right_output_filter2,
                        &mut filter2_processed_l,
                        &mut filter2_processed_r,
                        modulations_1.temp_mod_cutoff_2
                            + modulations_2.temp_mod_cutoff_2
                            + modulations_3.temp_mod_cutoff_2
                            + modulations_4.temp_mod_cutoff_2,
                        //+ vel_cutoff_2,
                        modulations_1.temp_mod_resonance_2
                            + modulations_2.temp_mod_resonance_2
                            + modulations_3.temp_mod_resonance_2
                            + modulations_4.temp_mod_resonance_2,
                        //+ vel_resonance_2,
                    );
                    left_output += filter1_processed_l + filter2_processed_l;
                    right_output += filter1_processed_r + filter2_processed_r;
                }
                FilterRouting::Series12 => {
                    self.filter_process_1(
                        note_off_filter_controller1,
                        note_off_filter_controller2,
                        note_off_filter_controller3,
                        reset_filter_controller1,
                        reset_filter_controller2,
                        reset_filter_controller3,
                        left_output_filter1,
                        right_output_filter1,
                        &mut filter1_processed_l,
                        &mut filter1_processed_r,
                        modulations_1.temp_mod_cutoff_1
                            + modulations_2.temp_mod_cutoff_1
                            + modulations_3.temp_mod_cutoff_1
                            + modulations_4.temp_mod_cutoff_1,
                        //+ vel_cutoff_1,
                        modulations_1.temp_mod_resonance_1
                            + modulations_2.temp_mod_resonance_1
                            + modulations_3.temp_mod_resonance_1
                            + modulations_4.temp_mod_resonance_1,
                        //+ vel_resonance_1,
                    );
                    self.filter_process_2(
                        note_off_filter_controller1,
                        note_off_filter_controller2,
                        note_off_filter_controller3,
                        reset_filter_controller1,
                        reset_filter_controller2,
                        reset_filter_controller3,
                        filter1_processed_l,
                        filter1_processed_r,
                        &mut filter2_processed_l,
                        &mut filter2_processed_r,
                        modulations_1.temp_mod_cutoff_2
                            + modulations_2.temp_mod_cutoff_2
                            + modulations_3.temp_mod_cutoff_2
                            + modulations_4.temp_mod_cutoff_2,
                        //+ vel_cutoff_2,
                        modulations_1.temp_mod_resonance_2
                            + modulations_2.temp_mod_resonance_2
                            + modulations_3.temp_mod_resonance_2
                            + modulations_4.temp_mod_resonance_2,
                        //+ vel_resonance_2,
                    );
                    left_output += filter2_processed_l;
                    right_output += filter2_processed_r;
                }
                FilterRouting::Series21 => {
                    self.filter_process_2(
                        note_off_filter_controller1,
                        note_off_filter_controller2,
                        note_off_filter_controller3,
                        reset_filter_controller1,
                        reset_filter_controller2,
                        reset_filter_controller3,
                        left_output_filter2,
                        right_output_filter2,
                        &mut filter2_processed_l,
                        &mut filter2_processed_r,
                        modulations_1.temp_mod_cutoff_2
                            + modulations_2.temp_mod_cutoff_2
                            + modulations_3.temp_mod_cutoff_2
                            + modulations_4.temp_mod_cutoff_2,
                        //+ vel_cutoff_2,
                        modulations_1.temp_mod_resonance_2
                            + modulations_2.temp_mod_resonance_2
                            + modulations_3.temp_mod_resonance_2
                            + modulations_4.temp_mod_resonance_2,
                        //+ vel_resonance_2,
                    );
                    self.filter_process_1(
                        note_off_filter_controller1,
                        note_off_filter_controller2,
                        note_off_filter_controller3,
                        reset_filter_controller1,
                        reset_filter_controller2,
                        reset_filter_controller3,
                        filter2_processed_l,
                        filter2_processed_r,
                        &mut filter1_processed_l,
                        &mut filter1_processed_r,
                        modulations_1.temp_mod_cutoff_1
                            + modulations_2.temp_mod_cutoff_1
                            + modulations_3.temp_mod_cutoff_1
                            + modulations_4.temp_mod_cutoff_1,
                        //+ vel_cutoff_1,
                        modulations_1.temp_mod_resonance_1
                            + modulations_2.temp_mod_resonance_1
                            + modulations_3.temp_mod_resonance_1
                            + modulations_4.temp_mod_resonance_1,
                        //+ vel_resonance_1,
                    );
                    left_output += filter1_processed_l;
                    right_output += filter1_processed_r;
                }
            }

            // FX
            ////////////////////////////////////////////////////////////////////////////////////////
            if self.params.use_fx.value() {
                // Equalizer use
                if self.params.pre_use_eq.value() {
                    let eq_ref = self.bands.clone();
                    let mut eq = eq_ref.lock().unwrap();
                    eq[0].set_type(FilterType::LowShelf);
                    eq[1].set_type(FilterType::Peak);
                    eq[2].set_type(FilterType::HighShelf);
                    let q_value: f32 = 0.93;
                    eq[0].update(
                        self.sample_rate,
                        self.params.pre_low_freq.value(),
                        self.params.pre_low_gain.value(),
                        q_value,
                    );
                    eq[1].update(
                        self.sample_rate,
                        self.params.pre_mid_freq.value(),
                        self.params.pre_mid_gain.value(),
                        q_value,
                    );
                    eq[2].update(
                        self.sample_rate,
                        self.params.pre_high_freq.value(),
                        self.params.pre_high_gain.value(),
                        q_value,
                    );

                    let mut temp_l: f32;
                    let mut temp_r: f32;
                    // This is the first time we run a filter at all
                    (temp_l, temp_r) = eq[0].process_sample(left_output, right_output);
                    (temp_l, temp_r) = eq[1].process_sample(temp_l, temp_r);
                    (temp_l, temp_r) = eq[2].process_sample(temp_l, temp_r);
                    // Reassign our new output
                    left_output = temp_l;
                    right_output = temp_r;
                }
                // Compressor
                if self.params.use_compressor.value() {
                    self.compressor.update(
                        self.sample_rate,
                        self.params.comp_amt.value(),
                        self.params.comp_atk.value(),
                        self.params.comp_rel.value(),
                        self.params.comp_drive.value(),
                    );
                    (left_output, right_output) =
                        self.compressor.process(left_output, right_output);
                }
                // ABass Algorithm
                if self.params.use_abass.value() {
                    left_output = a_bass_saturation(left_output, self.params.abass_amount.value());
                    right_output =
                        a_bass_saturation(right_output, self.params.abass_amount.value());
                }
                // Distortion
                if self.params.use_saturation.value() {
                    self.saturator.set_type(self.params.sat_type.value());
                    (left_output, right_output) = self.saturator.process(
                        left_output,
                        right_output,
                        self.params.sat_amt.value(),
                    );
                }
                // Buffer Modulator
                if self.params.use_buffermod.value() {
                    self.buffermod.update(
                        self.sample_rate,
                        self.params.buffermod_depth.value(),
                        self.params.buffermod_rate.value(),
                        self.params.buffermod_spread.value(),
                        self.params.buffermod_timing.value(),
                    );
                    (left_output, right_output) = self.buffermod.process(
                        left_output,
                        right_output,
                        self.params.buffermod_amount.value(),
                    );
                }
                // Phaser
                if self.params.use_phaser.value() {
                    self.phaser.set_sample_rate(self.sample_rate);
                    self.phaser.set_depth(self.params.phaser_depth.value());
                    self.phaser.set_rate(self.params.phaser_rate.value());
                    self.phaser
                        .set_feedback(self.params.phaser_feedback.value());
                    (left_output, right_output) = self.phaser.process(
                        left_output,
                        right_output,
                        self.params.phaser_amount.value(),
                    );
                }
                // Flanger
                if self.params.use_flanger.value() {
                    self.flanger.update(
                        self.sample_rate,
                        self.params.flanger_depth.value(),
                        self.params.flanger_rate.value(),
                        self.params.flanger_feedback.value(),
                    );
                    (left_output, right_output) = self.flanger.process(
                        left_output,
                        right_output,
                        self.params.flanger_amount.value(),
                    );
                }
                // Delay
                if self.params.use_delay.value() {
                    self.delay.set_sample_rate(
                        self.sample_rate,
                        context.transport().tempo.unwrap_or(1.0) as f32,
                    );
                    self.delay.set_length(self.params.delay_time.value());
                    self.delay.set_feedback(self.params.delay_decay.value());
                    self.delay.set_type(self.params.delay_type.value());
                    (left_output, right_output) = self.delay.process(
                        left_output,
                        right_output,
                        self.params.delay_amount.value(),
                    );
                }
                // Reverb
                if self.params.use_reverb.value() {
                    match self.params.reverb_model.value() {
                        // Stacked TDLs to make reverb
                        ReverbModel::Default => {
                            self.reverb[0]
                                .set_size(self.params.reverb_size.value(), self.sample_rate);
                            self.reverb[1]
                                .set_size(self.params.reverb_size.value() * 0.546, self.sample_rate);
                            self.reverb[2]
                                .set_size(self.params.reverb_size.value() * 0.251, self.sample_rate);
                            self.reverb[3]
                                .set_size(self.params.reverb_size.value() * 0.735, self.sample_rate);
                            self.reverb[4]
                                .set_size(self.params.reverb_size.value() * 0.669, self.sample_rate);
                            self.reverb[5]
                                .set_size(self.params.reverb_size.value() * 0.374, self.sample_rate);
                            self.reverb[6]
                                .set_size(self.params.reverb_size.value() * 0.8, self.sample_rate);
                            self.reverb[7]
                                .set_size(self.params.reverb_size.value() * 0.4, self.sample_rate);
                            for verb in self.reverb.iter_mut() {
                                verb.set_feedback(self.params.reverb_feedback.value());
                                (left_output, right_output) = verb.process_tdl(
                                    left_output,
                                    right_output,
                                    self.params.reverb_amount.value());                    
                            }
                        },
                        ReverbModel::Galactic => {
                            // AW Galactic modified
                            self.galactic_reverb.update(
                                self.sample_rate,
                                self.params.reverb_size.value() / 2.0,
                                self.params.reverb_feedback.value(),
                                self.params.reverb_amount.value());
                            (left_output, right_output) = self.galactic_reverb.process(left_output, right_output);
                        },
                        ReverbModel::ASpace => {
                            // AW Galactic simplified and changed
                            self.simple_space[0].update(
                                self.sample_rate,
                                self.params.reverb_size.value() / 2.0,
                                self.params.reverb_feedback.value(),
                                self.params.reverb_amount.value());
                            (left_output, right_output) = self.simple_space[0].process(left_output, right_output);
                            self.simple_space[1].update(
                                self.sample_rate,
                                self.params.reverb_size.value() / 2.5,
                                self.params.reverb_feedback.value() + 0.2,
                                self.params.reverb_amount.value());
                            (left_output, right_output) = self.simple_space[1].process(left_output, right_output);
                            self.simple_space[2].update(
                                self.sample_rate,
                                self.params.reverb_size.value() / 3.0,
                                self.params.reverb_feedback.value() + 0.4,
                                self.params.reverb_amount.value());
                            (left_output, right_output) = self.simple_space[2].process(left_output, right_output);
                            self.simple_space[3].update(
                                self.sample_rate,
                                self.params.reverb_size.value() / 4.0,
                                self.params.reverb_feedback.value() + 0.6,
                                self.params.reverb_amount.value());
                            (left_output, right_output) = self.simple_space[3].process(left_output, right_output);
                        },
                    }
                }
                // Limiter
                if self.params.use_limiter.value() {
                    self.limiter.update(
                        self.params.limiter_knee.value(),
                        self.params.limiter_threshold.value(),
                    );
                    (left_output, right_output) = self.limiter.process(left_output, right_output);
                }
            }

            // DC Offset Removal
            ////////////////////////////////////////////////////////////////////////////////////////
            // There were several filter settings that caused massive DC spikes so I added this here
            if !self.file_dialog.load(Ordering::SeqCst) {
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

    
    fn export_preset(saving_preset: Option<PathBuf>, mut preset: ActuatePresetV126) {
        if let Some(location) = saving_preset {
            // Create our new save file
            let file = File::create(location.clone());

            if let Ok(_file) = file {
                // Clear out our generated notes and only keep the samples themselves
                preset.mod1_sample_lib.clear();
                preset.mod2_sample_lib.clear();
                preset.mod3_sample_lib.clear();

                // Serialize to MessagePack bytes
                let serialized_data = rmp_serde::to_vec::<ActuatePresetV126>(&preset);

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
    }

    // import_preset() uses message packing with serde
    fn import_preset(imported_preset: Option<PathBuf>) -> (String, Option<ActuatePresetV126>) {
        let return_name;

        if let Some(imported_preset) = imported_preset {
            return_name = imported_preset
                .to_str()
                .unwrap_or("Invalid Path")
                .to_string();

            // Read the compressed data from the file
            let mut compressed_data = Vec::new();
            if let Err(err) = std::fs::File::open(&return_name)
                .and_then(|mut file| file.read_to_end(&mut compressed_data))
            {
                eprintln!("Error reading compressed data from file: {}", err);
                return (err.to_string(), Option::None);
            }

            // Decompress the data
            let decompressed_data = Self::decompress_bytes(&compressed_data);
            if let Err(err) = decompressed_data {
                eprintln!("Error decompressing data: {}", err);
                return (err.to_string(), Option::None);
            }

            // Deserialize the MessagePack data
            let file_string_data = decompressed_data.unwrap();

            // Deserialize into preset struct - return default empty lib if error
            let mut unserialized: ActuatePresetV126 = rmp_serde::from_slice(&file_string_data)
                .unwrap_or(ERROR_PRESET.clone());

            if unserialized.preset_name.contains("Error") {
                unserialized = load_unserialized_v125(file_string_data.clone());
                if unserialized.preset_name.contains("Error") {
                    // Attempt to load the previous version preset type
                    unserialized = load_unserialized_v123(file_string_data.clone());

                    if unserialized.preset_name.contains("Error") {
                        // Try loading the previous preset struct version
                        unserialized = load_unserialized_v122(file_string_data.clone());

                        // Attempt to load the previous version preset type
                        if unserialized.preset_name.contains("Error") {
                            // Try loading the previous preset struct version
                            unserialized = load_unserialized_v114(file_string_data.clone());

                            // Attempt to load the oldest preset type
                            if unserialized.preset_name.contains("Error") {
                                // Try loading the previous preset struct version
                                unserialized = load_unserialized_old(file_string_data.clone());
                            }
                        }
                    }
                }
            }

            return (return_name, Some(unserialized));
        }
        return (String::from("Error"), Option::None);
    }

    // Load presets uses message packing with serde
    fn load_preset_bank(loading_bank: Option<PathBuf>) -> (String, Vec<ActuatePresetV126>) {
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

            // Deserialize into preset struct - return default empty lib if error
            let unserialized: Vec<ActuatePresetV126> = rmp_serde::from_slice(&file_string_data)
                .unwrap_or(vec![
                    ERROR_PRESET.clone();
                    PRESET_BANK_SIZE
                ]);

            // Attempt loading 1.2.5 bank if error
            if unserialized[0].preset_name.contains("Error") {
                let unserialized: Vec<ActuatePresetV125> = rmp_serde::from_slice(&file_string_data)
                    .unwrap_or(vec![
                        ERROR_PRESETV125.clone();
                        PRESET_BANK_SIZE
                    ]);
                // Convert each v1.2.3 entry into latest
                let mut converted: Vec<ActuatePresetV126> = Vec::new();
                for v125_preset in unserialized.iter() {
                    converted.push(old_preset_structs::convert_preset_v125(v125_preset.clone()));
                }

                // Attempt loading 1.2.3 bank if error
                if unserialized[0].preset_name.contains("Error") {
                    let unserialized: Vec<ActuatePresetV123> = rmp_serde::from_slice(&file_string_data)
                        .unwrap_or(vec![
                            ERROR_PRESETV123.clone();
                            PRESET_BANK_SIZE
                        ]);
                    // Convert each v1.2.3 entry into latest
                    let mut converted: Vec<ActuatePresetV126> = Vec::new();
                    for v123_preset in unserialized.iter() {
                        converted.push(old_preset_structs::convert_preset_v123(v123_preset.clone()));
                    }
                    return (return_name, converted);
                }
                return (return_name, converted);
            }

            return (return_name, unserialized);
        }
        return (String::from("Error"), Vec::new());
    }

    // This gets triggered to force a load/change and to recalculate sample dependent notes
    fn reload_entire_preset(
        setter: &ParamSetter,
        params: Arc<ActuateParams>,
        current_preset_index: usize,
        arc_preset: &Vec<ActuatePresetV126>,
        AMod1: &mut AudioModule,
        AMod2: &mut AudioModule,
        AMod3: &mut AudioModule,
    ) -> (
        ModulationSource,
        ModulationSource,
        ModulationSource,
        ModulationSource,
        ModulationDestination,
        ModulationDestination,
        ModulationDestination,
        ModulationDestination,
        PresetType,
    ) {
        // Try to load preset into our params if possible
        let loaded_preset = &arc_preset[current_preset_index as usize];

        setter.set_parameter(
            &params.audio_module_1_type,
            loaded_preset.mod1_audio_module_type,
        );
        setter.set_parameter(
            &params.audio_module_1_level,
            loaded_preset.mod1_audio_module_level,
        );
        setter.set_parameter(
            &params.audio_module_1_routing,
            loaded_preset.mod1_audio_module_routing.clone(),
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
            &params.audio_module_2_type,
            loaded_preset.mod2_audio_module_type,
        );
        setter.set_parameter(
            &params.audio_module_2_level,
            loaded_preset.mod2_audio_module_level,
        );
        setter.set_parameter(
            &params.audio_module_2_routing,
            loaded_preset.mod2_audio_module_routing.clone(),
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
            &params.audio_module_3_type,
            loaded_preset.mod3_audio_module_type,
        );
        setter.set_parameter(
            &params.audio_module_3_level,
            loaded_preset.mod3_audio_module_level,
        );
        setter.set_parameter(
            &params.audio_module_3_routing,
            loaded_preset.mod3_audio_module_routing.clone(),
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

        setter.set_parameter(&params.lfo1_enable, loaded_preset.lfo1_enable);
        setter.set_parameter(&params.lfo1_freq, loaded_preset.lfo1_freq);
        setter.set_parameter(&params.lfo1_phase, loaded_preset.lfo1_phase);
        setter.set_parameter(&params.lfo1_retrigger, loaded_preset.lfo1_retrigger);
        setter.set_parameter(&params.lfo1_snap, loaded_preset.lfo1_snap);
        setter.set_parameter(&params.lfo1_sync, loaded_preset.lfo1_sync);
        setter.set_parameter(&params.lfo1_waveform, loaded_preset.lfo1_waveform);
        setter.set_parameter(&params.lfo2_enable, loaded_preset.lfo2_enable);
        setter.set_parameter(&params.lfo2_freq, loaded_preset.lfo2_freq);
        setter.set_parameter(&params.lfo2_phase, loaded_preset.lfo2_phase);
        setter.set_parameter(&params.lfo2_retrigger, loaded_preset.lfo2_retrigger);
        setter.set_parameter(&params.lfo2_snap, loaded_preset.lfo2_snap);
        setter.set_parameter(&params.lfo2_sync, loaded_preset.lfo2_sync);
        setter.set_parameter(&params.lfo2_waveform, loaded_preset.lfo2_waveform);
        setter.set_parameter(&params.lfo3_enable, loaded_preset.lfo3_enable);
        setter.set_parameter(&params.lfo3_freq, loaded_preset.lfo3_freq);
        setter.set_parameter(&params.lfo3_phase, loaded_preset.lfo3_phase);
        setter.set_parameter(&params.lfo3_retrigger, loaded_preset.lfo3_retrigger);
        setter.set_parameter(&params.lfo3_snap, loaded_preset.lfo3_snap);
        setter.set_parameter(&params.lfo3_sync, loaded_preset.lfo3_sync);
        setter.set_parameter(&params.lfo3_waveform, loaded_preset.lfo3_waveform);

        setter.set_parameter(&params.mod_amount_knob_1, loaded_preset.mod_amount_1);
        setter.set_parameter(&params.mod_destination_1, loaded_preset.mod_dest_1.clone());
        setter.set_parameter(&params.mod_source_1, loaded_preset.mod_source_1.clone());
        setter.set_parameter(&params.mod_amount_knob_2, loaded_preset.mod_amount_2);
        setter.set_parameter(&params.mod_destination_2, loaded_preset.mod_dest_2.clone());
        setter.set_parameter(&params.mod_source_2, loaded_preset.mod_source_2.clone());
        setter.set_parameter(&params.mod_amount_knob_3, loaded_preset.mod_amount_3);
        setter.set_parameter(&params.mod_destination_3, loaded_preset.mod_dest_3.clone());
        setter.set_parameter(&params.mod_source_3, loaded_preset.mod_source_3.clone());
        setter.set_parameter(&params.mod_amount_knob_4, loaded_preset.mod_amount_4);
        setter.set_parameter(&params.mod_destination_4, loaded_preset.mod_dest_4.clone());
        setter.set_parameter(&params.mod_source_4, loaded_preset.mod_source_4.clone());
        let mod_source_1_override = loaded_preset.mod_source_1.clone();
        let mod_source_2_override = loaded_preset.mod_source_2.clone();
        let mod_source_3_override = loaded_preset.mod_source_3.clone();
        let mod_source_4_override = loaded_preset.mod_source_4.clone();
        let mod_dest_1_override = loaded_preset.mod_dest_1.clone();
        let mod_dest_2_override = loaded_preset.mod_dest_2.clone();
        let mod_dest_3_override = loaded_preset.mod_dest_3.clone();
        let mod_dest_4_override = loaded_preset.mod_dest_4.clone();

        setter.set_parameter(&params.use_fx, loaded_preset.use_fx);
        setter.set_parameter(&params.pre_use_eq, loaded_preset.pre_use_eq);
        setter.set_parameter(&params.pre_low_freq, loaded_preset.pre_low_freq);
        setter.set_parameter(&params.pre_mid_freq, loaded_preset.pre_mid_freq);
        setter.set_parameter(&params.pre_high_freq, loaded_preset.pre_high_freq);
        setter.set_parameter(&params.pre_low_gain, loaded_preset.pre_low_gain);
        setter.set_parameter(&params.pre_mid_gain, loaded_preset.pre_mid_gain);
        setter.set_parameter(&params.pre_high_gain, loaded_preset.pre_high_gain);
        setter.set_parameter(&params.use_compressor, loaded_preset.use_compressor);
        setter.set_parameter(&params.comp_amt, loaded_preset.comp_amt);
        setter.set_parameter(&params.comp_atk, loaded_preset.comp_atk);
        setter.set_parameter(&params.comp_drive, loaded_preset.comp_drive);
        setter.set_parameter(&params.comp_rel, loaded_preset.comp_rel);
        setter.set_parameter(&params.use_saturation, loaded_preset.use_saturation);
        setter.set_parameter(&params.sat_amt, loaded_preset.sat_amount);
        setter.set_parameter(&params.use_abass, loaded_preset.use_abass);
        setter.set_parameter(&params.abass_amount, loaded_preset.abass_amount);
        setter.set_parameter(&params.sat_type, loaded_preset.sat_type.clone());
        setter.set_parameter(&params.use_delay, loaded_preset.use_delay);
        setter.set_parameter(&params.delay_amount, loaded_preset.delay_amount);
        setter.set_parameter(&params.delay_type, loaded_preset.delay_type.clone());
        setter.set_parameter(&params.delay_decay, loaded_preset.delay_decay);
        setter.set_parameter(&params.delay_time, loaded_preset.delay_time.clone());
        setter.set_parameter(&params.use_reverb, loaded_preset.use_reverb);
        setter.set_parameter(&params.reverb_model, loaded_preset.reverb_model.clone());
        setter.set_parameter(&params.reverb_size, loaded_preset.reverb_size);
        setter.set_parameter(&params.reverb_amount, loaded_preset.reverb_amount);
        setter.set_parameter(&params.reverb_feedback, loaded_preset.reverb_feedback);
        setter.set_parameter(&params.use_phaser, loaded_preset.use_phaser);
        setter.set_parameter(&params.phaser_amount, loaded_preset.phaser_amount);
        setter.set_parameter(&params.phaser_depth, loaded_preset.phaser_depth);
        setter.set_parameter(&params.phaser_feedback, loaded_preset.phaser_feedback);
        setter.set_parameter(&params.phaser_rate, loaded_preset.phaser_rate);
        setter.set_parameter(&params.use_buffermod, loaded_preset.use_buffermod);
        setter.set_parameter(&params.buffermod_amount, loaded_preset.buffermod_amount);
        setter.set_parameter(&params.buffermod_depth, loaded_preset.buffermod_depth);
        setter.set_parameter(&params.buffermod_rate, loaded_preset.buffermod_rate);
        setter.set_parameter(&params.buffermod_spread, loaded_preset.buffermod_spread);
        setter.set_parameter(&params.buffermod_timing, loaded_preset.buffermod_timing);
        setter.set_parameter(&params.use_flanger, loaded_preset.use_flanger);
        setter.set_parameter(&params.flanger_amount, loaded_preset.flanger_amount);
        setter.set_parameter(&params.flanger_depth, loaded_preset.flanger_depth);
        setter.set_parameter(&params.flanger_feedback, loaded_preset.flanger_feedback);
        setter.set_parameter(&params.flanger_rate, loaded_preset.flanger_rate);
        setter.set_parameter(&params.use_limiter, loaded_preset.use_limiter);
        setter.set_parameter(&params.limiter_threshold, loaded_preset.limiter_threshold);
        setter.set_parameter(&params.limiter_knee, loaded_preset.limiter_knee);

        setter.set_parameter(&params.filter_wet, loaded_preset.filter_wet);
        setter.set_parameter(&params.filter_cutoff, loaded_preset.filter_cutoff);
        setter.set_parameter(&params.filter_resonance, loaded_preset.filter_resonance);
        setter.set_parameter(
            &params.filter_res_type,
            loaded_preset.filter_res_type.clone(),
        );
        setter.set_parameter(
            &params.filter_alg_type,
            loaded_preset.filter_alg_type.clone(),
        );
        setter.set_parameter(
            &params.filter_alg_type_2,
            loaded_preset.filter_alg_type_2.clone(),
        );
        setter.set_parameter(
            &params.tilt_filter_type,
            loaded_preset.tilt_filter_type.clone(),
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

        setter.set_parameter(&params.filter_wet_2, loaded_preset.filter_wet_2);
        setter.set_parameter(&params.filter_cutoff_2, loaded_preset.filter_cutoff_2);
        setter.set_parameter(&params.filter_resonance_2, loaded_preset.filter_resonance_2);
        setter.set_parameter(
            &params.filter_res_type_2,
            loaded_preset.filter_res_type_2.clone(),
        );
        setter.set_parameter(
            &params.tilt_filter_type_2,
            loaded_preset.tilt_filter_type_2.clone(),
        );
        setter.set_parameter(&params.filter_lp_amount_2, loaded_preset.filter_lp_amount_2);
        setter.set_parameter(&params.filter_hp_amount_2, loaded_preset.filter_hp_amount_2);
        setter.set_parameter(&params.filter_bp_amount_2, loaded_preset.filter_bp_amount_2);
        setter.set_parameter(&params.filter_env_peak_2, loaded_preset.filter_env_peak_2);
        setter.set_parameter(&params.filter_env_decay_2, loaded_preset.filter_env_decay_2);
        setter.set_parameter(
            &params.filter_env_atk_curve_2,
            loaded_preset.filter_env_atk_curve_2,
        );
        setter.set_parameter(
            &params.filter_env_dec_curve_2,
            loaded_preset.filter_env_dec_curve_2,
        );
        setter.set_parameter(
            &params.filter_env_rel_curve_2,
            loaded_preset.filter_env_rel_curve_2,
        );
        // Somehow I didn't notice these were missing for the longest time
        setter.set_parameter(&params.filter_env_attack, loaded_preset.filter_env_attack);
        setter.set_parameter(&params.filter_env_decay, loaded_preset.filter_env_decay);
        setter.set_parameter(&params.filter_env_sustain, loaded_preset.filter_env_sustain);
        setter.set_parameter(&params.filter_env_release, loaded_preset.filter_env_release);
        setter.set_parameter(
            &params.filter_env_attack_2,
            loaded_preset.filter_env_attack_2,
        );
        setter.set_parameter(&params.filter_env_decay_2, loaded_preset.filter_env_decay_2);
        setter.set_parameter(
            &params.filter_env_sustain_2,
            loaded_preset.filter_env_sustain_2,
        );
        setter.set_parameter(
            &params.filter_env_release_2,
            loaded_preset.filter_env_release_2,
        );
        setter.set_parameter(&params.filter_routing, loaded_preset.filter_routing.clone());

        #[allow(unreachable_patterns)]
        let preset_category_override = match loaded_preset.preset_category {
            PresetType::Bass
            | PresetType::FX
            | PresetType::Lead
            | PresetType::Other
            | PresetType::Pad
            | PresetType::Percussion
            | PresetType::Select
            | PresetType::Synth
            | PresetType::Atmosphere
            | PresetType::Keys
            | PresetType::Pluck => {
                setter.set_parameter(
                    &params.preset_category,
                    loaded_preset.preset_category.clone(),
                );
                loaded_preset.preset_category.clone()
            }
            // This should be unreachable since unserialize will fail before we get here anyways actually
            _ => PresetType::Select,
        };

        // 1.2.1 Pitch update
        setter.set_parameter(&params.pitch_enable, loaded_preset.pitch_enable);
        setter.set_parameter(&params.pitch_env_peak, loaded_preset.pitch_env_peak);
        setter.set_parameter(
            &params.pitch_env_atk_curve,
            loaded_preset.pitch_env_atk_curve,
        );
        setter.set_parameter(
            &params.pitch_env_dec_curve,
            loaded_preset.pitch_env_dec_curve,
        );
        setter.set_parameter(
            &params.pitch_env_rel_curve,
            loaded_preset.pitch_env_rel_curve,
        );
        setter.set_parameter(&params.pitch_env_attack, loaded_preset.pitch_env_attack);
        setter.set_parameter(&params.pitch_env_decay, loaded_preset.pitch_env_decay);
        setter.set_parameter(&params.pitch_env_sustain, loaded_preset.pitch_env_sustain);
        setter.set_parameter(&params.pitch_env_release, loaded_preset.pitch_env_release);
        setter.set_parameter(&params.pitch_routing, loaded_preset.pitch_routing.clone());

        setter.set_parameter(&params.pitch_enable_2, loaded_preset.pitch_enable_2);
        setter.set_parameter(&params.pitch_env_peak_2, loaded_preset.pitch_env_peak_2);
        setter.set_parameter(
            &params.pitch_env_atk_curve_2,
            loaded_preset.pitch_env_atk_curve_2,
        );
        setter.set_parameter(
            &params.pitch_env_dec_curve_2,
            loaded_preset.pitch_env_dec_curve_2,
        );
        setter.set_parameter(
            &params.pitch_env_rel_curve_2,
            loaded_preset.pitch_env_rel_curve_2,
        );
        setter.set_parameter(&params.pitch_env_attack_2, loaded_preset.pitch_env_attack_2);
        setter.set_parameter(&params.pitch_env_decay_2, loaded_preset.pitch_env_decay_2);
        setter.set_parameter(
            &params.pitch_env_sustain_2,
            loaded_preset.pitch_env_sustain_2,
        );
        setter.set_parameter(
            &params.pitch_env_release_2,
            loaded_preset.pitch_env_release_2,
        );
        setter.set_parameter(
            &params.pitch_routing_2,
            loaded_preset.pitch_routing_2.clone(),
        );

        // FM Update 1.2.6
        setter.set_parameter(&params.fm_one_to_two, loaded_preset.fm_one_to_two);
        setter.set_parameter(&params.fm_one_to_three, loaded_preset.fm_one_to_three);
        setter.set_parameter(&params.fm_two_to_three, loaded_preset.fm_two_to_three);
        setter.set_parameter(&params.fm_cycles, loaded_preset.fm_cycles);
        setter.set_parameter(&params.fm_attack, loaded_preset.fm_attack);
        setter.set_parameter(&params.fm_decay, loaded_preset.fm_decay);
        setter.set_parameter(&params.fm_sustain, loaded_preset.fm_sustain);
        setter.set_parameter(&params.fm_release, loaded_preset.fm_release);
        setter.set_parameter(&params.fm_attack_curve, loaded_preset.fm_attack_curve);
        setter.set_parameter(&params.fm_decay_curve, loaded_preset.fm_decay_curve);
        setter.set_parameter(&params.fm_release_curve, loaded_preset.fm_release_curve);

        // Assign the preset tags
        setter.set_parameter(&params.tag_acid, loaded_preset.tag_acid);
        setter.set_parameter(&params.tag_analog, loaded_preset.tag_analog);
        setter.set_parameter(&params.tag_bright, loaded_preset.tag_bright);
        setter.set_parameter(&params.tag_chord, loaded_preset.tag_chord);
        setter.set_parameter(&params.tag_crisp, loaded_preset.tag_crisp);
        setter.set_parameter(&params.tag_deep, loaded_preset.tag_deep);
        setter.set_parameter(&params.tag_delicate, loaded_preset.tag_delicate);
        setter.set_parameter(&params.tag_hard, loaded_preset.tag_hard);
        setter.set_parameter(&params.tag_harsh, loaded_preset.tag_harsh);
        setter.set_parameter(&params.tag_lush, loaded_preset.tag_lush);
        setter.set_parameter(&params.tag_mellow, loaded_preset.tag_mellow);
        setter.set_parameter(&params.tag_resonant, loaded_preset.tag_resonant);
        setter.set_parameter(&params.tag_rich, loaded_preset.tag_rich);
        setter.set_parameter(&params.tag_sharp, loaded_preset.tag_sharp);
        setter.set_parameter(&params.tag_silky, loaded_preset.tag_silky);
        setter.set_parameter(&params.tag_smooth, loaded_preset.tag_smooth);
        setter.set_parameter(&params.tag_soft, loaded_preset.tag_soft);
        setter.set_parameter(&params.tag_stab, loaded_preset.tag_stab);
        setter.set_parameter(&params.tag_warm, loaded_preset.tag_warm);

        setter.set_parameter(&params.filter_cutoff_link, loaded_preset.filter_cutoff_link);

        AMod1.loaded_sample = loaded_preset.mod1_loaded_sample.clone();
        AMod1.sample_lib = loaded_preset.mod1_sample_lib.clone();
        AMod1.restretch = loaded_preset.mod1_restretch;

        AMod2.loaded_sample = loaded_preset.mod2_loaded_sample.clone();
        AMod2.sample_lib = loaded_preset.mod2_sample_lib.clone();
        AMod2.restretch = loaded_preset.mod2_restretch;

        AMod3.loaded_sample = loaded_preset.mod3_loaded_sample.clone();
        AMod3.sample_lib = loaded_preset.mod3_sample_lib.clone();
        AMod3.restretch = loaded_preset.mod3_restretch;

        // Note audio module type from the module is used here instead of from the main self type
        // This is because preset loading has changed it here first!
        AMod1.regenerate_samples();
        AMod2.regenerate_samples();
        AMod3.regenerate_samples();

        // Save persistent sample data
        match AMod1.audio_module_type {
            AudioModuleType::Sampler | AudioModuleType::Granulizer => {
                *params.am1_sample.lock().unwrap() = AMod1.loaded_sample.clone();
            },
            _ => {},
        }
        match AMod2.audio_module_type {
            AudioModuleType::Sampler | AudioModuleType::Granulizer => {
                *params.am2_sample.lock().unwrap() = AMod2.loaded_sample.clone();
            },
            _ => {},
        }
        match AMod3.audio_module_type {
            AudioModuleType::Sampler | AudioModuleType::Granulizer => {
                *params.am3_sample.lock().unwrap() = AMod3.loaded_sample.clone();
            },
            _ => {},
        }

        (
            mod_source_1_override,
            mod_source_2_override,
            mod_source_3_override,
            mod_source_4_override,
            mod_dest_1_override,
            mod_dest_2_override,
            mod_dest_3_override,
            mod_dest_4_override,
            preset_category_override,
        )
    }

    fn save_preset_bank(preset_store: &mut Vec<ActuatePresetV126>, saving_bank: Option<PathBuf>) {
        if let Some(location) = saving_bank {
            // Create our new save file
            let file = File::create(location.clone());

            if let Ok(_file) = file {
                // Clear out our generated notes and only keep the samples themselves
                for preset in preset_store.iter_mut() {
                    preset.mod1_sample_lib.clear();
                    preset.mod2_sample_lib.clear();
                    preset.mod3_sample_lib.clear();
                }

                // Serialize to MessagePack bytes
                let serialized_data =
                    rmp_serde::to_vec::<&Vec<ActuatePresetV126>>(&preset_store.as_ref());

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
            ActuatePresetV126 {
                preset_name: self.preset_name.lock().unwrap().clone(),
                preset_info: self.preset_info.lock().unwrap().clone(),
                preset_category: self.params.preset_category.value(),
                tag_acid: self.params.tag_acid.value(),
                tag_analog: self.params.tag_analog.value(),
                tag_bright: self.params.tag_bright.value(),
                tag_chord: self.params.tag_chord.value(),
                tag_crisp: self.params.tag_crisp.value(),
                tag_deep: self.params.tag_deep.value(),
                tag_delicate: self.params.tag_delicate.value(),
                tag_hard: self.params.tag_hard.value(),
                tag_harsh: self.params.tag_harsh.value(),
                tag_lush: self.params.tag_lush.value(),
                tag_mellow: self.params.tag_mellow.value(),
                tag_resonant: self.params.tag_resonant.value(),
                tag_rich: self.params.tag_rich.value(),
                tag_sharp: self.params.tag_sharp.value(),
                tag_silky: self.params.tag_silky.value(),
                tag_smooth: self.params.tag_smooth.value(),
                tag_soft: self.params.tag_soft.value(),
                tag_stab: self.params.tag_stab.value(),
                tag_warm: self.params.tag_warm.value(),
                // Modules 1
                ///////////////////////////////////////////////////////////
                mod1_audio_module_type: self.params.audio_module_1_type.value(),
                mod1_audio_module_level: self.params.audio_module_1_level.value(),
                mod1_audio_module_routing: self.params.audio_module_1_routing.value(),
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
                mod2_audio_module_type: self.params.audio_module_2_type.value(),
                mod2_audio_module_level: self.params.audio_module_2_level.value(),
                mod2_audio_module_routing: self.params.audio_module_2_routing.value(),
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
                mod3_audio_module_type: self.params.audio_module_3_type.value(),
                mod3_audio_module_level: self.params.audio_module_3_level.value(),
                mod3_audio_module_routing: self.params.audio_module_3_routing.value(),
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
                filter_alg_type: self.params.filter_alg_type.value(),
                tilt_filter_type: self.params.tilt_filter_type.value(),

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
                filter_alg_type_2: self.params.filter_alg_type_2.value(),
                tilt_filter_type_2: self.params.tilt_filter_type_2.value(),

                filter_routing: self.params.filter_routing.value(),
                filter_cutoff_link: self.params.filter_cutoff_link.value(),

                // Pitch
                pitch_enable: self.params.pitch_enable.value(),
                pitch_env_atk_curve: self.params.pitch_env_atk_curve.value(),
                pitch_env_dec_curve: self.params.pitch_env_dec_curve.value(),
                pitch_env_rel_curve: self.params.pitch_env_rel_curve.value(),
                pitch_env_attack: self.params.pitch_env_attack.value(),
                pitch_env_decay: self.params.pitch_env_decay.value(),
                pitch_env_sustain: self.params.pitch_env_sustain.value(),
                pitch_env_release: self.params.pitch_env_release.value(),
                pitch_env_peak: self.params.pitch_env_peak.value(),
                pitch_routing: self.params.pitch_routing.value(),

                pitch_enable_2: self.params.pitch_enable_2.value(),
                pitch_env_atk_curve_2: self.params.pitch_env_atk_curve_2.value(),
                pitch_env_dec_curve_2: self.params.pitch_env_dec_curve_2.value(),
                pitch_env_rel_curve_2: self.params.pitch_env_rel_curve_2.value(),
                pitch_env_attack_2: self.params.pitch_env_attack_2.value(),
                pitch_env_decay_2: self.params.pitch_env_decay_2.value(),
                pitch_env_sustain_2: self.params.pitch_env_sustain_2.value(),
                pitch_env_release_2: self.params.pitch_env_release_2.value(),
                pitch_env_peak_2: self.params.pitch_env_peak_2.value(),
                pitch_routing_2: self.params.pitch_routing_2.value(),

                // LFOs
                lfo1_enable: self.params.lfo1_enable.value(),
                lfo2_enable: self.params.lfo2_enable.value(),
                lfo3_enable: self.params.lfo3_enable.value(),

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

                mod_source_1: self.params.mod_source_1.value(),
                mod_source_2: self.params.mod_source_2.value(),
                mod_source_3: self.params.mod_source_3.value(),
                mod_source_4: self.params.mod_source_4.value(),
                mod_dest_1: self.params.mod_destination_1.value(),
                mod_dest_2: self.params.mod_destination_2.value(),
                mod_dest_3: self.params.mod_destination_3.value(),
                mod_dest_4: self.params.mod_destination_4.value(),
                mod_amount_1: self.params.mod_amount_knob_1.value(),
                mod_amount_2: self.params.mod_amount_knob_2.value(),
                mod_amount_3: self.params.mod_amount_knob_3.value(),
                mod_amount_4: self.params.mod_amount_knob_4.value(),

                fm_one_to_two: self.params.fm_one_to_two.value(),
                fm_one_to_three: self.params.fm_one_to_three.value(),
                fm_two_to_three: self.params.fm_two_to_three.value(),
                fm_cycles: self.params.fm_cycles.value(),
                fm_attack: self.params.fm_attack.value(),
                fm_decay: self.params.fm_decay.value(),
                fm_sustain: self.params.fm_sustain.value(),
                fm_release: self.params.fm_release.value(),
                fm_attack_curve: self.params.fm_attack_curve.value(),
                fm_decay_curve: self.params.fm_decay_curve.value(),
                fm_release_curve: self.params.fm_release_curve.value(),

                pre_use_eq: self.params.pre_use_eq.value(),
                pre_low_freq: self.params.pre_low_freq.value(),
                pre_mid_freq: self.params.pre_mid_freq.value(),
                pre_high_freq: self.params.pre_high_freq.value(),
                pre_low_gain: self.params.pre_low_gain.value(),
                pre_mid_gain: self.params.pre_mid_gain.value(),
                pre_high_gain: self.params.pre_high_gain.value(),

                use_fx: self.params.use_fx.value(),
                use_compressor: self.params.use_compressor.value(),
                comp_amt: self.params.comp_amt.value(),
                comp_atk: self.params.comp_atk.value(),
                comp_rel: self.params.comp_rel.value(),
                comp_drive: self.params.comp_drive.value(),
                use_abass: self.params.use_abass.value(),
                abass_amount: self.params.abass_amount.value(),
                use_saturation: self.params.use_saturation.value(),
                sat_amount: self.params.sat_amt.value(),
                sat_type: self.params.sat_type.value(),
                use_delay: self.params.use_delay.value(),
                delay_amount: self.params.delay_amount.value(),
                delay_time: self.params.delay_time.value(),
                delay_decay: self.params.delay_decay.value(),
                delay_type: self.params.delay_type.value(),
                use_reverb: self.params.use_reverb.value(),
                reverb_model: self.params.reverb_model.value(),
                reverb_amount: self.params.reverb_amount.value(),
                reverb_size: self.params.reverb_size.value(),
                reverb_feedback: self.params.reverb_feedback.value(),
                use_phaser: self.params.use_phaser.value(),
                phaser_amount: self.params.phaser_amount.value(),
                phaser_depth: self.params.phaser_depth.value(),
                phaser_rate: self.params.phaser_rate.value(),
                phaser_feedback: self.params.phaser_feedback.value(),
                use_buffermod: self.params.use_buffermod.value(),
                buffermod_amount: self.params.buffermod_amount.value(),
                buffermod_depth: self.params.buffermod_depth.value(),
                buffermod_rate: self.params.buffermod_rate.value(),
                buffermod_spread: self.params.buffermod_spread.value(),
                buffermod_timing: self.params.buffermod_timing.value(),
                use_flanger: self.params.use_flanger.value(),
                flanger_amount: self.params.flanger_amount.value(),
                flanger_depth: self.params.flanger_depth.value(),
                flanger_rate: self.params.flanger_rate.value(),
                flanger_feedback: self.params.flanger_feedback.value(),
                use_limiter: self.params.use_limiter.value(),
                limiter_threshold: self.params.limiter_threshold.value(),
                limiter_knee: self.params.limiter_knee.value(),
            };
    }

    fn filter_process_1(
        &mut self,
        note_off_filter_controller1: bool,
        note_off_filter_controller2: bool,
        note_off_filter_controller3: bool,
        reset_filter_controller1: bool,
        reset_filter_controller2: bool,
        reset_filter_controller3: bool,
        left_input_filter1: f32,
        right_input_filter1: f32,
        left_output: &mut f32,
        right_output: &mut f32,
        filter_cutoff_mod: f32,
        filter_resonance_mod: f32,
    ) {
        // Filter 1 Processing
        ///////////////////////////////////////////////////////////////
        if self.params.filter_wet.value() > 0.0 && !self.file_dialog.load(Ordering::SeqCst) {
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
                    SmoothStyle::LogSteep => Smoother::new(SmoothingStyle::LogSteep(
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
            if reset_filter_controller1 || reset_filter_controller2 || reset_filter_controller3 {
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
                    SmoothStyle::LogSteep => Smoother::new(SmoothingStyle::LogSteep(
                        self.params.filter_env_attack.value(),
                    )),
                };
                // Reset our attack to start from the filter cutoff
                self.filter_atk_smoother_1
                    .reset(self.params.filter_cutoff.value());
                // Since we're in attack state at the start of our note we need to setup the attack going to the env peak
                self.filter_atk_smoother_1.set_target(
                    self.sample_rate,
                    (self.params.filter_cutoff.value()
                        + (
                            // This scales the peak env to be much gentler for the TILT filter
                            match self.params.filter_alg_type.value() {
                                FilterAlgorithms::SVF => self.params.filter_env_peak.value(),
                                FilterAlgorithms::TILT => adv_scale_value(
                                    self.params.filter_env_peak.value(),
                                    -19980.0,
                                    19980.0,
                                    -5000.0,
                                    5000.0,
                                ),
                                FilterAlgorithms::VCF => self.params.filter_env_peak.value(),
                            }
                        ))
                    .clamp(20.0, 20000.0),
                );
            }
            // If our attack has finished
            if self.filter_atk_smoother_1.steps_left() == 0
                && self.filter_state_1 == OscState::Attacking
            {
                self.filter_state_1 = OscState::Decaying;
                self.filter_dec_smoother_1 = match self.params.filter_env_dec_curve.value() {
                    SmoothStyle::Linear => {
                        Smoother::new(SmoothingStyle::Linear(self.params.filter_env_decay.value()))
                    }
                    SmoothStyle::Logarithmic => Smoother::new(SmoothingStyle::Logarithmic(
                        self.params.filter_env_decay.value(),
                    )),
                    SmoothStyle::Exponential => Smoother::new(SmoothingStyle::Exponential(
                        self.params.filter_env_decay.value(),
                    )),
                    SmoothStyle::LogSteep => Smoother::new(SmoothingStyle::LogSteep(
                        self.params.filter_env_decay.value(),
                    )),
                };
                // This makes our filter decay start at env peak point
                self.filter_dec_smoother_1.reset(
                    (self.params.filter_cutoff.value()
                        + (
                            // This scales the peak env to be much gentler for the TILT filter
                            match self.params.filter_alg_type.value() {
                                FilterAlgorithms::SVF => self.params.filter_env_peak.value(),
                                FilterAlgorithms::TILT => adv_scale_value(
                                    self.params.filter_env_peak.value(),
                                    -19980.0,
                                    19980.0,
                                    -5000.0,
                                    5000.0,
                                ),
                                FilterAlgorithms::VCF => self.params.filter_env_peak.value(),
                            }
                        ))
                    .clamp(20.0, 20000.0),
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
                OscState::Attacking => {
                    (self.filter_atk_smoother_1.next() + filter_cutoff_mod).clamp(20.0, 20000.0)
                }
                OscState::Decaying | OscState::Sustaining => {
                    (self.filter_dec_smoother_1.next() + filter_cutoff_mod).clamp(20.0, 20000.0)
                }
                OscState::Releasing => {
                    (self.filter_rel_smoother_1.next() + filter_cutoff_mod).clamp(20.0, 20000.0)
                }
                // I don't expect this to be used
                _ => (self.params.filter_cutoff.value() + filter_cutoff_mod).clamp(20.0, 20000.0),
            };
            match self.params.filter_alg_type.value() {
                FilterAlgorithms::SVF => {
                    // Filtering before output
                    self.filter_l_1.update(
                        next_filter_step,
                        self.params.filter_resonance.value() - filter_resonance_mod,
                        self.sample_rate,
                        self.params.filter_res_type.value(),
                    );
                    self.filter_r_1.update(
                        next_filter_step,
                        self.params.filter_resonance.value() - filter_resonance_mod,
                        self.sample_rate,
                        self.params.filter_res_type.value(),
                    );
                    let low_l: f32;
                    let band_l: f32;
                    let high_l: f32;
                    let low_r: f32;
                    let band_r: f32;
                    let high_r: f32;
                    (low_l, band_l, high_l) = self.filter_l_1.process(left_input_filter1);
                    (low_r, band_r, high_r) = self.filter_r_1.process(right_input_filter1);
                    *left_output += (low_l * self.params.filter_lp_amount.value()
                        + band_l * self.params.filter_bp_amount.value()
                        + high_l * self.params.filter_hp_amount.value())
                        * self.params.filter_wet.value()
                        + left_input_filter1 * (1.0 - self.params.filter_wet.value());
                    *right_output += (low_r * self.params.filter_lp_amount.value()
                        + band_r * self.params.filter_bp_amount.value()
                        + high_r * self.params.filter_hp_amount.value())
                        * self.params.filter_wet.value()
                        + right_input_filter1 * (1.0 - self.params.filter_wet.value());
                }
                FilterAlgorithms::TILT => {
                    self.tilt_filter_l_1.update(
                        self.sample_rate,
                        next_filter_step,
                        self.params.filter_resonance.value() - filter_resonance_mod,
                        self.params.tilt_filter_type.value(),
                    );
                    self.tilt_filter_r_1.update(
                        self.sample_rate,
                        next_filter_step,
                        self.params.filter_resonance.value() - filter_resonance_mod,
                        self.params.tilt_filter_type.value(),
                    );
                    let tilt_out_l = self.tilt_filter_l_1.process(left_input_filter1);
                    let tilt_out_r = self.tilt_filter_r_1.process(right_input_filter1);
                    *left_output += tilt_out_l * self.params.filter_wet.value()
                        + left_input_filter1 * (1.0 - self.params.filter_wet.value());
                    *right_output += tilt_out_r * self.params.filter_wet.value()
                        + right_input_filter1 * (1.0 - self.params.filter_wet.value());
                }
                FilterAlgorithms::VCF => {
                    self.vcf_filter_l_1.update(
                        next_filter_step,
                        self.params.filter_resonance.value() - filter_resonance_mod,
                        self.params.vcf_filter_type.value(),
                        self.sample_rate,
                    );
                    self.vcf_filter_r_1.update(
                        next_filter_step,
                        self.params.filter_resonance.value() - filter_resonance_mod,
                        self.params.vcf_filter_type.value(),
                        self.sample_rate,
                    );
                    let vcf_out_l = self.vcf_filter_l_1.process(left_input_filter1);
                    let vcf_out_r = self.vcf_filter_r_1.process(right_input_filter1);
                    *left_output += vcf_out_l * self.params.filter_wet.value()
                        + left_input_filter1 * (1.0 - self.params.filter_wet.value());
                    *right_output += vcf_out_r * self.params.filter_wet.value()
                        + right_input_filter1 * (1.0 - self.params.filter_wet.value());
                }
            }
        }
    }

    fn filter_process_2(
        &mut self,
        note_off_filter_controller1: bool,
        note_off_filter_controller2: bool,
        note_off_filter_controller3: bool,
        reset_filter_controller1: bool,
        reset_filter_controller2: bool,
        reset_filter_controller3: bool,
        left_input_filter2: f32,
        right_input_filter2: f32,
        left_output: &mut f32,
        right_output: &mut f32,
        filter_cutoff_mod: f32,
        filter_resonance_mod: f32,
    ) {
        // Filter 2 Processing
        ///////////////////////////////////////////////////////////////
        if self.params.filter_wet_2.value() > 0.0 && !self.file_dialog.load(Ordering::SeqCst) {
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
                    SmoothStyle::LogSteep => Smoother::new(SmoothingStyle::LogSteep(
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
            if reset_filter_controller1 || reset_filter_controller2 || reset_filter_controller3 {
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
                    SmoothStyle::LogSteep => Smoother::new(SmoothingStyle::LogSteep(
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
                        + (
                            // This scales the peak env to be much gentler for the TILT filter
                            match self.params.filter_alg_type_2.value() {
                                FilterAlgorithms::SVF => self.params.filter_env_peak_2.value(),
                                FilterAlgorithms::TILT => adv_scale_value(
                                    self.params.filter_env_peak_2.value(),
                                    -19980.0,
                                    19980.0,
                                    -5000.0,
                                    5000.0,
                                ),
                                FilterAlgorithms::VCF => self.params.filter_env_peak_2.value(),
                            }
                        ))
                    .clamp(20.0, 20000.0),
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
                    SmoothStyle::LogSteep => Smoother::new(SmoothingStyle::LogSteep(
                        self.params.filter_env_decay_2.value(),
                    )),
                };
                // This makes our filter decay start at env peak point
                self.filter_dec_smoother_2.reset(
                    (self.params.filter_cutoff_2.value()
                        + (
                            // This scales the peak env to be much gentler for the TILT filter
                            match self.params.filter_alg_type_2.value() {
                                FilterAlgorithms::SVF => self.params.filter_env_peak_2.value(),
                                FilterAlgorithms::TILT => adv_scale_value(
                                    self.params.filter_env_peak_2.value(),
                                    -19980.0,
                                    19980.0,
                                    -5000.0,
                                    5000.0,
                                ),
                                FilterAlgorithms::VCF => self.params.filter_env_peak_2.value(),
                            }
                        ))
                    .clamp(20.0, 20000.0),
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
                OscState::Attacking => {
                    (self.filter_atk_smoother_2.next() + filter_cutoff_mod).clamp(20.0, 20000.0)
                }
                OscState::Decaying | OscState::Sustaining => {
                    (self.filter_dec_smoother_2.next() + filter_cutoff_mod).clamp(20.0, 20000.0)
                }
                OscState::Releasing => {
                    (self.filter_rel_smoother_2.next() + filter_cutoff_mod).clamp(20.0, 20000.0)
                }
                // I don't expect this to be used
                _ => self.params.filter_cutoff_2.value() + filter_cutoff_mod,
            };
            match self.params.filter_alg_type.value() {
                FilterAlgorithms::SVF => {
                    // Filtering before output
                    self.filter_l_2.update(
                        next_filter_step,
                        self.params.filter_resonance_2.value(),
                        self.sample_rate,
                        self.params.filter_res_type_2.value(),
                    );
                    self.filter_r_2.update(
                        next_filter_step,
                        self.params.filter_resonance_2.value() + filter_resonance_mod,
                        self.sample_rate,
                        self.params.filter_res_type_2.value(),
                    );
                    let low_l: f32;
                    let band_l: f32;
                    let high_l: f32;
                    let low_r: f32;
                    let band_r: f32;
                    let high_r: f32;
                    (low_l, band_l, high_l) = self.filter_l_2.process(left_input_filter2);
                    (low_r, band_r, high_r) = self.filter_r_2.process(right_input_filter2);
                    *left_output += (low_l * self.params.filter_lp_amount_2.value()
                        + band_l * self.params.filter_bp_amount_2.value()
                        + high_l * self.params.filter_hp_amount_2.value())
                        * self.params.filter_wet_2.value()
                        + *left_output * (1.0 - self.params.filter_wet_2.value());
                    *right_output += (low_r * self.params.filter_lp_amount_2.value()
                        + band_r * self.params.filter_bp_amount_2.value()
                        + high_r * self.params.filter_hp_amount_2.value())
                        * self.params.filter_wet_2.value()
                        + *right_output * (1.0 - self.params.filter_wet_2.value());
                }
                FilterAlgorithms::TILT => {
                    self.tilt_filter_l_2.update(
                        self.sample_rate,
                        next_filter_step,
                        self.params.filter_resonance_2.value(),
                        self.params.tilt_filter_type_2.value(),
                    );
                    self.tilt_filter_r_2.update(
                        self.sample_rate,
                        next_filter_step,
                        self.params.filter_resonance_2.value(),
                        self.params.tilt_filter_type_2.value(),
                    );
                    let tilt_out_l = self.tilt_filter_l_2.process(left_input_filter2);
                    let tilt_out_r = self.tilt_filter_r_2.process(right_input_filter2);
                    *left_output += tilt_out_l * self.params.filter_wet_2.value()
                        + left_input_filter2 * (1.0 - self.params.filter_wet_2.value());
                    *right_output += tilt_out_r * self.params.filter_wet_2.value()
                        + right_input_filter2 * (1.0 - self.params.filter_wet_2.value());
                }
                FilterAlgorithms::VCF => {
                    self.vcf_filter_l_2.update(
                        next_filter_step,
                        self.params.filter_resonance_2.value(),
                        self.params.vcf_filter_type_2.value(),
                        self.sample_rate,
                    );
                    self.vcf_filter_r_2.update(
                        next_filter_step,
                        self.params.filter_resonance_2.value(),
                        self.params.vcf_filter_type_2.value(),
                        self.sample_rate,
                    );
                    let vcf_out_l = self.vcf_filter_l_2.process(left_input_filter2);
                    let vcf_out_r = self.vcf_filter_r_2.process(right_input_filter2);
                    *left_output += vcf_out_l * self.params.filter_wet_2.value()
                        + left_input_filter2 * (1.0 - self.params.filter_wet_2.value());
                    *right_output += vcf_out_r * self.params.filter_wet_2.value()
                        + right_input_filter2 * (1.0 - self.params.filter_wet_2.value());
                }
            }
        }
    }
}

impl ClapPlugin for Actuate {
    const CLAP_ID: &'static str = "com.ardura.actuate";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("Sampler + Synth");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::Instrument,
        ClapFeature::Sampler,
    ];
}

impl Vst3Plugin for Actuate {
    const VST3_CLASS_ID: [u8; 16] = *b"ActuateArduraAAA";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[
        Vst3SubCategory::Instrument, 
        Vst3SubCategory::Sampler
    ];
}

nih_export_clap!(Actuate);
nih_export_vst3!(Actuate);

// I use this when I want to remove label and unit from a param in gui
pub fn format_nothing() -> Arc<dyn Fn(f32) -> String + Send + Sync> {
    Arc::new(move |_| String::new())
}

fn adv_scale_value(input: f32, in_min: f32, in_max: f32, out_min: f32, out_max: f32) -> f32 {
    // Ensure that the input value is within the specified input range
    let input = input.max(in_min).min(in_max);

    // Calculate the scaled value using linear mapping
    let scaled_value = (input - in_min) * (out_max - out_min) / (in_max - in_min) + out_min;

    scaled_value
}


lazy_static::lazy_static!(
    static ref ERROR_PRESETV123: ActuatePresetV123 = ActuatePresetV123 {
        preset_name: String::from("Error Loading"),
        preset_info: String::from("Corrupt or incompatible versions"),
        preset_category: PresetType::Select,
        tag_acid: false,
        tag_analog: false,
        tag_bright: false,
        tag_chord: false,
        tag_crisp: false,
        tag_deep: false,
        tag_delicate: false,
        tag_hard: false,
        tag_harsh: false,
        tag_lush: false,
        tag_mellow: false,
        tag_resonant: false,
        tag_rich: false,
        tag_sharp: false,
        tag_silky: false,
        tag_smooth: false,
        tag_soft: false,
        tag_stab: false,
        tag_warm: false,
        mod1_audio_module_type: AudioModuleType::Osc,
        mod1_audio_module_level: 1.0,
        mod1_audio_module_routing: AMFilterRouting::Filter1,
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
        mod2_audio_module_routing: AMFilterRouting::Filter1,
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
        mod3_audio_module_routing: AMFilterRouting::Filter1,
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
        filter_cutoff: 20000.0,
        filter_resonance: 1.0,
        filter_res_type: ResonanceType::Default,
        filter_lp_amount: 1.0,
        filter_hp_amount: 0.0,
        filter_bp_amount: 0.0,
        filter_env_peak: 0.0,
        filter_env_attack: 0.0,
        filter_env_decay: 0.0001,
        filter_env_sustain: 999.9,
        filter_env_release: 5.0,
        filter_env_atk_curve: SmoothStyle::Linear,
        filter_env_dec_curve: SmoothStyle::Linear,
        filter_env_rel_curve: SmoothStyle::Linear,
        filter_alg_type: FilterAlgorithms::SVF,
        tilt_filter_type: ArduraFilter::ResponseType::Lowpass,

        filter_wet_2: 1.0,
        filter_cutoff_2: 20000.0,
        filter_resonance_2: 1.0,
        filter_res_type_2: ResonanceType::Default,
        filter_lp_amount_2: 1.0,
        filter_hp_amount_2: 0.0,
        filter_bp_amount_2: 0.0,
        filter_env_peak_2: 0.0,
        filter_env_attack_2: 0.0,
        filter_env_decay_2: 0.0001,
        filter_env_sustain_2: 999.9,
        filter_env_release_2: 5.0,
        filter_env_atk_curve_2: SmoothStyle::Linear,
        filter_env_dec_curve_2: SmoothStyle::Linear,
        filter_env_rel_curve_2: SmoothStyle::Linear,
        filter_alg_type_2: FilterAlgorithms::SVF,
        tilt_filter_type_2: ArduraFilter::ResponseType::Lowpass,

        filter_routing: FilterRouting::Parallel,
        filter_cutoff_link: false,

        pitch_enable: false,
        pitch_env_atk_curve: SmoothStyle::Linear,
        pitch_env_dec_curve: SmoothStyle::Linear,
        pitch_env_rel_curve: SmoothStyle::Linear,
        pitch_env_attack: 0.0,
        pitch_env_decay: 300.0,
        pitch_env_sustain: 0.0,
        pitch_env_release: 0.0,
        pitch_env_peak: 0.0,
        pitch_routing: PitchRouting::Osc1,

        pitch_enable_2: false,
        pitch_env_peak_2: 0.0,
        pitch_env_atk_curve_2: SmoothStyle::Linear,
        pitch_env_dec_curve_2: SmoothStyle::Linear,
        pitch_env_rel_curve_2: SmoothStyle::Linear,
        pitch_env_attack_2: 0.0,
        pitch_env_decay_2: 300.0,
        pitch_env_release_2: 0.0,
        pitch_env_sustain_2: 0.0,
        pitch_routing_2: PitchRouting::Osc1,

        // LFOs
        lfo1_enable: false,
        lfo2_enable: false,
        lfo3_enable: false,

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

        // Modulations
        mod_source_1: ModulationSource::None,
        mod_source_2: ModulationSource::None,
        mod_source_3: ModulationSource::None,
        mod_source_4: ModulationSource::None,
        mod_dest_1: ModulationDestination::None,
        mod_dest_2: ModulationDestination::None,
        mod_dest_3: ModulationDestination::None,
        mod_dest_4: ModulationDestination::None,
        mod_amount_1: 0.0,
        mod_amount_2: 0.0,
        mod_amount_3: 0.0,
        mod_amount_4: 0.0,

        // EQ
        pre_use_eq: false,
        pre_low_freq: 800.0,
        pre_mid_freq: 3000.0,
        pre_high_freq: 10000.0,
        pre_low_gain: 0.0,
        pre_mid_gain: 0.0,
        pre_high_gain: 0.0,

        // FX
        use_fx: true,

        use_compressor: false,
        comp_amt: 0.5,
        comp_atk: 0.5,
        comp_rel: 0.5,
        comp_drive: 0.5,

        use_abass: false,
        abass_amount: 0.0011,

        use_saturation: false,
        sat_amount: 0.0,
        sat_type: SaturationType::Tape,

        use_delay: false,
        delay_amount: 0.5,
        delay_time: DelaySnapValues::Quarter,
        delay_decay: 0.5,
        delay_type: DelayType::Stereo,

        use_reverb: false,
        reverb_amount: 0.85,
        reverb_size: 1.0,
        reverb_feedback: 0.28,

        use_phaser: false,
        phaser_amount: 0.5,
        phaser_depth: 0.5,
        phaser_rate: 0.5,
        phaser_feedback: 0.5,

        use_buffermod: false,
        buffermod_amount: 0.5,
        buffermod_depth: 0.5,
        buffermod_rate: 0.5,
        buffermod_spread: 0.0,
        buffermod_timing: 620.0,

        use_flanger: false,
        flanger_amount: 0.5,
        flanger_depth: 0.5,
        flanger_rate: 0.5,
        flanger_feedback: 0.5,

        use_limiter: false,
        limiter_threshold: 0.5,
        limiter_knee: 0.5,
    };

    static ref ERROR_PRESETV125: ActuatePresetV125 = ActuatePresetV125 {
        preset_name: String::from("Error Loading"),
        preset_info: String::from("Corrupt or incompatible versions"),
        preset_category: PresetType::Select,
        tag_acid: false,
        tag_analog: false,
        tag_bright: false,
        tag_chord: false,
        tag_crisp: false,
        tag_deep: false,
        tag_delicate: false,
        tag_hard: false,
        tag_harsh: false,
        tag_lush: false,
        tag_mellow: false,
        tag_resonant: false,
        tag_rich: false,
        tag_sharp: false,
        tag_silky: false,
        tag_smooth: false,
        tag_soft: false,
        tag_stab: false,
        tag_warm: false,
        mod1_audio_module_type: AudioModuleType::Osc,
        mod1_audio_module_level: 1.0,
        mod1_audio_module_routing: AMFilterRouting::Filter1,
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
        mod2_audio_module_routing: AMFilterRouting::Filter1,
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
        mod3_audio_module_routing: AMFilterRouting::Filter1,
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
        filter_cutoff: 20000.0,
        filter_resonance: 1.0,
        filter_res_type: ResonanceType::Default,
        filter_lp_amount: 1.0,
        filter_hp_amount: 0.0,
        filter_bp_amount: 0.0,
        filter_env_peak: 0.0,
        filter_env_attack: 0.0,
        filter_env_decay: 0.0001,
        filter_env_sustain: 999.9,
        filter_env_release: 5.0,
        filter_env_atk_curve: SmoothStyle::Linear,
        filter_env_dec_curve: SmoothStyle::Linear,
        filter_env_rel_curve: SmoothStyle::Linear,
        filter_alg_type: FilterAlgorithms::SVF,
        tilt_filter_type: ArduraFilter::ResponseType::Lowpass,

        filter_wet_2: 1.0,
        filter_cutoff_2: 20000.0,
        filter_resonance_2: 1.0,
        filter_res_type_2: ResonanceType::Default,
        filter_lp_amount_2: 1.0,
        filter_hp_amount_2: 0.0,
        filter_bp_amount_2: 0.0,
        filter_env_peak_2: 0.0,
        filter_env_attack_2: 0.0,
        filter_env_decay_2: 0.0001,
        filter_env_sustain_2: 999.9,
        filter_env_release_2: 5.0,
        filter_env_atk_curve_2: SmoothStyle::Linear,
        filter_env_dec_curve_2: SmoothStyle::Linear,
        filter_env_rel_curve_2: SmoothStyle::Linear,
        filter_alg_type_2: FilterAlgorithms::SVF,
        tilt_filter_type_2: ArduraFilter::ResponseType::Lowpass,

        filter_routing: FilterRouting::Parallel,
        filter_cutoff_link: false,

        pitch_enable: false,
        pitch_env_atk_curve: SmoothStyle::Linear,
        pitch_env_dec_curve: SmoothStyle::Linear,
        pitch_env_rel_curve: SmoothStyle::Linear,
        pitch_env_attack: 0.0,
        pitch_env_decay: 300.0,
        pitch_env_sustain: 0.0,
        pitch_env_release: 0.0,
        pitch_env_peak: 0.0,
        pitch_routing: PitchRouting::Osc1,

        pitch_enable_2: false,
        pitch_env_peak_2: 0.0,
        pitch_env_atk_curve_2: SmoothStyle::Linear,
        pitch_env_dec_curve_2: SmoothStyle::Linear,
        pitch_env_rel_curve_2: SmoothStyle::Linear,
        pitch_env_attack_2: 0.0,
        pitch_env_decay_2: 300.0,
        pitch_env_release_2: 0.0,
        pitch_env_sustain_2: 0.0,
        pitch_routing_2: PitchRouting::Osc1,

        // LFOs
        lfo1_enable: false,
        lfo2_enable: false,
        lfo3_enable: false,

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

        // Modulations
        mod_source_1: ModulationSource::None,
        mod_source_2: ModulationSource::None,
        mod_source_3: ModulationSource::None,
        mod_source_4: ModulationSource::None,
        mod_dest_1: ModulationDestination::None,
        mod_dest_2: ModulationDestination::None,
        mod_dest_3: ModulationDestination::None,
        mod_dest_4: ModulationDestination::None,
        mod_amount_1: 0.0,
        mod_amount_2: 0.0,
        mod_amount_3: 0.0,
        mod_amount_4: 0.0,

        // EQ
        pre_use_eq: false,
        pre_low_freq: 800.0,
        pre_mid_freq: 3000.0,
        pre_high_freq: 10000.0,
        pre_low_gain: 0.0,
        pre_mid_gain: 0.0,
        pre_high_gain: 0.0,

        // FX
        use_fx: true,

        use_compressor: false,
        comp_amt: 0.5,
        comp_atk: 0.5,
        comp_rel: 0.5,
        comp_drive: 0.5,

        use_abass: false,
        abass_amount: 0.0011,

        use_saturation: false,
        sat_amount: 0.0,
        sat_type: SaturationType::Tape,

        use_delay: false,
        delay_amount: 0.5,
        delay_time: DelaySnapValues::Quarter,
        delay_decay: 0.5,
        delay_type: DelayType::Stereo,

        use_reverb: false,
        reverb_model: ReverbModel::Default,
        reverb_amount: 0.85,
        reverb_size: 1.0,
        reverb_feedback: 0.28,

        use_phaser: false,
        phaser_amount: 0.5,
        phaser_depth: 0.5,
        phaser_rate: 0.5,
        phaser_feedback: 0.5,

        use_buffermod: false,
        buffermod_amount: 0.5,
        buffermod_depth: 0.5,
        buffermod_rate: 0.5,
        buffermod_spread: 0.0,
        buffermod_timing: 620.0,

        use_flanger: false,
        flanger_amount: 0.5,
        flanger_depth: 0.5,
        flanger_rate: 0.5,
        flanger_feedback: 0.5,

        use_limiter: false,
        limiter_threshold: 0.5,
        limiter_knee: 0.5,
    };

    static ref ERROR_PRESET: ActuatePresetV126 = ActuatePresetV126 {
        preset_name: String::from("Error Loading"),
        preset_info: String::from("Corrupt or incompatible versions"),
        preset_category: PresetType::Select,
        tag_acid: false,
        tag_analog: false,
        tag_bright: false,
        tag_chord: false,
        tag_crisp: false,
        tag_deep: false,
        tag_delicate: false,
        tag_hard: false,
        tag_harsh: false,
        tag_lush: false,
        tag_mellow: false,
        tag_resonant: false,
        tag_rich: false,
        tag_sharp: false,
        tag_silky: false,
        tag_smooth: false,
        tag_soft: false,
        tag_stab: false,
        tag_warm: false,
        mod1_audio_module_type: AudioModuleType::Osc,
        mod1_audio_module_level: 1.0,
        mod1_audio_module_routing: AMFilterRouting::Filter1,
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
        mod2_audio_module_routing: AMFilterRouting::Filter1,
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
        mod3_audio_module_routing: AMFilterRouting::Filter1,
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
        filter_cutoff: 20000.0,
        filter_resonance: 1.0,
        filter_res_type: ResonanceType::Default,
        filter_lp_amount: 1.0,
        filter_hp_amount: 0.0,
        filter_bp_amount: 0.0,
        filter_env_peak: 0.0,
        filter_env_attack: 0.0,
        filter_env_decay: 0.0001,
        filter_env_sustain: 999.9,
        filter_env_release: 5.0,
        filter_env_atk_curve: SmoothStyle::Linear,
        filter_env_dec_curve: SmoothStyle::Linear,
        filter_env_rel_curve: SmoothStyle::Linear,
        filter_alg_type: FilterAlgorithms::SVF,
        tilt_filter_type: ArduraFilter::ResponseType::Lowpass,

        filter_wet_2: 1.0,
        filter_cutoff_2: 20000.0,
        filter_resonance_2: 1.0,
        filter_res_type_2: ResonanceType::Default,
        filter_lp_amount_2: 1.0,
        filter_hp_amount_2: 0.0,
        filter_bp_amount_2: 0.0,
        filter_env_peak_2: 0.0,
        filter_env_attack_2: 0.0,
        filter_env_decay_2: 0.0001,
        filter_env_sustain_2: 999.9,
        filter_env_release_2: 5.0,
        filter_env_atk_curve_2: SmoothStyle::Linear,
        filter_env_dec_curve_2: SmoothStyle::Linear,
        filter_env_rel_curve_2: SmoothStyle::Linear,
        filter_alg_type_2: FilterAlgorithms::SVF,
        tilt_filter_type_2: ArduraFilter::ResponseType::Lowpass,

        filter_routing: FilterRouting::Parallel,
        filter_cutoff_link: false,

        pitch_enable: false,
        pitch_env_atk_curve: SmoothStyle::Linear,
        pitch_env_dec_curve: SmoothStyle::Linear,
        pitch_env_rel_curve: SmoothStyle::Linear,
        pitch_env_attack: 0.0,
        pitch_env_decay: 300.0,
        pitch_env_sustain: 0.0,
        pitch_env_release: 0.0,
        pitch_env_peak: 0.0,
        pitch_routing: PitchRouting::Osc1,

        pitch_enable_2: false,
        pitch_env_peak_2: 0.0,
        pitch_env_atk_curve_2: SmoothStyle::Linear,
        pitch_env_dec_curve_2: SmoothStyle::Linear,
        pitch_env_rel_curve_2: SmoothStyle::Linear,
        pitch_env_attack_2: 0.0,
        pitch_env_decay_2: 300.0,
        pitch_env_release_2: 0.0,
        pitch_env_sustain_2: 0.0,
        pitch_routing_2: PitchRouting::Osc1,

        // LFOs
        lfo1_enable: false,
        lfo2_enable: false,
        lfo3_enable: false,

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

        // Modulations
        mod_source_1: ModulationSource::None,
        mod_source_2: ModulationSource::None,
        mod_source_3: ModulationSource::None,
        mod_source_4: ModulationSource::None,
        mod_dest_1: ModulationDestination::None,
        mod_dest_2: ModulationDestination::None,
        mod_dest_3: ModulationDestination::None,
        mod_dest_4: ModulationDestination::None,
        mod_amount_1: 0.0,
        mod_amount_2: 0.0,
        mod_amount_3: 0.0,
        mod_amount_4: 0.0,

        // 1.2.6
        fm_one_to_two: 0.0,
        fm_one_to_three: 0.0,
        fm_two_to_three: 0.0,
        fm_cycles: 1,
        fm_attack: 0.0001,
        fm_decay: 0.0001,
        fm_sustain: 999.9,
        fm_release: 0.0001,
        fm_attack_curve: SmoothStyle::Linear,
        fm_decay_curve: SmoothStyle::Linear,
        fm_release_curve: SmoothStyle::Linear,
        // 1.2.6

        // EQ
        pre_use_eq: false,
        pre_low_freq: 800.0,
        pre_mid_freq: 3000.0,
        pre_high_freq: 10000.0,
        pre_low_gain: 0.0,
        pre_mid_gain: 0.0,
        pre_high_gain: 0.0,

        // FX
        use_fx: true,

        use_compressor: false,
        comp_amt: 0.5,
        comp_atk: 0.5,
        comp_rel: 0.5,
        comp_drive: 0.5,

        use_abass: false,
        abass_amount: 0.0011,

        use_saturation: false,
        sat_amount: 0.0,
        sat_type: SaturationType::Tape,

        use_delay: false,
        delay_amount: 0.5,
        delay_time: DelaySnapValues::Quarter,
        delay_decay: 0.5,
        delay_type: DelayType::Stereo,

        use_reverb: false,
        reverb_model: ReverbModel::Default,
        reverb_amount: 0.85,
        reverb_size: 1.0,
        reverb_feedback: 0.28,

        use_phaser: false,
        phaser_amount: 0.5,
        phaser_depth: 0.5,
        phaser_rate: 0.5,
        phaser_feedback: 0.5,

        use_buffermod: false,
        buffermod_amount: 0.5,
        buffermod_depth: 0.5,
        buffermod_rate: 0.5,
        buffermod_spread: 0.0,
        buffermod_timing: 620.0,

        use_flanger: false,
        flanger_amount: 0.5,
        flanger_depth: 0.5,
        flanger_rate: 0.5,
        flanger_feedback: 0.5,

        use_limiter: false,
        limiter_threshold: 0.5,
        limiter_knee: 0.5,
    };

    static ref DEFAULT_PRESET: ActuatePresetV126 = ActuatePresetV126 {
        preset_name: "Default".to_string(),
        preset_info: "Info".to_string(),
        preset_category: PresetType::Select,
        tag_acid: false,
        tag_analog: false,
        tag_bright: false,
        tag_chord: false,
        tag_crisp: false,
        tag_deep: false,
        tag_delicate: false,
        tag_hard: false,
        tag_harsh: false,
        tag_lush: false,
        tag_mellow: false,
        tag_resonant: false,
        tag_rich: false,
        tag_sharp: false,
        tag_silky: false,
        tag_smooth: false,
        tag_soft: false,
        tag_stab: false,
        tag_warm: false,
        mod1_audio_module_type: AudioModuleType::Osc,
        mod1_audio_module_level: 1.0,
        mod1_audio_module_routing: AMFilterRouting::Filter1,
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
        mod2_audio_module_routing: AMFilterRouting::Filter1,
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
        mod3_audio_module_routing: AMFilterRouting::Filter1,
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
        filter_cutoff: 20000.0,
        filter_resonance: 1.0,
        filter_res_type: ResonanceType::Default,
        filter_lp_amount: 1.0,
        filter_hp_amount: 0.0,
        filter_bp_amount: 0.0,
        filter_env_peak: 0.0,
        filter_env_attack: 0.0001,
        filter_env_decay: 0.0001,
        filter_env_sustain: 999.9,
        filter_env_release: 5.0,
        filter_env_atk_curve: SmoothStyle::Linear,
        filter_env_dec_curve: SmoothStyle::Linear,
        filter_env_rel_curve: SmoothStyle::Linear,
        filter_alg_type: FilterAlgorithms::SVF,
        tilt_filter_type: ArduraFilter::ResponseType::Lowpass,

        filter_wet_2: 1.0,
        filter_cutoff_2: 20000.0,
        filter_resonance_2: 1.0,
        filter_res_type_2: ResonanceType::Default,
        filter_lp_amount_2: 1.0,
        filter_hp_amount_2: 0.0,
        filter_bp_amount_2: 0.0,
        filter_env_peak_2: 0.0,
        filter_env_attack_2: 0.0001,
        filter_env_decay_2: 0.0001,
        filter_env_sustain_2: 999.9,
        filter_env_release_2: 5.0,
        filter_env_atk_curve_2: SmoothStyle::Linear,
        filter_env_dec_curve_2: SmoothStyle::Linear,
        filter_env_rel_curve_2: SmoothStyle::Linear,
        filter_alg_type_2: FilterAlgorithms::SVF,
        tilt_filter_type_2: ArduraFilter::ResponseType::Lowpass,

        filter_routing: FilterRouting::Parallel,
        filter_cutoff_link: false,

        // Pitch Routing
        pitch_enable: false,
        pitch_routing: PitchRouting::Osc1,
        pitch_env_peak: 0.0,
        pitch_env_attack: 0.0001,
        pitch_env_decay: 300.0,
        pitch_env_sustain: 0.0,
        pitch_env_release: 0.0001,
        pitch_env_atk_curve: SmoothStyle::Linear,
        pitch_env_dec_curve: SmoothStyle::Linear,
        pitch_env_rel_curve: SmoothStyle::Linear,

        pitch_enable_2: false,
        pitch_routing_2: PitchRouting::Osc1,
        pitch_env_peak_2: 0.0,
        pitch_env_attack_2: 0.0001,
        pitch_env_decay_2: 300.0,
        pitch_env_sustain_2: 0.0,
        pitch_env_release_2: 0.0001,
        pitch_env_atk_curve_2: SmoothStyle::Linear,
        pitch_env_dec_curve_2: SmoothStyle::Linear,
        pitch_env_rel_curve_2: SmoothStyle::Linear,

        // LFOs
        lfo1_enable: false,
        lfo2_enable: false,
        lfo3_enable: false,

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

        // Modulations
        mod_source_1: ModulationSource::None,
        mod_source_2: ModulationSource::None,
        mod_source_3: ModulationSource::None,
        mod_source_4: ModulationSource::None,
        mod_dest_1: ModulationDestination::None,
        mod_dest_2: ModulationDestination::None,
        mod_dest_3: ModulationDestination::None,
        mod_dest_4: ModulationDestination::None,
        mod_amount_1: 0.0,
        mod_amount_2: 0.0,
        mod_amount_3: 0.0,
        mod_amount_4: 0.0,

        // 1.2.6
        fm_one_to_two: 0.0,
        fm_one_to_three: 0.0,
        fm_two_to_three: 0.0,
        fm_cycles: 1,
        fm_attack: 0.0001,
        fm_decay: 0.0001,
        fm_sustain: 999.9,
        fm_release: 0.0001,
        fm_attack_curve: SmoothStyle::Linear,
        fm_decay_curve: SmoothStyle::Linear,
        fm_release_curve: SmoothStyle::Linear,
        // 1.2.6

        // EQ
        pre_use_eq: false,
        pre_low_freq: 800.0,
        pre_mid_freq: 3000.0,
        pre_high_freq: 10000.0,
        pre_low_gain: 0.0,
        pre_mid_gain: 0.0,
        pre_high_gain: 0.0,

        //FX
        use_fx: true,

        use_compressor: false,

        comp_amt: 0.3,
        comp_atk: 0.8,
        comp_rel: 0.3,
        comp_drive: 0.3,

        use_abass: false,
        abass_amount: 0.00067,

        use_saturation: false,
        sat_amount: 0.0,
        sat_type: SaturationType::Tape,

        use_delay: false,
        delay_amount: 0.5,
        delay_time: DelaySnapValues::Quarter,
        delay_decay: 0.5,
        delay_type: DelayType::Stereo,

        use_reverb: false,
        reverb_model: ReverbModel::Default,
        reverb_amount: 0.85,
        reverb_size: 1.0,
        reverb_feedback: 0.28,

        use_phaser: false,
        phaser_amount: 0.5,
        phaser_depth: 0.5,
        phaser_rate: 0.5,
        phaser_feedback: 0.5,

        use_buffermod: false,
        buffermod_amount: 0.5,
        buffermod_depth: 0.5,
        buffermod_rate: 0.5,
        buffermod_spread: 0.0,
        buffermod_timing: 620.0,

        use_flanger: false,
        flanger_amount: 0.5,
        flanger_depth: 0.5,
        flanger_rate: 0.5,
        flanger_feedback: 0.5,

        use_limiter: false,
        limiter_threshold: 0.5,
        limiter_knee: 0.5,
    };
);