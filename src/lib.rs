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

use nih_plug_egui::{create_egui_editor, egui::{self, Color32, Rect, Rounding, RichText, FontId, Pos2}, EguiState, widgets::ParamSlider};
use rubato::Resampler;
use std::{sync::{Arc}, ops::RangeInclusive};
use nih_plug::prelude::*;

// My Files
mod Oscillator;
mod state_variable_filter;
mod ui_knob;

pub struct LoadedSample(Vec<Vec<f32>>);

/// The number of simultaneous voices for this synth.
const _NUM_VOICES: usize = 16;

// Plugin sizing
const WIDTH: u32 = 832;
const HEIGHT: u32 = 632;

// GUI Colors
const A_KNOB_OUTSIDE_COLOR: Color32 = Color32::from_rgb(10,103,210);
const DARK_GREY_UI_COLOR: Color32 = Color32::from_rgb(49,53,71);
const A_BACKGROUND_COLOR_TOP: Color32 = Color32::from_rgb(185,186,198);
const A_BACKGROUND_COLOR_BOTTOM: Color32 = Color32::from_rgb(60,60,68);
const SYNTH_BARS_PURPLE: Color32 = Color32::from_rgb(45,41,99);
const SYNTH_MIDDLE_BLUE: Color32 = Color32::from_rgb(98,145,204);

// Font
const FONT: nih_plug_egui::egui::FontId = FontId::monospace(14.0);
const FONT_COLOR: Color32 = A_KNOB_OUTSIDE_COLOR;

pub struct Actuate {
    pub params: Arc<ActuateParams>,
    pub sample_rate: f32,
    // The MIDI note ID of the active note triggered by MIDI.
    midi_note_id: u8,
    // The frequency if the active note triggered by MIDI.
    midi_note_freq: f32,
    
    // Main Oscillator
    osc_1: Oscillator::Oscillator,
    osc_1_current_gain: Smoother<f32>,
}

impl Default for Actuate {
    fn default() -> Self {
        Self {
            params: Arc::new(Default::default()),
            sample_rate: 44100.0,
            midi_note_id: 0,
            midi_note_freq: 1.0,
            osc_1: Oscillator::Oscillator { 
                sample_rate: 44100.0, 
                osc_type: Oscillator::VoiceType::Sine, 
                osc_attack: Smoother::new(SmoothingStyle::Linear(50.0)), 
                osc_release: Smoother::new(SmoothingStyle::Linear(50.0)), 
                prev_attack: 0.0, 
                prev_release: 0.0, 
                osc_mod_amount: 0.0, 
                prev_note_phase_delta: 0.0, 
                phase: 0.0,
                osc_state: Oscillator::OscState::Off,
            },
            osc_1_current_gain: Smoother::new(SmoothingStyle::Linear(5.0)),
        }
    }
}

/// Plugin parameters struct
#[derive(Params)]
pub struct ActuateParams {
    #[persist = "editor-state"]
    editor_state: Arc<EguiState>,

    #[id = "osc_1_type"]
    pub osc_1_type: EnumParam<Oscillator::VoiceType>,

    #[id = "osc_1_attack"]
    pub osc_1_attack: FloatParam,

    #[id = "osc_1_decay"]
    pub osc_1_decay: FloatParam,

    #[id = "osc_1_sustain"]
    pub osc_1_sustain: FloatParam,

    #[id = "osc_1_release"]
    pub osc_1_release: FloatParam,

    #[id = "osc_1_mod_amount"]
    pub osc_1_mod_amount: FloatParam,

    #[id = "osc_1_retrigger"]
    pub osc_1_retrigger: BoolParam,
}

