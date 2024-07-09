//actuate_enums.rs

use std::fmt;

use nih_plug::params::enums::Enum;
use serde::{Deserialize, Serialize};


// Gui for which filter to display on bottom
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub enum UIBottomSelection {
    Filter1,
    Filter2,
    Pitch1,
    Pitch2,
}

// Gui for which panel to display in bottom right
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub enum LFOSelect {
    INFO,
    LFO1,
    LFO2,
    LFO3,
    Modulation,
    Misc,
    FX,
    FM,
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
    V4,
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

// Reverb options
#[derive(Debug, Enum, PartialEq, Clone, Serialize, Deserialize)]
pub enum ReverbModel {
    Default,
    Galactic,
    ASpace,
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

#[derive(Enum, PartialEq, Clone, Copy, Serialize, Deserialize)]
pub enum StereoAlgorithm {
    Original,
    CubeSpread,
    ExpSpread,
}


// These let us output ToString for the ComboBox stuff + Nih-Plug or string usage
impl fmt::Display for PresetType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl fmt::Display for ModulationSource {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl fmt::Display for ModulationDestination {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}