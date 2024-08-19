// Ardura 2024 update - ui_knob.rs - egui + nih-plug parameter widget with customization
//  this ui_knob.rs is built off a2aaron's knob base as part of nyasynth and Robbert's ParamSlider code
// https://github.com/a2aaron/nyasynth/blob/canon/src/ui_knob.rs

use std::{
    f32::consts::TAU,
    ops::{Add, Mul, Sub},
};

use lazy_static::lazy_static;
use nih_plug::prelude::{Param, ParamSetter};
use nih_plug_egui::egui::{
    self,
    epaint::{CircleShape, PathShape, PathStroke},
    pos2, Align2, Color32, FontId, Pos2, Rect, Response, Rgba, Rounding, Sense, Shape, Stroke, Ui,
    Vec2, Widget,
};

/// When shift+dragging a parameter, one pixel dragged corresponds to this much change in the
/// noramlized parameter.
const GRANULAR_DRAG_MULTIPLIER: f32 = 0.001;
const NORMAL_DRAG_MULTIPLIER: f32 = 0.005;

lazy_static! {
    static ref DRAG_NORMALIZED_START_VALUE_MEMORY_ID: egui::Id = egui::Id::new((file!(), 0));
    static ref DRAG_AMOUNT_MEMORY_ID: egui::Id = egui::Id::new((file!(), 1));
    static ref VALUE_ENTRY_MEMORY_ID: egui::Id = egui::Id::new((file!(), 2));
}

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

    fn set_normalized_value(&self, normalized: f32) {
        // This snaps to the nearest plain value if the parameter is stepped in some way.
        // TODO: As an optimization, we could add a `const CONTINUOUS: bool` to the parameter to
        //       avoid this normalized->plain->normalized conversion for parameters that don't need
        //       it
        let value = self.param.preview_plain(normalized);
        if value != self.plain_value() {
            self.param_setter.set_parameter(self.param, value);
        }
    }

    fn plain_value(&self) -> P::Plain {
        self.param.modulated_plain_value()
    }

    fn normalized_value(&self) -> f32 {
        self.param.modulated_normalized_value()
    }

    fn get_drag_normalized_start_value_memory(ui: &Ui) -> f32 {
        ui.memory(|mem| mem.data.get_temp(*DRAG_NORMALIZED_START_VALUE_MEMORY_ID))
            .unwrap_or(0.5)
    }

    fn set_drag_normalized_start_value_memory(ui: &Ui, amount: f32) {
        ui.memory_mut(|mem| {
            mem.data
                .insert_temp(*DRAG_NORMALIZED_START_VALUE_MEMORY_ID, amount)
        });
    }

    fn get_drag_amount_memory(ui: &Ui) -> f32 {
        ui.memory(|mem| mem.data.get_temp(*DRAG_AMOUNT_MEMORY_ID))
            .unwrap_or(0.0)
    }

    fn set_drag_amount_memory(ui: &Ui, amount: f32) {
        ui.memory_mut(|mem| mem.data.insert_temp(*DRAG_AMOUNT_MEMORY_ID, amount));
    }

    /// Begin and end drag still need to be called when using this..
    fn reset_param(&self) {
        self.param_setter
            .set_parameter(self.param, self.param.default_plain_value());
    }

    fn granular_drag(&self, ui: &Ui, drag_delta: Vec2) {
        // Remember the intial position when we started with the granular drag. This value gets
        // reset whenever we have a normal itneraction with the slider.
        let start_value = if Self::get_drag_amount_memory(ui) == 0.0 {
            Self::set_drag_normalized_start_value_memory(ui, self.normalized_value());
            self.normalized_value()
        } else {
            Self::get_drag_normalized_start_value_memory(ui)
        };

        let total_drag_distance = -drag_delta.y + Self::get_drag_amount_memory(ui);
        Self::set_drag_amount_memory(ui, total_drag_distance);

        self.set_normalized_value(
            (start_value + (total_drag_distance * GRANULAR_DRAG_MULTIPLIER)).clamp(0.0, 1.0),
        );
    }

    // Copied this to modify the normal drag behavior to not match a slider
    fn normal_drag(&self, ui: &Ui, drag_delta: Vec2) {
        let start_value = if Self::get_drag_amount_memory(ui) == 0.0 {
            Self::set_drag_normalized_start_value_memory(ui, self.normalized_value());
            self.normalized_value()
        } else {
            Self::get_drag_normalized_start_value_memory(ui)
        };

        let total_drag_distance = -drag_delta.y + Self::get_drag_amount_memory(ui);
        Self::set_drag_amount_memory(ui, total_drag_distance);

        self.set_normalized_value(
            (start_value + (total_drag_distance * NORMAL_DRAG_MULTIPLIER)).clamp(0.0, 1.0),
        );
    }

    // Handle the input for a given response. Returns an f32 containing the normalized value of
    // the parameter.
    fn handle_response(&self, ui: &Ui, response: &mut Response) -> f32 {
        // This has been replaced with the ParamSlider/CustomParamSlider structure and supporting
        // functions (above) since that was still working in egui 0.22

        if response.drag_started() {
            // When beginning a drag or dragging normally, reset the memory used to keep track of
            // our granular drag
            self.param_setter.begin_set_parameter(self.param);
            Self::set_drag_amount_memory(ui, 0.0);
        }
        if let Some(_clicked_pos) = response.interact_pointer_pos() {
            if ui.input(|mem| mem.modifiers.command) {
                // Like double clicking, Ctrl+Click should reset the parameter
                self.reset_param();
                response.mark_changed();
            } else if ui.input(|mem| mem.modifiers.shift) {
                // And shift dragging should switch to a more granular input method
                self.granular_drag(ui, response.drag_delta());
                response.mark_changed();
            } else {
                self.normal_drag(ui, response.drag_delta());
                response.mark_changed();
                //Self::set_drag_amount_memory(ui, 0.0);
            }
        }
        if response.double_clicked() {
            self.reset_param();
            response.mark_changed();
        }
        if response.drag_stopped() {
            self.param_setter.end_set_parameter(self.param);
            Self::set_drag_amount_memory(ui, 0.0);
        }
        self.normalized_value()
    }

    fn get_string(&self) -> String {
        self.param.to_string()
    }
}

