// Ardura 2024 - Visual representation of the curves in a button format!
// This is a custom widget so everything is specific to the beizer curves Actuate uses

use nih_plug::prelude::{Param, ParamSetter};
use nih_plug_egui::egui::{
    self,
    epaint::{CircleShape, CubicBezierShape},
    style::WidgetVisuals,
    Align2, Color32, FontId, Pos2, Rect, Response, Rounding, Shape, Stroke, Ui, Vec2, Widget,
};

pub enum ButtonLayout {
    HorizontalInline,
    Vertical,
}

struct SliderRegion<'a, P: Param> {
    param: &'a P,
    param_setter: &'a ParamSetter<'a>,
    background_color: Color32,
    line_color: Color32,
    button_layout: ButtonLayout,
}

impl<'a, P: Param> SliderRegion<'a, P> {
    fn new(
        param: &'a P,
        param_setter: &'a ParamSetter,
        background_color: Color32,
        line_color: Color32,
        button_layout: ButtonLayout,
    ) -> Self {
        SliderRegion {
            param,
            param_setter,
            background_color,
            line_color,
            button_layout,
        }
    }

    // Handle the input for a given response. Returns an f32 containing the normalized value of
    // the parameter.
    fn handle_response(&mut self, ui: &Ui, response: &Response, rect: Rect) -> f32 {
        let visuals: WidgetVisuals;
        visuals = ui.style().interact_selectable(&response, true);
        let value = self.param.unmodulated_normalized_value();
        let rect = rect.expand(visuals.expansion);
        let spacer = 12.0;
        let hspacer = 6.0;
        let mut control_points = match self.button_layout {
            ButtonLayout::Vertical => {[
                Pos2 {
                    x: rect.left_top().x + spacer,
                    y: rect.left_top().y + spacer,
                },
                rect.center(),
                Pos2 {
                    x: rect.right_bottom().x - spacer,
                    y: rect.right_bottom().y - 10.0 - spacer,
                },
                Pos2 {
                    x: rect.right_bottom().x - spacer,
                    y: rect.right_bottom().y - 10.0 - spacer,
                },
            ]},
            ButtonLayout::HorizontalInline => {[
                Pos2 {
                    x: rect.right_top().x + hspacer - rect.height(),
                    y: rect.right_top().y + hspacer,
                },
                Pos2 {
                    x: ((rect.right_bottom().x - hspacer) + (rect.right_top().x + hspacer - rect.height())) * 0.5,
                    y: ((rect.right_bottom().y - hspacer) + (rect.right_top().y + hspacer)) * 0.5,
                },
                Pos2 {
                    x: rect.right_bottom().x - hspacer,
                    y: rect.right_bottom().y - 2.0 - hspacer,
                },
                Pos2 {
                    x: rect.right_bottom().x - hspacer,
                    y: rect.right_bottom().y - 2.0 - hspacer,
                },
            ]},
        };

        // Check if our button is clicked
        if response.clicked() {
            match self.button_layout {
                ButtonLayout::Vertical => {
                    if value == 0.0 {
                        self.param_setter
                            .set_parameter_normalized(self.param, 0.333333343);
                        control_points[1] = rect.center();
                    } else if value == 0.333333343 {
                        self.param_setter
                            .set_parameter_normalized(self.param, 0.666666687);
                        control_points[1] = rect.left_center();
                    } else if value == 0.666666687 {
                        self.param_setter.set_parameter_normalized(self.param, 1.0);
                        control_points[1] = Pos2 {
                            x: rect.left_center().x + spacer,
                            y: rect.left_center().y + 40.0,
                        };
                    } else if value == 1.0 {
                        self.param_setter.set_parameter_normalized(self.param, 0.0);
                        control_points[1] = rect.right_center();
                    }
                },
                ButtonLayout::HorizontalInline => {
                    if value == 0.0 {
                        self.param_setter
                            .set_parameter_normalized(self.param, 0.333333343);
                        control_points[1] = Pos2 {
                            x: rect.center_bottom().x + rect.height()*0.5,
                            y: rect.center_bottom().y
                        };
                    } else if value == 0.333333343 {
                        self.param_setter
                            .set_parameter_normalized(self.param, 0.666666687);
                        control_points[1] = Pos2 {
                            x: rect.center_bottom().x + rect.height()*0.2,
                            y: rect.center_bottom().y
                        };
                    } else if value == 0.666666687 {
                        self.param_setter.set_parameter_normalized(self.param, 1.0);
                        control_points[1] = Pos2 {
                            x: rect.right_top().x - rect.height()*0.25,
                            y: rect.right_top().y
                        };
                    } else if value == 1.0 {
                        self.param_setter.set_parameter_normalized(self.param, 0.0);
                        control_points[1] = Pos2 {
                            x: ((rect.right_bottom().x - hspacer) + (rect.right_top().x + hspacer - rect.height())) * 0.5,
                            y: ((rect.right_bottom().y - hspacer) + (rect.right_top().y + hspacer)) * 0.5,
                        };
                    }
                }
            }
            
        } else {
            match self.button_layout {
                ButtonLayout::Vertical => {
                    if value == 0.0 {
                        control_points[1] = rect.center();
                    } else if value == 0.333333343 {
                        control_points[1] = rect.left_center();
                    } else if value == 0.666666687 {
                        control_points[1] = Pos2 {
                            x: rect.left_center().x,
                            y: rect.left_center().y + 40.0,
                        };
                    } else if value == 1.0 {
                        control_points[1] = rect.right_center();
                    }
                },
                ButtonLayout::HorizontalInline => {
                    if value == 0.0 {
                        control_points[1] = Pos2 {
                            x: ((rect.right_bottom().x - hspacer) + (rect.right_top().x - hspacer - rect.height())) * 0.5,
                            y: ((rect.right_bottom().y - hspacer) + (rect.right_top().y - hspacer)) * 0.5,
                        };
                    } else if value == 0.333333343 {
                        control_points[1] = Pos2 {
                            x: rect.center_bottom().x + rect.height()*0.5,
                            y: rect.center_bottom().y
                        };
                    } else if value == 0.666666687 {
                        control_points[1] = Pos2 {
                            x: rect.center_bottom().x + rect.height()*0.2,
                            y: rect.center_bottom().y
                        };
                    } else if value == 1.0 {
                        control_points[1] = Pos2 {
                            x: rect.right_top().x - rect.height()*0.25,
                            y: rect.right_top().y
                        };
                    }
                }
            }
            
        }

        // DRAWING
        ui.painter().rect(
            match self.button_layout {
                ButtonLayout::Vertical => {
                    Rect {
                        min: rect.left_top(),
                        max: Pos2 {
                            x: rect.right_bottom().x,
                            y: rect.right_bottom().y - 16.0,
                        },
                    }
                },
                ButtonLayout::HorizontalInline => {
                    Rect {
                        min: Pos2 { 
                            x: rect.right_top().x - rect.height(),
                            y: rect.right_top().y },
                        max: Pos2 {
                            x: rect.right_bottom().x,
                            y: rect.right_bottom().y,
                        },
                    }
                }
            },
            Rounding::from(4.0),
            if self.background_color == Color32::TEMPORARY_COLOR {
                visuals.bg_fill.linear_multiply(0.8)
            } else {
                self.background_color
            },
            visuals.bg_stroke,
        );
        ui.painter().rect(
            rect,
            Rounding::from(4.0),
            if self.background_color == Color32::TEMPORARY_COLOR {
                visuals.bg_fill.linear_multiply(0.8)
            } else {
                self.background_color.linear_multiply(0.8)
            },
            visuals.bg_stroke,
        );
        let start_ball = Shape::Circle(CircleShape {
            center: control_points[0],
            radius: 4.0,
            fill: self.line_color,
            stroke: Stroke::NONE,
        });
        ui.painter().add(start_ball);
        let end_ball = Shape::Circle(CircleShape {
            center: control_points[2],
            radius: 4.0,
            fill: self.line_color,
            stroke: Stroke::NONE,
        });
        ui.painter().add(end_ball);
        // Paint the Beizers
        let shape = CubicBezierShape::from_points_stroke(
            control_points,
            false,
            Color32::TRANSPARENT,
            Stroke::new(
                3.0,
                if self.line_color == Color32::TEMPORARY_COLOR {
                    visuals.fg_stroke.color
                } else {
                    self.line_color
                },
            ),
        );
        /*
        ui.painter().add(epaint::RectShape::stroke(
            shape.visual_bounding_rect(),
            0.0,
            Stroke::new(3.0, visuals.fg_stroke.color),
        ));
        */
        ui.painter().add(shape);
        match self.button_layout {
            ButtonLayout::Vertical => {
                ui.painter().text(
                    Pos2 {
                        x: rect.center_bottom().x,
                        y: rect.center_bottom().y - 8.0,
                    },
                    Align2::CENTER_CENTER,
                    self.param.name(),
                    FontId::proportional(11.0),
                    Color32::WHITE.linear_multiply(0.5),
                );
            },
            ButtonLayout::HorizontalInline => {
                ui.painter().text(
                    Pos2 {
                        x: rect.left_center().x + hspacer,
                        y: rect.left_center().y,
                    },
                    Align2::LEFT_CENTER,
                    self.param.name(),
                    FontId::proportional(11.0),
                    Color32::WHITE.linear_multiply(0.5),
                );
            }
        }

        value
    }
}

