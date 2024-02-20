// Modified combo box to support Actuate's UI  - Ardura
// I don't really know how this works at low level but I managed to make it work.

use crate::{egui::style::WidgetVisuals, CustomPopupComboBox::popup_below_widget, *};
use epaint::Shape;
use nih_plug_egui::egui::{
    epaint, vec2, InnerResponse, NumExt, Painter, Response, ScrollArea, Sense, Stroke, TextStyle,
    Ui, WidgetInfo, WidgetText, WidgetType,
};

/// A function that paints the [`ComboBox`] icon
pub type IconPainter = Box<dyn FnOnce(&Ui, Rect, &WidgetVisuals, bool)>;

/// A drop-down selection menu with a descriptive label.
///
/// ```
/// # #[derive(Debug, PartialEq)]
/// # enum Enum { First, Second, Third }
/// # let mut selected = Enum::First;
/// # egui::__run_test_ui(|ui| {
/// egui::ComboBox::from_label("Select one!")
///     .selected_text(format!("{:?}", selected))
///     .show_ui(ui, |ui| {
///         ui.selectable_value(&mut selected, Enum::First, "First");
///         ui.selectable_value(&mut selected, Enum::Second, "Second");
///         ui.selectable_value(&mut selected, Enum::Third, "Third");
///     }
/// );
/// # });
/// ```
#[must_use = "You should call .show*"]
pub struct ComboBox {
    id_source: Id,
    label: Option<WidgetText>,
    selected_text: WidgetText,
    width: Option<f32>,
    icon: Option<IconPainter>,
    display_above: bool,
    num_options: u16,
    show_label: bool,
}

impl ComboBox {
    /// Create new [`ComboBox`] with id and label
    pub fn new(
        id_source: impl std::hash::Hash,
        label: impl Into<WidgetText>,
        display_above: bool,
        num_options: u16,
    ) -> Self {
        Self {
            id_source: Id::new(id_source),
            label: Some(label.into()),
            selected_text: Default::default(),
            width: None,
            icon: None,
            display_above: display_above,
            num_options: num_options,
            show_label: false,
        }
    }

    /// Set the width of the button and menu
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// What we show as the currently selected value
    pub fn selected_text(mut self, selected_text: impl Into<WidgetText>) -> Self {
        self.selected_text = selected_text.into();
        self
    }

    /// Show the combo box, with the given ui code for the menu contents.
    ///
    /// Returns `InnerResponse { inner: None }` if the combo box is closed.
    pub fn show_ui<R>(
        self,
        ui: &mut Ui,
        menu_contents: impl FnOnce(&mut Ui) -> R,
    ) -> InnerResponse<Option<R>> {
        self.show_ui_dyn(ui, Box::new(menu_contents))
    }

    #[allow(unused_variables)]
    fn show_ui_dyn<'c, R>(
        self,
        ui: &mut Ui,
        menu_contents: Box<dyn FnOnce(&mut Ui) -> R + 'c>,
    ) -> InnerResponse<Option<R>> {
        let Self {
            id_source,
            label,
            selected_text,
            width,
            icon,
            display_above,
            num_options,
            show_label,
        } = self;

        let button_id = ui.make_persistent_id(id_source);

        ui.horizontal(|ui| {
            if let Some(width) = width {
                ui.spacing_mut().slider_width = width; // yes, this is ugly. Will remove later.
            }
            let mut ir = combo_box_dyn(
                ui,
                button_id,
                selected_text,
                menu_contents,
                icon,
                display_above,
                num_options,
            );
            if self.show_label {
                if let Some(label) = label {
                    ir.response
                        .widget_info(|| WidgetInfo::labeled(WidgetType::ComboBox, label.text()));
                    ir.response |= ui.label(label);
                } else {
                    ir.response
                        .widget_info(|| WidgetInfo::labeled(WidgetType::ComboBox, ""));
                }
            }
            ir
        })
        .inner
    }
}

