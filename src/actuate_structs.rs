//actuate_structs.rs

use serde::{Deserialize, Serialize};

use crate::{actuate_enums::{AMFilterRouting, FilterAlgorithms, FilterRouting, ModulationDestination, ModulationSource, PitchRouting, PresetType, ReverbModel, StereoAlgorithm}, audio_module::{AudioModuleType, Oscillator::{self, RetriggerStyle, SmoothStyle, VoiceType}}, fx::{delay::{DelaySnapValues, DelayType}, saturation::SaturationType, ArduraFilter, StateVariableFilter::ResonanceType}, LFOController};

/// Modulation struct for passing mods to audio modules
#[derive(Serialize, Deserialize, Clone)]
pub struct ModulationStruct {
    pub temp_mod_cutoff_1: f32,
    pub temp_mod_cutoff_2: f32,
    pub temp_mod_resonance_1: f32,
    pub temp_mod_resonance_2: f32,
    pub temp_mod_detune_1: f32,
    pub temp_mod_detune_2: f32,
    pub temp_mod_detune_3: f32,
    pub temp_mod_uni_detune_1: f32,
    pub temp_mod_uni_detune_2: f32,
    pub temp_mod_uni_detune_3: f32,
    pub temp_mod_vel_sum: f32,
}

/// This is the structure that represents a storable preset value
#[derive(Serialize, Deserialize, Clone)]
pub struct ActuatePresetV131 {
    // Information
    pub preset_name: String,
    pub preset_info: String,
    pub preset_category: PresetType,
    // Preset tag information - made into bools to make my life easier
    pub tag_acid: bool,
    pub tag_analog: bool,
    pub tag_bright: bool,
    pub tag_chord: bool,
    pub tag_crisp: bool,
    pub tag_deep: bool,
    pub tag_delicate: bool,
    pub tag_hard: bool,
    pub tag_harsh: bool,
    pub tag_lush: bool,
    pub tag_mellow: bool,
    pub tag_resonant: bool,
    pub tag_rich: bool,
    pub tag_sharp: bool,
    pub tag_silky: bool,
    pub tag_smooth: bool,
    pub tag_soft: bool,
    pub tag_stab: bool,
    pub tag_warm: bool,

    // Modules 1
    ///////////////////////////////////////////////////////////
    pub mod1_audio_module_type: AudioModuleType,
    pub mod1_audio_module_level: f32,
    pub mod1_audio_module_routing: AMFilterRouting,
    // Granulizer/Sampler
    pub mod1_loaded_sample: Vec<Vec<f32>>,
    pub mod1_sample_lib: Vec<Vec<Vec<f32>>>,
    pub mod1_loop_wavetable: bool,
    pub mod1_single_cycle: bool,
    pub mod1_restretch: bool,
    pub mod1_prev_restretch: bool,
    pub mod1_grain_hold: i32,
    pub mod1_grain_gap: i32,
    pub mod1_start_position: f32,
    pub mod1_end_position: f32,
    pub mod1_grain_crossfade: i32,

    // Osc module knob storage
    pub mod1_osc_type: VoiceType,
    pub mod1_osc_octave: i32,
    pub mod1_osc_semitones: i32,
    pub mod1_osc_detune: f32,
    pub mod1_osc_attack: f32,
    pub mod1_osc_decay: f32,
    pub mod1_osc_sustain: f32,
    pub mod1_osc_release: f32,
    pub mod1_osc_retrigger: RetriggerStyle,
    pub mod1_osc_atk_curve: SmoothStyle,
    pub mod1_osc_dec_curve: SmoothStyle,
    pub mod1_osc_rel_curve: SmoothStyle,
    pub mod1_osc_unison: i32,
    pub mod1_osc_unison_detune: f32,
    pub mod1_osc_stereo: f32,

