// Copy of CustomParamSlider from Canopy Reverb modified further into verticality
// Needed to make some weird import changes to get this to work...Definitely should find a better way to do this in future...
// Ardura
use nih_plug::{
    prelude::{Param, ParamSetter},
    wrapper::clap::lazy_static,
};
use nih_plug_egui::egui::{self, vec2, Color32, Response, Sense, Stroke, TextStyle, Ui, Vec2, Widget, WidgetText};
use nih_plug_egui::{
    egui::{Pos2, Rect},
    widgets::util as nUtil,
};
use parking_lot::Mutex;
use std::sync::Arc;

/// When shift+dragging a parameter, one pixel dragged corresponds to this much change in the
/// noramlized parameter.
const GRANULAR_DRAG_MULTIPLIER: f32 = 0.0015;

lazy_static! {
    static ref DRAG_NORMALIZED_START_VALUE_MEMORY_ID: egui::Id = egui::Id::new((file!(), 0));
    static ref DRAG_AMOUNT_MEMORY_ID: egui::Id = egui::Id::new((file!(), 1));
    static ref VALUE_ENTRY_MEMORY_ID: egui::Id = egui::Id::new((file!(), 2));
}

/// A slider widget similar to [`egui::widgets::Slider`] that knows about NIH-plug parameters ranges
/// and can get values for it. The slider supports double click and control click to reset,
/// shift+drag for granular dragging, text value entry by clicking on the value text.
///
/// TODO: Vertical orientation
/// TODO: Check below for more input methods that should be added
/// TODO: Decouple the logic from the drawing so we can also do things like nobs without having to
///       repeat everything
/// TODO: Add WidgetInfo annotations for accessibility
#[must_use = "You should put this widget in an ui with `ui.add(widget);`"]
pub struct ParamSlider<'a, P: Param> {
    param: &'a P,
    setter: &'a ParamSetter<'a>,

    draw_value: bool,
    slider_width: Option<f32>,
    slider_height: Option<f32>,
    // Added in reversed function to have bar drawn other way
    reversed: bool,
    background_set_color: Color32,
    bar_set_color: Color32,
    use_padding: bool,

    /// Will be set in the `ui()` function so we can request keyboard input focus on Alt+click.
    keyboard_focus_id: Option<egui::Id>,
}

#[allow(dead_code)]
impl<'a, P: Param> ParamSlider<'a, P> {
    /// Create a new slider for a parameter. Use the other methods to modify the slider before
    /// passing it to [`Ui::add()`].
    pub fn for_param(param: &'a P, setter: &'a ParamSetter<'a>) -> Self {
        Self {
            param,
            setter,

            draw_value: true,
            slider_width: None,
            slider_height: None,
            // Added in reversed function to have bar drawn other way
            reversed: false,
            background_set_color: Color32::PLACEHOLDER,
            bar_set_color: Color32::PLACEHOLDER,
            use_padding: false,

            // I removed this because it was causing errors on plugin load somehow in FL
            keyboard_focus_id: None,
        }
    }

    pub fn override_colors(
        mut self,
        background_set_color: Color32,
        bar_set_color: Color32,
    ) -> Self {
        self.background_set_color = background_set_color;
        self.bar_set_color = bar_set_color;
        self
    }

    /// Don't draw the text slider's current value after the slider.
    pub fn without_value(mut self) -> Self {
        self.draw_value = false;
        self
    }

    /// Set a custom width for the slider.
    pub fn with_width(mut self, width: f32) -> Self {
        self.slider_width = Some(width);
        self
    }

    pub fn with_height(mut self, height: f32) -> Self {
        self.slider_height = Some(height);
        self
    }

    /// Set reversed bar drawing - Ardura
    pub fn set_reversed(mut self, reversed: bool) -> Self {
        self.reversed = reversed;
        self
    }

