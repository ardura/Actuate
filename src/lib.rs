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
Version 1.1

#####################################

This is the first synth I've ever written and first large Rust project. Thanks for checking it out and have fun!

#####################################
*/

#![allow(non_snake_case)]
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use nih_plug::prelude::*;
use nih_plug_egui::{
    create_egui_editor,
    egui::{self, Align2, Color32, FontId, Pos2, Rect, RichText, Rounding, ScrollArea, Vec2},
    widgets::ParamSlider,
    EguiState,
};
use rfd::FileDialog;
use serde::{Deserialize, Serialize};
use std::{fmt, io::Read};
use std::{
    fs::File,
    io::Write,
    ops::RangeInclusive,
    sync::{
        atomic::{AtomicBool, AtomicU32, Ordering},
        Arc, Mutex,
    },
};
use tinyfiledialogs;

// My Files/crates
use audio_module::{
    AudioModule, AudioModuleType,
    Oscillator::{self, OscState, RetriggerStyle, SmoothStyle, VoiceType},
};
use fx::{
    abass::a_bass_saturation,
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
    VCFilter::ResponseType as VCResponseType,
};
use old_preset_structs::{load_unserialized_old, load_unserialized_v114};
use CustomWidgets::{
    toggle_switch, ui_knob, BoolButton, CustomParamSlider,
    CustomParamSlider::ParamSlider as HorizontalParamSlider,
    CustomVerticalSlider::ParamSlider as VerticalParamSlider,
};

mod CustomWidgets;
mod LFOController;
mod audio_module;
mod fx;
mod old_preset_structs;

// This holds our current sample/granulizer sample (L,R) per sample
pub struct LoadedSample(Vec<Vec<f32>>);

// Plugin sizing
const WIDTH: u32 = 920;
const HEIGHT: u32 = 656;

// Until we have a real preset editor and browser it's better to keep the preset bank smaller
const PRESET_BANK_SIZE: usize = 32;

// File Open Buffer Timer - fixes sync issues from load/save to the gui
const FILE_OPEN_BUFFER_MAX: u32 = 1;

// GUI values to refer to
pub const A_KNOB_OUTSIDE_COLOR: Color32 = Color32::from_rgb(67, 157, 148);
pub const DARK_GREY_UI_COLOR: Color32 = Color32::from_rgb(49, 53, 71);
pub const LIGHT_GREY_UI_COLOR: Color32 = Color32::from_rgb(99, 103, 121);
pub const LIGHTER_GREY_UI_COLOR: Color32 = Color32::from_rgb(149, 153, 171);
pub const SYNTH_SOFT_BLUE: Color32 = Color32::from_rgb(142, 166, 201);
pub const SYNTH_SOFT_BLUE2: Color32 = Color32::from_rgb(102, 126, 181);
pub const A_BACKGROUND_COLOR_TOP: Color32 = Color32::from_rgb(185, 186, 198);
pub const SYNTH_BARS_PURPLE: Color32 = Color32::from_rgb(45, 41, 99);
pub const LIGHTER_PURPLE: Color32 = Color32::from_rgb(85, 81, 139);
pub const SYNTH_MIDDLE_BLUE: Color32 = Color32::from_rgb(98, 145, 204);
pub const FONT_COLOR: Color32 = Color32::from_rgb(10, 103, 210);

// Gui for which filter to display on bottom
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
enum UIBottomSelection {
    Filter1,
    Filter2,
    Pitch
}

// Gui for which panel to display in bottom right
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
enum LFOSelect {
    INFO,
    LFO1,
    LFO2,
    LFO3,
    Modulation,
    Misc,
    FX,
}

// Sources that can modulate a value
#[derive(Debug, PartialEq, Enum, Clone, Copy, Serialize, Deserialize)]
pub enum ModulationSource {
    None,
    Velocity,
    LFO1,
    LFO2,
    LFO3,
    UnsetModulation,
}

// Destinations modulations can go
#[allow(non_camel_case_types)]
#[derive(Debug, PartialEq, Enum, Clone, Copy, Serialize, Deserialize)]
pub enum ModulationDestination {
    None,
    Cutoff_1,
    Cutoff_2,
    Resonance_1,
    Resonance_2,
    All_Gain,
    Osc1_Gain,
    Osc2_Gain,
    Osc3_Gain,
    All_Detune,
    Osc1Detune,
    Osc2Detune,
    Osc3Detune,
    All_UniDetune,
    Osc1UniDetune,
    Osc2UniDetune,
    Osc3UniDetune,
    UnsetModulation,
}

// Values for Audio Module Routing to filters
#[derive(Enum, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub enum AMFilterRouting {
    Bypass,
    Filter1,
    Filter2,
    Both,
}

// Filter implementations
#[derive(Enum, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub enum FilterAlgorithms {
    SVF,
    TILT,
    VCF,
}

// Preset categories in dropdown
#[derive(Debug, Enum, PartialEq, Eq, Serialize, Deserialize, Clone, Copy)]
pub enum PresetType {
    Select,
    Atmosphere,
    Bass,
    FX,
    Keys,
    Lead,
    Pad,
    Percussion,
    Pluck,
    Synth,
    Other,
}

// These let us output ToString for the ComboBox stuff + Nih-Plug
impl fmt::Display for PresetType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
        // or, alternatively:
        // fmt::Debug::fmt(self, f)
    }
}

// These let us output ToString for the ComboBox stuff + Nih-Plug
impl fmt::Display for ModulationSource {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
        // or, alternatively:
        // fmt::Debug::fmt(self, f)
    }
}

// These let us output ToString for the ComboBox stuff + Nih-Plug
impl fmt::Display for ModulationDestination {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
        // or, alternatively:
        // fmt::Debug::fmt(self, f)
    }
}

// Filter order routing
#[derive(Enum, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub enum FilterRouting {
    Parallel,
    Series12,
    Series21,
}

// Pitch Envelope routing
#[allow(non_camel_case_types)]
#[derive(Enum, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub enum PitchRouting {
    All,
    Osc1,
    Osc2,
    Osc3,
    Osc1_Osc2,
    Osc1_Osc3,
    Osc2_Osc3,
}

// Fonts
const FONT: nih_plug_egui::egui::FontId = FontId::proportional(14.0);
const LOADING_FONT: nih_plug_egui::egui::FontId = FontId::proportional(20.0);
const SMALLER_FONT: nih_plug_egui::egui::FontId = FontId::proportional(11.0);

/// Modulation struct for passing mods to audio modules
#[derive(Serialize, Deserialize, Clone)]
pub struct ModulationStruct {
    temp_mod_cutoff_1: f32,
    temp_mod_cutoff_2: f32,
    temp_mod_resonance_1: f32,
    temp_mod_resonance_2: f32,
    temp_mod_detune_1: f32,
    temp_mod_detune_2: f32,
    temp_mod_detune_3: f32,
    temp_mod_uni_detune_1: f32,
    temp_mod_uni_detune_2: f32,
    temp_mod_uni_detune_3: f32,
    temp_mod_vel_sum: f32,
}

/// This is the structure that represents a storable preset value
#[derive(Serialize, Deserialize, Clone)]
pub struct ActuatePreset {
    // Information
    preset_name: String,
    preset_info: String,
    preset_category: PresetType,
    // Preset tag information - made into bools to make my life easier
    tag_acid: bool,
    tag_analog: bool,
    tag_bright: bool,
    tag_chord: bool,
    tag_crisp: bool,
    tag_deep: bool,
    tag_delicate: bool,
    tag_hard: bool,
    tag_harsh: bool,
    tag_lush: bool,
    tag_mellow: bool,
    tag_resonant: bool,
    tag_rich: bool,
    tag_sharp: bool,
    tag_silky: bool,
    tag_smooth: bool,
    tag_soft: bool,
    tag_stab: bool,
    tag_warm: bool,

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
    filter_alg_type: FilterAlgorithms,
    tilt_filter_type: ArduraFilter::ResponseType,

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
    filter_alg_type_2: FilterAlgorithms,
    tilt_filter_type_2: ArduraFilter::ResponseType,

    filter_routing: FilterRouting,
    filter_cutoff_link: bool,

    // Pitch Env
    pitch_enable: bool,
    pitch_routing: PitchRouting,
    pitch_env_peak: f32,
    pitch_env_attack: f32,
    pitch_env_decay: f32,
    pitch_env_sustain: f32,
    pitch_env_release: f32,
    pitch_env_atk_curve: Oscillator::SmoothStyle,
    pitch_env_dec_curve: Oscillator::SmoothStyle,
    pitch_env_rel_curve: Oscillator::SmoothStyle,

    // LFOs
    lfo1_enable: bool,
    lfo2_enable: bool,
    lfo3_enable: bool,

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

    // Modulation
    mod_source_1: ModulationSource,
    mod_source_2: ModulationSource,
    mod_source_3: ModulationSource,
    mod_source_4: ModulationSource,
    mod_dest_1: ModulationDestination,
    mod_dest_2: ModulationDestination,
    mod_dest_3: ModulationDestination,
    mod_dest_4: ModulationDestination,
    mod_amount_1: f32,
    mod_amount_2: f32,
    mod_amount_3: f32,
    mod_amount_4: f32,

    // EQ
    pre_use_eq: bool,
    pre_low_freq: f32,
    pre_mid_freq: f32,
    pre_high_freq: f32,
    pre_low_gain: f32,
    pre_mid_gain: f32,
    pre_high_gain: f32,

    // FX
    use_fx: bool,

    use_compressor: bool,
    comp_amt: f32,
    comp_atk: f32,
    comp_rel: f32,
    comp_drive: f32,

    use_abass: bool,
    abass_amount: f32,

    use_saturation: bool,
    sat_amount: f32,
    sat_type: SaturationType,

    use_delay: bool,
    delay_amount: f32,
    delay_time: DelaySnapValues,
    delay_decay: f32,
    delay_type: DelayType,

    use_reverb: bool,
    reverb_amount: f32,
    reverb_size: f32,
    reverb_feedback: f32,

    use_phaser: bool,
    phaser_amount: f32,
    phaser_depth: f32,
    phaser_rate: f32,
    phaser_feedback: f32,

    use_buffermod: bool,
    buffermod_amount: f32,
    buffermod_depth: f32,
    buffermod_rate: f32,
    buffermod_spread: f32,
    buffermod_timing: f32,

    use_flanger: bool,
    flanger_amount: f32,
    flanger_depth: f32,
    flanger_rate: f32,
    flanger_feedback: f32,

    use_limiter: bool,
    limiter_threshold: f32,
    limiter_knee: f32,
}

// This is the struct of the actual plugin object that tracks everything
//#[derive(Clone)]
pub struct Actuate {
    pub params: Arc<ActuateParams>,
    pub sample_rate: f32,

    // Plugin control Arcs
    update_something: Arc<AtomicBool>,
    clear_voices: Arc<AtomicBool>,
    reload_entire_preset: Arc<Mutex<bool>>,
    file_dialog: Arc<AtomicBool>,
    file_open_buffer_timer: Arc<AtomicU32>,
    // Using this like a state: 0 = Closed, 1 = Opening, 2 = Open
    current_preset: Arc<AtomicU32>,

    update_current_preset: Arc<AtomicBool>,
    load_bank: Arc<Mutex<bool>>,
    save_bank: Arc<Mutex<bool>>,
    import_preset: Arc<Mutex<bool>>,
    export_preset: Arc<Mutex<bool>>,

    current_note_on_velocity: Arc<AtomicF32>,

    // Modules
    audio_module_1: Arc<Mutex<AudioModule>>,
    _audio_module_1_type: AudioModuleType,
    audio_module_2: Arc<Mutex<AudioModule>>,
    _audio_module_2_type: AudioModuleType,
    audio_module_3: Arc<Mutex<AudioModule>>,
    _audio_module_3_type: AudioModuleType,

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
    preset_lib_name: String,
    preset_name: Arc<Mutex<String>>,
    preset_info: Arc<Mutex<String>>,
    preset_category: Arc<Mutex<PresetType>>,
    preset_lib: Arc<Mutex<Vec<ActuatePreset>>>,

    // Used for DC Offset calculations
    dc_filter_l: StateVariableFilter,
    dc_filter_r: StateVariableFilter,

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

    // Phaser
    phaser: StereoPhaser,

    // Buffer Modulation
    buffermod: BufferModulator,

    // Flanger
    flanger: StereoFlanger,

    // Limiter
    limiter: StereoLimiter,
}