impl Default for ActuateParams {
    fn default() -> Self {
        Self {
            editor_state: EguiState::from_size(WIDTH, HEIGHT),
            osc_1_type: EnumParam::new("Osc 1 Type", Oscillator::VoiceType::Sine),
            osc_1_attack: FloatParam::new("Osc 1 Attack", 0.0, FloatRange::Skewed { min: 0.0, max: 999.9, factor: 0.5 }).with_unit(" Attack").with_value_to_string(formatters::v2s_f32_rounded(1)),
            osc_1_decay: FloatParam::new("Osc 1 Decay", 0.0, FloatRange::Skewed { min: 0.0, max: 999.9, factor: 0.5 }).with_unit(" Decay").with_value_to_string(formatters::v2s_f32_rounded(1)),
            osc_1_sustain: FloatParam::new("Osc 1 Sustain", 0.0, FloatRange::Linear { min: 0.0, max: 999.9 }).with_unit(" Sustain").with_value_to_string(formatters::v2s_f32_rounded(1)),
            osc_1_release: FloatParam::new("Osc 1 Release", 5.0, FloatRange::Skewed { min: 0.0, max: 999.9, factor: 0.5 }).with_unit(" Release").with_value_to_string(formatters::v2s_f32_rounded(1)),
            osc_1_mod_amount: FloatParam::new("Osc 1 Modifier", 0.0, FloatRange::Linear { min: 0.0, max: 1.0 }).with_unit(" Modifier").with_value_to_string(formatters::v2s_f32_rounded(2)),
            osc_1_retrigger: BoolParam::new("Osc 1 Retrigger", true),
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
                        style_var.visuals.widgets.inactive.bg_stroke.color = A_KNOB_OUTSIDE_COLOR;
                        style_var.visuals.widgets.inactive.bg_fill = DARK_GREY_UI_COLOR;
                        style_var.visuals.widgets.active.fg_stroke.color = A_KNOB_OUTSIDE_COLOR;
                        style_var.visuals.widgets.active.bg_stroke.color = A_KNOB_OUTSIDE_COLOR;
                        style_var.visuals.widgets.open.fg_stroke.color = A_KNOB_OUTSIDE_COLOR;
                        style_var.visuals.widgets.open.bg_fill = DARK_GREY_UI_COLOR;
                        // Lettering on param sliders
                        style_var.visuals.widgets.inactive.fg_stroke.color = A_KNOB_OUTSIDE_COLOR;
                        // Background of the bar in param sliders
                        style_var.visuals.selection.bg_fill = A_KNOB_OUTSIDE_COLOR;
                        style_var.visuals.selection.stroke.color = A_KNOB_OUTSIDE_COLOR;
                        // Unfilled background of the bar
                        style_var.visuals.widgets.noninteractive.bg_fill = DARK_GREY_UI_COLOR;

                        // Trying to draw background colors as rects
                        ui.painter().rect_filled(
                            Rect::from_x_y_ranges(
                                RangeInclusive::new(0.0, WIDTH as f32), 
                                RangeInclusive::new(0.0, (HEIGHT as f32)*0.7)), 
                            Rounding::from(16.0), A_BACKGROUND_COLOR_TOP);
                        ui.painter().rect_filled(
                            Rect::from_x_y_ranges(
                                RangeInclusive::new(0.0, WIDTH as f32), 
                                RangeInclusive::new((HEIGHT as f32)*0.7, HEIGHT as f32)), 
                            Rounding::from(16.0), A_BACKGROUND_COLOR_BOTTOM);

                        ui.set_style(style_var);
                        ui.horizontal(|ui| {
                        // Synth Bars on left and right
                        let synth_bar_space = 40.0;
                        ui.painter().rect_filled(
                            Rect::from_x_y_ranges(
                                RangeInclusive::new(0.0, synth_bar_space), 
                                RangeInclusive::new(0.0, HEIGHT as f32)),
                            Rounding::none(),
                            SYNTH_BARS_PURPLE
                        );

                        ui.add_space(synth_bar_space);

                        // GUI Structure
                            ui.vertical(|ui| {
                                // Spacing :)
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new("Actuate").font(FONT).color(FONT_COLOR)).on_hover_text("by Ardura!");
                                });
                                ui.separator();
                                const KNOB_SIZE: f32 = 40.0;
                                const TEXT_SIZE: f32 = 16.0;

                                ui.horizontal(|ui| {
                                    let osc_1_type_knob = ui_knob::ArcKnob::for_param(
                                        &params.osc_1_type, 
                                        setter, 
                                        KNOB_SIZE)
                                        .preset_style(ui_knob::KnobStyle::NewPresets1)
                                        .set_fill_color(DARK_GREY_UI_COLOR)
                                        .set_line_color(SYNTH_MIDDLE_BLUE)
                                        .set_text_size(TEXT_SIZE);
                                    ui.add(osc_1_type_knob);

                                    let osc_1_mod_knob = ui_knob::ArcKnob::for_param(
                                        &params.osc_1_mod_amount, 
                                        setter, 
                                        KNOB_SIZE)
                                        .preset_style(ui_knob::KnobStyle::NewPresets1)
                                        .set_fill_color(DARK_GREY_UI_COLOR)
                                        .set_line_color(SYNTH_MIDDLE_BLUE)
                                        .set_text_size(TEXT_SIZE);
                                    ui.add(osc_1_mod_knob);

                                    let osc_1_attack_knob = ui_knob::ArcKnob::for_param(
                                        &params.osc_1_attack, 
                                        setter, 
                                        KNOB_SIZE)
                                        .preset_style(ui_knob::KnobStyle::NewPresets1)
                                        .set_fill_color(DARK_GREY_UI_COLOR)
                                        .set_line_color(A_KNOB_OUTSIDE_COLOR)
                                        .set_text_size(TEXT_SIZE);
                                    ui.add(osc_1_attack_knob);

                                    let osc_1_decay_knob = ui_knob::ArcKnob::for_param(
                                        &params.osc_1_decay, 
                                        setter, 
                                        KNOB_SIZE)
                                        .preset_style(ui_knob::KnobStyle::NewPresets1)
                                        .set_fill_color(DARK_GREY_UI_COLOR)
                                        .set_line_color(A_KNOB_OUTSIDE_COLOR)
                                        .set_text_size(TEXT_SIZE);
                                    ui.add(osc_1_decay_knob);

                                    let osc_1_sustain_knob = ui_knob::ArcKnob::for_param(
                                        &params.osc_1_sustain, 
                                        setter, 
                                        KNOB_SIZE)
                                        .preset_style(ui_knob::KnobStyle::NewPresets1)
                                        .set_fill_color(DARK_GREY_UI_COLOR)
                                        .set_line_color(A_KNOB_OUTSIDE_COLOR)
                                        .set_text_size(TEXT_SIZE);
                                    ui.add(osc_1_sustain_knob);

                                    let osc_1_release_knob = ui_knob::ArcKnob::for_param(
                                        &params.osc_1_release, 
                                        setter, 
                                        KNOB_SIZE)
                                        .preset_style(ui_knob::KnobStyle::NewPresets1)
                                        .set_fill_color(DARK_GREY_UI_COLOR)
                                        .set_line_color(A_KNOB_OUTSIDE_COLOR)
                                        .set_text_size(TEXT_SIZE);
                                    ui.add(osc_1_release_knob);

                                    ui.vertical(|ui| {
                                        ui.label("Retrigger");
                                        ui.add(ParamSlider::for_param(&params.osc_1_retrigger, setter).with_width(KNOB_SIZE));
                                    });
                                });


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
                                SYNTH_BARS_PURPLE
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

        //let sample_list = self.params.sample_list.lock().unwrap().clone();
        //for path in sample_list {
        //    self.load_sample(path.clone());
        //}

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
    fn process_midi(&mut self, context: &mut impl ProcessContext<Self>, buffer: &mut Buffer) {
        let mut next_event = context.next_event();
        // Copy Subhoofer Structure because it's cozy
        for (sample_id, mut channel_samples) in buffer.iter_samples().enumerate() {
            // Update our envelopes if needed
            self.osc_1.check_update_attack(self.params.osc_1_attack.value());
            self.osc_1.check_update_release(self.params.osc_1_release.value());

            let waveform = {
                while let Some(event) = next_event {
                    if event.timing() > sample_id as u32 {
                        break;
                    }
                    match event {
                        // Midi Calculation Code
                        NoteEvent::NoteOn { note, velocity, .. } => {
                            // Reset the retrigger on Oscs
                            if self.params.osc_1_retrigger.value() {
                                self.osc_1.reset_phase();
                            }

                            self.midi_note_id = note;
                            self.midi_note_freq = util::midi_note_to_freq(note);

                            // Osc Updates
                            self.osc_1.reset_attack_smoother(0.0);
                            // Reset release for logic to know note is happening
                            self.osc_1.reset_release_smoother(0.0);
                            self.osc_1.set_attack_target(self.sample_rate, velocity);
                            self.osc_1_current_gain = self.osc_1.get_attack_smoother();
                            self.osc_1.set_osc_state(Oscillator::OscState::Attacking);
                        },
                        NoteEvent::NoteOff { note, .. } if note == self.midi_note_id => {
                            // This reset lets us fade from any max or other value to 0
                            self.osc_1.reset_release_smoother(self.osc_1_current_gain.next());
                            // Reset attack
                            self.osc_1.reset_attack_smoother(0.0);
                            self.osc_1.set_release_target(self.sample_rate, 0.0);
                            self.osc_1_current_gain = self.osc_1.get_release_smoother();
                            self.osc_1.set_osc_state(Oscillator::OscState::Releasing);
                        },
                        _ => (),
                    }
                    next_event = context.next_event();
                }
                // Move our phase outside of the midi events
                // I couldn't find much on how to model this so I based it off previous note phase
                if !self.params.osc_1_retrigger.value() {
                    self.osc_1.increment_phase();
                }

                // Attack is over so use decay amount to reach sustain level - reusing current smoother
                if  self.osc_1_current_gain.steps_left() == 0 && 
                    self.osc_1.get_osc_state() == Oscillator::OscState::Attacking
                {
                    self.osc_1.set_osc_state(Oscillator::OscState::Decaying);
                    let temp_gain = self.osc_1_current_gain.next();
                    self.osc_1_current_gain = Smoother::new(SmoothingStyle::Linear(self.params.osc_1_decay.value()));
                    self.osc_1_current_gain.reset(temp_gain);
                    let sustain_scaled = self.params.osc_1_sustain.value() / 999.9;
                    self.osc_1_current_gain.set_target(self.sample_rate, sustain_scaled);
                }

                // Move from Decaying to Sustain hold
                if  self.osc_1_current_gain.steps_left() == 0 && 
                    self.osc_1.get_osc_state() == Oscillator::OscState::Decaying
                {
                    let sustain_scaled = self.params.osc_1_sustain.value() / 999.9;
                    self.osc_1_current_gain.set_target(self.sample_rate, sustain_scaled);
                    self.osc_1.set_osc_state(Oscillator::OscState::Sustaining);
                }

                // End of release
                if  self.osc_1.get_osc_state() == Oscillator::OscState::Releasing &&
                    self.osc_1_current_gain.steps_left() == 0
                {
                    self.osc_1.set_osc_state(Oscillator::OscState::Off);
                }

                // Get our current gain amount for use in match below
                let temp_osc_1_gain_multiplier: f32 = self.osc_1_current_gain.next();

                // OSC 1
                match self.params.osc_1_type.value() {
                    Oscillator::VoiceType::Sine  => self.osc_1.calculate_sine(self.midi_note_freq, self.params.osc_1_mod_amount.value()) * temp_osc_1_gain_multiplier,
                    Oscillator::VoiceType::Saw   => self.osc_1.calculate_saw(self.midi_note_freq, self.params.osc_1_mod_amount.value()) * temp_osc_1_gain_multiplier,
                    Oscillator::VoiceType::RoundedSaw  => self.osc_1.calculate_rsaw(self.midi_note_freq, self.params.osc_1_mod_amount.value()) * temp_osc_1_gain_multiplier,
                    Oscillator::VoiceType::InwardSaw  => self.osc_1.calculate_inward_saw(self.midi_note_freq, self.params.osc_1_mod_amount.value()) * temp_osc_1_gain_multiplier,
                    Oscillator::VoiceType::DoubleExpSaw => self.osc_1.calculate_dub_exp_saw(self.midi_note_freq, self.params.osc_1_mod_amount.value()) * temp_osc_1_gain_multiplier,
                    Oscillator::VoiceType::Ramp => self.osc_1.calculate_ramp(self.midi_note_freq, self.params.osc_1_mod_amount.value()) * temp_osc_1_gain_multiplier,
                    Oscillator::VoiceType::Wave1 => self.osc_1.calculate_wave_1(self.midi_note_freq, self.params.osc_1_mod_amount.value()) * temp_osc_1_gain_multiplier,
                }
            }; 
            
            *channel_samples.get_mut(0).unwrap() = waveform;
            *channel_samples.get_mut(1).unwrap() = waveform;
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
        Vst3SubCategory::Drum,
        Vst3SubCategory::Sampler,
        Vst3SubCategory::Instrument,
    ];
}

nih_export_clap!(Actuate);
nih_export_vst3!(Actuate);