fn combo_box_dyn<'c, R>(
    ui: &mut Ui,
    button_id: Id,
    selected_text: WidgetText,
    menu_contents: Box<dyn FnOnce(&mut Ui) -> R + 'c>,
    icon: Option<IconPainter>,
    display_above: bool,
    num_options: u16,
) -> InnerResponse<Option<R>> {
    let popup_id = button_id.with("popup");

    let is_popup_open = ui.memory(|mem|mem.is_popup_open(popup_id));
    let button_response = button_frame(ui, button_id, is_popup_open, Sense::click(), |ui| {
        // We don't want to change width when user selects something new
        let full_minimum_width = ui.spacing().slider_width;
        let icon_size = Vec2::splat(ui.spacing().icon_width);

        let galley = selected_text.into_galley(ui, Some(false), f32::INFINITY, TextStyle::Button);

        let width = galley.size().x + ui.spacing().item_spacing.x + icon_size.x;
        let width = width.at_least(full_minimum_width);
        let height = galley.size().y.max(icon_size.y);

        let (_, rect) = ui.allocate_space(Vec2::new(width, height));
        let button_rect = ui.min_rect().expand2(ui.spacing().button_padding);
        let response = ui.interact(button_rect, button_id, Sense::click());
        // response.active |= is_popup_open;

        if ui.is_rect_visible(rect) {
            let icon_rect = Align2::RIGHT_CENTER.align_size_within_rect(icon_size, rect);
            let visuals = if is_popup_open {
                &ui.visuals().widgets.open
            } else {
                ui.style().interact(&response)
            };

            if let Some(icon) = icon {
                icon(
                    ui,
                    icon_rect.expand(visuals.expansion),
                    visuals,
                    is_popup_open,
                );
            } else {
                paint_default_icon(
                    ui.painter(),
                    icon_rect.expand(visuals.expansion),
                    visuals,
                    display_above,
                );
            }

            let text_rect = Align2::LEFT_CENTER.align_size_within_rect(galley.size(), rect);
            galley.paint_with_visuals(ui.painter(), text_rect.min, visuals);
        }
    });

    if button_response.clicked() {
        ui.memory_mut(|mem|mem.toggle_popup(popup_id));
    }
    //let inner = crate::egui::popup::popup_below_widget(ui, popup_id, &button_response, |ui| {
    let inner = popup_below_widget(
        ui,
        popup_id,
        &button_response,
        display_above,
        num_options as f32,
        |ui| {
            ScrollArea::vertical()
                .max_height(ui.spacing().combo_height * 2.0)
                .show(ui, menu_contents)
                .inner
        },
    );

    InnerResponse {
        inner,
        response: button_response,
    }
}

fn button_frame(
    ui: &mut Ui,
    id: Id,
    is_popup_open: bool,
    sense: Sense,
    add_contents: impl FnOnce(&mut Ui),
) -> Response {
    let where_to_put_background = ui.painter().add(Shape::Noop);

    let margin = ui.spacing().button_padding;
    let interact_size = ui.spacing().interact_size;

    let mut outer_rect = ui.available_rect_before_wrap();
    outer_rect.set_height(outer_rect.height().at_least(interact_size.y));

    let inner_rect = outer_rect.shrink2(margin);
    let mut content_ui = ui.child_ui(inner_rect, *ui.layout());
    add_contents(&mut content_ui);

    let mut outer_rect = content_ui.min_rect().expand2(margin);
    outer_rect.set_height(outer_rect.height().at_least(interact_size.y));

    let response = ui.interact(outer_rect, id, sense);

    if ui.is_rect_visible(outer_rect) {
        let visuals = if is_popup_open {
            &ui.visuals().widgets.open
        } else {
            ui.style().interact(&response)
        };

        ui.painter().set(
            where_to_put_background,
            epaint::RectShape {
                rect: outer_rect.expand(visuals.expansion),
                rounding: visuals.rounding,
                fill: visuals.bg_fill,
                stroke: visuals.bg_stroke,
            },
        );
    }

    // This was breaking compile for some reason even though the method is not private
    // ui.advance_cursor_after_rect(outer_rect);

    // I replaced it with this to create some rect over this struct that is passthrough
    ui.allocate_rect(outer_rect, Sense::click());

    response
}

fn paint_default_icon(painter: &Painter, rect: Rect, visuals: &WidgetVisuals, display_above: bool) {
    let rect = Rect::from_center_size(
        rect.center(),
        vec2(rect.width() * 0.7, rect.height() * 0.45),
    );
    if display_above {
        // Upward pointing triangle
        painter.add(Shape::convex_polygon(
            vec![rect.left_bottom(), rect.right_bottom(), rect.center_top()],
            visuals.fg_stroke.color,
            Stroke::new(1.0, Color32::TRANSPARENT),
        ));
    } else {
        // Downward pointing triangle
        painter.add(Shape::convex_polygon(
            vec![rect.left_top(), rect.right_top(), rect.center_bottom()],
            visuals.fg_stroke.color,
            Stroke::new(1.0, Color32::TRANSPARENT),
        ));
    }
    /*
    painter.add(Shape::closed_line(
        vec![rect.left_top(), rect.right_top(), rect.center_bottom()],
        visuals.fg_stroke,
    ));*/
}
