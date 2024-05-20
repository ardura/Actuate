// This is a copy of egui checkbox just with slimmer padding/spacing needed in Scrollscope
// I also added a version to work with Atomic structures

// ----------------------------------------------------------------------------

use std::sync::atomic::{AtomicBool, Ordering};
use nih_plug_egui::egui::{
    epaint, pos2, vec2, NumExt, Rect, Response, Sense, Shape, TextStyle, TextureId, Ui, Vec2, Widget, WidgetInfo, WidgetText, WidgetType
};

// TODO(emilk): allow checkbox without a text label
/// Boolean on/off control with text label.
///
/// Usually you'd use [`Ui::checkbox`] instead.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// # let mut my_bool = true;
/// // These are equivalent:
/// ui.checkbox(&mut my_bool, "Checked");
/// ui.add(egui::SlimCheckbox::new(&mut my_bool, "Checked"));
/// # });
/// ```
#[must_use = "You should put this widget in an ui with `ui.add(widget);`"]
pub struct SlimCheckbox<'a> {
    checked: &'a mut bool,
    text: WidgetText,
}

#[must_use = "You should put this widget in an ui with `ui.add(widget);`"]
pub struct AtomicSlimCheckbox<'a> {
    checked: &'a AtomicBool,
    text: WidgetText,
}

impl<'a> AtomicSlimCheckbox<'a> {
    pub fn new(checked: &'a AtomicBool, text: impl Into<WidgetText>) -> Self {
        AtomicSlimCheckbox {
            checked,
            text: text.into(),
        }
    }
}

impl<'a> Widget for AtomicSlimCheckbox<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        let AtomicSlimCheckbox { checked, text } = self;

        let spacing = &ui.spacing();
        let icon_width = spacing.icon_width;
        let icon_spacing = spacing.icon_spacing;

        let (text, mut desired_size) = if text.is_empty() {
            (None, vec2(icon_width, 0.0))
        } else {
            let total_extra = vec2(icon_width + icon_spacing, 0.0);

            let wrap_width = ui.available_width() - total_extra.x;
            let text = text.into_galley(ui, None, wrap_width, TextStyle::Button);

            let mut desired_size = total_extra + text.size();

            /////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
            //This is the only piece I changed -Ardura
            desired_size =
                desired_size.at_least(vec2(spacing.interact_size.x * 0.45, spacing.interact_size.y));
            /////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

            (Some(text), desired_size)
        };

        desired_size = desired_size.at_least(Vec2::splat(spacing.interact_size.y));
        desired_size.y = desired_size.y.max(icon_width);
        let (rect, mut response) = ui.allocate_exact_size(desired_size, Sense::click());

        if response.clicked() {
            checked.fetch_xor(true, Ordering::SeqCst);
            response.mark_changed();
        }
        response.widget_info(|| {
            WidgetInfo::selected(
                WidgetType::Checkbox,
                checked.load(Ordering::SeqCst),
                text.as_ref().map_or("", |x| x.text()),
            )
        });

        if ui.is_rect_visible(rect) {
            // let visuals = ui.style().interact_selectable(&response, *checked); // too colorful
            let visuals = ui.style().interact(&response);
            let (small_icon_rect, big_icon_rect) = ui.spacing().icon_rectangles(rect);
            ui.painter().add(epaint::RectShape {
                rect: big_icon_rect.expand(visuals.expansion),
                rounding: visuals.rounding,
                fill: visuals.bg_fill,
                stroke: visuals.bg_stroke,
                fill_texture_id: TextureId::default(),
                uv: big_icon_rect.expand(visuals.expansion),
            });

            if checked.load(Ordering::SeqCst) {
                // Check mark:
                ui.painter().add(Shape::line(
                    vec![
                        pos2(small_icon_rect.left(), small_icon_rect.center().y),
                        pos2(small_icon_rect.center().x, small_icon_rect.bottom()),
                        pos2(small_icon_rect.right(), small_icon_rect.top()),
                    ],
                    visuals.fg_stroke,
                ));
            }
            if let Some(text) = text {
                let text_pos = pos2(
                    rect.min.x + icon_width + icon_spacing,
                    rect.center().y - 0.5 * text.size().y,
                );
                ui.painter().galley(text_pos, text, visuals.fg_stroke.color);
            }
        }

        response
    }
}

// This is here but I don't use it in scrollscope
#[allow(dead_code)]
impl<'a> SlimCheckbox<'a> {
    pub fn new(checked: &'a mut bool, text: impl Into<WidgetText>) -> Self {
        SlimCheckbox {
            checked,
            text: text.into(),
        }
    }
}

impl<'a> Widget for SlimCheckbox<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        let SlimCheckbox { checked, text } = self;

        let spacing = &ui.spacing();
        let icon_width = spacing.icon_width;
        let icon_spacing = spacing.icon_spacing;

        let (text, mut desired_size) = if text.is_empty() {
            (None, vec2(icon_width, 0.0))
        } else {
            let total_extra = vec2(icon_width + icon_spacing, 0.0);

            let wrap_width = ui.available_width() - total_extra.x;
            let text = text.into_galley(ui, None, wrap_width, TextStyle::Button);

            let mut desired_size = total_extra + text.size();

            /////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
            //This is the only piece I changed -Ardura
            desired_size =
                desired_size.at_least(vec2(spacing.interact_size.x * 0.45, spacing.interact_size.y));
            /////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

            (Some(text), desired_size)
        };

        desired_size = desired_size.at_least(Vec2::splat(spacing.interact_size.y));
        desired_size.y = desired_size.y.max(icon_width);
        let (rect, mut response) = ui.allocate_exact_size(desired_size, Sense::click());

        if response.clicked() {
            *checked = !*checked;
            response.mark_changed();
        }
        response.widget_info(|| {
            WidgetInfo::selected(
                WidgetType::Checkbox,
                *checked,
                text.as_ref().map_or("", |x| x.text()),
            )
        });

        if ui.is_rect_visible(rect) {
            // let visuals = ui.style().interact_selectable(&response, *checked); // too colorful
            let visuals = ui.style().interact(&response);
            let (small_icon_rect, big_icon_rect) = ui.spacing().icon_rectangles(rect);
            ui.painter().add(epaint::RectShape {
                rect: big_icon_rect.expand(visuals.expansion),
                rounding: visuals.rounding,
                fill: visuals.weak_bg_fill,
                stroke: visuals.fg_stroke,
                fill_texture_id: TextureId::default(),
                uv: Rect::ZERO,
            });

            if *checked {
                // Check mark:
                ui.painter().add(Shape::line(
                    vec![
                        pos2(small_icon_rect.left(), small_icon_rect.center().y),
                        pos2(small_icon_rect.center().x, small_icon_rect.bottom()),
                        pos2(small_icon_rect.right(), small_icon_rect.top()),
                    ],
                    visuals.fg_stroke,
                ));
            }
            if let Some(text) = text {
                let text_pos = pos2(
                    rect.min.x + icon_width + icon_spacing,
                    rect.center().y - 0.5 * text.size().y,
                );
                ui.painter().galley(text_pos, text, visuals.fg_stroke.color);
            }
        }

        response
    }
}
