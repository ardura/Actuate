// This came as a result of cleaning up the Actuate GUI and needing a combobox to param solution
// Ardura
// ----------------------------------------------------------------------------

use std::{fmt::Debug, hash::RandomState, sync::Arc};
use nih_plug::{
    params::EnumParam, prelude::{Enum, Param, ParamSetter}, wrapper::clap::lazy_static
};
use nih_plug_egui::egui::{self, vec2, Color32, Key, Response, Sense, Stroke, TextEdit, TextStyle, Ui, Vec2, Widget, WidgetText};
use nih_plug_egui::widgets::util as nUtil;
use parking_lot::Mutex;

use crate::audio_module::AudioModuleType;

lazy_static! {
    static ref DRAG_NORMALIZED_START_VALUE_MEMORY_ID: egui::Id = egui::Id::new((file!(), 0));
    static ref DRAG_AMOUNT_MEMORY_ID: egui::Id = egui::Id::new((file!(), 1));
    static ref VALUE_ENTRY_MEMORY_ID: egui::Id = egui::Id::new((file!(), 2));
}

#[must_use = "You should put this widget in an ui with `ui.add(widget);`"]
pub struct ComboBoxParam<'a, P: Param> {
    param: &'a P,
    setter: &'a ParamSetter<'a>,
    current_value: AudioModuleType,
}

impl<'a, P: Param> ComboBoxParam<'a, P> {
    /// Create a new slider for a parameter. Use the other methods to modify the slider before
    /// passing it to [`Ui::add()`].
    pub fn for_param(param: &'a P, setter: &'a ParamSetter<'a>) -> Self {
        Self {
            param,
            setter,
        }
    }

    fn plain_value(&self) -> P::Plain {
        self.param.modulated_plain_value()
    }

    fn normalized_value(&self) -> f32 {
        self.param.modulated_normalized_value()
    }

    fn string_value(&self) -> String {
        self.param.to_string()
    }

    fn set_normalized_value(&self, normalized: f32) {
        // This snaps to the nearest plain value if the parameter is stepped in some way.
        // TODO: As an optimization, we could add a `const CONTINUOUS: bool` to the parameter to
        //       avoid this normalized->plain->normalized conversion for parameters that don't need
        //       it
        let value = self.param.preview_plain(normalized);
        if value != self.plain_value() {
            self.setter.set_parameter(self.param, value);
        }
    }

    /// Begin and end drag still need to be called when using this..
    fn reset_param(&self) {
        self.setter
            .set_parameter(self.param, self.param.default_plain_value());
    }

    fn slider_ui(&self, ui: &mut Ui, response: &mut Response) {
        let cb = egui::ComboBox::new(self.param.name(), "")
            //.selected_text(format!("{:?}", self.param.unmodulated_plain_value()))
            .selected_text(self.param.normalized_value_to_string(self.param.unmodulated_normalized_value(), false))
            .width(86.0)
            .height(336.0)
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut self.current_value, AudioModuleType::Off, "Off");
                ui.selectable_value(&mut self.current_value, AudioModuleType::Sine, "Sine");
                ui.selectable_value(&mut self.current_value, AudioModuleType::Tri, "Tri");
                ui.selectable_value(&mut self.current_value, AudioModuleType::Saw, "Saw");
                ui.selectable_value(&mut self.current_value, AudioModuleType::RSaw, "Rsaw");
                ui.selectable_value(&mut self.current_value, AudioModuleType::WSaw, "WSaw");
                ui.selectable_value(&mut self.current_value, AudioModuleType::SSaw, "SSaw");
                ui.selectable_value(&mut self.current_value, AudioModuleType::RASaw, "RASaw");
                ui.selectable_value(&mut self.current_value, AudioModuleType::Ramp, "Ramp");
                ui.selectable_value(&mut self.current_value, AudioModuleType::Square, "Square");
                ui.selectable_value(&mut self.current_value, AudioModuleType::RSquare, "RSquare");
                ui.selectable_value(&mut self.current_value, AudioModuleType::Pulse, "Pulse");
                ui.selectable_value(&mut self.current_value, AudioModuleType::Noise, "Noise");
                ui.selectable_value(&mut self.current_value, AudioModuleType::Sampler, "Sampler");
                ui.selectable_value(&mut self.current_value, AudioModuleType::Granulizer, "Granulizer");
                ui.selectable_value(&mut self.current_value, AudioModuleType::Additive, "Additive");
            }).response.on_hover_text_at_pointer("The type of generator to use.");
        if cb.clicked() {
            match self.current_value {
                AudioModuleType::Off => { self.set_normalized_value(0.0);}
                AudioModuleType::Sine => { self.set_normalized_value(0.0);}
                AudioModuleType::Tri => { self.set_normalized_value(0.0);}
                AudioModuleType::Saw => { self.set_normalized_value(0.0);}
                AudioModuleType::RSaw => { self.set_normalized_value(0.0);}
                AudioModuleType::WSaw => { self.set_normalized_value(0.0);}
                AudioModuleType::SSaw => { self.set_normalized_value(0.0);}
                AudioModuleType::RASaw => { self.set_normalized_value(0.0);}
                AudioModuleType::Ramp => { self.set_normalized_value(0.0);}
                AudioModuleType::Square => { self.set_normalized_value(0.0);}
                AudioModuleType::RSquare => { self.set_normalized_value(0.0);}
                AudioModuleType::Pulse => { self.set_normalized_value(0.0);}
                AudioModuleType::Noise => { self.set_normalized_value(0.0);}
                AudioModuleType::Sampler => { self.set_normalized_value(0.0);}
                AudioModuleType::Granulizer => { self.set_normalized_value(0.0);}
                AudioModuleType::Additive => { self.set_normalized_value(0.0);}
            }
            self.set_normalized_value(normalized);
            response.mark_changed();
        }
        if response.double_clicked() {
            self.reset_param();
            response.mark_changed();
        }
    }
}


impl<P: Param> Widget for ComboBoxParam<'_, P> {
    fn ui(mut self, ui: &mut Ui) -> Response {
        ui.horizontal(|ui| {
            let cb_width = 86.0;
            let cb_height = 15.0;
            let mut response = ui
                .vertical(|ui| {
                    ui.allocate_space(vec2(cb_width, cb_height));
                    let response = ui.allocate_response(
                        vec2(cb_width, cb_height),
                        Sense::click(),
                    );
                    let (kb_edit_id, _) =
                        ui.allocate_space(vec2(cb_width, cb_height));

                    response
                })
                .inner;

            self.slider_ui(ui, &mut response);

            response
        })
        .inner
    }
}