    // Modules 2
    ///////////////////////////////////////////////////////////
    pub mod2_audio_module_type: AudioModuleType,
    pub mod2_audio_module_level: f32,
    pub mod2_audio_module_routing: AMFilterRouting,
    // Granulizer/Sampler
    pub mod2_loaded_sample: Vec<Vec<f32>>,
    pub mod2_sample_lib: Vec<Vec<Vec<f32>>>,
    pub mod2_loop_wavetable: bool,
    pub mod2_single_cycle: bool,
    pub mod2_restretch: bool,
    pub mod2_prev_restretch: bool,
    pub mod2_grain_hold: i32,
    pub mod2_grain_gap: i32,
    pub mod2_start_position: f32,
    pub mod2_end_position: f32,
    pub mod2_grain_crossfade: i32,

    // Osc module knob storage
    pub mod2_osc_type: VoiceType,
    pub mod2_osc_octave: i32,
    pub mod2_osc_semitones: i32,
    pub mod2_osc_detune: f32,
    pub mod2_osc_attack: f32,
    pub mod2_osc_decay: f32,
    pub mod2_osc_sustain: f32,
    pub mod2_osc_release: f32,
    pub mod2_osc_retrigger: RetriggerStyle,
    pub mod2_osc_atk_curve: SmoothStyle,
    pub mod2_osc_dec_curve: SmoothStyle,
    pub mod2_osc_rel_curve: SmoothStyle,
    pub mod2_osc_unison: i32,
    pub mod2_osc_unison_detune: f32,
    pub mod2_osc_stereo: f32,

    // Modules 3
    ///////////////////////////////////////////////////////////
    pub mod3_audio_module_type: AudioModuleType,
    pub mod3_audio_module_level: f32,
    pub mod3_audio_module_routing: AMFilterRouting,
    // Granulizer/Sampler
    pub mod3_loaded_sample: Vec<Vec<f32>>,
    pub mod3_sample_lib: Vec<Vec<Vec<f32>>>,
    pub mod3_loop_wavetable: bool,
    pub mod3_single_cycle: bool,
    pub mod3_restretch: bool,
    pub mod3_prev_restretch: bool,
    pub mod3_grain_hold: i32,
    pub mod3_grain_gap: i32,
    pub mod3_start_position: f32,
    pub mod3_end_position: f32,
    pub mod3_grain_crossfade: i32,

    // Osc module knob storage
    pub mod3_osc_type: VoiceType,
    pub mod3_osc_octave: i32,
    pub mod3_osc_semitones: i32,
    pub mod3_osc_detune: f32,
    pub mod3_osc_attack: f32,
    pub mod3_osc_decay: f32,
    pub mod3_osc_sustain: f32,
    pub mod3_osc_release: f32,
    pub mod3_osc_retrigger: RetriggerStyle,
    pub mod3_osc_atk_curve: SmoothStyle,
    pub mod3_osc_dec_curve: SmoothStyle,
    pub mod3_osc_rel_curve: SmoothStyle,
    pub mod3_osc_unison: i32,
    pub mod3_osc_unison_detune: f32,
    pub mod3_osc_stereo: f32,

    // Filters
    pub filter_wet: f32,
    pub filter_cutoff: f32,
    pub filter_resonance: f32,
    pub filter_res_type: ResonanceType,
    pub filter_lp_amount: f32,
    pub filter_hp_amount: f32,
    pub filter_bp_amount: f32,
    pub filter_env_peak: f32,
    pub filter_env_attack: f32,
    pub filter_env_decay: f32,
    pub filter_env_sustain: f32,
    pub filter_env_release: f32,
    pub filter_env_atk_curve: Oscillator::SmoothStyle,
    pub filter_env_dec_curve: Oscillator::SmoothStyle,
    pub filter_env_rel_curve: Oscillator::SmoothStyle,
    pub filter_alg_type: FilterAlgorithms,
    pub tilt_filter_type: ArduraFilter::ResponseType,

