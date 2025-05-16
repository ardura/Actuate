// This came as a result of cleaning up the Actuate GUI and needing a combobox to param solution
// Ardura
// ----------------------------------------------------------------------------

use nih_plug::prelude::{Param, ParamSetter};
use nih_plug_egui::egui::{ComboBox, Response, Ui, Widget};

pub struct ParamComboBox<'a, P: Param> {
    param: &'a P,
    setter: &'a ParamSetter<'a>,

    id_name: String,

    options: Vec<String>, // Options for the ComboBox
}

impl<'a, P: Param> ParamComboBox<'a, P> {
    pub fn for_param(param: &'a P, setter: &'a ParamSetter<'a>, options: Vec<String>, id_name: String) -> Self {
        Self { param, setter, options, id_name }
    }

    fn set_selected_value(&self, selected_value: String) {
        // Convert the selected value back to the normalized parameter value and set it.
        if let Some(normalized_value) = self.param.string_to_normalized_value(&selected_value) {
            let value = self.param.preview_plain(normalized_value);
            if value != self.param.modulated_plain_value() {
                self.setter.set_parameter(self.param, value);
            }
        }
    }

    fn get_current_value(&self) -> String {
        self.param.to_string()
    }
}

impl<'a, P: Param> Widget for ParamComboBox<'a, P> {
    fn ui(self, ui: &mut Ui) -> Response {
        // Store the current value to check for changes later
        let mut current_value = self.get_current_value();
        let mut changed = false; // Flag to detect change

        let response = ComboBox::from_id_salt(self.param.name().to_owned() + &self.id_name)
            .selected_text(current_value.clone())
            .show_ui(ui, |ui| {
                for option in &self.options {
                    // Update current_value and set changed flag if a new option is selected
                    if ui.selectable_value(&mut current_value, option.clone(), option).clicked() {
                        changed = true;
                    }
                }
            })
            .response
            .on_hover_text("Select a parameter value");

        // If the value has changed, call set_selected_value
        if changed {
            self.set_selected_value(current_value);
        }

        response
    }
}


/*
use nih_plug::prelude::{Param, ParamSetter};
use nih_plug_egui::egui::{ComboBox, Response, Ui, Widget};

pub struct ParamComboBox<'a, P: Param> {
    param: &'a P,
    setter: &'a ParamSetter<'a>,

    options: Vec<String>,  // Options for the ComboBox
}

impl<'a, P: Param> ParamComboBox<'a, P> {
    pub fn for_param(param: &'a P, setter: &'a ParamSetter<'a>, options: Vec<String>) -> Self {
        Self {
            param,
            setter,
            options,
        }
    }

    fn set_selected_value(&self, selected_value: String) {
        // Convert the selected value back to the normalized parameter value and set it.
        if let Some(normalized_value) = self.param.string_to_normalized_value(&selected_value) {
            let value = self.param.preview_plain(normalized_value);
            if value != self.param.modulated_plain_value() {
                self.setter.set_parameter(self.param, value);
            }
        }
    }

    fn get_current_value(&self) -> String {
        self.param.to_string()
    }
}

impl<'a, P: Param> Widget for ParamComboBox<'a, P> {
    fn ui(self, ui: &mut Ui) -> Response {
        ComboBox::from_label(self.param.name())
            .selected_text(self.get_current_value())
            .show_ui(ui, |ui| {
                for option in &self.options {
                    ui.selectable_value(&mut self.get_current_value(), option.clone(), option);
                }
            })
            .response
            .on_hover_text("Select a parameter value")
    }
}
*/