pub struct BeizerButton<'a, P: Param> {
    slider_region: SliderRegion<'a, P>,
    // Scaling is in ui.spacing().interact_size.y Units
    scaling_x: f32,
    scaling_y: f32,
}

#[allow(dead_code)]
/// Create a BeizerButton Object sized by ui.spacing().interact_size.y Units
impl<'a, P: Param> BeizerButton<'a, P> {
    pub fn for_param(
        param: &'a P,
        param_setter: &'a ParamSetter,
        x_scaling: f32,
        y_scaling: f32,
        button_layout: ButtonLayout,
    ) -> Self {
        BeizerButton {
            // Pass things to slider to get around
            slider_region: SliderRegion::new(
                param,
                param_setter,
                Color32::TEMPORARY_COLOR,
                Color32::TEMPORARY_COLOR,
                button_layout,
            ),
            scaling_x: x_scaling,
            scaling_y: y_scaling,
        }
    }

    pub fn with_background_color(mut self, new_color: Color32) -> Self {
        self.slider_region.background_color = new_color;
        self
    }

    pub fn with_line_color(mut self, new_color: Color32) -> Self {
        self.slider_region.line_color = new_color;
        self
    }
}

impl<'a, P: Param> Widget for BeizerButton<'a, P> {
    fn ui(mut self, ui: &mut Ui) -> Response {
        // Figure out the size to reserve on screen for widget
        let (rect, response) = ui.allocate_exact_size(
            ui.spacing().interact_size.y * Vec2::new(self.scaling_x, self.scaling_y),
            egui::Sense::click(),
        );
        self.slider_region.handle_response(&ui, &response, rect);
        response
    }
}
