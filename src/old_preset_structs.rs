use crate::{
    actuate_enums::StereoAlgorithm, audio_module::{
        AudioModuleType,
        Oscillator::{self, RetriggerStyle, SmoothStyle},
    }, fx::{
        delay::{DelaySnapValues, DelayType},
        saturation::SaturationType,
        ArduraFilter,
        StateVariableFilter::ResonanceType,
    }, AMFilterRouting, ActuatePresetV131, FilterAlgorithms, FilterRouting, LFOController, ModulationDestination, ModulationSource, PitchRouting, PresetType, ReverbModel
};
use serde::{Deserialize, Serialize};

// This file is supposed to contain all the long form preset formats and convert from older formats to newer by filling in missing fields
// This will probably get messier in future but since it is outside the main lib.rs it should keep some of the changes simpler overall

/// This is the structure that represents a storable preset value
#[derive(Serialize, Deserialize, Clone)]
pub struct ActuatePresetV130 {
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
}

// This takes the deserialized message pack and converts it into the latest struct
// This then attempts to return the newer preset format after
pub fn _load_unserialized_v130(file_data: Vec<u8>) -> ActuatePresetV131 {
    let old_unserialized: ActuatePresetV130 =
        serde_json::from_slice(&file_data).unwrap_or(ActuatePresetV130 {
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
            mod1_audio_module_routing: AMFilterRouting::Filter1,
            mod1_audio_module_type: AudioModuleType::Sine,
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

            mod2_audio_module_routing: AMFilterRouting::Filter1,
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

            mod3_audio_module_routing: AMFilterRouting::Filter1,
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

            // Pitch Env
            pitch_enable: false,
            pitch_routing: PitchRouting::Osc1,
            pitch_env_peak: 0.0,
            pitch_env_attack: 0.0,
            pitch_env_decay: 300.0,
            pitch_env_sustain: 0.0,
            pitch_env_release: 0.0,
            pitch_env_atk_curve: Oscillator::SmoothStyle::Linear,
            pitch_env_dec_curve: Oscillator::SmoothStyle::Linear,
            pitch_env_rel_curve: Oscillator::SmoothStyle::Linear,

            pitch_enable_2: false,
            pitch_routing_2: PitchRouting::Osc1,
            pitch_env_peak_2: 0.0,
            pitch_env_attack_2: 0.0,
            pitch_env_decay_2: 300.0,
            pitch_env_sustain_2: 0.0,
            pitch_env_release_2: 0.0,
            pitch_env_atk_curve_2: Oscillator::SmoothStyle::Linear,
            pitch_env_dec_curve_2: Oscillator::SmoothStyle::Linear,
            pitch_env_rel_curve_2: Oscillator::SmoothStyle::Linear,

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

            // FM
            fm_one_to_two: 0.0,
            fm_one_to_three: 0.0,
            fm_two_to_three: 0.0,
            fm_cycles: 1,
            fm_attack: 0.0001,
            fm_decay: 0.0001,
            fm_sustain: 0.999,
            fm_release: 0.0001,
            fm_attack_curve: SmoothStyle::Linear,
            fm_decay_curve: SmoothStyle::Linear,
            fm_release_curve: SmoothStyle::Linear,

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
            reverb_model: ReverbModel::Default,
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

            //V130 Fields
            chorus_amount: 0.5,
            chorus_range: 0.5,
            chorus_speed: 0.5,
            use_chorus: false,
            stereo_algorithm: StereoAlgorithm::Original,
        });
    _convert_preset_v130(old_unserialized)
}