    pub fn use_padding(mut self, use_padding: bool) -> Self {
        self.use_padding = use_padding;
        self
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

    /// Enable the keyboard entry part of the widget.
    fn begin_keyboard_entry(&self, ui: &Ui) {
        ui.memory_mut(|mem| mem.request_focus(self.keyboard_focus_id.unwrap()));

        // Always initialize the field to the current value, that seems nicer than having to
        // being typing from scratch
        let value_entry_mutex = ui.memory_mut(|mem| {
            mem.data
                .get_temp_mut_or_default::<Arc<Mutex<String>>>(*VALUE_ENTRY_MEMORY_ID)
                .clone()
        });
        *value_entry_mutex.lock() = self.string_value();
    }

    fn keyboard_entry_active(&self, ui: &Ui) -> bool {
        ui.memory(|mem| mem.has_focus(self.keyboard_focus_id.unwrap()))
    }

    fn begin_drag(&self) {
        self.setter.begin_set_parameter(self.param);
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

    /// Begin and end drag still need to be called when using this. Returns `false` if the string
    /// could no tbe parsed.
    fn set_from_string(&self, string: &str) -> bool {
        match self.param.string_to_normalized_value(string) {
            Some(normalized_value) => {
                self.set_normalized_value(normalized_value);
                true
            }
            None => false,
        }
    }

    /// Begin and end drag still need to be called when using this..
    fn reset_param(&self) {
        self.setter
            .set_parameter(self.param, self.param.default_plain_value());
    }

    fn granular_drag(&self, ui: &Ui, drag_delta: Vec2) {
        // Remember the intial position when we started with the granular drag. This value gets
        // reset whenever we have a normal interaction with the slider.
        let start_value = if Self::get_drag_amount_memory(ui) == 0.0 {
            Self::set_drag_normalized_start_value_memory(ui, self.normalized_value());
            self.normalized_value()
        } else {
            Self::get_drag_normalized_start_value_memory(ui)
        };

        let total_drag_distance = drag_delta.x + Self::get_drag_amount_memory(ui);
        Self::set_drag_amount_memory(ui, total_drag_distance);

        self.set_normalized_value(
            (start_value + (total_drag_distance * GRANULAR_DRAG_MULTIPLIER)).clamp(0.0, 1.0),
        );
    }

    fn end_drag(&self) {
        self.setter.end_set_parameter(self.param);
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

    fn slider_ui(&self, ui: &mut Ui, response: &mut Response) {
        // Handle user input
        // TODO: Optionally (since it can be annoying) add scrolling behind a builder option
        if response.drag_started() {
            // When beginning a drag or dragging normally, reset the memory used to keep track of
            // our granular drag
            self.begin_drag();
            Self::set_drag_amount_memory(ui, 0.0);
        }
        if let Some(click_pos) = response.interact_pointer_pos() {
            if ui.input(|mem| mem.modifiers.command) {
                // Like double clicking, Ctrl+Click should reset the parameter
                self.reset_param();
                response.mark_changed();
            } else if ui.input(|mem| mem.modifiers.shift) {
                // And shift dragging should switch to a more granular input method
                self.granular_drag(ui, response.drag_delta());
                response.mark_changed();
            } else {
                // This was changed to y values from X to read the up down
                let proportion =
                    egui::emath::remap_clamp(click_pos.y, response.rect.y_range(), 0.0..=1.0)
                        as f64;
                self.set_normalized_value(1.0 - proportion as f32);
                response.mark_changed();
                Self::set_drag_amount_memory(ui, 0.0);
            }
        }
        if response.double_clicked() {
            self.reset_param();
            response.mark_changed();
        }
        if response.drag_released() {
            self.end_drag();
        }

        // And finally draw the thing
        if ui.is_rect_visible(response.rect) {
            // Also flipped these orders for vertical
            if self.reversed {
                if self.background_set_color == Color32::PLACEHOLDER {
                    // We'll do a flat widget with background -> filled foreground -> slight border
                    ui.painter().rect_filled(
                        response.rect,
                        4.0,
                        ui.visuals().widgets.inactive.bg_fill,
                    );
                } else {
                    ui.painter()
                        .rect_filled(response.rect, 4.0, self.background_set_color);
                }
            } else {
                ui.painter()
                    .rect_filled(response.rect, 4.0, ui.visuals().selection.bg_fill);
            }

            let filled_proportion = self.normalized_value();
            if filled_proportion > 0.0 {
                let left_bottom = response.rect.left_bottom();
                let right_bottom = response.rect.right_bottom();
                let rect_points = [
                    Pos2::new(
                        left_bottom.x,
                        left_bottom.y - (response.rect.height() * filled_proportion),
                    ), // Top left
                    Pos2::new(
                        right_bottom.x,
                        right_bottom.y - (response.rect.height() * filled_proportion),
                    ), // Top right
                    left_bottom,
                    right_bottom,
                ];

                //let mut filled_rect = response.rect;
                let mut filled_rect = Rect::from_points(&rect_points);
                filled_rect.set_bottom(response.rect.bottom());
                // This was changed from width to make it height
                filled_rect.set_height(response.rect.height() * filled_proportion);

                // Added to reverse filling - Ardura
                // Vertical has this flipped to make sense vs the horizontal bar
                if self.reversed {
                    let filled_bg = if response.dragged() {
                        if self.bar_set_color == Color32::PLACEHOLDER {
                            nUtil::add_hsv(ui.visuals().selection.bg_fill, 0.0, -0.1, 0.1)
                        } else {
                            nUtil::add_hsv(self.bar_set_color, 0.0, -0.1, 0.1)
                        }
                    } else {
                        if self.bar_set_color == Color32::PLACEHOLDER {
                            ui.visuals().selection.bg_fill
                        } else {
                            self.bar_set_color
                        }
                    };
                    ui.painter().rect_filled(filled_rect, 4.0, filled_bg);
                } else {
                    let filled_bg = if response.dragged() {
                        nUtil::add_hsv(ui.visuals().widgets.inactive.bg_fill, 0.0, -0.1, 0.1)
                    } else {
                        ui.visuals().widgets.inactive.bg_fill
                    };
                    ui.painter().rect_filled(filled_rect, 4.0, filled_bg);
                }
            }

            if self.background_set_color == Color32::PLACEHOLDER {
                ui.painter().rect_stroke(
                    response.rect,
                    4.0,
                    Stroke::new(1.0, ui.visuals().widgets.active.bg_fill),
                );
            } else {
                ui.painter().rect_stroke(
                    response.rect,
                    4.0,
                    Stroke::new(1.0, self.background_set_color),
                );
            }
        }
    }

    fn value_ui(&self, ui: &mut Ui) {
        let visuals = ui.visuals().widgets.inactive;
        let should_draw_frame = ui.visuals().button_frame;
        let padding = if self.use_padding {
            ui.spacing().button_padding
        } else {
            ui.spacing().button_padding / 2.0
        };

        /*
        // I had to comment this out since the init of ParamSlider breaks because of the keyboard focus not existing in FL
        // I'm not sure how the original ParamSlider code works as a result :|

        // Either show the parameter's label, or show a text entry field if the parameter's label
        // has been clicked on
        let keyboard_focus_id = self.keyboard_focus_id.unwrap();
        if self.keyboard_entry_active(ui) {
            let value_entry_mutex = ui
                .memory()
                .data
                .get_temp_mut_or_default::<Arc<Mutex<String>>>(*VALUE_ENTRY_MEMORY_ID)
                .clone();
            let mut value_entry = value_entry_mutex.lock();

            ui.add(
                TextEdit::singleline(&mut *value_entry)
                    .id(keyboard_focus_id)
                    .font(TextStyle::Monospace),
            );
            if ui.input().key_pressed(Key::Escape) {
                // Cancel when pressing escape
                ui.memory().surrender_focus(keyboard_focus_id);
            } else if ui.input().key_pressed(Key::Enter) {
                // And try to set the value by string when pressing enter
                self.begin_drag();
                self.set_from_string(&value_entry);
                self.end_drag();

                ui.memory().surrender_focus(keyboard_focus_id);
            }
        } else {
            */
        let text = WidgetText::from(self.string_value()).into_galley(
            ui,
            None,
            ui.available_width() - (padding.x * 2.0),
            TextStyle::Button,
        );

        let response = ui.allocate_response(text.size() + (padding * 2.0), Sense::click());
        if response.clicked() {
            //self.begin_keyboard_entry(ui);
        }

        if ui.is_rect_visible(response.rect) {
            if should_draw_frame {
                let fill = visuals.bg_fill;
                let stroke = visuals.bg_stroke;
                ui.painter().rect(
                    response.rect.expand(visuals.expansion),
                    visuals.rounding,
                    fill,
                    stroke,
                );
            }

            let text_pos = ui
                .layout()
                .align_size_within_rect(text.size(), response.rect.shrink2(padding))
                .min;
            ui.painter().add(egui::epaint::TextShape::new(
                text_pos,
                text,
                visuals.fg_stroke.color,
            ));
        }
        //}
    }
}

impl<P: Param> Widget for ParamSlider<'_, P> {
    fn ui(self, ui: &mut Ui) -> Response {
        let slider_width = self
            .slider_width
            .unwrap_or_else(|| ui.spacing().interact_size.y);
        let slider_height = self
            .slider_height
            .unwrap_or_else(|| ui.spacing().interact_size.x);

        // Changed to vertical to fix the label
        ui.vertical(|ui| {
            // Allocate space
            let mut response = ui
                .vertical(|ui| {
                    //ui.allocate_space(vec2(slider_width, slider_height));
                    let response = ui.allocate_response(
                        vec2(slider_width, slider_height),
                        Sense::click_and_drag(),
                    );
                    response
                })
                .inner;

            self.slider_ui(ui, &mut response);
            if self.draw_value {
                self.value_ui(ui);
            }

            response
        })
        .inner
    }
}
