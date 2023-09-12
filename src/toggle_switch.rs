// Ardura 2023 - Changing the toggle switch from egui demo to work with BoolParams for nih-plug
// https://github.com/emilk/egui/blob/master/crates/egui_demo_lib/src/demo/toggle_switch.rs


use nih_plug::prelude::{Param, ParamSetter};
use nih_plug_egui::egui::{
    Response,
    Ui, Widget, self, Rect,
};

struct SliderRegion<'a, P: Param> {
    param: &'a P,
    param_setter: &'a ParamSetter<'a>,
}

impl<'a, P: Param> SliderRegion<'a, P> {
    fn new(param: &'a P, param_setter: &'a ParamSetter) -> Self {
        SliderRegion {
            param,
            param_setter,
        }
    }

    // Handle the input for a given response. Returns an f32 containing the normalized value of
    // the parameter.
    fn handle_response(&self, ui: &Ui, response: &Response, rect: Rect) -> f32 {
        let value = self.param.unmodulated_normalized_value();

        // Check if our button is clicked
        if response.clicked() {
            if value == 0.0 {
                self.param_setter.set_parameter_normalized(self.param, 1.0);
            }
            else {
                self.param_setter.set_parameter_normalized(self.param, 0.0);
            }
        }

        // DRAWING
        let on_off = if self.param.default_normalized_value() == 1.0 { true } else { false };

        let how_on = ui.ctx().animate_bool(response.id, on_off );
        let visuals = ui.style().interact_selectable(&response, on_off);
        let rect = rect.expand(visuals.expansion);
        let radius = 0.5 * rect.height();
        ui.painter()
            .rect(rect, radius, visuals.bg_fill, visuals.bg_stroke);
        // Paint the circle, animating it from left to right with `how_on`:
        let circle_x = egui::lerp((rect.left() + radius)..=(rect.right() - radius), how_on);
        let center = egui::pos2(circle_x, rect.center().y);
        ui.painter()
            .circle(center, 0.75 * radius, visuals.bg_fill, visuals.fg_stroke);

        value
    }
}

pub struct ToggleSwitch<'a, P: Param> {
    slider_region: SliderRegion<'a, P>,
}

impl<'a, P: Param> ToggleSwitch<'a, P> {
    pub fn for_param(param: &'a P, param_setter: &'a ParamSetter) -> Self {
        ToggleSwitch {
            slider_region: SliderRegion::new(param, param_setter),
        }
    }
}

impl<'a, P: Param> Widget for ToggleSwitch<'a, P> {
    fn ui(self, ui: &mut Ui) -> Response {
        // Figure out the size to reserve on screen for widget
        let desired_size = ui.spacing().interact_size.y * egui::vec2(2.0, 1.0);
        let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::click());

        self.slider_region.handle_response(&ui, &response, rect);

        response
    }
}