    pub filter_wet_2: f32,
    pub filter_cutoff_2: f32,
    pub filter_resonance_2: f32,
    pub filter_res_type_2: ResonanceType,
    pub filter_lp_amount_2: f32,
    pub filter_hp_amount_2: f32,
    pub filter_bp_amount_2: f32,
    pub filter_env_peak_2: f32,
    pub filter_env_attack_2: f32,
    pub filter_env_decay_2: f32,
    pub filter_env_sustain_2: f32,
    pub filter_env_release_2: f32,
    pub filter_env_atk_curve_2: Oscillator::SmoothStyle,
    pub filter_env_dec_curve_2: Oscillator::SmoothStyle,
    pub filter_env_rel_curve_2: Oscillator::SmoothStyle,
    pub filter_alg_type_2: FilterAlgorithms,
    pub tilt_filter_type_2: ArduraFilter::ResponseType,

    pub filter_routing: FilterRouting,
    pub filter_cutoff_link: bool,

    // Pitch Env
    pub pitch_enable: bool,
    pub pitch_routing: PitchRouting,
    pub pitch_env_peak: f32,
    pub pitch_env_attack: f32,
    pub pitch_env_decay: f32,
    pub pitch_env_sustain: f32,
    pub pitch_env_release: f32,
    pub pitch_env_atk_curve: Oscillator::SmoothStyle,
    pub pitch_env_dec_curve: Oscillator::SmoothStyle,
    pub pitch_env_rel_curve: Oscillator::SmoothStyle,

    pub pitch_enable_2: bool,
    pub pitch_routing_2: PitchRouting,
    pub pitch_env_peak_2: f32,
    pub pitch_env_attack_2: f32,
    pub pitch_env_decay_2: f32,
    pub pitch_env_sustain_2: f32,
    pub pitch_env_release_2: f32,
    pub pitch_env_atk_curve_2: Oscillator::SmoothStyle,
    pub pitch_env_dec_curve_2: Oscillator::SmoothStyle,
    pub pitch_env_rel_curve_2: Oscillator::SmoothStyle,

    // LFOs
    pub lfo1_enable: bool,
    pub lfo2_enable: bool,
    pub lfo3_enable: bool,

    pub lfo1_freq: f32,
    pub lfo1_retrigger: LFOController::LFORetrigger,
    pub lfo1_sync: bool,
    pub lfo1_snap: LFOController::LFOSnapValues,
    pub lfo1_waveform: LFOController::Waveform,
    pub lfo1_phase: f32,

    pub lfo2_freq: f32,
    pub lfo2_retrigger: LFOController::LFORetrigger,
    pub lfo2_sync: bool,
    pub lfo2_snap: LFOController::LFOSnapValues,
    pub lfo2_waveform: LFOController::Waveform,
    pub lfo2_phase: f32,

    pub lfo3_freq: f32,
    pub lfo3_retrigger: LFOController::LFORetrigger,
    pub lfo3_sync: bool,
    pub lfo3_snap: LFOController::LFOSnapValues,
    pub lfo3_waveform: LFOController::Waveform,
    pub lfo3_phase: f32,

    // Modulation
    pub mod_source_1: ModulationSource,
    pub mod_source_2: ModulationSource,
    pub mod_source_3: ModulationSource,
    pub mod_source_4: ModulationSource,
    pub mod_dest_1: ModulationDestination,
    pub mod_dest_2: ModulationDestination,
    pub mod_dest_3: ModulationDestination,
    pub mod_dest_4: ModulationDestination,
    pub mod_amount_1: f32,
    pub mod_amount_2: f32,
    pub mod_amount_3: f32,
    pub mod_amount_4: f32,

    // FM
    pub fm_one_to_two: f32,
    pub fm_one_to_three: f32,
    pub fm_two_to_three: f32,
    pub fm_cycles: i32,
    pub fm_attack: f32,
    pub fm_decay: f32,
    pub fm_sustain: f32,
    pub fm_release: f32,
    pub fm_attack_curve: Oscillator::SmoothStyle,
    pub fm_decay_curve: Oscillator::SmoothStyle,
    pub fm_release_curve: Oscillator::SmoothStyle,