pub struct ArcKnob<'a, P: Param> {
    slider_region: SliderRegion<'a, P>,
    radius: f32,
    line_color: Color32,
    fill_color: Color32,
    center_size: f32,
    line_width: f32,
    center_to_line_space: f32,
    hover_text: bool,
    hover_text_content: String,
    label_text: String,
    show_center_value: bool,
    text_size: f32,
    outline: bool,
    padding: f32,
    show_label: bool,
    swap_label_and_value: bool,
    text_color_override: Color32,
    readable_box: bool,
    layout: KnobLayout,
    arc_start: f32,
    arc_end: f32,
}

#[allow(dead_code)]
pub enum KnobStyle {
    Preset1,
    Preset2,
}

#[allow(dead_code)]
#[derive(Copy, Clone, PartialEq)]
pub enum KnobLayout {
    Vertical,
    Horizonal,
    HorizontalInline,
    SquareNoLabel,
    Default,
}

#[allow(dead_code)]
impl<'a, P: Param> ArcKnob<'a, P> {
    pub fn for_param(
        param: &'a P,
        param_setter: &'a ParamSetter,
        radius: f32,
        layout: KnobLayout,
    ) -> Self {
        ArcKnob {
            slider_region: SliderRegion::new(param, param_setter),
            radius: radius,
            line_color: Color32::BLACK,
            fill_color: Color32::BLACK,
            center_size: 20.0,
            line_width: 2.0,
            center_to_line_space: 0.0,
            hover_text: false,
            hover_text_content: String::new(),
            text_size: 16.0,
            label_text: String::new(),
            show_center_value: true,
            outline: false,
            padding: 10.0,
            show_label: true,
            swap_label_and_value: true,
            text_color_override: Color32::PLACEHOLDER,
            readable_box: false,
            layout: layout,
            arc_start: match layout {
                KnobLayout::Default => 0.75,
                KnobLayout::SquareNoLabel => 0.625,
                KnobLayout::Vertical => 0.625,
                KnobLayout::Horizonal => 0.625,
                KnobLayout::HorizontalInline => 0.625,
            },
            arc_end: match layout {
                KnobLayout::Default => -1.0,
                KnobLayout::SquareNoLabel => -0.75,
                KnobLayout::Vertical => -0.75,
                KnobLayout::Horizonal => -0.75,
                KnobLayout::HorizontalInline => -0.75,
            },
        }
    }

