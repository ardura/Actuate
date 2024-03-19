// Ardura 2023 - Changing the toggle switch from egui demo to work with BoolParams for nih-plug
// https://github.com/emilk/egui/blob/master/crates/egui_demo_lib/src/demo/toggle_switch.rs

use nih_plug::prelude::{Param, ParamSetter};
use nih_plug_egui::egui::{self, style::WidgetVisuals, Color32, Rect, Response, Stroke, Ui, Widget};

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
        let mut value = self.param.modulated_normalized_value();
        let how_on;
        let visuals: WidgetVisuals;

        // Check if our button is clicked
        if response.clicked() {
            if value == 0.0 {
                self.param_setter.set_parameter_normalized(self.param, 1.0);
                how_on = ui.ctx().animate_bool(response.id, true);
                visuals = ui.style().interact_selectable(&response, true);
                value = 1.0;
            } else {
                self.param_setter.set_parameter_normalized(self.param, 0.0);
                how_on = ui.ctx().animate_bool(response.id, false);
                visuals = ui.style().interact_selectable(&response, false);
                value = 0.0;
            }
        } else {
            let temp: bool = if value > 0.0 { true } else { false };
            how_on = ui.ctx().animate_bool(response.id, temp);
            visuals = ui.style().interact_selectable(&response, temp);
        }

        // DRAWING
        let rect = rect.expand(visuals.expansion);
        let radius = 0.5 * rect.height();
        ui.painter()
            .rect(rect, radius, visuals.bg_fill, visuals.bg_stroke);
        // Paint the circle, animating it from left to right with `how_on`:
        let circle_x = egui::lerp((rect.left() + radius)..=(rect.right() - radius), how_on);
        let center = egui::pos2(circle_x, rect.center().y);
        ui.painter()
            .circle(center, 0.75 * radius, visuals.bg_fill, if how_on > 0.0 { Stroke::new(1.0, Color32::BLACK) } else { visuals.fg_stroke });

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