impl Default for Actuate {
    fn default() -> Self {
        // These are persistent fields to trigger updates like Diopser
        let update_something = Arc::new(AtomicBool::new(false));
        let clear_voices = Arc::new(AtomicBool::new(false));
        let reload_entire_preset = Arc::new(Mutex::new(false));
        let file_dialog = Arc::new(AtomicBool::new(false));
        let file_open_buffer_timer = Arc::new(AtomicU32::new(0));
        let current_preset = Arc::new(AtomicU32::new(0));

        let load_bank = Arc::new(Mutex::new(false));
        let save_bank = Arc::new(Mutex::new(false));
        let import_preset = Arc::new(Mutex::new(false));
        let export_preset = Arc::new(Mutex::new(false));
        let update_current_preset = Arc::new(AtomicBool::new(false));

        Self {
            params: Arc::new(ActuateParams::new(
                update_something.clone(),
                clear_voices.clone(),
                file_dialog.clone(),
                update_current_preset.clone(),
                load_bank.clone(),
                save_bank.clone(),
                import_preset.clone(),
                export_preset.clone(),
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
            import_preset: import_preset,
            export_preset: export_preset,
            update_current_preset: update_current_preset,

            current_note_on_velocity: Arc::new(AtomicF32::new(0.0)),

            // Module 1
            audio_module_1: Arc::new(Mutex::new(AudioModule::default())),
            _audio_module_1_type: AudioModuleType::Osc,
            audio_module_2: Arc::new(Mutex::new(AudioModule::default())),
            _audio_module_2_type: AudioModuleType::Off,
            audio_module_3: Arc::new(Mutex::new(AudioModule::default())),
            _audio_module_3_type: AudioModuleType::Off,

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
            preset_lib_name: String::from("Default"),
            preset_name: Arc::new(Mutex::new(String::new())),
            preset_info: Arc::new(Mutex::new(String::new())),
            preset_category: Arc::new(Mutex::new(PresetType::Select)),
            preset_lib: Arc::new(Mutex::new(vec![
                ActuatePreset {
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

                    // Pitch Routing
                    pitch_enable: false,
                    pitch_routing: PitchRouting::Osc1,
                    pitch_env_peak: 0.0,
                    pitch_env_attack: 0.0,
                    pitch_env_decay: 300.0,
                    pitch_env_sustain: 0.0,
                    pitch_env_release: 0.0,
                    pitch_env_atk_curve: SmoothStyle::Linear,
                    pitch_env_dec_curve: SmoothStyle::Linear,
                    pitch_env_rel_curve: SmoothStyle::Linear,

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

                    //FX
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
                    delay_amount: 0.0,
                    delay_time: DelaySnapValues::Quarter,
                    delay_decay: 0.0,
                    delay_type: DelayType::Stereo,

                    use_reverb: false,
                    reverb_amount: 0.5,
                    reverb_size: 0.5,
                    reverb_feedback: 0.5,

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
                PRESET_BANK_SIZE
            ])),

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

            // Buffer Modulator
            buffermod: BufferModulator::new(44100.0, 0.5, 10.0),

            // Flanger initialized to use delay range of 50, for 100 samples
            flanger: StereoFlanger::new(44100.0, 0.5, 0.5, 10.0, 0.5, 20),

            // Phaser
            phaser: StereoPhaser::new(),

            // Limiter
            limiter: StereoLimiter::new(0.5, 0.5),
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
        load_bank: Arc<Mutex<bool>>,
        save_bank: Arc<Mutex<bool>>,
        import_preset: Arc<Mutex<bool>>,
        export_preset: Arc<Mutex<bool>>,
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
            osc_1_semitones: IntParam::new("Semi", 0, IntRange::Linear { min: -11, max: 11 })
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
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
            osc_1_retrigger: EnumParam::new("Retrig", RetriggerStyle::Retrigger).with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
            }),
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
                "UDetune",
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
                .with_value_to_string(formatters::v2s_f32_rounded(2))
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
            osc_2_semitones: IntParam::new("Semi", 0, IntRange::Linear { min: -11, max: 11 })
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
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
            osc_2_retrigger: EnumParam::new("Retrig", RetriggerStyle::Retrigger).with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
            }),
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
                "UDetune",
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
                .with_value_to_string(formatters::v2s_f32_rounded(2))
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
            osc_3_semitones: IntParam::new("Semi", 0, IntRange::Linear { min: -11, max: 11 })
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
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
            osc_3_retrigger: EnumParam::new("Retrig", RetriggerStyle::Retrigger).with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
            }),
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
                "UDetune",
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
                .with_value_to_string(formatters::v2s_f32_rounded(2))
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
                }),

            // Granulizer/Sampler
            ////////////////////////////////////////////////////////////////////////////////////
            load_sample_1: BoolParam::new("Load Sample", false)
                .with_callback({
                    let file_dialog = file_dialog.clone();
                    Arc::new(move |_| file_dialog.store(true, Ordering::Relaxed))
                })
                .hide(),
            load_sample_2: BoolParam::new("Load Sample", false)
                .with_callback({
                    let file_dialog = file_dialog.clone();
                    Arc::new(move |_| file_dialog.store(true, Ordering::Relaxed))
                })
                .hide(),
            load_sample_3: BoolParam::new("Load Sample", false)
                .with_callback({
                    let file_dialog = file_dialog.clone();
                    Arc::new(move |_| file_dialog.store(true, Ordering::Relaxed))
                })
                .hide(),
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
            grain_hold_1: IntParam::new("Hold", 200, IntRange::Linear { min: 5, max: 22050 })
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
                }),
            grain_hold_2: IntParam::new("Hold", 200, IntRange::Linear { min: 5, max: 22050 })
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
                }),
            grain_hold_3: IntParam::new("Hold", 200, IntRange::Linear { min: 5, max: 22050 })
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
                }),
            grain_gap_1: IntParam::new("Gap", 200, IntRange::Linear { min: 0, max: 22050 })
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
                }),
            grain_gap_2: IntParam::new("Gap", 200, IntRange::Linear { min: 0, max: 22050 })
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
                }),
            grain_gap_3: IntParam::new("Gap", 200, IntRange::Linear { min: 0, max: 22050 })
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
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
                Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
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
                Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
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
                Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
            }),
            end_position_1: FloatParam::new("End", 1.0, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_unit("%")
                .with_value_to_string(formatters::v2s_f32_percentage(0))
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
                }),
            end_position_2: FloatParam::new("End", 1.0, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_unit("%")
                .with_value_to_string(formatters::v2s_f32_percentage(0))
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
                }),
            end_position_3: FloatParam::new("End", 1.0, FloatRange::Linear { min: 0.0, max: 1.0 })
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
                "LPF",
                1.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_value_to_string(formatters::v2s_f32_percentage(0))
            .with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
            }),
            filter_hp_amount: FloatParam::new(
                "HPF",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_value_to_string(formatters::v2s_f32_percentage(0))
            .with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
            }),
            filter_bp_amount: FloatParam::new(
                "BPF",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_value_to_string(formatters::v2s_f32_percentage(0))
            .with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
            }),

            filter_wet: FloatParam::new(
                "Filter",
                1.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_unit("%")
            .with_value_to_string(formatters::v2s_f32_percentage(0))
            .with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
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
                Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
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
                Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
            }),
            filter_alg_type: EnumParam::new("Filter 1 Alg", FilterAlgorithms::SVF),
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
                "LPF",
                1.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_value_to_string(formatters::v2s_f32_percentage(0))
            .with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
            }),
            filter_hp_amount_2: FloatParam::new(
                "HPF",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_value_to_string(formatters::v2s_f32_percentage(0))
            .with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
            }),
            filter_bp_amount_2: FloatParam::new(
                "BPF",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_value_to_string(formatters::v2s_f32_percentage(0))
            .with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
            }),

            filter_wet_2: FloatParam::new(
                "Filter",
                1.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_unit("%")
            .with_value_to_string(formatters::v2s_f32_percentage(0))
            .with_callback({
                let update_something = update_something.clone();
                Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
            }),
            filter_resonance_2: FloatParam::new(
                "Res",
                1.0,
                FloatRange::Reversed(&FloatRange::Linear { min: 0.1, max: 1.0 }),
            )
            .with_unit("%")
            .with_value_to_string(formatters::v2s_f32_percentage(0)),
            filter_res_type_2: EnumParam::new("Res Type", ResonanceType::Default).with_callback(
                {
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
                },
            ),
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
                Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
            }),
            filter_alg_type_2: EnumParam::new("Filter 2 Alg", FilterAlgorithms::SVF),
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

            filter_cutoff_link: BoolParam::new("Filter Cutoffs Linked", false),

            // Pitch Envelope
            ////////////////////////////////////////////////////////////////////////////////////
            pitch_env_peak: FloatParam::new(
                "PITCHAA",
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
                Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
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
                Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
            }),
            pitch_env_decay: FloatParam::new(
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
            pitch_env_sustain: FloatParam::new(
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
                Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
            }),
            pitch_env_atk_curve: EnumParam::new("Atk Curve", Oscillator::SmoothStyle::Linear)
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
                }),
            pitch_env_dec_curve: EnumParam::new("Dec Curve", Oscillator::SmoothStyle::Linear)
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
                }),
            pitch_env_rel_curve: EnumParam::new("Rel Curve", Oscillator::SmoothStyle::Linear)
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
                }),
            pitch_enable: BoolParam::new("Pitch Enable", false),
            pitch_routing: EnumParam::new("Routing", PitchRouting::Osc1)
                .with_callback({
                    let update_something = update_something.clone();
                    Arc::new(move |_| update_something.store(true, Ordering::Relaxed))
                }),


            // LFOs
            ////////////////////////////////////////////////////////////////////////////////////
            lfo1_enable: BoolParam::new("LFO 1 Enable", false),
            lfo2_enable: BoolParam::new("LFO 2 Enable", false),
            lfo3_enable: BoolParam::new("LFO 3 Enable", false),
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
                    max: 9600.0,
                    factor: 0.3,
                }, // Based on max bpm of 300 w/ 32nd notes
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
                    max: 9600.0,
                    factor: 0.3,
                }, // Based on max bpm of 300 w/ 32nd notes
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
                    max: 9600.0,
                    factor: 0.3,
                }, // Based on max bpm of 300 w/ 32nd notes
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
            use_fx: BoolParam::new("FX", true),

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
                0.0011,
                FloatRange::Skewed {
                    min: 0.0,
                    max: 1.0,
                    factor: 0.3,
                },
            )
            .with_value_to_string(formatters::v2s_f32_rounded(4)),

            use_saturation: BoolParam::new("Saturation", false),
            sat_amt: FloatParam::new("Amount", 0.0, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_value_to_string(formatters::v2s_f32_rounded(2)),
            sat_type: EnumParam::new("Type", SaturationType::Tape),

            use_delay: BoolParam::new("Delay", false),
            delay_amount: FloatParam::new("Amount", 0.5, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_value_to_string(formatters::v2s_f32_rounded(2)),
            delay_time: EnumParam::new("Time", DelaySnapValues::Quarter),
            delay_decay: FloatParam::new("Decay", 0.5, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_value_to_string(formatters::v2s_f32_rounded(2)),
            delay_type: EnumParam::new("Type", DelayType::Stereo),

            use_reverb: BoolParam::new("Reverb", false),
            reverb_amount: FloatParam::new(
                "Amount",
                0.85,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_value_to_string(formatters::v2s_f32_rounded(2)),
            reverb_size: FloatParam::new("Size", 1.0, FloatRange::Linear { min: 0.0, max: 2.0 })
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
                    min: 0.1,
                    max: 100.0,
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
                    min: 0.01,
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

            // UI Non-Param Params
            ////////////////////////////////////////////////////////////////////////////////////
            param_load_bank: BoolParam::new("Load Bank", false)
                .with_callback({
                    let load_bank = load_bank.clone();
                    Arc::new(move |_| *load_bank.lock().unwrap() = true)
                })
                .hide(),
            param_save_bank: BoolParam::new("Save Bank", false)
                .with_callback({
                    let save_bank = save_bank.clone();
                    Arc::new(move |_| *save_bank.lock().unwrap() = true)
                })
                .hide(),
            param_import_preset: BoolParam::new("Import Preset", false)
                .with_callback({
                    let import_preset = import_preset.clone();
                    Arc::new(move |_| *import_preset.lock().unwrap() = true)
                })
                .hide(),
            param_export_preset: BoolParam::new("Export Preset", false)
                .with_callback({
                    let export_preset = export_preset.clone();
                    Arc::new(move |_| *export_preset.lock().unwrap() = true)
                })
                .hide(),
            preset_category: EnumParam::new("Type", PresetType::Select),
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
                    Arc::new(move |_| update_current_preset.store(true, Ordering::Relaxed))
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
        let params: Arc<ActuateParams> = self.params.clone();
        let arc_preset: Arc<Mutex<Vec<ActuatePreset>>> = Arc::clone(&self.preset_lib); //Arc<Mutex<Vec<ActuatePreset>>>
        let arc_preset_name: Arc<Mutex<String>> = Arc::clone(&self.preset_name);
        let arc_preset_info: Arc<Mutex<String>> = Arc::clone(&self.preset_info);
        let arc_preset_category: Arc<Mutex<PresetType>> = Arc::clone(&self.preset_category);
        let clear_voices: Arc<AtomicBool> = Arc::clone(&self.clear_voices);
        let reload_entire_preset: Arc<Mutex<bool>> = Arc::clone(&self.reload_entire_preset);
        let current_preset: Arc<AtomicU32> = Arc::clone(&self.current_preset);
        let AM1: Arc<Mutex<AudioModule>> = Arc::clone(&self.audio_module_1);
        let AM2: Arc<Mutex<AudioModule>> = Arc::clone(&self.audio_module_2);
        let AM3: Arc<Mutex<AudioModule>> = Arc::clone(&self.audio_module_3);

        let update_current_preset: Arc<AtomicBool> = Arc::clone(&self.update_current_preset);
        let load_bank: Arc<Mutex<bool>> = Arc::clone(&self.load_bank);
        let save_bank: Arc<Mutex<bool>> = Arc::clone(&self.save_bank);
        let import_preset: Arc<Mutex<bool>> = Arc::clone(&self.import_preset);
        let export_preset: Arc<Mutex<bool>> = Arc::clone(&self.export_preset);

        let loading: Arc<AtomicBool> = Arc::clone(&self.file_dialog);
        let filter_select_outside: Arc<Mutex<UIBottomSelection>> =
            Arc::new(Mutex::new(UIBottomSelection::Filter1));
        let lfo_select_outside: Arc<Mutex<LFOSelect>> = Arc::new(Mutex::new(LFOSelect::INFO));
        let mod_source_1_tracker_outside: Arc<Mutex<ModulationSource>> =
            Arc::new(Mutex::new(ModulationSource::None));
        let mod_source_2_tracker_outside: Arc<Mutex<ModulationSource>> =
            Arc::new(Mutex::new(ModulationSource::None));
        let mod_source_3_tracker_outside: Arc<Mutex<ModulationSource>> =
            Arc::new(Mutex::new(ModulationSource::None));
        let mod_source_4_tracker_outside: Arc<Mutex<ModulationSource>> =
            Arc::new(Mutex::new(ModulationSource::None));
        let mod_dest_1_tracker_outside: Arc<Mutex<ModulationDestination>> =
            Arc::new(Mutex::new(ModulationDestination::None));
        let mod_dest_2_tracker_outside: Arc<Mutex<ModulationDestination>> =
            Arc::new(Mutex::new(ModulationDestination::None));
        let mod_dest_3_tracker_outside: Arc<Mutex<ModulationDestination>> =
            Arc::new(Mutex::new(ModulationDestination::None));
        let mod_dest_4_tracker_outside: Arc<Mutex<ModulationDestination>> =
            Arc::new(Mutex::new(ModulationDestination::None));

        let preset_category_tracker_outside: Arc<Mutex<PresetType>> =
            Arc::new(Mutex::new(PresetType::Select));

        let mod_source_override_1 = self.mod_override_source_1.clone();
        let mod_source_override_2 = self.mod_override_source_2.clone();
        let mod_source_override_3 = self.mod_override_source_3.clone();
        let mod_source_override_4 = self.mod_override_source_4.clone();
        let mod_dest_override_1 = self.mod_override_dest_1.clone();
        let mod_dest_override_2 = self.mod_override_dest_2.clone();
        let mod_dest_override_3 = self.mod_override_dest_3.clone();
        let mod_dest_override_4 = self.mod_override_dest_4.clone();
        let preset_category_override = self.preset_category_override.clone();

        // Do our GUI stuff. Store this to later get parent window handle from it
        create_egui_editor(
            self.params.editor_state.clone(),
            (),
            |_, _| {},
            move |egui_ctx, setter, _state| {
                egui::CentralPanel::default()
                    .show(egui_ctx, |ui| {
                        let current_preset_index = current_preset.load(Ordering::Relaxed);
                        let filter_select = filter_select_outside.clone();
                        let lfo_select = lfo_select_outside.clone();
                        let mod_source_1_tracker = mod_source_1_tracker_outside.clone();
                        let mod_source_2_tracker = mod_source_2_tracker_outside.clone();
                        let mod_source_3_tracker = mod_source_3_tracker_outside.clone();
                        let mod_source_4_tracker = mod_source_4_tracker_outside.clone();
                        let mod_dest_1_tracker = mod_dest_1_tracker_outside.clone();
                        let mod_dest_2_tracker = mod_dest_2_tracker_outside.clone();
                        let mod_dest_3_tracker = mod_dest_3_tracker_outside.clone();
                        let mod_dest_4_tracker = mod_dest_4_tracker_outside.clone();
                        let preset_category_tracker = preset_category_tracker_outside.clone();

                        // Reset our buttons
                        if params.param_next_preset.value() {
                            if current_preset_index < (PRESET_BANK_SIZE - 1) as u32 {
                                loading.store(true, Ordering::Relaxed);
                                setter.set_parameter(&params.loading, true);

                                current_preset.store(current_preset_index + 1, Ordering::Relaxed);

                                setter.set_parameter(&params.param_next_preset, false);
                                clear_voices.store(true, Ordering::Relaxed);

                                // Move to info tab on preset change
                                *lfo_select.lock().unwrap() = LFOSelect::INFO;

                                // Update our displayed info
                                let temp_current_preset = arc_preset.lock().unwrap()[current_preset_index as usize + 1].clone();
                                *arc_preset_name.lock().unwrap() = temp_current_preset.preset_name;
                                *arc_preset_info.lock().unwrap() = temp_current_preset.preset_info;

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
                                (
                                    *mod_source_override_1.lock().unwrap(),
                                    *mod_source_override_2.lock().unwrap(),
                                    *mod_source_override_3.lock().unwrap(),
                                    *mod_source_override_4.lock().unwrap(),
                                    *mod_dest_override_1.lock().unwrap(),
                                    *mod_dest_override_2.lock().unwrap(),
                                    *mod_dest_override_3.lock().unwrap(),
                                    *mod_dest_override_4.lock().unwrap(),
                                    *preset_category_override.lock().unwrap(),
                                ) = Actuate::reload_entire_preset(
                                    setter,
                                    params.clone(),
                                    (current_preset_index + 1) as usize,
                                    arc_preset.clone(),
                                    AM1.clone(),
                                    AM2.clone(),
                                    AM3.clone(),);

                                // This is the gui value only - the preset type itself is loaded in the preset already
                                *arc_preset_category.lock().unwrap() = *preset_category_override.lock().unwrap();

                                // This is set for the process thread
                                *reload_entire_preset.lock().unwrap() = true;
                            }
                            setter.set_parameter(&params.loading, false);
                        }
                        if params.param_prev_preset.value() {
                            if current_preset_index > 0 {
                                loading.store(true, Ordering::Relaxed);
                                setter.set_parameter(&params.loading, true);

                                current_preset.store(current_preset_index - 1, Ordering::Relaxed);

                                setter.set_parameter(&params.param_prev_preset, false);
                                clear_voices.store(true, Ordering::Relaxed);

                                // Move to info tab on preset change
                                *lfo_select.lock().unwrap() = LFOSelect::INFO;

                                // Update our displayed info
                                let temp_current_preset = arc_preset.lock().unwrap()[current_preset_index as usize - 1].clone();
                                *arc_preset_name.lock().unwrap() = temp_current_preset.preset_name;
                                *arc_preset_info.lock().unwrap() = temp_current_preset.preset_info;

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
                                (
                                    *mod_source_override_1.lock().unwrap(),
                                    *mod_source_override_2.lock().unwrap(),
                                    *mod_source_override_3.lock().unwrap(),
                                    *mod_source_override_4.lock().unwrap(),
                                    *mod_dest_override_1.lock().unwrap(),
                                    *mod_dest_override_2.lock().unwrap(),
                                    *mod_dest_override_3.lock().unwrap(),
                                    *mod_dest_override_4.lock().unwrap(),
                                    *preset_category_override.lock().unwrap(),
                                ) = Actuate::reload_entire_preset(
                                    setter,
                                    params.clone(),
                                    (current_preset_index - 1) as usize,
                                    arc_preset.clone(),
                                    AM1.clone(),
                                    AM2.clone(),
                                    AM3.clone(),);

                                // This is the gui value only - the preset type itself is loaded in the preset already
                                *arc_preset_category.lock().unwrap() = *preset_category_override.lock().unwrap();

                                // This is set for the process thread
                                *reload_entire_preset.lock().unwrap() = true;
                            }
                            setter.set_parameter(&params.loading, false);
                        }
                        if *load_bank.lock().unwrap() {
                            setter.set_parameter(&params.loading, true);
                            *reload_entire_preset.lock().unwrap() = true;
                            loading.store(true, Ordering::Relaxed);

                            // Move to info tab on preset change
                            *lfo_select.lock().unwrap() = LFOSelect::INFO;

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

                            (
                                *mod_source_override_1.lock().unwrap(),
                                *mod_source_override_2.lock().unwrap(),
                                *mod_source_override_3.lock().unwrap(),
                                *mod_source_override_4.lock().unwrap(),
                                *mod_dest_override_1.lock().unwrap(),
                                *mod_dest_override_2.lock().unwrap(),
                                *mod_dest_override_3.lock().unwrap(),
                                *mod_dest_override_4.lock().unwrap(),
                                *preset_category_override.lock().unwrap(),
                            ) = Actuate::reload_entire_preset(
                                setter,
                                params.clone(),
                                current_preset_index as usize,
                                arc_preset.clone(),
                                AM1.clone(),
                                AM2.clone(),
                                AM3.clone(),);
                            setter.set_parameter(&params.param_load_bank, false);
                            *load_bank.lock().unwrap() = false;
                            *reload_entire_preset.lock().unwrap() = false;
                            setter.set_parameter(&params.loading, false);
                        }
                        if *save_bank.lock().unwrap() {
                            setter.set_parameter(&params.param_save_bank, false);
                            *save_bank.lock().unwrap() = false;
                        }
                        if *export_preset.lock().unwrap() {
                            setter.set_parameter(&params.param_export_preset, false);
                            *export_preset.lock().unwrap() = false;
                        }
                        if *import_preset.lock().unwrap() {
                            setter.set_parameter(&params.loading, true);
                            *reload_entire_preset.lock().unwrap() = true;
                            loading.store(true, Ordering::Relaxed);
                            setter.set_parameter(&params.param_import_preset, false);
                            *import_preset.lock().unwrap() = false;

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

                            (
                                *mod_source_override_1.lock().unwrap(),
                                *mod_source_override_2.lock().unwrap(),
                                *mod_source_override_3.lock().unwrap(),
                                *mod_source_override_4.lock().unwrap(),
                                *mod_dest_override_1.lock().unwrap(),
                                *mod_dest_override_2.lock().unwrap(),
                                *mod_dest_override_3.lock().unwrap(),
                                *mod_dest_override_4.lock().unwrap(),
                                *preset_category_override.lock().unwrap(),
                            ) = Actuate::reload_entire_preset(
                                setter,
                                params.clone(),
                                current_preset_index as usize,
                                arc_preset.clone(),
                                AM1.clone(),
                                AM2.clone(),
                                AM3.clone(),);
                            setter.set_parameter(&params.param_load_bank, false);
                            *load_bank.lock().unwrap() = false;
                            *reload_entire_preset.lock().unwrap() = false;
                            setter.set_parameter(&params.loading, false);
                        }
                        if params.param_export_preset.value() {
                            setter.set_parameter(&params.param_export_preset, false);
                            *export_preset.lock().unwrap() = false;
                        }
                        if params.param_import_preset.value() {
                            setter.set_parameter(&params.param_import_preset, false);
                            *import_preset.lock().unwrap() = false;
                        }
                        // Extra checks for sanity
                        if params.param_load_bank.value() {
                            setter.set_parameter(&params.param_load_bank, false);
                        }
                        if params.param_save_bank.value() {
                            setter.set_parameter(&params.param_save_bank, false);
                        }
                        if update_current_preset.load(Ordering::Relaxed) || params.param_update_current_preset.value() {
                            setter.set_parameter(&params.param_update_current_preset, false);
                            update_current_preset.store(false, Ordering::Relaxed);
                        }
                        if params.filter_cutoff_link.value() {
                            setter.set_parameter(&params.filter_cutoff_2, params.filter_cutoff.value());
                        }

                        // Assign default colors
                        ui.style_mut().visuals.widgets.inactive.bg_stroke.color = A_KNOB_OUTSIDE_COLOR;
                        ui.style_mut().visuals.widgets.inactive.bg_fill = DARK_GREY_UI_COLOR;
                        ui.style_mut().visuals.widgets.active.fg_stroke.color = A_KNOB_OUTSIDE_COLOR;
                        ui.style_mut().visuals.widgets.active.bg_stroke.color = A_KNOB_OUTSIDE_COLOR;
                        ui.style_mut().visuals.widgets.open.fg_stroke.color = A_KNOB_OUTSIDE_COLOR;
                        ui.style_mut().visuals.widgets.open.bg_fill = DARK_GREY_UI_COLOR;
                        // Lettering on param sliders
                        ui.style_mut().visuals.widgets.inactive.fg_stroke.color = A_KNOB_OUTSIDE_COLOR;
                        // Background of the bar in param sliders
                        ui.style_mut().visuals.selection.bg_fill = A_KNOB_OUTSIDE_COLOR;
                        ui.style_mut().visuals.selection.stroke.color = A_KNOB_OUTSIDE_COLOR;
                        // Unfilled background of the bar
                        ui.style_mut().visuals.widgets.noninteractive.bg_fill = DARK_GREY_UI_COLOR;
                        // egui 0.20 to 0.22 changed this styling then I later decided proportional looks nice
                        //ui.style_mut().drag_value_text_style = egui::TextStyle::Monospace;

                        // Trying to draw background colors as rects
                        ui.painter().rect_filled(
                            Rect::from_x_y_ranges(
                                RangeInclusive::new(0.0, WIDTH as f32),
                                RangeInclusive::new(0.0, (HEIGHT as f32)*0.72)),
                            Rounding::from(16.0), A_BACKGROUND_COLOR_TOP);
                        ui.painter().rect_filled(
                            Rect::from_x_y_ranges(
                                RangeInclusive::new(0.0, WIDTH as f32),
                                RangeInclusive::new((HEIGHT as f32)*0.72, HEIGHT as f32)),
                            Rounding::from(16.0), DARK_GREY_UI_COLOR);

                        //ui.set_style(ui.style_mut());

                        ui.horizontal(|ui| {
                            // Synth Bars on left and right
                            let synth_bar_space = 32.0;
                            ui.painter().rect_filled(
                                Rect::from_x_y_ranges(
                                    RangeInclusive::new(0.0, synth_bar_space),
                                    RangeInclusive::new(0.0, HEIGHT as f32)),
                                Rounding::none(),
                                SYNTH_BARS_PURPLE
                            );

                            // Spacers for primary generator knobs
                            let generator_separator_length: f32 = 170.0;
                            ui.painter().rect_filled(
                                Rect::from_x_y_ranges(
                                    RangeInclusive::new(synth_bar_space + 4.0, synth_bar_space + generator_separator_length),
                                    RangeInclusive::new(192.0, 194.0)),
                                Rounding::none(),
                                LIGHTER_GREY_UI_COLOR
                            );

                            // Spacers for primary generator knobs
                            ui.painter().rect_filled(
                                Rect::from_x_y_ranges(
                                    RangeInclusive::new(synth_bar_space + 4.0, synth_bar_space + generator_separator_length),
                                    RangeInclusive::new(328.0, 330.0)),
                                Rounding::none(),
                                LIGHTER_GREY_UI_COLOR
                            );

                            ui.add_space(synth_bar_space);

                            // GUI Structure
                            ui.vertical(|ui| {
                                ui.horizontal(|ui|{
                                    ui.label(RichText::new("Actuate")
                                        .font(FONT)
                                        .color(FONT_COLOR))
                                        .on_hover_text("by Ardura!");
                                    ui.separator();
                                    ui.add(CustomParamSlider::ParamSlider::for_param(&params.master_level, setter)
                                        .slimmer(0.5)
                                        .set_left_sided_label(true)
                                        .set_label_width(70.0)
                                        .with_width(30.0));
                                    ui.separator();
                                    ui.add(CustomParamSlider::ParamSlider::for_param(&params.voice_limit, setter)
                                        .slimmer(0.5)
                                        .set_left_sided_label(true)
                                        .set_label_width(84.0)
                                        .with_width(30.0));
                                    ui.separator();
                                    ui.label(RichText::new("FX")
                                        .font(FONT)
                                        .background_color(A_BACKGROUND_COLOR_TOP)
                                        .color(FONT_COLOR)
                                    )
                                        .on_hover_text("Process FX");
                                    let use_fx_toggle = toggle_switch::ToggleSwitch::for_param(&params.use_fx, setter);
                                    ui.add(use_fx_toggle);
                                    ui.separator();
                                    let load_bank_button = BoolButton::BoolButton::for_param(&params.param_load_bank, setter, 3.5, 0.9, SMALLER_FONT)
                                        .with_background_color(DARK_GREY_UI_COLOR);
                                    ui.add(load_bank_button);
                                    let save_bank_button = BoolButton::BoolButton::for_param(&params.param_save_bank, setter, 3.5, 0.9, SMALLER_FONT)
                                        .with_background_color(DARK_GREY_UI_COLOR);
                                    ui.add(save_bank_button);

                                    let prev_preset_button = BoolButton::BoolButton::for_param(&params.param_prev_preset, setter, 1.5, 0.9, FONT)
                                        .with_background_color(DARK_GREY_UI_COLOR);
                                    ui.add(prev_preset_button);
                                    ui.label(RichText::new("Preset")
                                        .background_color(A_BACKGROUND_COLOR_TOP)
                                        .color(FONT_COLOR)
                                        .size(16.0));
                                    ui.label(RichText::new(current_preset_index.to_string())
                                        .background_color(A_BACKGROUND_COLOR_TOP)
                                        .color(FONT_COLOR)
                                        .size(16.0));
                                    let next_preset_button = BoolButton::BoolButton::for_param(&params.param_next_preset, setter, 1.5, 0.9, FONT)
                                        .with_background_color(DARK_GREY_UI_COLOR);
                                    ui.add(next_preset_button);

                                    ui.separator();
                                    ui.button(RichText::new("Browse Presets")
                                        .font(SMALLER_FONT)
                                        .background_color(DARK_GREY_UI_COLOR)
                                        .color(FONT_COLOR)
                                    ).on_hover_text("Coming soon!");
                                });
                                ui.separator();
                                const KNOB_SIZE: f32 = 32.0;
                                const TEXT_SIZE: f32 = 11.0;
                                ui.horizontal(|ui|{
                                    ui.vertical(|ui|{
                                        ui.label(RichText::new("Generators")
                                            .font(FONT)
                                            .color(FONT_COLOR))
                                            .on_hover_text("These are the audio modules that create sound on midi events");
                                        // Side knobs for types
                                        ui.horizontal(|ui|{
                                            let audio_module_1_knob = ui_knob::ArcKnob::for_param(
                                                &params._audio_module_1_type,
                                                setter,
                                                KNOB_SIZE)
                                                .preset_style(ui_knob::KnobStyle::NewPresets1)
                                                .set_fill_color(DARK_GREY_UI_COLOR)
                                                .set_line_color(SYNTH_MIDDLE_BLUE)
                                                .set_text_size(TEXT_SIZE);
                                            ui.add(audio_module_1_knob);
                                            let audio_module_1_level_knob = ui_knob::ArcKnob::for_param(
                                                &params.audio_module_1_level,
                                                setter,
                                                KNOB_SIZE)
                                                .preset_style(ui_knob::KnobStyle::NewPresets1)
                                                .set_fill_color(DARK_GREY_UI_COLOR)
                                                .set_line_color(SYNTH_MIDDLE_BLUE)
                                                .set_text_size(TEXT_SIZE);
                                            ui.add(audio_module_1_level_knob);
                                        });
                                        ui.horizontal(|ui|{
                                            let audio_module_1_filter_routing = ui_knob::ArcKnob::for_param(
                                                &params.audio_module_1_routing,
                                                setter,
                                                KNOB_SIZE)
                                                .preset_style(ui_knob::KnobStyle::NewPresets1)
                                                .set_fill_color(DARK_GREY_UI_COLOR)
                                                .set_line_color(SYNTH_MIDDLE_BLUE)
                                                .override_text_color(Color32::GRAY)
                                                .set_text_size(TEXT_SIZE);
                                            ui.add(audio_module_1_filter_routing);
                                        });

                                        ui.horizontal(|ui|{
                                            let audio_module_2_knob = ui_knob::ArcKnob::for_param(
                                                &params._audio_module_2_type,
                                                setter,
                                                KNOB_SIZE)
                                                .preset_style(ui_knob::KnobStyle::NewPresets1)
                                                .set_fill_color(DARK_GREY_UI_COLOR)
                                                .set_line_color(SYNTH_MIDDLE_BLUE)
                                                .set_text_size(TEXT_SIZE);
                                            ui.add(audio_module_2_knob);
                                            let audio_module_2_level_knob = ui_knob::ArcKnob::for_param(
                                                &params.audio_module_2_level,
                                                setter,
                                                KNOB_SIZE)
                                                .preset_style(ui_knob::KnobStyle::NewPresets1)
                                                .set_fill_color(DARK_GREY_UI_COLOR)
                                                .set_line_color(SYNTH_MIDDLE_BLUE)
                                                .set_text_size(TEXT_SIZE);
                                            ui.add(audio_module_2_level_knob);
                                        });
                                        ui.horizontal(|ui|{
                                            let audio_module_2_filter_routing = ui_knob::ArcKnob::for_param(
                                                &params.audio_module_2_routing,
                                                setter,
                                                KNOB_SIZE)
                                                .preset_style(ui_knob::KnobStyle::NewPresets1)
                                                .set_fill_color(DARK_GREY_UI_COLOR)
                                                .set_line_color(SYNTH_MIDDLE_BLUE)
                                                .set_text_size(TEXT_SIZE);
                                            ui.add(audio_module_2_filter_routing);
                                        });

                                        ui.horizontal(|ui| {
                                            let audio_module_3_knob = ui_knob::ArcKnob::for_param(
                                                &params._audio_module_3_type,
                                                setter,
                                                KNOB_SIZE)
                                                .preset_style(ui_knob::KnobStyle::NewPresets1)
                                                .set_fill_color(DARK_GREY_UI_COLOR)
                                                .set_line_color(SYNTH_MIDDLE_BLUE)
                                                .set_text_size(TEXT_SIZE);
                                            ui.add(audio_module_3_knob);
                                            let audio_module_3_level_knob = ui_knob::ArcKnob::for_param(
                                                &params.audio_module_3_level,
                                                setter,
                                                KNOB_SIZE)
                                                .preset_style(ui_knob::KnobStyle::NewPresets1)
                                                .set_fill_color(DARK_GREY_UI_COLOR)
                                                .set_line_color(SYNTH_MIDDLE_BLUE)
                                                .set_text_size(TEXT_SIZE);
                                            ui.add(audio_module_3_level_knob);
                                        });
                                        ui.horizontal(|ui|{
                                            let audio_module_3_filter_routing = ui_knob::ArcKnob::for_param(
                                                &params.audio_module_3_routing,
                                                setter,
                                                KNOB_SIZE)
                                                .preset_style(ui_knob::KnobStyle::NewPresets1)
                                                .set_fill_color(DARK_GREY_UI_COLOR)
                                                .set_line_color(SYNTH_MIDDLE_BLUE)
                                                .set_text_size(TEXT_SIZE);
                                            ui.add(audio_module_3_filter_routing);
                                        });
                                    });

                                    ui.separator();
                                    ui.vertical(|ui|{
                                        ui.label(RichText::new("Generator Controls")
                                            .font(SMALLER_FONT)
                                            .color(FONT_COLOR))
                                            .on_hover_text("These are the controls for the active/selected generators");
                                        audio_module::AudioModule::draw_modules(ui, params.clone(), setter);
                                    });
                                });
                                //ui.add_space(32.0);
                                ui.label("Filters and Modulations");

                                // Filter section

                                ui.horizontal(|ui| {
                                    ui.vertical(|ui|{
                                        ui.horizontal(|ui|{
                                            match *filter_select.lock().unwrap() {
                                                UIBottomSelection::Filter1 => {
                                                    match params.filter_alg_type.value() {
                                                        FilterAlgorithms::SVF => {
                                                            ui.vertical(|ui|{
                                                                let filter_wet_knob = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_wet,
                                                                    setter,
                                                                    KNOB_SIZE)
                                                                    .preset_style(ui_knob::KnobStyle::NewPresets1)
                                                                    .set_fill_color(SYNTH_BARS_PURPLE)
                                                                    .set_line_color(SYNTH_MIDDLE_BLUE)
                                                                    .set_text_size(TEXT_SIZE);
                                                                ui.add(filter_wet_knob);
                                                                let filter_resonance_knob = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_resonance,
                                                                    setter,
                                                                    KNOB_SIZE)
                                                                    .preset_style(ui_knob::KnobStyle::NewPresets1)
                                                                    .set_fill_color(SYNTH_BARS_PURPLE)
                                                                    .set_line_color(SYNTH_MIDDLE_BLUE)
                                                                    .set_text_size(TEXT_SIZE);
                                                                ui.add(filter_resonance_knob);
                                                            });
                                                            ui.vertical(|ui|{
                                                                let filter_cutoff_knob = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_cutoff,
                                                                    setter,
                                                                    KNOB_SIZE)
                                                                    .preset_style(ui_knob::KnobStyle::NewPresets1)
                                                                    .set_fill_color(SYNTH_BARS_PURPLE)
                                                                    .set_line_color(SYNTH_MIDDLE_BLUE)
                                                                    .set_text_size(TEXT_SIZE);
                                                                ui.add(filter_cutoff_knob);
                                                                let filter_res_type_knob = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_res_type,
                                                                    setter,
                                                                    KNOB_SIZE)
                                                                    .preset_style(ui_knob::KnobStyle::NewPresets1)
                                                                    .set_fill_color(SYNTH_BARS_PURPLE)
                                                                    .set_line_color(SYNTH_MIDDLE_BLUE)
                                                                    .set_text_size(TEXT_SIZE);
                                                                ui.add(filter_res_type_knob);
                                                            });
                                                            ui.vertical(|ui|{
                                                                let filter_hp_knob = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_hp_amount,
                                                                    setter,
                                                                    KNOB_SIZE)
                                                                    .preset_style(ui_knob::KnobStyle::NewPresets1)
                                                                    .set_fill_color(SYNTH_BARS_PURPLE)
                                                                    .set_line_color(SYNTH_MIDDLE_BLUE)
                                                                    .set_text_size(TEXT_SIZE);
                                                                ui.add(filter_hp_knob);
                                                                let filter_env_peak = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_env_peak,
                                                                    setter,
                                                                    KNOB_SIZE)
                                                                    .preset_style(ui_knob::KnobStyle::NewPresets1)
                                                                    .set_fill_color(SYNTH_BARS_PURPLE)
                                                                    .set_line_color(SYNTH_MIDDLE_BLUE)
                                                                    .set_readable_box(false)
                                                                    .set_text_size(TEXT_SIZE);
                                                                ui.add(filter_env_peak);
                                                            });
                                                            ui.vertical(|ui| {
                                                                let filter_lp_knob = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_lp_amount,
                                                                    setter,
                                                                    KNOB_SIZE)
                                                                    .preset_style(ui_knob::KnobStyle::NewPresets1)
                                                                    .set_fill_color(SYNTH_BARS_PURPLE)
                                                                    .set_line_color(SYNTH_MIDDLE_BLUE)
                                                                    .set_text_size(TEXT_SIZE);
                                                                ui.add(filter_lp_knob);
                                                                let filter_bp_knob = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_bp_amount,
                                                                    setter,
                                                                    KNOB_SIZE)
                                                                    .preset_style(ui_knob::KnobStyle::NewPresets1)
                                                                    .set_fill_color(SYNTH_BARS_PURPLE)
                                                                    .set_line_color(SYNTH_MIDDLE_BLUE)
                                                                    .set_text_size(TEXT_SIZE);
                                                                ui.add(filter_bp_knob);
                                                            });
                                                        },
                                                        FilterAlgorithms::TILT => {
                                                            ui.vertical(|ui|{
                                                                let filter_wet_knob = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_wet,
                                                                    setter,
                                                                    KNOB_SIZE)
                                                                    .preset_style(ui_knob::KnobStyle::NewPresets1)
                                                                    .set_fill_color(SYNTH_BARS_PURPLE)
                                                                    .set_line_color(SYNTH_MIDDLE_BLUE)
                                                                    .set_text_size(TEXT_SIZE);
                                                                ui.add(filter_wet_knob);
                                                                let filter_resonance_knob = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_resonance,
                                                                    setter,
                                                                    KNOB_SIZE)
                                                                    .preset_style(ui_knob::KnobStyle::NewPresets1)
                                                                    .set_fill_color(SYNTH_BARS_PURPLE)
                                                                    .set_line_color(SYNTH_MIDDLE_BLUE)
                                                                    .set_text_size(TEXT_SIZE);
                                                                ui.add(filter_resonance_knob);
                                                            });
                                                            ui.vertical(|ui|{
                                                                let filter_cutoff_knob = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_cutoff,
                                                                    setter,
                                                                    KNOB_SIZE)
                                                                    .preset_style(ui_knob::KnobStyle::NewPresets1)
                                                                    .set_fill_color(SYNTH_BARS_PURPLE)
                                                                    .set_line_color(SYNTH_MIDDLE_BLUE)
                                                                    .set_text_size(TEXT_SIZE);
                                                                ui.add(filter_cutoff_knob);
                                                                let filter_tilt_type_knob = ui_knob::ArcKnob::for_param(
                                                                    &params.tilt_filter_type,
                                                                    setter,
                                                                    KNOB_SIZE)
                                                                    .preset_style(ui_knob::KnobStyle::NewPresets1)
                                                                    .set_fill_color(SYNTH_BARS_PURPLE)
                                                                    .set_line_color(SYNTH_MIDDLE_BLUE)
                                                                    .set_text_size(TEXT_SIZE);
                                                                ui.add(filter_tilt_type_knob);
                                                            });
                                                            ui.vertical(|ui|{
                                                                let filter_env_peak = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_env_peak,
                                                                    setter,
                                                                    KNOB_SIZE)
                                                                    .preset_style(ui_knob::KnobStyle::NewPresets1)
                                                                    .set_fill_color(SYNTH_BARS_PURPLE)
                                                                    .set_line_color(SYNTH_MIDDLE_BLUE)
                                                                    .set_readable_box(false)
                                                                    .set_text_size(TEXT_SIZE);
                                                                ui.add(filter_env_peak);
                                                            });
                                                            ui.add_space(KNOB_SIZE*2.0);
                                                        },
                                                        FilterAlgorithms::VCF => {
                                                            ui.vertical(|ui|{
                                                                let filter_wet_knob = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_wet,
                                                                    setter,
                                                                    KNOB_SIZE)
                                                                    .preset_style(ui_knob::KnobStyle::NewPresets1)
                                                                    .set_fill_color(SYNTH_BARS_PURPLE)
                                                                    .set_line_color(SYNTH_MIDDLE_BLUE)
                                                                    .set_text_size(TEXT_SIZE);
                                                                ui.add(filter_wet_knob);
                                                                let filter_resonance_knob = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_resonance,
                                                                    setter,
                                                                    KNOB_SIZE)
                                                                    .preset_style(ui_knob::KnobStyle::NewPresets1)
                                                                    .set_fill_color(SYNTH_BARS_PURPLE)
                                                                    .set_line_color(SYNTH_MIDDLE_BLUE)
                                                                    .set_text_size(TEXT_SIZE);
                                                                ui.add(filter_resonance_knob);
                                                            });
                                                            ui.vertical(|ui|{
                                                                let filter_cutoff_knob = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_cutoff,
                                                                    setter,
                                                                    KNOB_SIZE)
                                                                    .preset_style(ui_knob::KnobStyle::NewPresets1)
                                                                    .set_fill_color(SYNTH_BARS_PURPLE)
                                                                    .set_line_color(SYNTH_MIDDLE_BLUE)
                                                                    .set_text_size(TEXT_SIZE);
                                                                ui.add(filter_cutoff_knob);
                                                                let vcf_filter_type_knob = ui_knob::ArcKnob::for_param(
                                                                    &params.vcf_filter_type,
                                                                    setter,
                                                                    KNOB_SIZE)
                                                                    .preset_style(ui_knob::KnobStyle::NewPresets1)
                                                                    .set_fill_color(SYNTH_BARS_PURPLE)
                                                                    .set_line_color(SYNTH_MIDDLE_BLUE)
                                                                    .set_text_size(TEXT_SIZE);
                                                                ui.add(vcf_filter_type_knob);
                                                            });
                                                            ui.vertical(|ui|{
                                                                let filter_env_peak = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_env_peak,
                                                                    setter,
                                                                    KNOB_SIZE)
                                                                    .preset_style(ui_knob::KnobStyle::NewPresets1)
                                                                    .set_fill_color(SYNTH_BARS_PURPLE)
                                                                    .set_line_color(SYNTH_MIDDLE_BLUE)
                                                                    .set_readable_box(false)
                                                                    .set_text_size(TEXT_SIZE);
                                                                ui.add(filter_env_peak);
                                                            });
                                                            ui.add_space(KNOB_SIZE*2.0);
                                                        },
                                                    }

                                                    // Middle bottom light section
                                                    ui.painter().rect_filled(
                                                        Rect::from_x_y_ranges(
                                                            RangeInclusive::new((WIDTH as f32)*0.35, (WIDTH as f32)*0.64),
                                                            RangeInclusive::new((HEIGHT as f32)*0.73, (HEIGHT as f32) - 4.0)),
                                                        Rounding::from(16.0),
                                                        SYNTH_SOFT_BLUE
                                                    );
                                                    // Middle Bottom Filter select background
                                                    ui.painter().rect_filled(
                                                        Rect::from_x_y_ranges(
                                                            RangeInclusive::new((WIDTH as f32)*0.38, (WIDTH as f32)*0.62),
                                                            RangeInclusive::new((HEIGHT as f32) - 26.0, (HEIGHT as f32) - 2.0)),
                                                        Rounding::from(16.0),
                                                        A_BACKGROUND_COLOR_TOP
                                                    );
                                                },
                                                UIBottomSelection::Filter2 => {
                                                    match params.filter_alg_type_2.value() {
                                                        FilterAlgorithms::SVF => {
                                                            ui.vertical(|ui|{
                                                                let filter_wet_knob = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_wet_2,
                                                                    setter,
                                                                    KNOB_SIZE)
                                                                    .preset_style(ui_knob::KnobStyle::NewPresets1)
                                                                    .set_fill_color(SYNTH_BARS_PURPLE)
                                                                    .set_line_color(A_KNOB_OUTSIDE_COLOR)
                                                                    .set_text_size(TEXT_SIZE);
                                                                ui.add(filter_wet_knob);

                                                                let filter_resonance_knob = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_resonance_2,
                                                                    setter,
                                                                    KNOB_SIZE)
                                                                    .preset_style(ui_knob::KnobStyle::NewPresets1)
                                                                    .set_fill_color(SYNTH_BARS_PURPLE)
                                                                    .set_line_color(A_KNOB_OUTSIDE_COLOR)
                                                                    .set_text_size(TEXT_SIZE);
                                                                ui.add(filter_resonance_knob);
                                                            });
                                                            ui.vertical(|ui|{
                                                                let filter_cutoff_knob = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_cutoff_2,
                                                                    setter,
                                                                    KNOB_SIZE)
                                                                    .preset_style(ui_knob::KnobStyle::NewPresets1)
                                                                    .set_fill_color(SYNTH_BARS_PURPLE)
                                                                    .set_line_color(A_KNOB_OUTSIDE_COLOR)
                                                                    .set_text_size(TEXT_SIZE);
                                                                ui.add(filter_cutoff_knob);

                                                                let filter_res_type_knob = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_res_type_2,
                                                                    setter,
                                                                    KNOB_SIZE)
                                                                    .preset_style(ui_knob::KnobStyle::NewPresets1)
                                                                    .set_fill_color(SYNTH_BARS_PURPLE)
                                                                    .set_line_color(A_KNOB_OUTSIDE_COLOR)
                                                                    .set_text_size(TEXT_SIZE);
                                                                ui.add(filter_res_type_knob);
                                                            });
                                                            ui.vertical(|ui|{
                                                                let filter_hp_knob = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_hp_amount_2,
                                                                    setter,
                                                                    KNOB_SIZE)
                                                                    .preset_style(ui_knob::KnobStyle::NewPresets1)
                                                                    .set_fill_color(SYNTH_BARS_PURPLE)
                                                                    .set_line_color(A_KNOB_OUTSIDE_COLOR)
                                                                    .set_text_size(TEXT_SIZE);
                                                                ui.add(filter_hp_knob);
                                                                let filter_env_peak = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_env_peak_2,
                                                                    setter,
                                                                    KNOB_SIZE)
                                                                    .preset_style(ui_knob::KnobStyle::NewPresets1)
                                                                    .set_fill_color(SYNTH_BARS_PURPLE)
                                                                    .set_line_color(A_KNOB_OUTSIDE_COLOR)
                                                                    .set_readable_box(false)
                                                                    .set_text_size(TEXT_SIZE);
                                                                ui.add(filter_env_peak);
                                                            });
                                                            ui.vertical(|ui| {
                                                                let filter_lp_knob = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_lp_amount_2,
                                                                    setter,
                                                                    KNOB_SIZE)
                                                                    .preset_style(ui_knob::KnobStyle::NewPresets1)
                                                                    .set_fill_color(SYNTH_BARS_PURPLE)
                                                                    .set_line_color(A_KNOB_OUTSIDE_COLOR)
                                                                    .set_text_size(TEXT_SIZE);
                                                                ui.add(filter_lp_knob);
                                                                let filter_bp_knob = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_bp_amount_2,
                                                                    setter,
                                                                    KNOB_SIZE)
                                                                    .preset_style(ui_knob::KnobStyle::NewPresets1)
                                                                    .set_fill_color(SYNTH_BARS_PURPLE)
                                                                    .set_line_color(A_KNOB_OUTSIDE_COLOR)
                                                                    .set_text_size(TEXT_SIZE);
                                                                ui.add(filter_bp_knob);
                                                            });
                                                        },
                                                        FilterAlgorithms::TILT => {
                                                            ui.vertical(|ui|{
                                                                let filter_wet_knob = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_wet_2,
                                                                    setter,
                                                                    KNOB_SIZE)
                                                                    .preset_style(ui_knob::KnobStyle::NewPresets1)
                                                                    .set_fill_color(SYNTH_BARS_PURPLE)
                                                                    .set_line_color(A_KNOB_OUTSIDE_COLOR)
                                                                    .set_text_size(TEXT_SIZE);
                                                                ui.add(filter_wet_knob);
                                                                let filter_resonance_knob = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_resonance_2,
                                                                    setter,
                                                                    KNOB_SIZE)
                                                                    .preset_style(ui_knob::KnobStyle::NewPresets1)
                                                                    .set_fill_color(SYNTH_BARS_PURPLE)
                                                                    .set_line_color(A_KNOB_OUTSIDE_COLOR)
                                                                    .set_text_size(TEXT_SIZE);
                                                                ui.add(filter_resonance_knob);
                                                            });
                                                            ui.vertical(|ui|{
                                                                let filter_cutoff_knob = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_cutoff_2,
                                                                    setter,
                                                                    KNOB_SIZE)
                                                                    .preset_style(ui_knob::KnobStyle::NewPresets1)
                                                                    .set_fill_color(SYNTH_BARS_PURPLE)
                                                                    .set_line_color(A_KNOB_OUTSIDE_COLOR)
                                                                    .set_text_size(TEXT_SIZE);
                                                                ui.add(filter_cutoff_knob);
                                                                let filter_tilt_type_knob = ui_knob::ArcKnob::for_param(
                                                                    &params.tilt_filter_type_2,
                                                                    setter,
                                                                    KNOB_SIZE)
                                                                    .preset_style(ui_knob::KnobStyle::NewPresets1)
                                                                    .set_fill_color(SYNTH_BARS_PURPLE)
                                                                    .set_line_color(A_KNOB_OUTSIDE_COLOR)
                                                                    .set_text_size(TEXT_SIZE);
                                                                ui.add(filter_tilt_type_knob);
                                                            });
                                                            ui.vertical(|ui|{
                                                                let filter_env_peak = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_env_peak_2,
                                                                    setter,
                                                                    KNOB_SIZE)
                                                                    .preset_style(ui_knob::KnobStyle::NewPresets1)
                                                                    .set_fill_color(SYNTH_BARS_PURPLE)
                                                                    .set_line_color(A_KNOB_OUTSIDE_COLOR)
                                                                    .set_readable_box(false)
                                                                    .set_text_size(TEXT_SIZE);
                                                                ui.add(filter_env_peak);
                                                            });
                                                            ui.add_space(KNOB_SIZE*2.0);
                                                        },
                                                        FilterAlgorithms::VCF => {
                                                            ui.vertical(|ui|{
                                                                let filter_wet_knob = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_wet_2,
                                                                    setter,
                                                                    KNOB_SIZE)
                                                                    .preset_style(ui_knob::KnobStyle::NewPresets1)
                                                                    .set_fill_color(SYNTH_BARS_PURPLE)
                                                                    .set_line_color(A_KNOB_OUTSIDE_COLOR)
                                                                    .set_text_size(TEXT_SIZE);
                                                                ui.add(filter_wet_knob);
                                                                let filter_resonance_knob = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_resonance_2,
                                                                    setter,
                                                                    KNOB_SIZE)
                                                                    .preset_style(ui_knob::KnobStyle::NewPresets1)
                                                                    .set_fill_color(SYNTH_BARS_PURPLE)
                                                                    .set_line_color(A_KNOB_OUTSIDE_COLOR)
                                                                    .set_text_size(TEXT_SIZE);
                                                                ui.add(filter_resonance_knob);
                                                            });
                                                            ui.vertical(|ui|{
                                                                let filter_cutoff_knob = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_cutoff_2,
                                                                    setter,
                                                                    KNOB_SIZE)
                                                                    .preset_style(ui_knob::KnobStyle::NewPresets1)
                                                                    .set_fill_color(SYNTH_BARS_PURPLE)
                                                                    .set_line_color(A_KNOB_OUTSIDE_COLOR)
                                                                    .set_text_size(TEXT_SIZE);
                                                                ui.add(filter_cutoff_knob);
                                                                let vcf_filter_type_knob = ui_knob::ArcKnob::for_param(
                                                                    &params.vcf_filter_type_2,
                                                                    setter,
                                                                    KNOB_SIZE)
                                                                    .preset_style(ui_knob::KnobStyle::NewPresets1)
                                                                    .set_fill_color(SYNTH_BARS_PURPLE)
                                                                    .set_line_color(A_KNOB_OUTSIDE_COLOR)
                                                                    .set_text_size(TEXT_SIZE);
                                                                ui.add(vcf_filter_type_knob);
                                                            });
                                                            ui.vertical(|ui|{
                                                                let filter_env_peak = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_env_peak_2,
                                                                    setter,
                                                                    KNOB_SIZE)
                                                                    .preset_style(ui_knob::KnobStyle::NewPresets1)
                                                                    .set_fill_color(SYNTH_BARS_PURPLE)
                                                                    .set_line_color(A_KNOB_OUTSIDE_COLOR)
                                                                    .set_readable_box(false)
                                                                    .set_text_size(TEXT_SIZE);
                                                                ui.add(filter_env_peak);
                                                            });
                                                            ui.add_space(KNOB_SIZE*2.0);
                                                        },
                                                    }
                                                    // Middle bottom light section
                                                    ui.painter().rect_filled(
                                                        Rect::from_x_y_ranges(
                                                            RangeInclusive::new((WIDTH as f32)*0.35, (WIDTH as f32)*0.64),
                                                            RangeInclusive::new((HEIGHT as f32)*0.73, (HEIGHT as f32) - 4.0)),
                                                        Rounding::from(16.0),
                                                        SYNTH_SOFT_BLUE2
                                                    );
                                                    // Middle Bottom Filter select background
                                                    ui.painter().rect_filled(
                                                        Rect::from_x_y_ranges(
                                                            RangeInclusive::new((WIDTH as f32)*0.38, (WIDTH as f32)*0.62),
                                                            RangeInclusive::new((HEIGHT as f32) - 26.0, (HEIGHT as f32) - 2.0)),
                                                        Rounding::from(16.0),
                                                        A_BACKGROUND_COLOR_TOP
                                                    );
                                                },
                                                UIBottomSelection::Pitch => {
                                                    ui.vertical(|ui|{
                                                        ui.horizontal(|ui|{
                                                            let pitch_toggle = toggle_switch::ToggleSwitch::for_param(&params.pitch_enable, setter);
                                                            ui.add(pitch_toggle);
                                                            ui.label(RichText::new("Enable Pitch Envelope")
                                                                .font(SMALLER_FONT)
                                                                .color(A_BACKGROUND_COLOR_TOP)
                                                            );
                                                        });

                                                        ui.horizontal(|ui|{
                                                            let pitch_env_peak_knob = ui_knob::ArcKnob::for_param(
                                                                &params.pitch_env_peak,
                                                                setter,
                                                                KNOB_SIZE)
                                                                .preset_style(ui_knob::KnobStyle::NewPresets1)
                                                                .set_fill_color(SYNTH_BARS_PURPLE)
                                                                .set_line_color(A_KNOB_OUTSIDE_COLOR)
                                                                .set_readable_box(false)
                                                                .set_text_size(TEXT_SIZE);
                                                            ui.add(pitch_env_peak_knob);

                                                            let pitch_routing_knob = ui_knob::ArcKnob::for_param(
                                                                &params.pitch_routing,
                                                                setter,
                                                                KNOB_SIZE)
                                                                .preset_style(ui_knob::KnobStyle::NewPresets1)
                                                                .set_fill_color(SYNTH_BARS_PURPLE)
                                                                .set_line_color(A_KNOB_OUTSIDE_COLOR)
                                                                .set_readable_box(false)
                                                                .set_text_size(TEXT_SIZE);
                                                            ui.add(pitch_routing_knob);
                                                        });
                                                    });
                                                    ui.add_space(KNOB_SIZE*3.5);
                                                    ui.add_space(8.0);

                                                    // Middle bottom light section
                                                    ui.painter().rect_filled(
                                                        Rect::from_x_y_ranges(
                                                            RangeInclusive::new((WIDTH as f32)*0.35, (WIDTH as f32)*0.64),
                                                            RangeInclusive::new((HEIGHT as f32)*0.73, (HEIGHT as f32) - 4.0)),
                                                        Rounding::from(16.0),
                                                        SYNTH_MIDDLE_BLUE
                                                    );
                                                    // Middle Bottom Filter select background
                                                    ui.painter().rect_filled(
                                                        Rect::from_x_y_ranges(
                                                            RangeInclusive::new((WIDTH as f32)*0.38, (WIDTH as f32)*0.62),
                                                            RangeInclusive::new((HEIGHT as f32) - 26.0, (HEIGHT as f32) - 2.0)),
                                                        Rounding::from(16.0),
                                                        A_BACKGROUND_COLOR_TOP
                                                    );
                                                }
                                            }
                                        });
                                    });

                                    ////////////////////////////////////////////////////////////
                                    // ADSR FOR FILTER
                                    const VERT_BAR_HEIGHT: f32 = 106.0;
                                    const VERT_BAR_WIDTH: f32 = 14.0;
                                    const HCURVE_WIDTH: f32 = 120.0;
                                    const HCURVE_BWIDTH: f32 = 28.0;
                                    ui.vertical(|ui|{
                                        ui.horizontal(|ui|{
                                            match *filter_select.lock().unwrap() {
                                                UIBottomSelection::Filter1 => {
                                                    // ADSR
                                                    ui.add(
                                                        VerticalParamSlider::for_param(&params.filter_env_attack, setter)
                                                            .with_width(VERT_BAR_WIDTH)
                                                            .with_height(VERT_BAR_HEIGHT)
                                                            .set_reversed(true)
                                                            .override_colors(
                                                                SYNTH_BARS_PURPLE,
                                                                SYNTH_MIDDLE_BLUE,
                                                            ),
                                                    );
                                                    ui.add(
                                                        VerticalParamSlider::for_param(&params.filter_env_decay, setter)
                                                            .with_width(VERT_BAR_WIDTH)
                                                            .with_height(VERT_BAR_HEIGHT)
                                                            .set_reversed(true)
                                                            .override_colors(
                                                                SYNTH_BARS_PURPLE,
                                                                SYNTH_MIDDLE_BLUE,
                                                            ),
                                                    );
                                                    ui.add(
                                                        VerticalParamSlider::for_param(&params.filter_env_sustain, setter)
                                                            .with_width(VERT_BAR_WIDTH)
                                                            .with_height(VERT_BAR_HEIGHT)
                                                            .set_reversed(true)
                                                            .override_colors(
                                                                SYNTH_BARS_PURPLE,
                                                                SYNTH_MIDDLE_BLUE,
                                                            ),
                                                    );
                                                    ui.add(
                                                        VerticalParamSlider::for_param(&params.filter_env_release, setter)
                                                            .with_width(VERT_BAR_WIDTH)
                                                            .with_height(VERT_BAR_HEIGHT)
                                                            .set_reversed(true)
                                                            .override_colors(
                                                                SYNTH_BARS_PURPLE,
                                                                SYNTH_MIDDLE_BLUE,
                                                            ),
                                                    );
                                                },
                                                UIBottomSelection::Filter2 => {
                                                    // ADSR
                                                    ui.add(
                                                        VerticalParamSlider::for_param(&params.filter_env_attack_2, setter)
                                                            .with_width(VERT_BAR_WIDTH)
                                                            .with_height(VERT_BAR_HEIGHT)
                                                            .set_reversed(true)
                                                            .override_colors(
                                                                SYNTH_BARS_PURPLE,
                                                                A_KNOB_OUTSIDE_COLOR,
                                                            ),
                                                    );
                                                    ui.add(
                                                        VerticalParamSlider::for_param(&params.filter_env_decay_2, setter)
                                                            .with_width(VERT_BAR_WIDTH)
                                                            .with_height(VERT_BAR_HEIGHT)
                                                            .set_reversed(true)
                                                            .override_colors(
                                                                SYNTH_BARS_PURPLE,
                                                                A_KNOB_OUTSIDE_COLOR,
                                                            ),
                                                    );
                                                    ui.add(
                                                        VerticalParamSlider::for_param(&params.filter_env_sustain_2, setter)
                                                            .with_width(VERT_BAR_WIDTH)
                                                            .with_height(VERT_BAR_HEIGHT)
                                                            .set_reversed(true)
                                                            .override_colors(
                                                                SYNTH_BARS_PURPLE,
                                                                A_KNOB_OUTSIDE_COLOR,
                                                            ),
                                                    );
                                                    ui.add(
                                                        VerticalParamSlider::for_param(&params.filter_env_release_2, setter)
                                                            .with_width(VERT_BAR_WIDTH)
                                                            .with_height(VERT_BAR_HEIGHT)
                                                            .set_reversed(true)
                                                            .override_colors(
                                                                SYNTH_BARS_PURPLE,
                                                                A_KNOB_OUTSIDE_COLOR,
                                                            ),
                                                    );
                                                },
                                                UIBottomSelection::Pitch => {
                                                    // ADSR
                                                    ui.add(
                                                        VerticalParamSlider::for_param(&params.pitch_env_attack, setter)
                                                            .with_width(VERT_BAR_WIDTH)
                                                            .with_height(VERT_BAR_HEIGHT)
                                                            .set_reversed(true)
                                                            .override_colors(
                                                                SYNTH_BARS_PURPLE,
                                                                SYNTH_SOFT_BLUE2,
                                                            ),
                                                    );
                                                    ui.add(
                                                        VerticalParamSlider::for_param(&params.pitch_env_decay, setter)
                                                            .with_width(VERT_BAR_WIDTH)
                                                            .with_height(VERT_BAR_HEIGHT)
                                                            .set_reversed(true)
                                                            .override_colors(
                                                                SYNTH_BARS_PURPLE,
                                                                SYNTH_SOFT_BLUE2,
                                                            ),
                                                    );
                                                    ui.add(
                                                        VerticalParamSlider::for_param(&params.pitch_env_sustain, setter)
                                                            .with_width(VERT_BAR_WIDTH)
                                                            .with_height(VERT_BAR_HEIGHT)
                                                            .set_reversed(true)
                                                            .override_colors(
                                                                SYNTH_BARS_PURPLE,
                                                                SYNTH_SOFT_BLUE2,
                                                            ),
                                                    );
                                                    ui.add(
                                                        VerticalParamSlider::for_param(&params.pitch_env_release, setter)
                                                            .with_width(VERT_BAR_WIDTH)
                                                            .with_height(VERT_BAR_HEIGHT)
                                                            .set_reversed(true)
                                                            .override_colors(
                                                                SYNTH_BARS_PURPLE,
                                                                SYNTH_SOFT_BLUE2,
                                                            ),
                                                    );
                                                }
                                            }

                                            // Curve sliders
                                            ui.vertical(|ui| {
                                                match *filter_select.lock().unwrap() {
                                                    UIBottomSelection::Filter1 => {
                                                        ui.add(
                                                            HorizontalParamSlider::for_param(&params.filter_env_atk_curve, setter)
                                                                .with_width(HCURVE_BWIDTH)
                                                                .slimmer(0.7)
                                                                .set_left_sided_label(true)
                                                                .set_label_width(HCURVE_WIDTH)
                                                                .override_colors(
                                                                    DARK_GREY_UI_COLOR,
                                                                    SYNTH_MIDDLE_BLUE),
                                                        );
                                                        ui.add(
                                                            HorizontalParamSlider::for_param(&params.filter_env_dec_curve, setter)
                                                                .with_width(HCURVE_BWIDTH)
                                                                .slimmer(0.7)
                                                                .set_left_sided_label(true)
                                                                .set_label_width(HCURVE_WIDTH)
                                                                .override_colors(
                                                                    DARK_GREY_UI_COLOR,
                                                                    SYNTH_MIDDLE_BLUE),
                                                        );
                                                        ui.add(
                                                            HorizontalParamSlider::for_param(&params.filter_env_rel_curve, setter)
                                                                .with_width(HCURVE_BWIDTH)
                                                                .slimmer(0.7)
                                                                .set_left_sided_label(true)
                                                                .set_label_width(HCURVE_WIDTH)
                                                                .override_colors(
                                                                    DARK_GREY_UI_COLOR,
                                                                    SYNTH_MIDDLE_BLUE),
                                                        );
                                                        ui.add(CustomParamSlider::ParamSlider::for_param(&params.filter_alg_type, setter)
                                                            .slimmer(0.4)
                                                            .set_left_sided_label(true)
                                                            .set_label_width(120.0)
                                                            .override_colors(Color32::WHITE, Color32::BLACK)
                                                            .with_width(30.0)
                                                        );
                                                        ui.add(CustomParamSlider::ParamSlider::for_param(&params.filter_alg_type_2, setter)
                                                                .slimmer(0.4)
                                                                .set_left_sided_label(true)
                                                                .set_label_width(120.0)
                                                                .override_colors(Color32::WHITE, Color32::BLACK)
                                                                .with_width(30.0)
                                                            );
                                                        ui.add(CustomParamSlider::ParamSlider::for_param(&params.filter_routing, setter)
                                                                .slimmer(0.4)
                                                                .set_left_sided_label(true)
                                                                .set_label_width(120.0)
                                                                .override_colors(Color32::WHITE, Color32::BLACK)
                                                                .with_width(30.0)
                                                            );
                                                    },
                                                    UIBottomSelection::Filter2 => {
                                                        ui.add(
                                                            HorizontalParamSlider::for_param(&params.filter_env_atk_curve_2, setter)
                                                                .with_width(HCURVE_BWIDTH)
                                                                .slimmer(0.7)
                                                                .set_left_sided_label(true)
                                                                .set_label_width(HCURVE_WIDTH)
                                                                .override_colors(
                                                                    SYNTH_BARS_PURPLE,
                                                                    A_KNOB_OUTSIDE_COLOR),
                                                        );
                                                        ui.add(
                                                            HorizontalParamSlider::for_param(&params.filter_env_dec_curve_2, setter)
                                                                .with_width(HCURVE_BWIDTH)
                                                                .slimmer(0.7)
                                                                .set_left_sided_label(true)
                                                                .set_label_width(HCURVE_WIDTH)
                                                                .override_colors(
                                                                    SYNTH_BARS_PURPLE,
                                                                    A_KNOB_OUTSIDE_COLOR),
                                                        );
                                                        ui.add(
                                                            HorizontalParamSlider::for_param(&params.filter_env_rel_curve_2, setter)
                                                                .with_width(HCURVE_BWIDTH)
                                                                .slimmer(0.7)
                                                                .set_left_sided_label(true)
                                                                .set_label_width(HCURVE_WIDTH)
                                                                .override_colors(
                                                                    SYNTH_BARS_PURPLE,
                                                                    A_KNOB_OUTSIDE_COLOR),
                                                        );
                                                        ui.add(CustomParamSlider::ParamSlider::for_param(&params.filter_alg_type, setter)
                                                            .slimmer(0.4)
                                                            .set_left_sided_label(true)
                                                            .set_label_width(120.0)
                                                            .override_colors(Color32::WHITE, Color32::BLACK)
                                                            .with_width(30.0)
                                                        );
                                                        ui.add(CustomParamSlider::ParamSlider::for_param(&params.filter_alg_type_2, setter)
                                                                .slimmer(0.4)
                                                                .set_left_sided_label(true)
                                                                .set_label_width(120.0)
                                                                .override_colors(Color32::WHITE, Color32::BLACK)
                                                                .with_width(30.0)
                                                            );
                                                        ui.add(CustomParamSlider::ParamSlider::for_param(&params.filter_routing, setter)
                                                                .slimmer(0.4)
                                                                .set_left_sided_label(true)
                                                                .set_label_width(120.0)
                                                                .override_colors(Color32::WHITE, Color32::BLACK)
                                                                .with_width(30.0)
                                                            );
                                                    },
                                                    UIBottomSelection::Pitch => {
                                                        ui.add(
                                                            HorizontalParamSlider::for_param(&params.pitch_env_atk_curve, setter)
                                                                .with_width(HCURVE_BWIDTH)
                                                                .slimmer(0.7)
                                                                .set_left_sided_label(true)
                                                                .set_label_width(HCURVE_WIDTH)
                                                                .override_colors(
                                                                    DARK_GREY_UI_COLOR,
                                                                    SYNTH_SOFT_BLUE2)
                                                                .with_width(30.0),
                                                        );
                                                        ui.add(
                                                            HorizontalParamSlider::for_param(&params.pitch_env_dec_curve, setter)
                                                                .with_width(HCURVE_BWIDTH)
                                                                .slimmer(0.7)
                                                                .set_left_sided_label(true)
                                                                .set_label_width(HCURVE_WIDTH)
                                                                .override_colors(
                                                                    DARK_GREY_UI_COLOR,
                                                                    SYNTH_SOFT_BLUE2)
                                                                .with_width(30.0),
                                                        );
                                                        ui.add(
                                                            HorizontalParamSlider::for_param(&params.pitch_env_rel_curve, setter)
                                                                .with_width(HCURVE_BWIDTH)
                                                                .slimmer(0.7)
                                                                .set_left_sided_label(true)
                                                                .set_label_width(HCURVE_WIDTH)
                                                                .override_colors(
                                                                    DARK_GREY_UI_COLOR,
                                                                    SYNTH_SOFT_BLUE2)
                                                                .with_width(30.0),
                                                        );
                                                        ui.add_space(67.0);
                                                    }
                                                }
                                            });
                                        });
                                        ui.horizontal(|ui|{
                                            ui.add_space(50.0);
                                            ui.selectable_value(&mut *filter_select.lock().unwrap(), UIBottomSelection::Filter1, RichText::new("Filter 1").color(Color32::BLACK));
                                            ui.selectable_value(&mut *filter_select.lock().unwrap(), UIBottomSelection::Filter2, RichText::new("Filter 2").color(Color32::BLACK));
                                            ui.selectable_value(&mut *filter_select.lock().unwrap(), UIBottomSelection::Pitch, RichText::new("Pitch Env").color(Color32::BLACK))
                                        });
                                    });

                                    // Move Presets over!
                                    ui.add_space(8.0);

                                    match *lfo_select.lock().unwrap() {
                                        LFOSelect::LFO1 => {
                                            ui.painter().rect_filled(
                                                Rect::from_x_y_ranges(
                                            RangeInclusive::new((WIDTH as f32)*0.64, (WIDTH as f32) - (synth_bar_space + 4.0)),
                                            RangeInclusive::new((HEIGHT as f32)*0.73, (HEIGHT as f32) - 4.0)),
                                                Rounding::from(16.0),
                                                A_BACKGROUND_COLOR_TOP
                                            );
                                        }
                                        LFOSelect::LFO2 => {
                                            ui.painter().rect_filled(
                                                Rect::from_x_y_ranges(
                                            RangeInclusive::new((WIDTH as f32)*0.64, (WIDTH as f32) - (synth_bar_space + 4.0)),
                                            RangeInclusive::new((HEIGHT as f32)*0.73, (HEIGHT as f32) - 4.0)),
                                                Rounding::from(16.0),
                                                SYNTH_SOFT_BLUE
                                            );
                                        }
                                        LFOSelect::LFO3 => {
                                            ui.painter().rect_filled(
                                                Rect::from_x_y_ranges(
                                            RangeInclusive::new((WIDTH as f32)*0.64, (WIDTH as f32) - (synth_bar_space + 4.0)),
                                            RangeInclusive::new((HEIGHT as f32)*0.73, (HEIGHT as f32) - 4.0)),
                                                Rounding::from(16.0),
                                                SYNTH_MIDDLE_BLUE
                                            );
                                        }
                                        LFOSelect::Misc => {
                                            ui.painter().rect_filled(
                                                Rect::from_x_y_ranges(
                                            RangeInclusive::new((WIDTH as f32)*0.64, (WIDTH as f32) - (synth_bar_space + 4.0)),
                                            RangeInclusive::new((HEIGHT as f32)*0.73, (HEIGHT as f32) - 4.0)),
                                                Rounding::from(16.0),
                                                SYNTH_SOFT_BLUE2
                                            );
                                        }
                                        LFOSelect::Modulation => {
                                            ui.painter().rect_filled(
                                                Rect::from_x_y_ranges(
                                            RangeInclusive::new((WIDTH as f32)*0.64, (WIDTH as f32) - (synth_bar_space + 4.0)),
                                            RangeInclusive::new((HEIGHT as f32)*0.73, (HEIGHT as f32) - 4.0)),
                                                Rounding::from(16.0),
                                                LIGHTER_PURPLE
                                            );
                                        }
                                        LFOSelect::INFO => {
                                            ui.painter().rect_filled(
                                                Rect::from_x_y_ranges(
                                            RangeInclusive::new((WIDTH as f32)*0.64, (WIDTH as f32) - (synth_bar_space + 4.0)),
                                            RangeInclusive::new((HEIGHT as f32)*0.73, (HEIGHT as f32) - 4.0)),
                                                Rounding::from(16.0),
                                                SYNTH_SOFT_BLUE2
                                            );
                                        }
                                        LFOSelect::FX => {
                                            ui.painter().rect_filled(
                                                Rect::from_x_y_ranges(
                                            RangeInclusive::new((WIDTH as f32)*0.64, (WIDTH as f32) - (synth_bar_space + 4.0)),
                                            RangeInclusive::new((HEIGHT as f32)*0.73, (HEIGHT as f32) - 4.0)),
                                                Rounding::from(16.0),
                                                FONT_COLOR
                                            );
                                        }
                                    }

                                    // LFO Box
                                    ui.vertical(|ui|{
                                        ui.horizontal(|ui| {
                                            ui.selectable_value(&mut *lfo_select.lock().unwrap(), LFOSelect::INFO, RichText::new("INFO").color(Color32::BLACK).font(SMALLER_FONT));
                                            ui.selectable_value(&mut *lfo_select.lock().unwrap(), LFOSelect::LFO1, RichText::new("LFO 1").color(Color32::BLACK).font(SMALLER_FONT));
                                            ui.selectable_value(&mut *lfo_select.lock().unwrap(), LFOSelect::LFO2, RichText::new("LFO 2").color(Color32::BLACK).font(SMALLER_FONT));
                                            ui.selectable_value(&mut *lfo_select.lock().unwrap(), LFOSelect::LFO3, RichText::new("LFO 3").color(Color32::BLACK).font(SMALLER_FONT));
                                            ui.selectable_value(&mut *lfo_select.lock().unwrap(), LFOSelect::Misc, RichText::new("Misc").color(Color32::BLACK).font(SMALLER_FONT));
                                            ui.selectable_value(&mut *lfo_select.lock().unwrap(), LFOSelect::Modulation, RichText::new("Mods").color(Color32::BLACK).font(SMALLER_FONT));
                                            ui.selectable_value(&mut *lfo_select.lock().unwrap(), LFOSelect::FX, RichText::new("FX").color(Color32::BLACK).font(SMALLER_FONT));
                                        });
                                        ui.separator();
                                        match *lfo_select.lock().unwrap() {
                                            LFOSelect::LFO1 => {
                                                ui.vertical(|ui|{
                                                    ui.horizontal(|ui|{
                                                        ui.label(RichText::new("LFO Enabled")
                                                            .font(SMALLER_FONT)
                                                            .color(Color32::BLACK)
                                                        );
                                                        let lfo1_toggle = toggle_switch::ToggleSwitch::for_param(&params.lfo1_enable, setter);
                                                        ui.add(lfo1_toggle);
                                                    });
                                                    ui.horizontal(|ui|{
                                                        ui.label(RichText::new("Sync")
                                                            .font(SMALLER_FONT)
                                                            .color(Color32::BLACK)
                                                        )
                                                            .on_hover_text("Sync LFO values to your DAW");
                                                        let lfosync1 = toggle_switch::ToggleSwitch::for_param(&params.lfo1_sync, setter);
                                                        ui.add(lfosync1);
                                                        ui.separator();
                                                        ui.label(RichText::new("Retrig")
                                                            .font(SMALLER_FONT)
                                                            .color(Color32::BLACK)
                                                        );
                                                        ui.add(ParamSlider::for_param(&params.lfo1_retrigger, setter).with_width(80.0));
                                                    });
                                                    ui.separator();
                                                    ui.horizontal(|ui|{
                                                        ui.label(RichText::new("Rate ")
                                                            .font(SMALLER_FONT)
                                                            .color(Color32::BLACK)
                                                        );
                                                        if params.lfo1_sync.value() {
                                                            ui.add(ParamSlider::for_param(&params.lfo1_snap, setter).with_width(180.0));
                                                        } else {
                                                            ui.add(ParamSlider::for_param(&params.lfo1_freq, setter).with_width(180.0));
                                                        }
                                                    });
                                                    ui.horizontal(|ui|{
                                                        ui.label(RichText::new("Shape")
                                                            .font(SMALLER_FONT)
                                                            .color(Color32::BLACK)
                                                        );
                                                        ui.add(ParamSlider::for_param(&params.lfo1_waveform, setter).with_width(180.0));
                                                    });
                                                    ui.horizontal(|ui|{
                                                        ui.label(RichText::new("Phase")
                                                            .font(SMALLER_FONT)
                                                            .color(Color32::BLACK)
                                                        );
                                                        ui.add(ParamSlider::for_param(&params.lfo1_phase, setter).with_width(180.0));
                                                    });
                                                });
                                            },
                                            LFOSelect::LFO2 => {
                                                ui.vertical(|ui|{
                                                    ui.horizontal(|ui|{
                                                        ui.label(RichText::new("LFO Enabled")
                                                            .font(SMALLER_FONT)
                                                            .color(Color32::BLACK)
                                                        );
                                                        let lfo2_toggle = toggle_switch::ToggleSwitch::for_param(&params.lfo2_enable, setter);
                                                        ui.add(lfo2_toggle);
                                                    });
                                                    ui.horizontal(|ui|{
                                                        ui.label(RichText::new("Sync")
                                                            .font(SMALLER_FONT)
                                                            .color(Color32::BLACK)
                                                        )
                                                            .on_hover_text("Sync LFO values to your DAW");
                                                        let lfosync2 = toggle_switch::ToggleSwitch::for_param(&params.lfo2_sync, setter);
                                                        ui.add(lfosync2);
                                                        ui.separator();
                                                        ui.label(RichText::new("Retrig")
                                                            .font(SMALLER_FONT)
                                                            .color(Color32::BLACK)
                                                        );
                                                        ui.add(ParamSlider::for_param(&params.lfo2_retrigger, setter).with_width(80.0));
                                                    });
                                                    ui.separator();
                                                    ui.horizontal(|ui|{
                                                        ui.label(RichText::new("Rate ")
                                                            .font(SMALLER_FONT)
                                                            .color(Color32::BLACK)
                                                        );
                                                        if params.lfo2_sync.value() {
                                                            ui.add(ParamSlider::for_param(&params.lfo2_snap, setter).with_width(180.0));
                                                        } else {
                                                            ui.add(ParamSlider::for_param(&params.lfo2_freq, setter).with_width(180.0));
                                                        }
                                                    });
                                                    ui.horizontal(|ui|{
                                                        ui.label(RichText::new("Shape")
                                                            .font(SMALLER_FONT)
                                                            .color(Color32::BLACK)
                                                        );
                                                        ui.add(ParamSlider::for_param(&params.lfo2_waveform, setter).with_width(180.0));
                                                    });
                                                    ui.horizontal(|ui|{
                                                        ui.label(RichText::new("Phase")
                                                            .font(SMALLER_FONT)
                                                            .color(Color32::BLACK)
                                                        );
                                                        ui.add(ParamSlider::for_param(&params.lfo2_phase, setter).with_width(180.0));
                                                    });
                                                });
                                            },
                                            LFOSelect::LFO3 => {
                                                ui.vertical(|ui|{
                                                    ui.horizontal(|ui|{
                                                        ui.label(RichText::new("LFO Enabled")
                                                            .font(SMALLER_FONT)
                                                            .color(Color32::BLACK)
                                                        );
                                                        let lfo3_toggle = toggle_switch::ToggleSwitch::for_param(&params.lfo3_enable, setter);
                                                        ui.add(lfo3_toggle);
                                                    });
                                                    ui.horizontal(|ui|{
                                                        ui.label(RichText::new("Sync")
                                                            .font(SMALLER_FONT)
                                                            .color(Color32::BLACK)
                                                        )
                                                            .on_hover_text("Sync LFO values to your DAW");
                                                        let lfosync3 = toggle_switch::ToggleSwitch::for_param(&params.lfo3_sync, setter);
                                                        ui.add(lfosync3);
                                                        ui.separator();
                                                        ui.label(RichText::new("Retrig")
                                                            .font(SMALLER_FONT)
                                                            .color(Color32::BLACK)
                                                        );
                                                        ui.add(ParamSlider::for_param(&params.lfo3_retrigger, setter).with_width(80.0));
                                                    });
                                                    ui.separator();
                                                    ui.horizontal(|ui|{
                                                        ui.label(RichText::new("Rate ")
                                                            .font(SMALLER_FONT)
                                                            .color(Color32::BLACK)
                                                        );
                                                        if params.lfo3_sync.value() {
                                                            ui.add(ParamSlider::for_param(&params.lfo3_snap, setter).with_width(180.0));
                                                        } else {
                                                            ui.add(ParamSlider::for_param(&params.lfo3_freq, setter).with_width(180.0));
                                                        }
                                                    });
                                                    ui.horizontal(|ui|{
                                                        ui.label(RichText::new("Shape")
                                                            .font(SMALLER_FONT)
                                                            .color(Color32::BLACK)
                                                        );
                                                        ui.add(ParamSlider::for_param(&params.lfo3_waveform, setter).with_width(180.0));
                                                    });
                                                    ui.horizontal(|ui|{
                                                        ui.label(RichText::new("Phase")
                                                            .font(SMALLER_FONT)
                                                            .color(Color32::BLACK)
                                                        );
                                                        ui.add(ParamSlider::for_param(&params.lfo3_phase, setter).with_width(180.0));
                                                    });
                                                });
                                            },
                                            LFOSelect::Misc => {
                                                ui.horizontal(|ui|{
                                                    ui.label(RichText::new("Link Cutoff 2 to Cutoff 1")
                                                        .font(SMALLER_FONT)
                                                        .color(Color32::BLACK)
                                                    )
                                                        .on_hover_text("Filter 1 will control both filter cutoff values");
                                                    let filter_cutoff_link = toggle_switch::ToggleSwitch::for_param(&params.filter_cutoff_link, setter);
                                                    ui.add(filter_cutoff_link);
                                                });
                                            }
                                            LFOSelect::Modulation => {
                                                // This is my creative "combobox" to use an enumparam with Nih-Plug
                                                ui.vertical(|ui|{
                                                    // Modulator section 1
                                                    //////////////////////////////////////////////////////////////////////////////////
                                                    ui.horizontal(|ui|{
                                                        let mod_1_knob = ui_knob::ArcKnob::for_param(
                                                            &params.mod_amount_knob_1,
                                                            setter,
                                                            12.0)
                                                            .preset_style(ui_knob::KnobStyle::NewPresets2)
                                                            .set_fill_color(SYNTH_BARS_PURPLE)
                                                            .set_line_color(A_KNOB_OUTSIDE_COLOR)
                                                            .set_show_label(false);
                                                        ui.add(mod_1_knob);
                                                        ui.separator();
                                                        egui::ComboBox::new("mod_source_supported", "")
                                                            .selected_text(format!("{:?}", *mod_source_1_tracker.lock().unwrap()))
                                                            .width(70.0)
                                                            .show_ui(ui, |ui| {
                                                                ui.selectable_value(&mut *mod_source_1_tracker.lock().unwrap(), ModulationSource::None, "None");
                                                                ui.selectable_value(&mut *mod_source_1_tracker.lock().unwrap(), ModulationSource::Velocity, "Velocity");
                                                                ui.selectable_value(&mut *mod_source_1_tracker.lock().unwrap(), ModulationSource::LFO1, "LFO 1");
                                                                ui.selectable_value(&mut *mod_source_1_tracker.lock().unwrap(), ModulationSource::LFO2, "LFO 2");
                                                                ui.selectable_value(&mut *mod_source_1_tracker.lock().unwrap(), ModulationSource::LFO3, "LFO 3");
                                                            });
                                                        /*
                                                        CustomComboBox::ComboBox::new("mod_source_1_ID",params.mod_source_1.value().to_string(), true, 5)
                                                            .selected_text(format!("{:?}", *mod_source_1_tracker.lock().unwrap()))
                                                            .width(70.0)
                                                            .show_ui(ui, |ui|{
                                                                ui.selectable_value(&mut *mod_source_1_tracker.lock().unwrap(), ModulationSource::None, "None");
                                                                ui.selectable_value(&mut *mod_source_1_tracker.lock().unwrap(), ModulationSource::Velocity, "Velocity");
                                                                ui.selectable_value(&mut *mod_source_1_tracker.lock().unwrap(), ModulationSource::LFO1, "LFO 1");
                                                                ui.selectable_value(&mut *mod_source_1_tracker.lock().unwrap(), ModulationSource::LFO2, "LFO 2");
                                                                ui.selectable_value(&mut *mod_source_1_tracker.lock().unwrap(), ModulationSource::LFO3, "LFO 3");
                                                            });
                                                            */
                                                            // This was a workaround for updating combobox on preset load but otherwise updating preset through combobox selection
                                                            if *mod_source_override_1.lock().unwrap() != ModulationSource::UnsetModulation {
                                                                // This happens on plugin preset load
                                                                *mod_source_1_tracker.lock().unwrap() = *mod_source_override_1.lock().unwrap();
                                                                setter.set_parameter( &params.mod_source_1, mod_source_1_tracker.lock().unwrap().clone());
                                                                *mod_source_override_1.lock().unwrap() = ModulationSource::UnsetModulation;
                                                            } else {
                                                                if *mod_source_1_tracker.lock().unwrap() != params.mod_source_1.value() {
                                                                    setter.set_parameter( &params.mod_source_1, mod_source_1_tracker.lock().unwrap().clone());
                                                                }
                                                            }
                                                        ui.label(RichText::new("Mods")
                                                            .font(SMALLER_FONT)
                                                            .color(Color32::BLACK));
                                                        egui::ComboBox::new("mod_dest_1_ID", "")
                                                            .selected_text(format!("{:?}", *mod_dest_1_tracker.lock().unwrap()))
                                                            .width(100.0)
                                                            .show_ui(ui, |ui|{
                                                                ui.selectable_value(&mut *mod_dest_1_tracker.lock().unwrap(), ModulationDestination::None, "None");
                                                                ui.selectable_value(&mut *mod_dest_1_tracker.lock().unwrap(), ModulationDestination::Cutoff_1, "Cutoff 1");
                                                                ui.selectable_value(&mut *mod_dest_1_tracker.lock().unwrap(), ModulationDestination::Cutoff_2, "Cutoff 2");
                                                                ui.selectable_value(&mut *mod_dest_1_tracker.lock().unwrap(), ModulationDestination::Resonance_1, "Resonance 1");
                                                                ui.selectable_value(&mut *mod_dest_1_tracker.lock().unwrap(), ModulationDestination::Resonance_2, "Resonance 2");
                                                                ui.selectable_value(&mut *mod_dest_1_tracker.lock().unwrap(), ModulationDestination::All_Gain, "All Gain");
                                                                ui.selectable_value(&mut *mod_dest_1_tracker.lock().unwrap(), ModulationDestination::Osc1_Gain, "Osc1 Gain");
                                                                ui.selectable_value(&mut *mod_dest_1_tracker.lock().unwrap(), ModulationDestination::Osc2_Gain, "Osc2 Gain");
                                                                ui.selectable_value(&mut *mod_dest_1_tracker.lock().unwrap(), ModulationDestination::Osc3_Gain, "Osc3 Gain");
                                                                ui.selectable_value(&mut *mod_dest_1_tracker.lock().unwrap(), ModulationDestination::All_Detune, "All Detune");
                                                                ui.selectable_value(&mut *mod_dest_1_tracker.lock().unwrap(), ModulationDestination::Osc1Detune, "Osc1 Detune");
                                                                ui.selectable_value(&mut *mod_dest_1_tracker.lock().unwrap(), ModulationDestination::Osc2Detune, "Osc2 Detune");
                                                                ui.selectable_value(&mut *mod_dest_1_tracker.lock().unwrap(), ModulationDestination::Osc3Detune, "Osc3 Detune");
                                                                ui.selectable_value(&mut *mod_dest_1_tracker.lock().unwrap(), ModulationDestination::All_UniDetune, "All UniDetune");
                                                                ui.selectable_value(&mut *mod_dest_1_tracker.lock().unwrap(), ModulationDestination::Osc1UniDetune, "Osc1 UniDetune");
                                                                ui.selectable_value(&mut *mod_dest_1_tracker.lock().unwrap(), ModulationDestination::Osc2UniDetune, "Osc2 UniDetune");
                                                                ui.selectable_value(&mut *mod_dest_1_tracker.lock().unwrap(), ModulationDestination::Osc3UniDetune, "Osc3 UniDetune");
                                                            });
                                                        // This was a workaround for updating combobox on preset load but otherwise updating preset through combobox selection
                                                        if *mod_dest_override_1.lock().unwrap() != ModulationDestination::UnsetModulation {
                                                            // This happens on plugin preset load
                                                            *mod_dest_1_tracker.lock().unwrap() = *mod_dest_override_1.lock().unwrap();
                                                            setter.set_parameter( &params.mod_destination_1, mod_dest_1_tracker.lock().unwrap().clone());
                                                            *mod_dest_override_1.lock().unwrap() = ModulationDestination::UnsetModulation;
                                                        } else {
                                                            if *mod_dest_1_tracker.lock().unwrap() != params.mod_destination_1.value() {
                                                                setter.set_parameter( &params.mod_destination_1, mod_dest_1_tracker.lock().unwrap().clone());
                                                            }
                                                        }
                                                    });
                                                    ui.separator();

                                                    // Modulator section 2
                                                    //////////////////////////////////////////////////////////////////////////////////
                                                    ui.horizontal(|ui|{
                                                        let mod_2_knob = ui_knob::ArcKnob::for_param(
                                                            &params.mod_amount_knob_2,
                                                            setter,
                                                            12.0)
                                                            .preset_style(ui_knob::KnobStyle::NewPresets2)
                                                            .set_fill_color(SYNTH_BARS_PURPLE)
                                                            .set_line_color(A_KNOB_OUTSIDE_COLOR)
                                                            .set_show_label(false);
                                                        ui.add(mod_2_knob);
                                                        ui.separator();
                                                        egui::ComboBox::new("mod_source_2_ID","")
                                                            .selected_text(format!("{:?}", *mod_source_2_tracker.lock().unwrap()))
                                                            .width(70.0)
                                                            .show_ui(ui, |ui|{
                                                                ui.selectable_value(&mut *mod_source_2_tracker.lock().unwrap(), ModulationSource::None, "None");
                                                                ui.selectable_value(&mut *mod_source_2_tracker.lock().unwrap(), ModulationSource::Velocity, "Velocity");
                                                                ui.selectable_value(&mut *mod_source_2_tracker.lock().unwrap(), ModulationSource::LFO1, "LFO 1");
                                                                ui.selectable_value(&mut *mod_source_2_tracker.lock().unwrap(), ModulationSource::LFO2, "LFO 2");
                                                                ui.selectable_value(&mut *mod_source_2_tracker.lock().unwrap(), ModulationSource::LFO3, "LFO 3");
                                                            });
                                                        // This was a workaround for updating combobox on preset load but otherwise updating preset through combobox selection
                                                        if *mod_source_override_2.lock().unwrap() != ModulationSource::UnsetModulation {
                                                            // This happens on plugin preset load
                                                            *mod_source_2_tracker.lock().unwrap() = *mod_source_override_2.lock().unwrap();
                                                            setter.set_parameter( &params.mod_source_2, mod_source_2_tracker.lock().unwrap().clone());
                                                            *mod_source_override_2.lock().unwrap() = ModulationSource::UnsetModulation;
                                                        } else {
                                                            if *mod_source_2_tracker.lock().unwrap() != params.mod_source_2.value() {
                                                                setter.set_parameter( &params.mod_source_2, mod_source_2_tracker.lock().unwrap().clone());
                                                            }
                                                        }
                                                        ui.label(RichText::new("Mods")
                                                            .font(SMALLER_FONT)
                                                            .color(Color32::BLACK));
                                                        egui::ComboBox::new("mod_dest_2_ID", "")
                                                            .selected_text(format!("{:?}", *mod_dest_2_tracker.lock().unwrap()))
                                                            .width(100.0)
                                                            .show_ui(ui, |ui|{
                                                                ui.selectable_value(&mut *mod_dest_2_tracker.lock().unwrap(), ModulationDestination::None, "None");
                                                                ui.selectable_value(&mut *mod_dest_2_tracker.lock().unwrap(), ModulationDestination::Cutoff_1, "Cutoff 1");
                                                                ui.selectable_value(&mut *mod_dest_2_tracker.lock().unwrap(), ModulationDestination::Cutoff_2, "Cutoff 2");
                                                                ui.selectable_value(&mut *mod_dest_2_tracker.lock().unwrap(), ModulationDestination::Resonance_1, "Resonance 1");
                                                                ui.selectable_value(&mut *mod_dest_2_tracker.lock().unwrap(), ModulationDestination::Resonance_2, "Resonance 2");
                                                                ui.selectable_value(&mut *mod_dest_2_tracker.lock().unwrap(), ModulationDestination::All_Gain, "All Gain");
                                                                ui.selectable_value(&mut *mod_dest_2_tracker.lock().unwrap(), ModulationDestination::Osc1_Gain, "Osc1 Gain");
                                                                ui.selectable_value(&mut *mod_dest_2_tracker.lock().unwrap(), ModulationDestination::Osc2_Gain, "Osc2 Gain");
                                                                ui.selectable_value(&mut *mod_dest_2_tracker.lock().unwrap(), ModulationDestination::Osc3_Gain, "Osc3 Gain");
                                                                ui.selectable_value(&mut *mod_dest_2_tracker.lock().unwrap(), ModulationDestination::All_Detune, "All Detune");
                                                                ui.selectable_value(&mut *mod_dest_2_tracker.lock().unwrap(), ModulationDestination::Osc1Detune, "Osc1 Detune");
                                                                ui.selectable_value(&mut *mod_dest_2_tracker.lock().unwrap(), ModulationDestination::Osc2Detune, "Osc2 Detune");
                                                                ui.selectable_value(&mut *mod_dest_2_tracker.lock().unwrap(), ModulationDestination::Osc3Detune, "Osc3 Detune");
                                                                ui.selectable_value(&mut *mod_dest_2_tracker.lock().unwrap(), ModulationDestination::All_UniDetune, "All UniDetune");
                                                                ui.selectable_value(&mut *mod_dest_2_tracker.lock().unwrap(), ModulationDestination::Osc1UniDetune, "Osc1 UniDetune");
                                                                ui.selectable_value(&mut *mod_dest_2_tracker.lock().unwrap(), ModulationDestination::Osc2UniDetune, "Osc2 UniDetune");
                                                                ui.selectable_value(&mut *mod_dest_2_tracker.lock().unwrap(), ModulationDestination::Osc3UniDetune, "Osc3 UniDetune");
                                                            });
                                                        // This was a workaround for updating combobox on preset load but otherwise updating preset through combobox selection
                                                        if *mod_dest_override_2.lock().unwrap() != ModulationDestination::UnsetModulation {
                                                            // This happens on plugin preset load
                                                            *mod_dest_2_tracker.lock().unwrap() = *mod_dest_override_2.lock().unwrap();
                                                            setter.set_parameter( &params.mod_destination_2, mod_dest_2_tracker.lock().unwrap().clone());
                                                            *mod_dest_override_2.lock().unwrap() = ModulationDestination::UnsetModulation;
                                                        } else {
                                                            if *mod_dest_2_tracker.lock().unwrap() != params.mod_destination_2.value() {
                                                                setter.set_parameter( &params.mod_destination_2, mod_dest_2_tracker.lock().unwrap().clone());
                                                            }
                                                        }
                                                    });
                                                    ui.separator();

                                                    // Modulator section 3
                                                    //////////////////////////////////////////////////////////////////////////////////
                                                    ui.horizontal(|ui|{
                                                        let mod_3_knob = ui_knob::ArcKnob::for_param(
                                                            &params.mod_amount_knob_3,
                                                            setter,
                                                            12.0)
                                                            .preset_style(ui_knob::KnobStyle::NewPresets2)
                                                            .set_fill_color(SYNTH_BARS_PURPLE)
                                                            .set_line_color(A_KNOB_OUTSIDE_COLOR)
                                                            .set_show_label(false);
                                                        ui.add(mod_3_knob);
                                                        ui.separator();
                                                        egui::ComboBox::new("mod_source_3_ID","")
                                                            .selected_text(format!("{:?}", *mod_source_3_tracker.lock().unwrap()))
                                                            .width(70.0)
                                                            .show_ui(ui, |ui|{
                                                                ui.selectable_value(&mut *mod_source_3_tracker.lock().unwrap(), ModulationSource::None, "None");
                                                                ui.selectable_value(&mut *mod_source_3_tracker.lock().unwrap(), ModulationSource::Velocity, "Velocity");
                                                                ui.selectable_value(&mut *mod_source_3_tracker.lock().unwrap(), ModulationSource::LFO1, "LFO 1");
                                                                ui.selectable_value(&mut *mod_source_3_tracker.lock().unwrap(), ModulationSource::LFO2, "LFO 2");
                                                                ui.selectable_value(&mut *mod_source_3_tracker.lock().unwrap(), ModulationSource::LFO3, "LFO 3");
                                                            });
                                                        // This was a workaround for updating combobox on preset load but otherwise updating preset through combobox selection
                                                        if *mod_source_override_3.lock().unwrap() != ModulationSource::UnsetModulation {
                                                            // This happens on plugin preset load
                                                            *mod_source_3_tracker.lock().unwrap() = *mod_source_override_3.lock().unwrap();
                                                            setter.set_parameter( &params.mod_source_3, mod_source_3_tracker.lock().unwrap().clone());
                                                            *mod_source_override_3.lock().unwrap() = ModulationSource::UnsetModulation;
                                                        } else {
                                                            if *mod_source_3_tracker.lock().unwrap() != params.mod_source_3.value() {
                                                                setter.set_parameter( &params.mod_source_3, mod_source_3_tracker.lock().unwrap().clone());
                                                            }
                                                        }
                                                        ui.label(RichText::new("Mods")
                                                            .font(SMALLER_FONT)
                                                            .color(Color32::BLACK));
                                                        egui::ComboBox::new("mod_dest_3_ID", "")
                                                            .selected_text(format!("{:?}", *mod_dest_3_tracker.lock().unwrap()))
                                                            .width(100.0)
                                                            .show_ui(ui, |ui|{
                                                                ui.selectable_value(&mut *mod_dest_3_tracker.lock().unwrap(), ModulationDestination::None, "None");
                                                                ui.selectable_value(&mut *mod_dest_3_tracker.lock().unwrap(), ModulationDestination::Cutoff_1, "Cutoff 1");
                                                                ui.selectable_value(&mut *mod_dest_3_tracker.lock().unwrap(), ModulationDestination::Cutoff_2, "Cutoff 2");
                                                                ui.selectable_value(&mut *mod_dest_3_tracker.lock().unwrap(), ModulationDestination::Resonance_1, "Resonance 1");
                                                                ui.selectable_value(&mut *mod_dest_3_tracker.lock().unwrap(), ModulationDestination::Resonance_2, "Resonance 2");
                                                                ui.selectable_value(&mut *mod_dest_3_tracker.lock().unwrap(), ModulationDestination::All_Gain, "All Gain");
                                                                ui.selectable_value(&mut *mod_dest_3_tracker.lock().unwrap(), ModulationDestination::Osc1_Gain, "Osc1 Gain");
                                                                ui.selectable_value(&mut *mod_dest_3_tracker.lock().unwrap(), ModulationDestination::Osc2_Gain, "Osc2 Gain");
                                                                ui.selectable_value(&mut *mod_dest_3_tracker.lock().unwrap(), ModulationDestination::Osc3_Gain, "Osc3 Gain");
                                                                ui.selectable_value(&mut *mod_dest_3_tracker.lock().unwrap(), ModulationDestination::All_Detune, "All Detune");
                                                                ui.selectable_value(&mut *mod_dest_3_tracker.lock().unwrap(), ModulationDestination::Osc1Detune, "Osc1 Detune");
                                                                ui.selectable_value(&mut *mod_dest_3_tracker.lock().unwrap(), ModulationDestination::Osc2Detune, "Osc2 Detune");
                                                                ui.selectable_value(&mut *mod_dest_3_tracker.lock().unwrap(), ModulationDestination::Osc3Detune, "Osc3 Detune");
                                                                ui.selectable_value(&mut *mod_dest_3_tracker.lock().unwrap(), ModulationDestination::All_UniDetune, "All UniDetune");
                                                                ui.selectable_value(&mut *mod_dest_3_tracker.lock().unwrap(), ModulationDestination::Osc1UniDetune, "Osc1 UniDetune");
                                                                ui.selectable_value(&mut *mod_dest_3_tracker.lock().unwrap(), ModulationDestination::Osc2UniDetune, "Osc2 UniDetune");
                                                                ui.selectable_value(&mut *mod_dest_3_tracker.lock().unwrap(), ModulationDestination::Osc3UniDetune, "Osc3 UniDetune");
                                                            });
                                                        // This was a workaround for updating combobox on preset load but otherwise updating preset through combobox selection
                                                        if *mod_dest_override_3.lock().unwrap() != ModulationDestination::UnsetModulation {
                                                            // This happens on plugin preset load
                                                            *mod_dest_3_tracker.lock().unwrap() = *mod_dest_override_3.lock().unwrap();
                                                            setter.set_parameter( &params.mod_destination_3, mod_dest_3_tracker.lock().unwrap().clone());
                                                            *mod_dest_override_3.lock().unwrap() = ModulationDestination::UnsetModulation;
                                                        } else {
                                                            if *mod_dest_3_tracker.lock().unwrap() != params.mod_destination_3.value() {
                                                                setter.set_parameter( &params.mod_destination_3, mod_dest_3_tracker.lock().unwrap().clone());
                                                            }
                                                        }
                                                    });
                                                    ui.separator();

                                                    // Modulator section 4
                                                    //////////////////////////////////////////////////////////////////////////////////
                                                    ui.horizontal(|ui|{
                                                        let mod_4_knob = ui_knob::ArcKnob::for_param(
                                                            &params.mod_amount_knob_4,
                                                            setter,
                                                            12.0)
                                                            .preset_style(ui_knob::KnobStyle::NewPresets2)
                                                            .set_fill_color(SYNTH_BARS_PURPLE)
                                                            .set_line_color(A_KNOB_OUTSIDE_COLOR)
                                                            .set_show_label(false);
                                                        ui.add(mod_4_knob);
                                                        ui.separator();
                                                        egui::ComboBox::new("mod_source_4_ID","")
                                                            .selected_text(format!("{:?}", *mod_source_4_tracker.lock().unwrap()))
                                                            .width(70.0)
                                                            .show_ui(ui, |ui|{
                                                                ui.selectable_value(&mut *mod_source_4_tracker.lock().unwrap(), ModulationSource::None, "None");
                                                                ui.selectable_value(&mut *mod_source_4_tracker.lock().unwrap(), ModulationSource::Velocity, "Velocity");
                                                                ui.selectable_value(&mut *mod_source_4_tracker.lock().unwrap(), ModulationSource::LFO1, "LFO 1");
                                                                ui.selectable_value(&mut *mod_source_4_tracker.lock().unwrap(), ModulationSource::LFO2, "LFO 2");
                                                                ui.selectable_value(&mut *mod_source_4_tracker.lock().unwrap(), ModulationSource::LFO3, "LFO 3");
                                                            });
                                                        // This was a workaround for updating combobox on preset load but otherwise updating preset through combobox selection
                                                        if *mod_source_override_4.lock().unwrap() != ModulationSource::UnsetModulation {
                                                            // This happens on plugin preset load
                                                            *mod_source_4_tracker.lock().unwrap() = *mod_source_override_4.lock().unwrap();
                                                            setter.set_parameter( &params.mod_source_4, mod_source_4_tracker.lock().unwrap().clone());
                                                            *mod_source_override_4.lock().unwrap() = ModulationSource::UnsetModulation;
                                                        } else {
                                                            if *mod_source_4_tracker.lock().unwrap() != params.mod_source_4.value() {
                                                                setter.set_parameter( &params.mod_source_4, mod_source_4_tracker.lock().unwrap().clone());
                                                            }
                                                        }
                                                        ui.label(RichText::new("Mods")
                                                            .font(SMALLER_FONT)
                                                            .color(Color32::BLACK));
                                                        egui::ComboBox::new("mod_dest_4_ID", "")
                                                        .selected_text(format!("{:?}", *mod_dest_4_tracker.lock().unwrap()))
                                                        .width(100.0)
                                                        .show_ui(ui, |ui|{
                                                            ui.selectable_value(&mut *mod_dest_4_tracker.lock().unwrap(), ModulationDestination::None, "None");
                                                            ui.selectable_value(&mut *mod_dest_4_tracker.lock().unwrap(), ModulationDestination::Cutoff_1, "Cutoff 1");
                                                            ui.selectable_value(&mut *mod_dest_4_tracker.lock().unwrap(), ModulationDestination::Cutoff_2, "Cutoff 2");
                                                            ui.selectable_value(&mut *mod_dest_4_tracker.lock().unwrap(), ModulationDestination::Resonance_1, "Resonance 1");
                                                            ui.selectable_value(&mut *mod_dest_4_tracker.lock().unwrap(), ModulationDestination::Resonance_2, "Resonance 2");
                                                            ui.selectable_value(&mut *mod_dest_4_tracker.lock().unwrap(), ModulationDestination::All_Gain, "All Gain");
                                                            ui.selectable_value(&mut *mod_dest_4_tracker.lock().unwrap(), ModulationDestination::Osc1_Gain, "Osc1 Gain");
                                                            ui.selectable_value(&mut *mod_dest_4_tracker.lock().unwrap(), ModulationDestination::Osc2_Gain, "Osc2 Gain");
                                                            ui.selectable_value(&mut *mod_dest_4_tracker.lock().unwrap(), ModulationDestination::Osc3_Gain, "Osc3 Gain");
                                                            ui.selectable_value(&mut *mod_dest_4_tracker.lock().unwrap(), ModulationDestination::All_Detune, "All Detune");
                                                            ui.selectable_value(&mut *mod_dest_4_tracker.lock().unwrap(), ModulationDestination::Osc1Detune, "Osc1 Detune");
                                                            ui.selectable_value(&mut *mod_dest_4_tracker.lock().unwrap(), ModulationDestination::Osc2Detune, "Osc2 Detune");
                                                            ui.selectable_value(&mut *mod_dest_4_tracker.lock().unwrap(), ModulationDestination::Osc3Detune, "Osc3 Detune");
                                                            ui.selectable_value(&mut *mod_dest_4_tracker.lock().unwrap(), ModulationDestination::All_UniDetune, "All UniDetune");
                                                            ui.selectable_value(&mut *mod_dest_4_tracker.lock().unwrap(), ModulationDestination::Osc1UniDetune, "Osc1 UniDetune");
                                                            ui.selectable_value(&mut *mod_dest_4_tracker.lock().unwrap(), ModulationDestination::Osc2UniDetune, "Osc2 UniDetune");
                                                            ui.selectable_value(&mut *mod_dest_4_tracker.lock().unwrap(), ModulationDestination::Osc3UniDetune, "Osc3 UniDetune");
                                                        });
                                                        // This was a workaround for updating combobox on preset load but otherwise updating preset through combobox selection
                                                        if *mod_dest_override_4.lock().unwrap() != ModulationDestination::UnsetModulation {
                                                            // This happens on plugin preset load
                                                            *mod_dest_4_tracker.lock().unwrap() = *mod_dest_override_4.lock().unwrap();
                                                            setter.set_parameter( &params.mod_destination_4, mod_dest_4_tracker.lock().unwrap().clone());
                                                            *mod_dest_override_4.lock().unwrap() = ModulationDestination::UnsetModulation;
                                                        } else {
                                                            if *mod_dest_4_tracker.lock().unwrap() != params.mod_destination_4.value() {
                                                                setter.set_parameter( &params.mod_destination_4, mod_dest_4_tracker.lock().unwrap().clone());
                                                            }
                                                        }
                                                    });
                                                    ui.separator();
                                                });
                                            },
                                            LFOSelect::INFO => {
                                                ui.horizontal(|ui| {
                                                    let text_response = ui.add(
                                                        egui::TextEdit::singleline(&mut *arc_preset_name.lock().unwrap())
                                                            .interactive(true)
                                                            .hint_text("Preset Name")
                                                            .desired_width(120.0));
                                                    if text_response.clicked() {
                                                        let mut temp_lock = arc_preset_name.lock().unwrap();

                                                        //TFD
                                                        match tinyfiledialogs::input_box("Rename preset", "Preset name:", &*temp_lock) {
                                                            Some(input) => *temp_lock = input,
                                                            None => {},
                                                        }
                                                    }
                                                    ui.label(RichText::new("Type")
                                                            .font(FONT)
                                                            .size(12.0)
                                                            .color(Color32::BLACK));
                                                        egui::ComboBox::new("preset_category", "")
                                                        .selected_text(format!("{:?}", *preset_category_tracker.lock().unwrap()))
                                                        .width(100.0)
                                                        .show_ui(ui, |ui|{
                                                            ui.selectable_value(&mut *preset_category_tracker.lock().unwrap(), PresetType::Select, "Select");
                                                            ui.selectable_value(&mut *preset_category_tracker.lock().unwrap(), PresetType::Atmosphere, "Atmosphere");
                                                            ui.selectable_value(&mut *preset_category_tracker.lock().unwrap(), PresetType::Bass, "Bass");
                                                            ui.selectable_value(&mut *preset_category_tracker.lock().unwrap(), PresetType::FX, "FX");
                                                            ui.selectable_value(&mut *preset_category_tracker.lock().unwrap(), PresetType::Keys, "Keys");
                                                            ui.selectable_value(&mut *preset_category_tracker.lock().unwrap(), PresetType::Lead, "Lead");
                                                            ui.selectable_value(&mut *preset_category_tracker.lock().unwrap(), PresetType::Pad, "Pad");
                                                            ui.selectable_value(&mut *preset_category_tracker.lock().unwrap(), PresetType::Percussion, "Percussion");
                                                            ui.selectable_value(&mut *preset_category_tracker.lock().unwrap(), PresetType::Pluck, "Pluck");
                                                            ui.selectable_value(&mut *preset_category_tracker.lock().unwrap(), PresetType::Synth, "Synth");
                                                            ui.selectable_value(&mut *preset_category_tracker.lock().unwrap(), PresetType::Other, "Other");
                                                        });
                                                        // This was a workaround for updating combobox on preset load but otherwise updating preset through combobox selection
                                                        if *preset_category_override.lock().unwrap() != PresetType::Select {
                                                            // This happens on plugin preset load
                                                            *preset_category_tracker.lock().unwrap() = *preset_category_override.lock().unwrap();
                                                            setter.set_parameter( &params.preset_category, preset_category_tracker.lock().unwrap().clone());
                                                            *preset_category_override.lock().unwrap() = PresetType::Select;
                                                        } else {
                                                            if *preset_category_tracker.lock().unwrap() != params.preset_category.value() {
                                                                setter.set_parameter( &params.preset_category, preset_category_tracker.lock().unwrap().clone());
                                                            }
                                                        }
                                                });

                                                ui.horizontal(|ui|{
                                                    let text_info_response = ui.add(
                                                        egui::TextEdit::multiline(&mut *arc_preset_info.lock().unwrap())
                                                            .interactive(true)
                                                            .hint_text("Preset Info")
                                                            .desired_width(120.0)
                                                            .desired_rows(5)
                                                            .lock_focus(true));
                                                    if text_info_response.clicked() {
                                                        let mut temp_lock = arc_preset_info.lock().unwrap();

                                                        //TFD
                                                        match tinyfiledialogs::input_box("Update preset description", "Preset description:", &*temp_lock) {
                                                            Some(input) => *temp_lock = input,
                                                            None => {},
                                                        }
                                                    }
                                                    ScrollArea::vertical()
                                                        .max_height(220.0)
                                                        .max_width(164.0)
                                                        //.always_show_scroll(true)
                                                        .show(ui, |ui|{
                                                            ui.vertical(|ui|{
                                                                ui.horizontal(|ui|{
                                                                    let tag_acid = BoolButton::BoolButton::for_param(&params.tag_acid, setter, 2.0, 0.9, SMALLER_FONT);
                                                                    ui.add(tag_acid);
                                                                    let tag_analog = BoolButton::BoolButton::for_param(&params.tag_analog, setter, 2.0, 0.9, SMALLER_FONT);
                                                                    ui.add(tag_analog);
                                                                    let tag_bright = BoolButton::BoolButton::for_param(&params.tag_bright, setter, 2.0, 0.9, SMALLER_FONT);
                                                                    ui.add(tag_bright);
                                                                });
                                                                ui.horizontal(|ui|{
                                                                    let tag_chord = BoolButton::BoolButton::for_param(&params.tag_chord, setter, 2.0, 0.9, SMALLER_FONT);
                                                                    ui.add(tag_chord);
                                                                    let tag_crisp = BoolButton::BoolButton::for_param(&params.tag_crisp, setter, 2.0, 0.9, SMALLER_FONT);
                                                                    ui.add(tag_crisp);
                                                                    let tag_deep = BoolButton::BoolButton::for_param(&params.tag_deep, setter, 2.0, 0.9, SMALLER_FONT);
                                                                    ui.add(tag_deep);
                                                                });
                                                                ui.horizontal(|ui|{
                                                                    let tag_delicate = BoolButton::BoolButton::for_param(&params.tag_delicate, setter, 2.7, 0.9, SMALLER_FONT);
                                                                    ui.add(tag_delicate);
                                                                    let tag_hard = BoolButton::BoolButton::for_param(&params.tag_hard, setter, 2.0, 0.9, SMALLER_FONT);
                                                                    ui.add(tag_hard);
                                                                });
                                                                ui.horizontal(|ui|{
                                                                    let tag_harsh = BoolButton::BoolButton::for_param(&params.tag_harsh, setter, 2.0, 0.9, SMALLER_FONT);
                                                                    ui.add(tag_harsh);
                                                                    let tag_lush = BoolButton::BoolButton::for_param(&params.tag_lush, setter, 2.0, 0.9, SMALLER_FONT);
                                                                    ui.add(tag_lush);
                                                                    let tag_mellow = BoolButton::BoolButton::for_param(&params.tag_mellow, setter, 2.0, 0.9, SMALLER_FONT);
                                                                    ui.add(tag_mellow);
                                                                });
                                                                ui.horizontal(|ui|{
                                                                    let tag_resonant = BoolButton::BoolButton::for_param(&params.tag_resonant, setter, 2.7, 0.9, SMALLER_FONT);
                                                                    ui.add(tag_resonant);
                                                                    let tag_rich = BoolButton::BoolButton::for_param(&params.tag_rich, setter, 2.0, 0.9, SMALLER_FONT);
                                                                    ui.add(tag_rich);
                                                                });
                                                                ui.horizontal(|ui|{
                                                                    let tag_sharp = BoolButton::BoolButton::for_param(&params.tag_sharp, setter, 2.0, 0.9, SMALLER_FONT);
                                                                    ui.add(tag_sharp);
                                                                    let tag_silky = BoolButton::BoolButton::for_param(&params.tag_silky, setter, 2.0, 0.9, SMALLER_FONT);
                                                                    ui.add(tag_silky);
                                                                    let tag_smooth = BoolButton::BoolButton::for_param(&params.tag_smooth, setter, 2.0, 0.9, SMALLER_FONT);
                                                                    ui.add(tag_smooth);
                                                                });
                                                                ui.horizontal(|ui|{
                                                                    let tag_soft = BoolButton::BoolButton::for_param(&params.tag_soft, setter, 2.0, 0.9, SMALLER_FONT);
                                                                    ui.add(tag_soft);
                                                                    let tag_stab = BoolButton::BoolButton::for_param(&params.tag_stab, setter, 2.0, 0.9, SMALLER_FONT);
                                                                    ui.add(tag_stab);
                                                                    let tag_warm = BoolButton::BoolButton::for_param(&params.tag_warm, setter, 2.0, 0.9, SMALLER_FONT);
                                                                    ui.add(tag_warm);
                                                                });
                                                            });
                                                        });
                                                });
                                                ui.horizontal(|ui| {
                                                    let update_current_preset = BoolButton::BoolButton::for_param(&params.param_update_current_preset, setter, 4.9, 0.9, SMALLER_FONT)
                                                        .with_background_color(DARK_GREY_UI_COLOR);
                                                    ui.add(update_current_preset);
                                                    let import_preset = BoolButton::BoolButton::for_param(&params.param_import_preset, setter, 4.9, 0.9, SMALLER_FONT)
                                                        .with_background_color(DARK_GREY_UI_COLOR);
                                                    ui.add(import_preset);
                                                    let export_preset = BoolButton::BoolButton::for_param(&params.param_export_preset, setter, 4.9, 0.9, SMALLER_FONT)
                                                        .with_background_color(DARK_GREY_UI_COLOR);
                                                    ui.add(export_preset);
                                                });
                                            },
                                            LFOSelect::FX => {
                                                ScrollArea::vertical()
                                                    .max_height(200.0)
                                                    .max_width(270.0)
                                                    //.always_show_scroll(true)
                                                    .show(ui, |ui|{
                                                        ui.vertical(|ui|{
                                                            // Equalizer
                                                            ui.horizontal(|ui|{
                                                                ui.vertical(|ui|{
                                                                    ui.label(RichText::new("EQ")
                                                                        .font(FONT)
                                                                        .color(Color32::BLACK)
                                                                    )
                                                                        .on_hover_text("non interleaved EQ from Interleaf");
                                                                    let use_eq_toggle = toggle_switch::ToggleSwitch::for_param(&params.pre_use_eq, setter);
                                                                    ui.add(use_eq_toggle);
                                                                });
                                                                ui.vertical(|ui|{
                                                                    ui.add(
                                                                        VerticalParamSlider::for_param(&params.pre_low_gain, setter)
                                                                            .with_width(VERT_BAR_WIDTH * 2.2)
                                                                            .with_height(VERT_BAR_HEIGHT * 0.6)
                                                                            .set_reversed(true)
                                                                            .override_colors(
                                                                                SYNTH_BARS_PURPLE,
                                                                                A_KNOB_OUTSIDE_COLOR,
                                                                            ),
                                                                    );
                                                                    let low_freq_knob = ui_knob::ArcKnob::for_param(
                                                                        &params.pre_low_freq,
                                                                        setter,
                                                                        KNOB_SIZE * 0.6)
                                                                        .preset_style(ui_knob::KnobStyle::NewPresets1)
                                                                        .set_fill_color(SYNTH_BARS_PURPLE)
                                                                        .set_line_color(A_KNOB_OUTSIDE_COLOR)
                                                                        .set_text_size(TEXT_SIZE)
                                                                        .override_text_color(Color32::DARK_GRAY);
                                                                    ui.add(low_freq_knob);
                                                                });
                                                                ui.vertical(|ui|{
                                                                    ui.add(
                                                                        VerticalParamSlider::for_param(&params.pre_mid_gain, setter)
                                                                            .with_width(VERT_BAR_WIDTH * 2.2)
                                                                            .with_height(VERT_BAR_HEIGHT * 0.6)
                                                                            .set_reversed(true)
                                                                            .override_colors(
                                                                                SYNTH_BARS_PURPLE,
                                                                                A_KNOB_OUTSIDE_COLOR,
                                                                            ),
                                                                    );
                                                                    let mid_freq_knob = ui_knob::ArcKnob::for_param(
                                                                        &params.pre_mid_freq,
                                                                        setter,
                                                                        KNOB_SIZE * 0.6)
                                                                        .preset_style(ui_knob::KnobStyle::NewPresets1)
                                                                        .set_fill_color(SYNTH_BARS_PURPLE)
                                                                        .set_line_color(A_KNOB_OUTSIDE_COLOR)
                                                                        .set_text_size(TEXT_SIZE)
                                                                        .override_text_color(Color32::DARK_GRAY);
                                                                    ui.add(mid_freq_knob);
                                                                });
                                                                ui.vertical(|ui|{
                                                                    ui.add(
                                                                        VerticalParamSlider::for_param(&params.pre_high_gain, setter)
                                                                            .with_width(VERT_BAR_WIDTH * 2.2)
                                                                            .with_height(VERT_BAR_HEIGHT * 0.6)
                                                                            .set_reversed(true)
                                                                            .override_colors(
                                                                                SYNTH_BARS_PURPLE,
                                                                                A_KNOB_OUTSIDE_COLOR,
                                                                            ),
                                                                    );
                                                                    let high_freq_knob = ui_knob::ArcKnob::for_param(
                                                                        &params.pre_high_freq,
                                                                        setter,
                                                                        KNOB_SIZE * 0.6)
                                                                        .preset_style(ui_knob::KnobStyle::NewPresets1)
                                                                        .set_fill_color(SYNTH_BARS_PURPLE)
                                                                        .set_line_color(A_KNOB_OUTSIDE_COLOR)
                                                                        .set_text_size(TEXT_SIZE)
                                                                        .override_text_color(Color32::DARK_GRAY);
                                                                    ui.add(high_freq_knob);
                                                                });
                                                                ui.separator();
                                                            });
                                                            // Compressor
                                                            ui.horizontal(|ui|{
                                                                ui.label(RichText::new("Compressor")
                                                                    .font(FONT)
                                                                    .color(Color32::BLACK));
                                                                let use_comp_toggle = toggle_switch::ToggleSwitch::for_param(&params.use_compressor, setter);
                                                                ui.add(use_comp_toggle);
                                                            });
                                                            ui.vertical(|ui|{
                                                                ui.add(CustomParamSlider::ParamSlider::for_param(&params.comp_amt, setter)
                                                                    .slimmer(0.7)
                                                                    .set_left_sided_label(true)
                                                                    .set_label_width(84.0)
                                                                    .with_width(140.0));
                                                                ui.add(CustomParamSlider::ParamSlider::for_param(&params.comp_atk, setter)
                                                                    .slimmer(0.7)
                                                                    .set_left_sided_label(true)
                                                                    .set_label_width(84.0)
                                                                    .with_width(140.0));
                                                                ui.add(CustomParamSlider::ParamSlider::for_param(&params.comp_rel, setter)
                                                                    .slimmer(0.7)
                                                                    .set_left_sided_label(true)
                                                                    .set_label_width(84.0)
                                                                    .with_width(140.0));
                                                                ui.add(CustomParamSlider::ParamSlider::for_param(&params.comp_drive, setter)
                                                                    .slimmer(0.7)
                                                                    .set_left_sided_label(true)
                                                                    .set_label_width(84.0)
                                                                    .with_width(140.0));
                                                            });
                                                            ui.separator();
                                                            // ABass
                                                            ui.horizontal(|ui|{
                                                                ui.label(RichText::new("ABass Algorithm")
                                                                    .font(FONT)
                                                                    .color(Color32::BLACK));
                                                                let use_abass_toggle = toggle_switch::ToggleSwitch::for_param(&params.use_abass, setter);
                                                                ui.add(use_abass_toggle);
                                                            });
                                                            ui.vertical(|ui|{
                                                                ui.add(CustomParamSlider::ParamSlider::for_param(&params.abass_amount, setter)
                                                                    .slimmer(0.7)
                                                                    .set_left_sided_label(true)
                                                                    .set_label_width(84.0)
                                                                    .with_width(140.0));
                                                            });
                                                            ui.separator();
                                                            // Saturation
                                                            ui.horizontal(|ui|{
                                                                ui.label(RichText::new("Saturation")
                                                                    .font(FONT)
                                                                    .color(Color32::BLACK));
                                                                let use_sat_toggle = toggle_switch::ToggleSwitch::for_param(&params.use_saturation, setter);
                                                                ui.add(use_sat_toggle);
                                                            });
                                                            ui.vertical(|ui|{
                                                                ui.add(CustomParamSlider::ParamSlider::for_param(&params.sat_type, setter)
                                                                    .slimmer(0.7)
                                                                    .set_left_sided_label(true)
                                                                    .set_label_width(84.0)
                                                                    .with_width(140.0));
                                                                ui.add(CustomParamSlider::ParamSlider::for_param(&params.sat_amt, setter)
                                                                    .slimmer(0.7)
                                                                    .set_left_sided_label(true)
                                                                    .set_label_width(84.0)
                                                                    .with_width(140.0));
                                                            });
                                                            ui.separator();
                                                            // Phaser
                                                            ui.horizontal(|ui|{
                                                                ui.label(RichText::new("Phaser")
                                                                    .font(FONT)
                                                                    .color(Color32::BLACK));
                                                                let use_phaser_toggle = toggle_switch::ToggleSwitch::for_param(&params.use_phaser, setter);
                                                                ui.add(use_phaser_toggle);
                                                            });
                                                            ui.vertical(|ui|{
                                                                ui.add(CustomParamSlider::ParamSlider::for_param(&params.phaser_amount, setter)
                                                                    .slimmer(0.7)
                                                                    .set_left_sided_label(true)
                                                                    .set_label_width(84.0)
                                                                    .with_width(140.0));
                                                                ui.add(CustomParamSlider::ParamSlider::for_param(&params.phaser_depth, setter)
                                                                    .slimmer(0.7)
                                                                    .set_left_sided_label(true)
                                                                    .set_label_width(84.0)
                                                                    .with_width(140.0));
                                                                ui.add(CustomParamSlider::ParamSlider::for_param(&params.phaser_rate, setter)
                                                                    .slimmer(0.7)
                                                                    .set_left_sided_label(true)
                                                                    .set_label_width(84.0)
                                                                    .with_width(140.0));
                                                            });
                                                            ui.separator();
                                                            // Flanger
                                                            ui.horizontal(|ui|{
                                                                ui.label(RichText::new("Flanger")
                                                                    .font(FONT)
                                                                    .color(Color32::BLACK));
                                                                let use_flanger_toggle = toggle_switch::ToggleSwitch::for_param(&params.use_flanger, setter);
                                                                ui.add(use_flanger_toggle);
                                                            });
                                                            ui.vertical(|ui|{
                                                                ui.add(CustomParamSlider::ParamSlider::for_param(&params.flanger_amount, setter)
                                                                    .slimmer(0.7)
                                                                    .set_left_sided_label(true)
                                                                    .set_label_width(84.0)
                                                                    .with_width(140.0));
                                                                ui.add(CustomParamSlider::ParamSlider::for_param(&params.flanger_depth, setter)
                                                                    .slimmer(0.7)
                                                                    .set_left_sided_label(true)
                                                                    .set_label_width(84.0)
                                                                    .with_width(140.0));
                                                                ui.add(CustomParamSlider::ParamSlider::for_param(&params.flanger_rate, setter)
                                                                    .slimmer(0.7)
                                                                    .set_left_sided_label(true)
                                                                    .set_label_width(84.0)
                                                                    .with_width(140.0));
                                                                ui.add(CustomParamSlider::ParamSlider::for_param(&params.flanger_feedback, setter)
                                                                    .slimmer(0.7)
                                                                    .set_left_sided_label(true)
                                                                    .set_label_width(84.0)
                                                                    .with_width(140.0));
                                                            });
                                                            ui.separator();
                                                            // Buffer Modulator
                                                            ui.horizontal(|ui|{
                                                                ui.label(RichText::new("Buffer Modulator")
                                                                    .font(FONT)
                                                                    .color(Color32::BLACK));
                                                                let use_buffermod_toggle = toggle_switch::ToggleSwitch::for_param(&params.use_buffermod, setter);
                                                                ui.add(use_buffermod_toggle);
                                                            });
                                                            ui.vertical(|ui|{
                                                                ui.add(CustomParamSlider::ParamSlider::for_param(&params.buffermod_amount, setter)
                                                                    .slimmer(0.7)
                                                                    .set_left_sided_label(true)
                                                                    .set_label_width(84.0)
                                                                    .with_width(140.0));
                                                                ui.add(CustomParamSlider::ParamSlider::for_param(&params.buffermod_depth, setter)
                                                                    .slimmer(0.7)
                                                                    .set_left_sided_label(true)
                                                                    .set_label_width(84.0)
                                                                    .with_width(140.0));
                                                                ui.add(CustomParamSlider::ParamSlider::for_param(&params.buffermod_rate, setter)
                                                                    .slimmer(0.7)
                                                                    .set_left_sided_label(true)
                                                                    .set_label_width(84.0)
                                                                    .with_width(140.0));
                                                                ui.add(CustomParamSlider::ParamSlider::for_param(&params.buffermod_spread, setter)
                                                                    .slimmer(0.7)
                                                                    .set_left_sided_label(true)
                                                                    .set_label_width(84.0)
                                                                    .with_width(140.0));
                                                                ui.add(CustomParamSlider::ParamSlider::for_param(&params.buffermod_timing, setter)
                                                                    .slimmer(0.7)
                                                                    .set_left_sided_label(true)
                                                                    .set_label_width(84.0)
                                                                    .with_width(140.0));
                                                            });
                                                            ui.separator();
                                                            // Delay
                                                            ui.horizontal(|ui|{
                                                                ui.label(RichText::new("Delay")
                                                                    .font(FONT)
                                                                    .color(Color32::BLACK));
                                                                let use_delay_toggle = toggle_switch::ToggleSwitch::for_param(&params.use_delay, setter);
                                                                ui.add(use_delay_toggle);
                                                            });
                                                            ui.vertical(|ui|{
                                                                ui.add(CustomParamSlider::ParamSlider::for_param(&params.delay_amount, setter)
                                                                    .slimmer(0.7)
                                                                    .set_left_sided_label(true)
                                                                    .set_label_width(84.0)
                                                                    .with_width(140.0));
                                                                ui.add(CustomParamSlider::ParamSlider::for_param(&params.delay_time, setter)
                                                                    .slimmer(0.7)
                                                                    .set_left_sided_label(true)
                                                                    .set_label_width(84.0)
                                                                    .with_width(140.0));
                                                                ui.add(CustomParamSlider::ParamSlider::for_param(&params.delay_decay, setter)
                                                                    .slimmer(0.7)
                                                                    .set_left_sided_label(true)
                                                                    .set_label_width(84.0)
                                                                    .with_width(140.0));
                                                                ui.add(CustomParamSlider::ParamSlider::for_param(&params.delay_type, setter)
                                                                    .slimmer(0.7)
                                                                    .set_left_sided_label(true)
                                                                    .set_label_width(84.0)
                                                                    .with_width(140.0));
                                                            });
                                                            ui.separator();
                                                            // Reverb
                                                            ui.horizontal(|ui|{
                                                                ui.label(RichText::new("Reverb")
                                                                    .font(FONT)
                                                                    .color(Color32::BLACK));
                                                                let use_reverb_toggle = toggle_switch::ToggleSwitch::for_param(&params.use_reverb, setter);
                                                                ui.add(use_reverb_toggle);
                                                            });
                                                            ui.vertical(|ui|{
                                                                ui.add(CustomParamSlider::ParamSlider::for_param(&params.reverb_amount, setter)
                                                                    .slimmer(0.7)
                                                                    .set_left_sided_label(true)
                                                                    .set_label_width(84.0)
                                                                    .with_width(140.0));
                                                                ui.add(CustomParamSlider::ParamSlider::for_param(&params.reverb_size, setter)
                                                                    .slimmer(0.7)
                                                                    .set_left_sided_label(true)
                                                                    .set_label_width(84.0)
                                                                    .with_width(140.0));
                                                                ui.add(CustomParamSlider::ParamSlider::for_param(&params.reverb_feedback, setter)
                                                                    .slimmer(0.7)
                                                                    .set_left_sided_label(true)
                                                                    .set_label_width(84.0)
                                                                    .with_width(140.0));
                                                            });
                                                            ui.separator();
                                                            // Limiter
                                                            ui.horizontal(|ui|{
                                                                ui.label(RichText::new("Limiter")
                                                                    .font(FONT)
                                                                    .color(Color32::BLACK));
                                                                let use_limiter_toggle = toggle_switch::ToggleSwitch::for_param(&params.use_limiter, setter);
                                                                ui.add(use_limiter_toggle);
                                                            });
                                                            ui.vertical(|ui|{
                                                                ui.add(CustomParamSlider::ParamSlider::for_param(&params.limiter_threshold, setter)
                                                                    .slimmer(0.7)
                                                                    .set_left_sided_label(true)
                                                                    .set_label_width(84.0)
                                                                    .with_width(140.0));
                                                                ui.add(CustomParamSlider::ParamSlider::for_param(&params.limiter_knee, setter)
                                                                    .slimmer(0.7)
                                                                    .set_left_sided_label(true)
                                                                    .set_label_width(84.0)
                                                                    .with_width(140.0));
                                                            });
                                                        });
                                                    })
                                                    .inner;
                                            }
                                        }
                                    });
                                });
                            });

                            // Synth Bars on left and right
                            ui.painter().rect_filled(
                                Rect::from_x_y_ranges(
                                    RangeInclusive::new(WIDTH as f32 - synth_bar_space, WIDTH as f32),
                                    RangeInclusive::new(0.0, HEIGHT as f32)),
                                Rounding::none(),
                                SYNTH_BARS_PURPLE
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
            // This is the end of create_egui_editor()
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

    // Main processing thread that happens before the midi processing per-sample
    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        // Clear any voices on change of module type (especially during play)
        // This fixes panics and other broken things attempting to play during preset change/load
        if self.clear_voices.clone().load(Ordering::Relaxed) {
            self.audio_module_1.lock().unwrap().clear_voices();
            self.audio_module_2.lock().unwrap().clear_voices();
            self.audio_module_3.lock().unwrap().clear_voices();

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
                    self.file_dialog.store(false, Ordering::Relaxed); //Changed from Release
                }
            }

            // If Import Preset button was pressed
            if *self.import_preset.lock().unwrap()
                && !self.file_dialog.load(Ordering::Relaxed)
                && self.file_open_buffer_timer.load(Ordering::Relaxed) == 0
                && !*self.reload_entire_preset.lock().unwrap()
            {
                // This is up here so it happens first - putting this down by
                // (self.preset_lib_name, unserialized) = Actuate::load_preset_bank();
                // Makes this block happen twice?
                *self.import_preset.lock().unwrap() = false;
                // This is here again purposefully
                *self.reload_entire_preset.lock().unwrap() = true;

                let unserialized: Option<ActuatePreset>;
                (_, unserialized) = Actuate::import_preset();

                if unserialized.is_some() {
                    let arc_lib: Arc<Mutex<Vec<ActuatePreset>>> = Arc::clone(&self.preset_lib);
                    let mut locked_lib = arc_lib.lock().unwrap();
                    locked_lib[self.current_preset.load(Ordering::Relaxed) as usize] =
                        unserialized.unwrap();
                    let temp_preset =
                        &locked_lib[self.current_preset.load(Ordering::Relaxed) as usize];
                    *self.preset_name.lock().unwrap() = temp_preset.preset_name.clone();
                    *self.preset_info.lock().unwrap() = temp_preset.preset_info.clone();
                    *self.preset_category.lock().unwrap() = temp_preset.preset_category.clone();
                }
            }

            // If the Load Bank button was pressed
            if *self.load_bank.lock().unwrap()
                && !self.file_dialog.load(Ordering::Relaxed)
                && self.file_open_buffer_timer.load(Ordering::Relaxed) == 0
                && !*self.reload_entire_preset.lock().unwrap()
            {
                // This is up here so it happens first - putting this down by
                // (self.preset_lib_name, unserialized) = Actuate::load_preset_bank();
                // Makes this block happen twice?
                *self.load_bank.lock().unwrap() = false;
                // This is here again purposefully
                *self.reload_entire_preset.lock().unwrap() = true;

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

                let temp_current_preset =
                    locked_lib[self.current_preset.load(Ordering::Relaxed) as usize].clone();
                *self.preset_name.lock().unwrap() = temp_current_preset.preset_name;
                *self.preset_info.lock().unwrap() = temp_current_preset.preset_info;
                *self.preset_category.lock().unwrap() = temp_current_preset.preset_category;
            }

            // If Export Preset button has been pressed
            if *self.export_preset.lock().unwrap()
                && !self.file_dialog.load(Ordering::Relaxed)
                && self.file_open_buffer_timer.load(Ordering::Relaxed) == 0
                && !*self.reload_entire_preset.lock().unwrap()
            {
                // Move the mutex update as early as possible
                *self.export_preset.lock().unwrap() = false;
                // This is here again purposefully
                *self.reload_entire_preset.lock().unwrap() = true;

                self.file_dialog.store(true, Ordering::Relaxed);
                self.file_open_buffer_timer.store(1, Ordering::Relaxed);
                self.export_preset();
            }

            // If the save button has been pressed
            if *self.save_bank.lock().unwrap()
                && !self.file_dialog.load(Ordering::Relaxed)
                && self.file_open_buffer_timer.load(Ordering::Relaxed) == 0
                && !*self.reload_entire_preset.lock().unwrap()
            {
                // Move the mutex update as early as possible
                *self.save_bank.lock().unwrap() = false;
                // This is here again purposefully
                *self.reload_entire_preset.lock().unwrap() = true;

                self.file_dialog.store(true, Ordering::Relaxed);
                self.file_open_buffer_timer.store(1, Ordering::Relaxed);
                self.save_preset_bank();
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
                                self.current_note_on_velocity.store(vel, Ordering::Relaxed);
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
                                    .store(velocity, Ordering::Relaxed);
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
                                    .store(velocity, Ordering::Relaxed);
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
                                    .store(velocity, Ordering::Relaxed);
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
                            8000.0 * self.current_note_on_velocity.load(Ordering::Relaxed);
                    }
                    ModulationDestination::Cutoff_2 => {
                        temp_mod_cutoff_2_source_1 +=
                            8000.0 * self.current_note_on_velocity.load(Ordering::Relaxed);
                    }
                    ModulationDestination::All_Gain => {
                        if self.params.mod_source_1.value() == ModulationSource::Velocity {
                            let vel = self.current_note_on_velocity.load(Ordering::Relaxed);
                            temp_mod_gain_1 = vel;
                            temp_mod_gain_2 = vel;
                            temp_mod_gain_3 = vel;
                        }
                    }
                    ModulationDestination::Osc1_Gain => {
                        if self.params.mod_source_1.value() == ModulationSource::Velocity {
                            temp_mod_gain_1 = self.current_note_on_velocity.load(Ordering::Relaxed);
                        }
                    }
                    ModulationDestination::Osc2_Gain => {
                        if self.params.mod_source_1.value() == ModulationSource::Velocity {
                            temp_mod_gain_2 = self.current_note_on_velocity.load(Ordering::Relaxed);
                        }
                    }
                    ModulationDestination::Osc3_Gain => {
                        if self.params.mod_source_1.value() == ModulationSource::Velocity {
                            temp_mod_gain_3 = self.current_note_on_velocity.load(Ordering::Relaxed);
                        }
                    }
                    ModulationDestination::Resonance_1 => {
                        if self.params.mod_source_1.value() == ModulationSource::Velocity {
                            temp_mod_resonance_1_source_1 +=
                                self.current_note_on_velocity.load(Ordering::Relaxed);
                        }
                    }
                    ModulationDestination::Resonance_2 => {
                        if self.params.mod_source_1.value() == ModulationSource::Velocity {
                            temp_mod_resonance_2_source_1 +=
                                self.current_note_on_velocity.load(Ordering::Relaxed);
                        }
                    }
                    _ => {}
                }
            }
            if self.params.mod_source_2.value() == ModulationSource::Velocity {
                match self.params.mod_destination_2.value() {
                    ModulationDestination::Cutoff_1 => {
                        temp_mod_cutoff_1_source_2 +=
                            8000.0 * self.current_note_on_velocity.load(Ordering::Relaxed);
                    }
                    ModulationDestination::Cutoff_2 => {
                        temp_mod_cutoff_2_source_2 +=
                            8000.0 * self.current_note_on_velocity.load(Ordering::Relaxed);
                    }
                    ModulationDestination::All_Gain => {
                        if self.params.mod_source_2.value() == ModulationSource::Velocity {
                            let vel = self.current_note_on_velocity.load(Ordering::Relaxed);
                            temp_mod_gain_1 = vel;
                            temp_mod_gain_2 = vel;
                            temp_mod_gain_3 = vel;
                        }
                    }
                    ModulationDestination::Osc1_Gain => {
                        if self.params.mod_source_2.value() == ModulationSource::Velocity {
                            temp_mod_gain_1 = self.current_note_on_velocity.load(Ordering::Relaxed);
                        }
                    }
                    ModulationDestination::Osc2_Gain => {
                        if self.params.mod_source_2.value() == ModulationSource::Velocity {
                            temp_mod_gain_2 = self.current_note_on_velocity.load(Ordering::Relaxed);
                        }
                    }
                    ModulationDestination::Osc3_Gain => {
                        if self.params.mod_source_2.value() == ModulationSource::Velocity {
                            temp_mod_gain_3 = self.current_note_on_velocity.load(Ordering::Relaxed);
                        }
                    }
                    ModulationDestination::Resonance_1 => {
                        if self.params.mod_source_2.value() == ModulationSource::Velocity {
                            temp_mod_resonance_1_source_2 -=
                                self.current_note_on_velocity.load(Ordering::Relaxed);
                        }
                    }
                    ModulationDestination::Resonance_2 => {
                        if self.params.mod_source_2.value() == ModulationSource::Velocity {
                            temp_mod_resonance_2_source_2 -=
                                self.current_note_on_velocity.load(Ordering::Relaxed);
                        }
                    }
                    _ => {}
                }
            }
            if self.params.mod_source_3.value() == ModulationSource::Velocity {
                match self.params.mod_destination_3.value() {
                    ModulationDestination::Cutoff_1 => {
                        temp_mod_cutoff_1_source_3 +=
                            8000.0 * self.current_note_on_velocity.load(Ordering::Relaxed);
                    }
                    ModulationDestination::Cutoff_2 => {
                        temp_mod_cutoff_2_source_3 +=
                            8000.0 * self.current_note_on_velocity.load(Ordering::Relaxed);
                    }
                    ModulationDestination::All_Gain => {
                        if self.params.mod_source_3.value() == ModulationSource::Velocity {
                            let vel = self.current_note_on_velocity.load(Ordering::Relaxed);
                            temp_mod_gain_1 = vel;
                            temp_mod_gain_2 = vel;
                            temp_mod_gain_3 = vel;
                        }
                    }
                    ModulationDestination::Osc1_Gain => {
                        if self.params.mod_source_3.value() == ModulationSource::Velocity {
                            temp_mod_gain_1 = self.current_note_on_velocity.load(Ordering::Relaxed);
                        }
                    }
                    ModulationDestination::Osc2_Gain => {
                        if self.params.mod_source_3.value() == ModulationSource::Velocity {
                            temp_mod_gain_2 = self.current_note_on_velocity.load(Ordering::Relaxed);
                        }
                    }
                    ModulationDestination::Osc3_Gain => {
                        if self.params.mod_source_3.value() == ModulationSource::Velocity {
                            temp_mod_gain_3 = self.current_note_on_velocity.load(Ordering::Relaxed);
                        }
                    }
                    ModulationDestination::Resonance_1 => {
                        if self.params.mod_source_3.value() == ModulationSource::Velocity {
                            temp_mod_resonance_1_source_3 +=
                                self.current_note_on_velocity.load(Ordering::Relaxed);
                        }
                    }
                    ModulationDestination::Resonance_2 => {
                        if self.params.mod_source_3.value() == ModulationSource::Velocity {
                            temp_mod_resonance_2_source_3 +=
                                self.current_note_on_velocity.load(Ordering::Relaxed);
                        }
                    }
                    _ => {}
                }
            }
            if self.params.mod_source_4.value() == ModulationSource::Velocity {
                match self.params.mod_destination_4.value() {
                    ModulationDestination::Cutoff_1 => {
                        temp_mod_cutoff_1_source_4 +=
                            8000.0 * self.current_note_on_velocity.load(Ordering::Relaxed);
                    }
                    ModulationDestination::Cutoff_2 => {
                        temp_mod_cutoff_2_source_4 +=
                            8000.0 * self.current_note_on_velocity.load(Ordering::Relaxed);
                    }
                    ModulationDestination::All_Gain => {
                        if self.params.mod_source_4.value() == ModulationSource::Velocity {
                            let vel = self.current_note_on_velocity.load(Ordering::Relaxed);
                            temp_mod_gain_1 = vel;
                            temp_mod_gain_2 = vel;
                            temp_mod_gain_3 = vel;
                        }
                    }
                    ModulationDestination::Osc1_Gain => {
                        if self.params.mod_source_4.value() == ModulationSource::Velocity {
                            temp_mod_gain_1 = self.current_note_on_velocity.load(Ordering::Relaxed);
                        }
                    }
                    ModulationDestination::Osc2_Gain => {
                        if self.params.mod_source_4.value() == ModulationSource::Velocity {
                            temp_mod_gain_2 = self.current_note_on_velocity.load(Ordering::Relaxed);
                        }
                    }
                    ModulationDestination::Osc3_Gain => {
                        if self.params.mod_source_4.value() == ModulationSource::Velocity {
                            temp_mod_gain_3 = self.current_note_on_velocity.load(Ordering::Relaxed);
                        }
                    }
                    ModulationDestination::Resonance_1 => {
                        if self.params.mod_source_4.value() == ModulationSource::Velocity {
                            temp_mod_resonance_1_source_4 +=
                                self.current_note_on_velocity.load(Ordering::Relaxed);
                        }
                    }
                    ModulationDestination::Resonance_2 => {
                        if self.params.mod_source_4.value() == ModulationSource::Velocity {
                            temp_mod_resonance_2_source_4 +=
                                self.current_note_on_velocity.load(Ordering::Relaxed);
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
                                20000.0 * self.current_note_on_velocity.load(Ordering::Relaxed);
                        } else {
                            temp_mod_cutoff_1_source_1 += 20000.0 * mod_value_1;
                        }
                    }
                    ModulationDestination::Cutoff_2 => {
                        if self.params.mod_source_1.value() == ModulationSource::Velocity {
                            temp_mod_cutoff_2_source_1 +=
                                20000.0 * self.current_note_on_velocity.load(Ordering::Relaxed);
                        } else {
                            temp_mod_cutoff_2_source_1 += 20000.0 * mod_value_1;
                        }
                    }
                    ModulationDestination::Resonance_1 => {
                        if self.params.mod_source_1.value() == ModulationSource::Velocity {
                            temp_mod_resonance_1_source_1 -=
                                self.current_note_on_velocity.load(Ordering::Relaxed);
                        } else {
                            temp_mod_resonance_1_source_1 -= mod_value_1;
                        }
                    }
                    ModulationDestination::Resonance_2 => {
                        if self.params.mod_source_1.value() == ModulationSource::Velocity {
                            temp_mod_resonance_2_source_1 -=
                                self.current_note_on_velocity.load(Ordering::Relaxed);
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
                            let vel = self.current_note_on_velocity.load(Ordering::Relaxed);
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
                            temp_mod_gain_1 = self.current_note_on_velocity.load(Ordering::Relaxed);
                        } else {
                            temp_mod_lfo_gain_1 = mod_value_1;
                        }
                    }
                    ModulationDestination::Osc2_Gain => {
                        if self.params.mod_source_1.value() == ModulationSource::Velocity {
                            temp_mod_gain_2 = self.current_note_on_velocity.load(Ordering::Relaxed);
                        } else {
                            temp_mod_lfo_gain_2 = mod_value_1;
                        }
                    }
                    ModulationDestination::Osc3_Gain => {
                        if self.params.mod_source_1.value() == ModulationSource::Velocity {
                            temp_mod_gain_3 = self.current_note_on_velocity.load(Ordering::Relaxed);
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
                                20000.0 * self.current_note_on_velocity.load(Ordering::Relaxed);
                        } else {
                            temp_mod_cutoff_1_source_2 += 20000.0 * mod_value_2;
                        }
                    }
                    ModulationDestination::Cutoff_2 => {
                        if self.params.mod_source_2.value() == ModulationSource::Velocity {
                            temp_mod_cutoff_2_source_2 +=
                                20000.0 * self.current_note_on_velocity.load(Ordering::Relaxed);
                        } else {
                            temp_mod_cutoff_2_source_2 += 20000.0 * mod_value_2;
                        }
                    }
                    ModulationDestination::Resonance_1 => {
                        if self.params.mod_source_2.value() == ModulationSource::Velocity {
                            temp_mod_resonance_1_source_2 -=
                                self.current_note_on_velocity.load(Ordering::Relaxed);
                        } else {
                            temp_mod_resonance_1_source_2 -= mod_value_2;
                        }
                    }
                    ModulationDestination::Resonance_2 => {
                        if self.params.mod_source_2.value() == ModulationSource::Velocity {
                            temp_mod_resonance_2_source_2 -=
                                self.current_note_on_velocity.load(Ordering::Relaxed);
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
                            let vel = self.current_note_on_velocity.load(Ordering::Relaxed);
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
                            temp_mod_gain_1 = self.current_note_on_velocity.load(Ordering::Relaxed);
                        } else {
                            temp_mod_lfo_gain_1 = mod_value_2;
                        }
                    }
                    ModulationDestination::Osc2_Gain => {
                        if self.params.mod_source_2.value() == ModulationSource::Velocity {
                            temp_mod_gain_2 = self.current_note_on_velocity.load(Ordering::Relaxed);
                        } else {
                            temp_mod_lfo_gain_2 = mod_value_2;
                        }
                    }
                    ModulationDestination::Osc3_Gain => {
                        if self.params.mod_source_2.value() == ModulationSource::Velocity {
                            temp_mod_gain_3 = self.current_note_on_velocity.load(Ordering::Relaxed);
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
                                20000.0 * self.current_note_on_velocity.load(Ordering::Relaxed);
                        } else {
                            temp_mod_cutoff_1_source_3 += 20000.0 * mod_value_3;
                        }
                    }
                    ModulationDestination::Cutoff_2 => {
                        if self.params.mod_source_3.value() == ModulationSource::Velocity {
                            temp_mod_cutoff_2_source_3 +=
                                20000.0 * self.current_note_on_velocity.load(Ordering::Relaxed);
                        } else {
                            temp_mod_cutoff_2_source_3 += 20000.0 * mod_value_3;
                        }
                    }
                    ModulationDestination::Resonance_1 => {
                        if self.params.mod_source_3.value() == ModulationSource::Velocity {
                            temp_mod_resonance_1_source_3 -=
                                self.current_note_on_velocity.load(Ordering::Relaxed);
                        } else {
                            temp_mod_resonance_1_source_3 -= mod_value_3;
                        }
                    }
                    ModulationDestination::Resonance_2 => {
                        if self.params.mod_source_3.value() == ModulationSource::Velocity {
                            temp_mod_resonance_2_source_3 -=
                                self.current_note_on_velocity.load(Ordering::Relaxed);
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
                            let vel = self.current_note_on_velocity.load(Ordering::Relaxed);
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
                            temp_mod_gain_1 = self.current_note_on_velocity.load(Ordering::Relaxed);
                        } else {
                            temp_mod_lfo_gain_1 = mod_value_3;
                        }
                    }
                    ModulationDestination::Osc2_Gain => {
                        if self.params.mod_source_3.value() == ModulationSource::Velocity {
                            temp_mod_gain_2 = self.current_note_on_velocity.load(Ordering::Relaxed);
                        } else {
                            temp_mod_lfo_gain_2 = mod_value_3;
                        }
                    }
                    ModulationDestination::Osc3_Gain => {
                        if self.params.mod_source_3.value() == ModulationSource::Velocity {
                            temp_mod_gain_3 = self.current_note_on_velocity.load(Ordering::Relaxed);
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
                                20000.0 * self.current_note_on_velocity.load(Ordering::Relaxed);
                        } else {
                            temp_mod_cutoff_1_source_4 += 20000.0 * mod_value_4;
                        }
                    }
                    ModulationDestination::Cutoff_2 => {
                        if self.params.mod_source_4.value() == ModulationSource::Velocity {
                            temp_mod_cutoff_2_source_4 +=
                                20000.0 * self.current_note_on_velocity.load(Ordering::Relaxed);
                        } else {
                            temp_mod_cutoff_2_source_4 += 20000.0 * mod_value_4;
                        }
                    }
                    ModulationDestination::Resonance_1 => {
                        if self.params.mod_source_4.value() == ModulationSource::Velocity {
                            temp_mod_resonance_1_source_4 -=
                                self.current_note_on_velocity.load(Ordering::Relaxed);
                        } else {
                            temp_mod_resonance_1_source_4 -= mod_value_4;
                        }
                    }
                    ModulationDestination::Resonance_2 => {
                        if self.params.mod_source_4.value() == ModulationSource::Velocity {
                            temp_mod_resonance_2_source_4 -=
                                self.current_note_on_velocity.load(Ordering::Relaxed);
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
                            let vel = self.current_note_on_velocity.load(Ordering::Relaxed);
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
                            temp_mod_gain_1 = self.current_note_on_velocity.load(Ordering::Relaxed);
                        } else {
                            temp_mod_lfo_gain_1 = mod_value_4;
                        }
                    }
                    ModulationDestination::Osc2_Gain => {
                        if self.params.mod_source_4.value() == ModulationSource::Velocity {
                            temp_mod_gain_2 = self.current_note_on_velocity.load(Ordering::Relaxed);
                        } else {
                            temp_mod_lfo_gain_2 = mod_value_4;
                        }
                    }
                    ModulationDestination::Osc3_Gain => {
                        if self.params.mod_source_4.value() == ModulationSource::Velocity {
                            temp_mod_gain_3 = self.current_note_on_velocity.load(Ordering::Relaxed);
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
                // I know this isn't a perfect 3rd, but 0.01 is acceptable headroom
                wave1_l *= self.params.audio_module_1_level.value() * 0.33;
                wave1_r *= self.params.audio_module_1_level.value() * 0.33;
            }
            if !self.file_dialog.load(Ordering::Relaxed)
                && self.params._audio_module_2_type.value() != AudioModuleType::Off
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
                wave2_l *= self.params.audio_module_2_level.value() * 0.33;
                wave2_r *= self.params.audio_module_2_level.value() * 0.33;
            }
            if !self.file_dialog.load(Ordering::Relaxed)
                && self.params._audio_module_3_type.value() != AudioModuleType::Off
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
                wave3_l *= self.params.audio_module_3_level.value() * 0.33;
                wave3_r *= self.params.audio_module_3_level.value() * 0.33;
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
                }
                AMFilterRouting::Filter1 => {
                    left_output_filter1 += wave1_l;
                    right_output_filter1 += wave1_r;
                }
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
                }
                AMFilterRouting::Filter1 => {
                    left_output_filter1 += wave2_l;
                    right_output_filter1 += wave2_r;
                }
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
                }
                AMFilterRouting::Filter1 => {
                    left_output_filter1 += wave3_l;
                    right_output_filter1 += wave3_r;
                }
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
                    // Stacked TDLs to make reverb
                    self.reverb[0].set_size(self.params.reverb_size.value(), self.sample_rate);
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
                            self.params.reverb_amount.value(),
                        );
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

            // This is down here for the save function gui and export preset unsetting to work
            if *self.reload_entire_preset.lock().unwrap() && self.params.param_save_bank.value() {
                *self.reload_entire_preset.lock().unwrap() = false;
            }
            if *self.reload_entire_preset.lock().unwrap() && self.params.param_export_preset.value()
            {
                *self.reload_entire_preset.lock().unwrap() = false;
            }
        }
    }

    // import_preset() uses message packing with serde
    fn import_preset() -> (String, Option<ActuatePreset>) {
        let imported_preset = FileDialog::new()
            .add_filter("actuate", &["actuate"])
            .pick_file();
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

            // Deserialize into ActuatePreset - return default empty lib if error
            let mut unserialized: ActuatePreset = rmp_serde::from_slice(&file_string_data)
                .unwrap_or(ActuatePreset {
                    preset_name: "Error Importing".to_string(),
                    preset_info: "Corrupted preset or incompatible version".to_string(),
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

                    // Pitch
                    pitch_enable: false,
                    pitch_env_peak: 0.0,
                    pitch_env_atk_curve: SmoothStyle::Linear,
                    pitch_env_dec_curve: SmoothStyle::Linear,
                    pitch_env_rel_curve: SmoothStyle::Linear,
                    pitch_env_attack: 0.0,
                    pitch_env_decay: 300.0,
                    pitch_env_release: 0.0,
                    pitch_env_sustain: 0.0,
                    pitch_routing: PitchRouting::Osc1,

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
                    delay_amount: 0.0,
                    delay_time: DelaySnapValues::Quarter,
                    delay_decay: 0.0,
                    delay_type: DelayType::Stereo,

                    use_reverb: false,
                    reverb_amount: 0.5,
                    reverb_size: 0.5,
                    reverb_feedback: 0.5,

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
                });
            
            // Attempt to load the previous version preset type
            if unserialized.preset_name.contains("Error") {
                // Try loading the previous preset struct version
                unserialized = load_unserialized_v114(file_string_data.clone());
            }

            // Attempt to load the oldest preset type
            if unserialized.preset_name.contains("Error") {
                // Try loading the previous preset struct version
                unserialized = load_unserialized_old(file_string_data.clone());
            }

            return (return_name, Some(unserialized));
        }
        return (String::from("Error"), Option::None);
    }

    // Load presets uses message packing with serde
    fn load_preset_bank() -> (String, Vec<ActuatePreset>) {
        let loading_bank = FileDialog::new()
            .add_filter("actuatebank", &["actuatebank"]) // Use the same filter as in save_preset_bank
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
                        delay_amount: 0.0,
                        delay_time: DelaySnapValues::Quarter,
                        delay_decay: 0.0,
                        delay_type: DelayType::Stereo,

                        use_reverb: false,
                        reverb_amount: 0.5,
                        reverb_size: 0.5,
                        reverb_feedback: 0.5,

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
                    PRESET_BANK_SIZE
                ]);

            return (return_name, unserialized);
        }
        return (String::from("Error"), Vec::new());
    }

    // This gets triggered to force a load/change and to recalculate sample dependent notes
    fn reload_entire_preset(
        setter: &ParamSetter,
        params: Arc<ActuateParams>,
        current_preset_index: usize,
        arc_preset: Arc<Mutex<Vec<ActuatePreset>>>,
        AMod1: Arc<Mutex<AudioModule>>,
        AMod2: Arc<Mutex<AudioModule>>,
        AMod3: Arc<Mutex<AudioModule>>,
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

    fn export_preset(&mut self) {
        let _updated_preset = self.update_current_preset();
        let saving_preset = FileDialog::new()
            .add_filter("actuate", &["actuate"]) // Use a binary format for audio data
            .set_file_name(&self.preset_name.lock().unwrap().replace(" ", "_"))
            .save_file();

        if let Some(location) = saving_preset {
            // Create our new save file
            let file = File::create(location.clone());

            if let Ok(_file) = file {
                // Serialize our data to a binary format (MessagePack)
                let preset_store = Arc::clone(&self.preset_lib);
                let mut loaded_preset = preset_store.lock().unwrap()
                    [self.current_preset.load(Ordering::Relaxed) as usize]
                    .clone();

                // Clear out our generated notes and only keep the samples themselves
                loaded_preset.mod1_sample_lib.clear();
                loaded_preset.mod2_sample_lib.clear();
                loaded_preset.mod3_sample_lib.clear();

                // Serialize to MessagePack bytes
                let serialized_data = rmp_serde::to_vec::<ActuatePreset>(&loaded_preset);

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
        *self.save_bank.lock().unwrap() = false;
    }

    fn save_preset_bank(&mut self) {
        let _updated_preset = self.update_current_preset();
        let saving_bank = FileDialog::new()
            .add_filter("actuatebank", &["actuatebank"]) // Use a binary format for audio data
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
        *self.save_bank.lock().unwrap() = false;
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