    // Set readability box visibility for text on other colors
    pub fn set_readable_box(mut self, show_box: bool) -> Self {
        self.readable_box = show_box;
        self
    }

    // Change the text color if you want it separate from line color
    pub fn override_text_color(mut self, text_color: Color32) -> Self {
        self.text_color_override = text_color;
        self
    }

    // Undo newer swap label and value
    pub fn set_swap_label_and_value(mut self, use_old: bool) -> Self {
        self.swap_label_and_value = use_old;
        self
    }

    // Specify outline drawing
    pub fn use_outline(mut self, new_bool: bool) -> Self {
        self.outline = new_bool;
        self
    }

    // Specify showing value when mouse-over
    pub fn use_hover_text(mut self, new_bool: bool) -> Self {
        self.hover_text = new_bool;
        self
    }

    // Specify value when mouse-over
    pub fn set_hover_text(mut self, new_text: String) -> Self {
        self.hover_text_content = new_text;
        self
    }

    // Specify knob label
    pub fn set_label(mut self, new_label: String) -> Self {
        self.label_text = new_label;
        self
    }

    // Specify line color for knob outside
    pub fn set_line_color(mut self, new_color: Color32) -> Self {
        self.line_color = new_color;
        self
    }

    // Specify fill color for knob
    pub fn set_fill_color(mut self, new_color: Color32) -> Self {
        self.fill_color = new_color;
        self
    }

    // Specify center knob size
    pub fn set_center_size(mut self, size: f32) -> Self {
        self.center_size = size;
        self
    }

    // Specify line width
    pub fn set_line_width(mut self, width: f32) -> Self {
        self.line_width = width;
        self
    }

    // Specify distance between center and arc
    pub fn set_center_to_line_space(mut self, new_width: f32) -> Self {
        self.center_to_line_space = new_width;
        self
    }

    // Set text size for label
    pub fn set_text_size(mut self, text_size: f32) -> Self {
        self.text_size = text_size;
        self
    }

    // Set knob padding
    pub fn set_padding(mut self, padding: f32) -> Self {
        self.padding = padding;
        self
    }

    // Set center value of knob visibility
    pub fn set_show_center_value(mut self, new_bool: bool) -> Self {
        self.show_center_value = new_bool;
        self
    }

    // Set center value of knob visibility
    pub fn set_show_label(mut self, new_bool: bool) -> Self {
        self.show_label = new_bool;
        self
    }

    pub fn preset_style(mut self, style_id: KnobStyle) -> Self {
        // These are all calculated off radius to scale better
        match style_id {
            KnobStyle::Preset1 => {
                self.center_size = self.radius * 0.7;
                self.line_width = self.radius * 0.3;
                self.center_to_line_space = self.radius * 0.012;
                self.padding = 0.0;
                self.outline = true;
                self.hover_text = true;
            }
            KnobStyle::Preset2 => {
                self.center_size = self.radius * 0.5;
                self.line_width = self.radius * 0.5;
                self.center_to_line_space = self.radius * 0.0125;
                self.padding = 0.0;
            }
        }
        self
    }
}