// This will get cloned each time we change preset styles in actuate
pub fn _convert_preset_v130(preset: ActuatePresetV130) -> ActuatePresetV131 {
    let new_format: ActuatePresetV131 = ActuatePresetV131 {
        preset_name: preset.preset_name,
        preset_info: preset.preset_info,
        preset_category: preset.preset_category,
        tag_acid: preset.tag_acid,
        tag_analog: preset.tag_analog,
        tag_bright: preset.tag_bright,
        tag_chord: preset.tag_chord,
        tag_crisp: preset.tag_crisp,
        tag_deep: preset.tag_deep,
        tag_delicate: preset.tag_delicate,
        tag_hard: preset.tag_hard,
        tag_harsh: preset.tag_harsh,
        tag_lush: preset.tag_lush,
        tag_mellow: preset.tag_mellow,
        tag_resonant: preset.tag_resonant,
        tag_rich: preset.tag_rich,
        tag_sharp: preset.tag_sharp,
        tag_silky: preset.tag_silky,
        tag_smooth: preset.tag_smooth,
        tag_soft: preset.tag_soft,
        tag_stab: preset.tag_stab,
        tag_warm: preset.tag_warm,
        mod1_audio_module_type: preset.mod1_audio_module_type,
        mod1_audio_module_level: preset.mod1_audio_module_level,
        // Added in 1.2.3
        mod1_audio_module_routing: preset.mod1_audio_module_routing,
        mod1_loaded_sample: preset.mod1_loaded_sample,
        mod1_sample_lib: preset.mod1_sample_lib,
        mod1_loop_wavetable: preset.mod1_loop_wavetable,
        mod1_single_cycle: preset.mod1_single_cycle,
        mod1_restretch: preset.mod1_restretch,
        mod1_prev_restretch: preset.mod1_prev_restretch,
        mod1_grain_hold: preset.mod1_grain_hold,
        mod1_grain_gap: preset.mod1_grain_gap,
        mod1_start_position: preset.mod1_start_position,
        mod1_end_position: preset.mod1_end_position,
        mod1_grain_crossfade: preset.mod1_grain_crossfade,
        mod1_osc_octave: preset.mod1_osc_octave,
        mod1_osc_semitones: preset.mod1_osc_semitones,
        mod1_osc_detune: preset.mod1_osc_detune,
        mod1_osc_attack: preset.mod1_osc_attack,
        mod1_osc_decay: preset.mod1_osc_decay,
        mod1_osc_sustain: preset.mod1_osc_sustain,
        mod1_osc_release: preset.mod1_osc_release,
        mod1_osc_retrigger: preset.mod1_osc_retrigger,
        mod1_osc_atk_curve: preset.mod1_osc_atk_curve,
        mod1_osc_dec_curve: preset.mod1_osc_dec_curve,
        mod1_osc_rel_curve: preset.mod1_osc_rel_curve,
        mod1_osc_unison: preset.mod1_osc_unison,
        mod1_osc_unison_detune: preset.mod1_osc_unison_detune,
        mod1_osc_stereo: preset.mod1_osc_stereo,
        mod2_audio_module_type: preset.mod2_audio_module_type,
        mod2_audio_module_level: preset.mod2_audio_module_level,
        // Added in 1.2.3
        mod2_audio_module_routing: preset.mod2_audio_module_routing,
        mod2_loaded_sample: preset.mod2_loaded_sample,
        mod2_sample_lib: preset.mod2_sample_lib,
        mod2_loop_wavetable: preset.mod2_loop_wavetable,
        mod2_single_cycle: preset.mod2_single_cycle,
        mod2_restretch: preset.mod2_restretch,
        mod2_prev_restretch: preset.mod2_prev_restretch,
        mod2_grain_hold: preset.mod2_grain_hold,
        mod2_grain_gap: preset.mod2_grain_gap,
        mod2_start_position: preset.mod2_start_position,
        mod2_end_position: preset.mod2_end_position,
        mod2_grain_crossfade: preset.mod2_grain_crossfade,
        mod2_osc_octave: preset.mod2_osc_octave,
        mod2_osc_semitones: preset.mod2_osc_semitones,
        mod2_osc_detune: preset.mod2_osc_detune,
        mod2_osc_attack: preset.mod2_osc_attack,
        mod2_osc_decay: preset.mod2_osc_decay,
        mod2_osc_sustain: preset.mod2_osc_sustain,
        mod2_osc_release: preset.mod2_osc_release,
        mod2_osc_retrigger: preset.mod2_osc_retrigger,
        mod2_osc_atk_curve: preset.mod2_osc_atk_curve,
        mod2_osc_dec_curve: preset.mod2_osc_dec_curve,
        mod2_osc_rel_curve: preset.mod2_osc_rel_curve,
        mod2_osc_unison: preset.mod2_osc_unison,
        mod2_osc_unison_detune: preset.mod2_osc_unison_detune,
        mod2_osc_stereo: preset.mod2_osc_stereo,
        mod3_audio_module_type: preset.mod3_audio_module_type,
        mod3_audio_module_level: preset.mod3_audio_module_level,
        // Added in 1.2.3
        mod3_audio_module_routing: preset.mod3_audio_module_routing,
        mod3_loaded_sample: preset.mod3_loaded_sample,
        mod3_sample_lib: preset.mod3_sample_lib,
        mod3_loop_wavetable: preset.mod3_loop_wavetable,
        mod3_single_cycle: preset.mod3_single_cycle,
        mod3_restretch: preset.mod3_restretch,
        mod3_prev_restretch: preset.mod3_prev_restretch,
        mod3_grain_hold: preset.mod3_grain_hold,
        mod3_grain_gap: preset.mod3_grain_gap,
        mod3_start_position: preset.mod3_start_position,
        mod3_end_position: preset.mod3_end_position,
        mod3_grain_crossfade: preset.mod3_grain_crossfade,
        mod3_osc_octave: preset.mod3_osc_octave,
        mod3_osc_semitones: preset.mod3_osc_semitones,
        mod3_osc_detune: preset.mod3_osc_detune,
        mod3_osc_attack: preset.mod3_osc_attack,
        mod3_osc_decay: preset.mod3_osc_decay,
        mod3_osc_sustain: preset.mod3_osc_sustain,
        mod3_osc_release: preset.mod3_osc_release,
        mod3_osc_retrigger: preset.mod3_osc_retrigger,
        mod3_osc_atk_curve: preset.mod3_osc_atk_curve,
        mod3_osc_dec_curve: preset.mod3_osc_dec_curve,
        mod3_osc_rel_curve: preset.mod3_osc_rel_curve,
        mod3_osc_unison: preset.mod3_osc_unison,
        mod3_osc_unison_detune: preset.mod3_osc_unison_detune,
        mod3_osc_stereo: preset.mod3_osc_stereo,
        filter_wet: preset.filter_wet,
        filter_cutoff: preset.filter_cutoff,
        filter_resonance: preset.filter_resonance,
        filter_res_type: preset.filter_res_type,
        filter_lp_amount: preset.filter_lp_amount,
        filter_hp_amount: preset.filter_hp_amount,
        filter_bp_amount: preset.filter_bp_amount,
        filter_env_peak: preset.filter_env_peak,
        filter_env_attack: preset.filter_env_attack,
        filter_env_decay: preset.filter_env_decay,
        filter_env_sustain: preset.filter_env_sustain,
        filter_env_release: preset.filter_env_release,
        filter_env_atk_curve: preset.filter_env_atk_curve,
        filter_env_dec_curve: preset.filter_env_dec_curve,
        filter_env_rel_curve: preset.filter_env_rel_curve,
        filter_alg_type: preset.filter_alg_type,
        tilt_filter_type: preset.tilt_filter_type,
        filter_wet_2: preset.filter_wet_2,
        filter_cutoff_2: preset.filter_cutoff_2,
        filter_resonance_2: preset.filter_resonance_2,
        filter_res_type_2: preset.filter_res_type_2,
        filter_lp_amount_2: preset.filter_lp_amount_2,
        filter_hp_amount_2: preset.filter_hp_amount_2,
        filter_bp_amount_2: preset.filter_bp_amount_2,
        filter_env_peak_2: preset.filter_env_peak_2,
        filter_env_attack_2: preset.filter_env_attack_2,
        filter_env_decay_2: preset.filter_env_decay_2,
        filter_env_sustain_2: preset.filter_env_sustain_2,
        filter_env_release_2: preset.filter_env_release_2,
        filter_env_atk_curve_2: preset.filter_env_atk_curve_2,
        filter_env_dec_curve_2: preset.filter_env_dec_curve_2,
        filter_env_rel_curve_2: preset.filter_env_rel_curve_2,
        filter_alg_type_2: preset.filter_alg_type_2,
        tilt_filter_type_2: preset.tilt_filter_type_2,
        filter_routing: preset.filter_routing,
        ///////////////////////////////////////////////////////////////////
        // Added in 1.1.4
        filter_cutoff_link: preset.filter_cutoff_link,
        ///////////////////////////////////////////////////////////////////
        // Added in pitch update 1.2.1
        pitch_enable: preset.pitch_enable,
        pitch_routing: preset.pitch_routing,
        pitch_env_peak: preset.pitch_env_peak,
        pitch_env_atk_curve: preset.pitch_env_atk_curve,
        pitch_env_dec_curve: preset.pitch_env_dec_curve,
        pitch_env_rel_curve: preset.pitch_env_rel_curve,
        pitch_env_attack: preset.pitch_env_attack,
        pitch_env_decay: preset.pitch_env_decay,
        pitch_env_release: preset.pitch_env_release,
        pitch_env_sustain: preset.pitch_env_sustain,
        pitch_enable_2: preset.pitch_enable_2,
        pitch_env_peak_2: preset.pitch_env_peak_2,
        pitch_env_atk_curve_2: preset.pitch_env_atk_curve_2,
        pitch_env_dec_curve_2: preset.pitch_env_dec_curve_2,
        pitch_env_rel_curve_2: preset.pitch_env_rel_curve_2,
        pitch_env_attack_2: preset.pitch_env_attack_2,
        pitch_env_decay_2: preset.pitch_env_decay_2,
        pitch_env_release_2: preset.pitch_env_release_2,
        pitch_env_sustain_2: preset.pitch_env_sustain_2,
        pitch_routing_2: preset.pitch_routing_2,
        ///////////////////////////////////////////////////////////////////
        lfo1_enable: preset.lfo1_enable,
        lfo2_enable: preset.lfo2_enable,
        lfo3_enable: preset.lfo3_enable,
        lfo1_freq: preset.lfo1_freq,
        lfo1_retrigger: preset.lfo1_retrigger,
        lfo1_sync: preset.lfo1_sync,
        lfo1_snap: preset.lfo1_snap,
        lfo1_waveform: preset.lfo1_waveform,
        lfo1_phase: preset.lfo1_phase,
        lfo2_freq: preset.lfo2_freq,
        lfo2_retrigger: preset.lfo2_retrigger,
        lfo2_sync: preset.lfo2_sync,
        lfo2_snap: preset.lfo2_snap,
        lfo2_waveform: preset.lfo2_waveform,
        lfo2_phase: preset.lfo2_phase,
        lfo3_freq: preset.lfo3_freq,
        lfo3_retrigger: preset.lfo3_retrigger,
        lfo3_sync: preset.lfo3_sync,
        lfo3_snap: preset.lfo3_snap,
        lfo3_waveform: preset.lfo3_waveform,
        lfo3_phase: preset.lfo3_phase,
        mod_source_1: preset.mod_source_1,
        mod_source_2: preset.mod_source_2,
        mod_source_3: preset.mod_source_3,
        mod_source_4: preset.mod_source_4,
        mod_dest_1: preset.mod_dest_1,
        mod_dest_2: preset.mod_dest_2,
        mod_dest_3: preset.mod_dest_3,
        mod_dest_4: preset.mod_dest_4,
        mod_amount_1: preset.mod_amount_1,
        mod_amount_2: preset.mod_amount_2,
        mod_amount_3: preset.mod_amount_3,
        mod_amount_4: preset.mod_amount_4,
        // 1.2.6
        fm_one_to_two: preset.fm_one_to_two,
        fm_one_to_three: preset.fm_one_to_three,
        fm_two_to_three: preset.fm_two_to_three,
        fm_cycles: preset.fm_cycles,
        fm_attack: preset.fm_attack,
        fm_decay: preset.fm_decay,
        fm_sustain: preset.fm_sustain,
        fm_release: preset.fm_release,
        fm_attack_curve: preset.fm_attack_curve,
        fm_decay_curve: preset.fm_decay_curve,
        fm_release_curve: preset.fm_release_curve,
        // 1.2.6
        pre_use_eq: preset.pre_use_eq,
        pre_low_freq: preset.pre_low_freq,
        pre_mid_freq: preset.pre_mid_freq,
        pre_high_freq: preset.pre_high_freq,
        pre_low_gain: preset.pre_low_gain,
        pre_mid_gain: preset.pre_mid_gain,
        pre_high_gain: preset.pre_high_gain,
        use_fx: preset.use_fx,
        use_compressor: preset.use_compressor,
        comp_amt: preset.comp_amt,
        comp_atk: preset.comp_atk,
        comp_rel: preset.comp_rel,
        comp_drive: preset.comp_drive,
        use_abass: preset.use_abass,
        abass_amount: preset.abass_amount,
        use_saturation: preset.use_saturation,
        sat_amount: preset.sat_amount,
        sat_type: preset.sat_type,
        use_delay: preset.use_delay,
        delay_amount: preset.delay_amount,
        delay_time: preset.delay_time,
        delay_decay: preset.delay_decay,
        delay_type: preset.delay_type,
        use_reverb: preset.use_reverb,
        reverb_model: preset.reverb_model,
        reverb_amount: preset.reverb_amount,
        reverb_size: preset.reverb_size,
        reverb_feedback: preset.reverb_feedback,
        //1.3.0
        use_chorus: false,
        chorus_amount: 0.8,
        chorus_range: 0.5,
        chorus_speed: 0.5,
        stereo_algorithm: StereoAlgorithm::Original,
        //1.3.0
        use_phaser: preset.use_phaser,
        phaser_amount: preset.phaser_amount,
        phaser_depth: preset.phaser_depth,
        phaser_rate: preset.phaser_rate,
        phaser_feedback: preset.phaser_feedback,
        use_buffermod: preset.use_buffermod,
        buffermod_amount: preset.buffermod_amount,
        buffermod_depth: preset.buffermod_depth,
        buffermod_rate: preset.buffermod_rate,
        buffermod_spread: preset.buffermod_spread,
        buffermod_timing: preset.buffermod_timing,
        use_flanger: preset.use_flanger,
        flanger_amount: preset.flanger_amount,
        flanger_depth: preset.flanger_depth,
        flanger_rate: preset.flanger_rate,
        flanger_feedback: preset.flanger_feedback,
        use_limiter: preset.use_limiter,
        limiter_threshold: preset.limiter_threshold,
        limiter_knee: preset.limiter_knee,

        // v 1.3.1 Additive fields
        additive_amp_1_0: 0.0,
        additive_amp_1_1: 0.0,
        additive_amp_1_2: 0.0,
        additive_amp_1_3: 0.0,
        additive_amp_1_4: 0.0,
        additive_amp_1_5: 0.0,
        additive_amp_1_6: 0.0,
        additive_amp_1_7: 0.0,
        additive_amp_1_8: 0.0,
        additive_amp_1_9: 0.0,
        additive_amp_1_10: 0.0,
        additive_amp_1_11: 0.0,
        additive_amp_1_12: 0.0,
        additive_amp_1_13: 0.0,
        additive_amp_1_14: 0.0,
        additive_amp_1_15: 0.0,
        additive_amp_2_0: 0.0,
        additive_amp_2_1: 0.0,
        additive_amp_2_2: 0.0,
        additive_amp_2_3: 0.0,
        additive_amp_2_4: 0.0,
        additive_amp_2_5: 0.0,
        additive_amp_2_6: 0.0,
        additive_amp_2_7: 0.0,
        additive_amp_2_8: 0.0,
        additive_amp_2_9: 0.0,
        additive_amp_2_10: 0.0,
        additive_amp_2_11: 0.0,
        additive_amp_2_12: 0.0,
        additive_amp_2_13: 0.0,
        additive_amp_2_14: 0.0,
        additive_amp_2_15: 0.0,
        additive_amp_3_0: 0.0,
        additive_amp_3_1: 0.0,
        additive_amp_3_2: 0.0,
        additive_amp_3_3: 0.0,
        additive_amp_3_4: 0.0,
        additive_amp_3_5: 0.0,
        additive_amp_3_6: 0.0,
        additive_amp_3_7: 0.0,
        additive_amp_3_8: 0.0,
        additive_amp_3_9: 0.0,
        additive_amp_3_10: 0.0,
        additive_amp_3_11: 0.0,
        additive_amp_3_12: 0.0,
        additive_amp_3_13: 0.0,
        additive_amp_3_14: 0.0,
        additive_amp_3_15: 0.0,
    };
    new_format
}