    // Stereo
    pub stereo_algorithm: StereoAlgorithm,

    // EQ
    pub pre_use_eq: bool,
    pub pre_low_freq: f32,
    pub pre_mid_freq: f32,
    pub pre_high_freq: f32,
    pub pre_low_gain: f32,
    pub pre_mid_gain: f32,
    pub pre_high_gain: f32,

    // FX
    pub use_fx: bool,

    pub use_compressor: bool,
    pub comp_amt: f32,
    pub comp_atk: f32,
    pub comp_rel: f32,
    pub comp_drive: f32,

    pub use_abass: bool,
    pub abass_amount: f32,

    pub use_saturation: bool,
    pub sat_amount: f32,
    pub sat_type: SaturationType,

    pub use_delay: bool,
    pub delay_amount: f32,
    pub delay_time: DelaySnapValues,
    pub delay_decay: f32,
    pub delay_type: DelayType,

    pub use_reverb: bool,
    pub reverb_model: ReverbModel,
    pub reverb_amount: f32,
    pub reverb_size: f32,
    pub reverb_feedback: f32,

    pub use_phaser: bool,
    pub phaser_amount: f32,
    pub phaser_depth: f32,
    pub phaser_rate: f32,
    pub phaser_feedback: f32,

    pub use_chorus: bool,
    pub chorus_amount: f32,
    pub chorus_range: f32,
    pub chorus_speed: f32,

    pub use_buffermod: bool,
    pub buffermod_amount: f32,
    pub buffermod_depth: f32,
    pub buffermod_rate: f32,
    pub buffermod_spread: f32,
    pub buffermod_timing: f32,

    pub use_flanger: bool,
    pub flanger_amount: f32,
    pub flanger_depth: f32,
    pub flanger_rate: f32,
    pub flanger_feedback: f32,

    pub use_limiter: bool,
    pub limiter_threshold: f32,
    pub limiter_knee: f32,

    // Additive fields
    pub additive_amp_1_0: f32,
    pub additive_amp_1_1: f32,
    pub additive_amp_1_2: f32,
    pub additive_amp_1_3: f32,
    pub additive_amp_1_4: f32,
    pub additive_amp_1_5: f32,
    pub additive_amp_1_6: f32,
    pub additive_amp_1_7: f32,
    pub additive_amp_1_8: f32,
    pub additive_amp_1_9: f32,
    pub additive_amp_1_10: f32,
    pub additive_amp_1_11: f32,
    pub additive_amp_1_12: f32,
    pub additive_amp_1_13: f32,
    pub additive_amp_1_14: f32,
    pub additive_amp_1_15: f32,
    pub additive_amp_2_0: f32,
    pub additive_amp_2_1: f32,
    pub additive_amp_2_2: f32,
    pub additive_amp_2_3: f32,
    pub additive_amp_2_4: f32,
    pub additive_amp_2_5: f32,
    pub additive_amp_2_6: f32,
    pub additive_amp_2_7: f32,
    pub additive_amp_2_8: f32,
    pub additive_amp_2_9: f32,
    pub additive_amp_2_10: f32,
    pub additive_amp_2_11: f32,
    pub additive_amp_2_12: f32,
    pub additive_amp_2_13: f32,
    pub additive_amp_2_14: f32,
    pub additive_amp_2_15: f32,
    pub additive_amp_3_0: f32,
    pub additive_amp_3_1: f32,
    pub additive_amp_3_2: f32,
    pub additive_amp_3_3: f32,
    pub additive_amp_3_4: f32,
    pub additive_amp_3_5: f32,
    pub additive_amp_3_6: f32,
    pub additive_amp_3_7: f32,
    pub additive_amp_3_8: f32,
    pub additive_amp_3_9: f32,
    pub additive_amp_3_10: f32,
    pub additive_amp_3_11: f32,
    pub additive_amp_3_12: f32,
    pub additive_amp_3_13: f32,
    pub additive_amp_3_14: f32,
    pub additive_amp_3_15: f32,
}