impl<'a, P: Param> Widget for ArcKnob<'a, P> {
    fn ui(mut self, ui: &mut Ui) -> Response {
        // Figure out the size to reserve on screen for widget
        let desired_size: Vec2 = match self.layout {
            KnobLayout::Horizonal => egui::vec2(
                self.padding + self.radius * 4.0,
                self.padding + self.radius * 2.0,
            ),
            KnobLayout::Vertical => egui::vec2(
                self.padding + self.radius * 2.0,
                self.padding + self.radius * 3.0,
            ),
            KnobLayout::HorizontalInline => egui::vec2(
                self.padding + self.radius * 9.0,
                self.padding + self.radius * 2.0,
            ),
            KnobLayout::SquareNoLabel => egui::vec2(
                self.padding + self.radius * 2.0,
                self.padding + self.radius * 2.0,
            ),
            KnobLayout::Default => egui::vec2(
                self.padding + self.radius * 2.0,
                self.padding + self.radius * 2.0,
            ),
        };

        let mut response = ui.allocate_response(desired_size, Sense::click_and_drag());
        let value = self.slider_region.handle_response(&ui, &mut response);

        ui.vertical(|ui| {
            let painter = ui.painter_at(response.rect);
            let center = match self.layout {
                KnobLayout::Default | KnobLayout::SquareNoLabel => response.rect.center(),
                KnobLayout::Vertical => response.rect.center(),
                KnobLayout::Horizonal => Pos2 {
                    x: response.rect.left_center().x + self.radius,
                    y: response.rect.left_center().y,
                },
                KnobLayout::HorizontalInline => Pos2 {
                    x: response.rect.left_center().x + self.radius,
                    y: response.rect.left_center().y,
                },
            };

            // Background Rect
            ui.painter().rect_filled(
                response.rect,
                Rounding::from(4.0),
                Color32::BLACK.linear_multiply(0.1),
            );
            ui.painter().rect_filled(
                response.rect,
                Rounding::from(4.0),
                self.fill_color.linear_multiply(0.4),
            );

            // Draw the outside ring around the control
            if self.outline {
                let outline_stroke = Stroke::new(1.0, self.fill_color.linear_multiply(0.7));
                let outline_shape = Shape::Path(PathShape {
                    points: get_arc_points(
                        self.arc_start,
                        self.arc_end,
                        center,
                        self.center_size + self.center_to_line_space + (self.line_width / 2.0),
                        1.0,
                        0.03,
                    ),
                    closed: false,
                    fill: self.fill_color.linear_multiply(0.7),
                    stroke: outline_stroke.into(),
                });
                painter.add(outline_shape);
            }

            // Draw the arc
            let arc_radius = self.center_size + self.center_to_line_space;
            let arc_stroke = Stroke::new(self.line_width, self.line_color);
            let shape = Shape::Path(PathShape {
                points: get_arc_points(
                    self.arc_start,
                    self.arc_end,
                    center,
                    arc_radius,
                    value,
                    0.03,
                ),
                closed: false,
                fill: Color32::TRANSPARENT,
                stroke: arc_stroke.into(),
            });
            painter.add(shape);

            // Arc Balls
            let ball_width = self.line_width / 5.0;
            let ball_line_stroke = Stroke::new(ball_width, self.line_color);
            let start_ball = Shape::Circle(CircleShape {
                center: get_start_point(self.arc_start, center, arc_radius + ball_width),
                radius: ball_width,
                fill: self.line_color,
                stroke: ball_line_stroke,
            });
            painter.add(start_ball);
            let end_ball = Shape::Circle(CircleShape {
                center: get_end_point(
                    self.arc_start,
                    self.arc_end,
                    center,
                    arc_radius + ball_width,
                    value,
                ),
                radius: ball_width,
                fill: self.line_color,
                stroke: ball_line_stroke,
            });
            painter.add(end_ball);

            //reset stroke here so we only have fill
            let line_stroke = Stroke::new(0.0, Color32::TRANSPARENT);

            // Center of Knob
            let circle_shape = Shape::Circle(CircleShape {
                center: center,
                radius: self.center_size,
                stroke: line_stroke,
                fill: self.fill_color,
            });
            painter.add(circle_shape);

            // Gradient values
            let g2 = value - 0.04;
            let g3 = value - 0.08;
            let g4 = value - 0.12;
            let g5 = value - 0.16;
            let g6 = value - 0.20;
            let g7 = value - 0.24;
            let g8 = value - 0.28;
            let g9 = value - 0.32;
            let g10 = value - 0.36;
            let g11 = value - 0.40;

            // Draw our marker lines/gradient
            let visual_end = self.arc_start - 1.375;
            let line_shape2 = Shape::Path(PathShape {
                points: get_pointer_points(
                    self.arc_start,
                    visual_end,
                    center,
                    arc_radius + ball_width,
                    g2,
                ),
                closed: false,
                fill: Color32::TRANSPARENT,
                stroke: PathStroke::new(
                    ball_width * 3.0,
                    if g2 > 0.0 {
                        Color32::DARK_GRAY.gamma_multiply(0.20)
                    } else {
                        Color32::TRANSPARENT
                    },
                ),
            });
            painter.add(line_shape2);
            let line_shape3 = Shape::Path(PathShape {
                points: get_pointer_points(
                    self.arc_start,
                    visual_end,
                    center,
                    arc_radius + ball_width,
                    g3,
                ),
                closed: false,
                fill: Color32::TRANSPARENT,
                stroke: PathStroke::new(
                    ball_width * 3.0,
                    if g3 > 0.0 {
                        Color32::DARK_GRAY.gamma_multiply(0.18)
                    } else {
                        Color32::TRANSPARENT
                    },
                ),
            });
            painter.add(line_shape3);
            let line_shape3 = Shape::Path(PathShape {
                points: get_pointer_points(
                    self.arc_start,
                    visual_end,
                    center,
                    arc_radius + ball_width,
                    g4,
                ),
                closed: false,
                fill: Color32::TRANSPARENT,
                stroke: PathStroke::new(
                    ball_width * 3.0,
                    if g4 > 0.0 {
                        Color32::DARK_GRAY.gamma_multiply(0.16)
                    } else {
                        Color32::TRANSPARENT
                    },
                ),
            });
            painter.add(line_shape3);
            let line_shape4 = Shape::Path(PathShape {
                points: get_pointer_points(
                    self.arc_start,
                    visual_end,
                    center,
                    arc_radius + ball_width,
                    g5,
                ),
                closed: false,
                fill: Color32::TRANSPARENT,
                stroke: PathStroke::new(
                    ball_width * 3.0,
                    if g5 > 0.0 {
                        Color32::DARK_GRAY.gamma_multiply(0.14)
                    } else {
                        Color32::TRANSPARENT
                    },
                ),
            });
            painter.add(line_shape4);
            let line_shape5 = Shape::Path(PathShape {
                points: get_pointer_points(
                    self.arc_start,
                    visual_end,
                    center,
                    arc_radius + ball_width,
                    g6,
                ),
                closed: false,
                fill: Color32::TRANSPARENT,
                stroke: PathStroke::new(
                    ball_width * 3.0,
                    if g6 > 0.0 {
                        Color32::DARK_GRAY.gamma_multiply(0.12)
                    } else {
                        Color32::TRANSPARENT
                    },
                ),
            });
            painter.add(line_shape5);
            let line_shape6 = Shape::Path(PathShape {
                points: get_pointer_points(
                    self.arc_start,
                    visual_end,
                    center,
                    arc_radius + ball_width,
                    g7,
                ),
                closed: false,
                fill: Color32::TRANSPARENT,
                stroke: PathStroke::new(
                    ball_width * 3.0,
                    if g7 > 0.0 {
                        Color32::DARK_GRAY.gamma_multiply(0.10)
                    } else {
                        Color32::TRANSPARENT
                    },
                ),
            });
            painter.add(line_shape6);
            let line_shape7 = Shape::Path(PathShape {
                points: get_pointer_points(
                    self.arc_start,
                    visual_end,
                    center,
                    arc_radius + ball_width,
                    g8,
                ),
                closed: false,
                fill: Color32::TRANSPARENT,
                stroke: PathStroke::new(
                    ball_width * 3.0,
                    if g8 > 0.0 {
                        Color32::DARK_GRAY.gamma_multiply(0.08)
                    } else {
                        Color32::TRANSPARENT
                    },
                ),
            });
            painter.add(line_shape7);
            let line_shape8 = Shape::Path(PathShape {
                points: get_pointer_points(
                    self.arc_start,
                    visual_end,
                    center,
                    arc_radius + ball_width,
                    g9,
                ),
                closed: false,
                fill: Color32::TRANSPARENT,
                stroke: PathStroke::new(
                    ball_width * 3.0,
                    if g9 > 0.0 {
                        Color32::DARK_GRAY.gamma_multiply(0.06)
                    } else {
                        Color32::TRANSPARENT
                    },
                ),
            });
            painter.add(line_shape8);
            let line_shape9 = Shape::Path(PathShape {
                points: get_pointer_points(
                    self.arc_start,
                    visual_end,
                    center,
                    arc_radius + ball_width,
                    g10,
                ),
                closed: false,
                fill: Color32::TRANSPARENT,
                stroke: PathStroke::new(
                    ball_width * 3.0,
                    if g10 > 0.0 {
                        Color32::DARK_GRAY.gamma_multiply(0.04)
                    } else {
                        Color32::TRANSPARENT
                    },
                ),
            });
            painter.add(line_shape9);
            let line_shape10 = Shape::Path(PathShape {
                points: get_pointer_points(
                    self.arc_start,
                    visual_end,
                    center,
                    arc_radius + ball_width,
                    g11,
                ),
                closed: false,
                fill: Color32::TRANSPARENT,
                stroke: PathStroke::new(
                    ball_width * 3.0,
                    if g11 > 0.0 {
                        Color32::DARK_GRAY.gamma_multiply(0.02)
                    } else {
                        Color32::TRANSPARENT
                    },
                ),
            });
            painter.add(line_shape10);

            let line_shape = Shape::Path(PathShape {
                points: get_pointer_points(
                    self.arc_start,
                    self.arc_end,
                    center,
                    arc_radius + ball_width,
                    value,
                ),
                closed: false,
                fill: self.line_color,
                stroke: PathStroke::new(ball_width * 3.0, self.line_color),
            });
            painter.add(line_shape);

            let center_ball = Shape::Circle(CircleShape {
                center: center,
                radius: ball_width,
                fill: self.line_color,
                stroke: ball_line_stroke,
            });
            painter.add(center_ball);

            // Hover text of value
            if self.hover_text {
                if self.hover_text_content.is_empty() {
                    self.hover_text_content = self.slider_region.get_string();
                }
                ui.allocate_rect(
                    Rect::from_center_size(center, Vec2::new(self.radius * 2.0, self.radius * 2.0)),
                    Sense::hover(),
                ).on_hover_text_at_pointer(self.hover_text_content);
            }

            // Label text from response rect bound
            let label_y = if self.padding == 0.0 {
                6.0
            } else {
                self.padding * 2.0
            };
            if self.show_label {
                let value_pos: Pos2;
                let label_pos: Pos2;
                match self.layout {
                    KnobLayout::SquareNoLabel => {
                        // This isn't indended to be a possibility but it has to be here since
                        // it's in the enum. Hence these making no sense
                        value_pos = response.rect.center();
                        label_pos = response.rect.center();
                    },
                    KnobLayout::Default => {
                        if self.swap_label_and_value {
                            // Newer rearranged positions to put value at bottom of knob
                            value_pos = Pos2::new(
                                response.rect.center_bottom().x,
                                response.rect.center_bottom().y - label_y,
                            );
                            label_pos =
                                Pos2::new(response.rect.center().x, response.rect.center().y);
                        } else {
                            // The old value and label positions
                            label_pos = Pos2::new(
                                response.rect.center_bottom().x,
                                response.rect.center_bottom().y - label_y,
                            );
                            value_pos =
                                Pos2::new(response.rect.center().x, response.rect.center().y);
                        }
                    }
                    // GUI Rewrite for Actuate made these
                    KnobLayout::Vertical => {
                        label_pos = Pos2::new(
                            response.rect.center_top().x,
                            response.rect.center_top().y + label_y * 1.5,
                        );
                        value_pos = Pos2::new(
                            response.rect.center_bottom().x,
                            response.rect.center_bottom().y - label_y * 1.5,
                        );
                    }
                    KnobLayout::Horizonal => {
                        label_pos = Pos2::new(
                            response.rect.center().x + self.radius / 1.5,
                            response.rect.right_center().y - label_y,
                        );
                        value_pos = Pos2::new(
                            response.rect.center().x + self.radius / 1.5,
                            response.rect.right_center().y + label_y,
                        );
                    }
                    KnobLayout::HorizontalInline => {
                        label_pos = Pos2::new(
                            response.rect.center().x + self.radius / 1.5,
                            response.rect.right_center().y,
                        );
                        value_pos = Pos2::new(
                            response.rect.center().x + self.radius / 1.5,
                            response.rect.right_center().y,
                        );
                    }
                }

                if self.readable_box {
                    // Background for text readability
                    let readability_box = Rect::from_two_pos(
                        response.rect.left_bottom(),
                        Pos2 {
                            x: response.rect.right_bottom().x,
                            y: response.rect.right_bottom().y - 12.0,
                        },
                    );
                    ui.painter().rect_filled(
                        readability_box,
                        Rounding::from(16.0),
                        self.fill_color,
                    );
                }

                let text_color: Color32;
                // Setting text color
                if self.text_color_override != Color32::PLACEHOLDER {
                    text_color = self.text_color_override;
                } else {
                    text_color = self.line_color;
                }

                if self.label_text.is_empty() {
                    if self.layout == KnobLayout::HorizontalInline {
                        painter.text(
                            label_pos,
                            Align2::CENTER_CENTER,
                            self.slider_region.param.name().to_owned()
                                + ": "
                                + &self.slider_region.get_string(),
                            FontId::proportional(self.text_size),
                            Color32::BLACK.linear_multiply(0.2),
                        );
                        painter.text(
                            label_pos,
                            Align2::CENTER_CENTER,
                            self.slider_region.param.name().to_owned()
                                + ": "
                                + &self.slider_region.get_string(),
                            FontId::proportional(self.text_size),
                            text_color.linear_multiply(0.4),
                        );
                    } else {
                        painter.text(
                            value_pos,
                            Align2::CENTER_CENTER,
                            self.slider_region.get_string(),
                            FontId::proportional(self.text_size),
                            Color32::WHITE.linear_multiply(0.1),
                        );
                        painter.text(
                            value_pos,
                            Align2::CENTER_CENTER,
                            self.slider_region.get_string(),
                            FontId::proportional(self.text_size),
                            text_color,
                        );
                        painter.text(
                            label_pos,
                            Align2::CENTER_CENTER,
                            self.slider_region.param.name(),
                            FontId::proportional(self.text_size),
                            Color32::BLACK.linear_multiply(0.2),
                        );
                        painter.text(
                            label_pos,
                            Align2::CENTER_CENTER,
                            self.slider_region.param.name(),
                            FontId::proportional(self.text_size),
                            text_color.linear_multiply(0.4),
                        );
                    }
                } else {
                    if self.layout == KnobLayout::HorizontalInline {
                        painter.text(
                            label_pos,
                            Align2::CENTER_CENTER,
                            self.label_text.to_string() + ": " + &self.slider_region.param.name(),
                            FontId::proportional(self.text_size),
                            Color32::BLACK.linear_multiply(0.2),
                        );
                        painter.text(
                            label_pos,
                            Align2::CENTER_CENTER,
                            self.label_text.to_string() + ": " + &self.slider_region.param.name(),
                            FontId::proportional(self.text_size),
                            text_color.linear_multiply(0.4),
                        );
                    } else {
                        painter.text(
                            value_pos,
                            Align2::CENTER_CENTER,
                            self.label_text.to_string(),
                            FontId::proportional(self.text_size),
                            Color32::WHITE.linear_multiply(0.1),
                        );
                        painter.text(
                            value_pos,
                            Align2::CENTER_CENTER,
                            self.label_text,
                            FontId::proportional(self.text_size),
                            text_color,
                        );
                        painter.text(
                            label_pos,
                            Align2::CENTER_CENTER,
                            self.slider_region.param.name(),
                            FontId::proportional(self.text_size),
                            Color32::BLACK.linear_multiply(0.2),
                        );
                        painter.text(
                            label_pos,
                            Align2::CENTER_CENTER,
                            self.slider_region.param.name(),
                            FontId::proportional(self.text_size),
                            text_color.linear_multiply(0.4),
                        );
                    }
                }
            }
        });
        response
    }
}

