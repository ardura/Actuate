// Ardura 2023 - Changing the toggle_switch.rs to be a Button now that I've proven that works as a param for sample loading
// This lets me have buttons in nih-plug without using native egui (especially with turning off params for users)

use nih_plug::prelude::{Param, ParamSetter};
use nih_plug_egui::egui::{
    self, style::WidgetVisuals, Align2, Color32, FontId, Rect, Response, Stroke, Ui, Vec2, Widget,
};

struct SliderRegion<'a, P: Param> {
    param: &'a P,
    param_setter: &'a ParamSetter<'a>,
    font: FontId,
    background_color: Color32,
    text_color: Color32,
}

impl<'a, P: Param> SliderRegion<'a, P> {
    fn new(
        param: &'a P,
        param_setter: &'a ParamSetter,
        font: FontId,
        background_color: Color32,
        text_color: Color32,
    ) -> Self {
        SliderRegion {
            param,
            param_setter,
            font,
            background_color,
            text_color,
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
        ui.painter().rect(
            rect,
            0.5,
            if self.background_color == Color32::TEMPORARY_COLOR {
                visuals.bg_fill.linear_multiply(0.8)
            } else {
                self.background_color
            },
            visuals.bg_stroke,
        );
        // Paint the circle, animating it from left to right with `how_on`:
        ui.painter().rect_stroke(
            rect,
            0.5,
            Stroke::new(
                1.0,
                visuals
                    .bg_stroke
                    .color
                    .linear_multiply((how_on + 0.2).clamp(0.2, 1.2)),
            ),
        );
        let center = egui::pos2(rect.center().x, rect.center().y);
        ui.painter().text(
            center,
            Align2::CENTER_CENTER,
            self.param.name(),
            self.font.clone(),
            if self.text_color == Color32::TEMPORARY_COLOR {
                visuals.text_color()
            } else {
                self.text_color
            },
        );

        value
    }
}

pub struct BoolButton<'a, P: Param> {
    slider_region: SliderRegion<'a, P>,
    // Scaling is in ui.spacing().interact_size.y Units
    scaling_x: f32,
    scaling_y: f32,
    deselect_timer: usize,
    inactive_iterator: usize,
}

#[allow(dead_code)]
/// Create a BoolButton Object sized by ui.spacing().interact_size.y Units
impl<'a, P: Param> BoolButton<'a, P> {
    pub fn for_param(
        param: &'a P,
        param_setter: &'a ParamSetter,
        x_scaling: f32,
        y_scaling: f32,
        font: FontId,
    ) -> Self {
        BoolButton {
            // Pass things to slider to get around
            slider_region: SliderRegion::new(
                param,
                param_setter,
                font,
                Color32::TEMPORARY_COLOR,
                Color32::TEMPORARY_COLOR,
            ),
            scaling_x: x_scaling,
            scaling_y: y_scaling,
            deselect_timer: 200,
            inactive_iterator: 0,
        }
    }

    // The time for the button to become available again
    pub fn with_deselect_timer(mut self, amount_in_samples: usize) -> Self {
        self.deselect_timer = amount_in_samples;
        self
    }

    // To be called from gui thread to move from inactive to active
    pub fn increment_deselect(mut self) {
        self.inactive_iterator += 1;
    }

    pub fn with_background_color(mut self, new_color: Color32) -> Self {
        self.slider_region.background_color = new_color;
        self
    }

    pub fn with_text_color(mut self, new_color: Color32) -> Self {
        self.slider_region.background_color = new_color;
        self
    }
}

impl<'a, P: Param> Widget for BoolButton<'a, P> {
    fn ui(self, ui: &mut Ui) -> Response {
        // Figure out the size to reserve on screen for widget
        let (rect, response) = ui.allocate_exact_size(
            ui.spacing().interact_size.y * Vec2::new(self.scaling_x, self.scaling_y),
            egui::Sense::click(),
        );
        self.slider_region.handle_response(&ui, &response, rect);
        if self.inactive_iterator < self.deselect_timer {
            self.increment_deselect();
        }
        response
    }
}