fn get_start_point(start: f32, center: Pos2, radius: f32) -> Pos2 {
    let start_turns: f32 = start;
    let angle = start_turns * TAU;
    let x = center.x + radius * angle.cos();
    let y = center.y + -radius * angle.sin();
    pos2(x, y)
}

fn get_end_point(start: f32, end: f32, center: Pos2, radius: f32, value: f32) -> Pos2 {
    let start_turns: f32 = start;
    let arc_length = lerp(0.0, end, value);
    let end_turns = start_turns + arc_length;

    let angle = end_turns * TAU;
    let x = center.x + radius * angle.cos();
    let y = center.y + -radius * angle.sin();
    pos2(x, y)
}

fn get_pointer_points(start: f32, end: f32, center: Pos2, radius: f32, value: f32) -> Vec<Pos2> {
    let start_turns: f32 = start;
    let arc_length = lerp(0.0, end, value);
    let end_turns = start_turns + arc_length;

    let angle = end_turns * TAU;
    let x = center.x + radius * angle.cos();
    let y = center.y + -radius * angle.sin();
    let short_x = center.x + (radius * 0.04) * angle.cos();
    let short_y = center.y + (-radius * 0.04) * angle.sin();
    vec![pos2(short_x, short_y), pos2(x, y)]
}

fn get_arc_points(
    start: f32,
    end: f32,
    center: Pos2,
    radius: f32,
    value: f32,
    max_arc_distance: f32,
) -> Vec<Pos2> {
    let start_turns: f32 = start;
    let arc_length = lerp(0.0, end, value);
    let end_turns = start_turns + arc_length;

    let points = (arc_length.abs() / max_arc_distance).ceil() as usize;
    let points = points.max(1);
    (0..=points)
        .map(|i| {
            let t = i as f32 / (points - 1) as f32;
            let angle = lerp(start_turns * TAU, end_turns * TAU, t);
            let x = radius * angle.cos();
            let y = -radius * angle.sin();
            pos2(x, y) + center.to_vec2()
        })
        .collect()
}

// Moved lerp to this file to reduce dependencies - Ardura
pub fn lerp<T>(start: T, end: T, t: f32) -> T
where
    T: Add<T, Output = T> + Sub<T, Output = T> + Mul<f32, Output = T> + Copy,
{
    (end - start) * t.clamp(0.0, 1.0) + start
}

pub struct TextSlider<'a, P: Param> {
    slider_region: SliderRegion<'a, P>,
    location: Rect,
}

#[allow(dead_code)]
impl<'a, P: Param> TextSlider<'a, P> {
    pub fn for_param(param: &'a P, param_setter: &'a ParamSetter, location: Rect) -> Self {
        TextSlider {
            slider_region: SliderRegion::new(param, param_setter),
            location,
        }
    }
}

impl<'a, P: Param> Widget for TextSlider<'a, P> {
    fn ui(self, ui: &mut Ui) -> Response {
        let mut response = ui.allocate_rect(self.location, Sense::click_and_drag());
        self.slider_region.handle_response(&ui, &mut response);

        let painter = ui.painter_at(self.location);
        let center = self.location.center();

        // Draw the text
        let text = self.slider_region.get_string();
        let anchor = Align2::CENTER_CENTER;
        let color = Color32::from(Rgba::WHITE);
        let font = FontId::monospace(16.0);
        painter.text(center, anchor, text, font, color);
        response
    }
}
