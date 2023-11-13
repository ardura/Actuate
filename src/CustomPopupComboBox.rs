// This is a supporting file that I needed for the CustomComboBox to work with this version of egui and Nih-Plug

//! Show popup windows, tooltips, context menus etc.

use nih_plug_egui::egui::{Align, Area, Frame, Id, Key, Layout, Order, Pos2, Response, Ui};

/// Shows a popup below another widget.
///
/// Useful for drop-down menus (combo boxes) or suggestion menus under text fields.
///
/// You must open the popup with [`Memory::open_popup`] or  [`Memory::toggle_popup`].
///
/// Returns `None` if the popup is not open.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// let response = ui.button("Open popup");
/// let popup_id = ui.make_persistent_id("my_unique_id");
/// if response.clicked() {
///     ui.memory().toggle_popup(popup_id);
/// }
/// egui::popup::popup_below_widget(ui, popup_id, &response, |ui| {
///     ui.set_min_width(200.0); // if you want to control the size
///     ui.label("Some more info, or things you can select:");
///     ui.label("…");
/// });
/// # });
/// ```
pub fn popup_below_widget<R>(
    ui: &Ui,
    popup_id: Id,
    widget_response: &Response,
    display_above: bool,
    num_options: f32,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> Option<R> {
    if ui.memory().is_popup_open(popup_id) {
        let inner;
        if display_above {
            inner = Area::new(popup_id)
                .order(Order::Foreground)
                .fixed_pos(Pos2 {
                    x: widget_response.rect.left_top().x,
                    y: widget_response.rect.left_top().y
                        - widget_response.rect.height() * (num_options + 1.0),
                })
                .show(ui.ctx(), |ui| {
                    // Note: we use a separate clip-rect for this area, so the popup can be outside the parent.
                    // See https://github.com/emilk/egui/issues/825
                    let frame = Frame::popup(ui.style());
                    let frame_margin = frame.inner_margin + frame.outer_margin;
                    frame
                        .show(ui, |ui| {
                            ui.with_layout(Layout::top_down_justified(Align::LEFT), |ui| {
                                ui.set_width(widget_response.rect.width() - frame_margin.sum().x);
                                // Added to force the height
                                //ui.set_height(widget_response.rect.height()*(num_options));
                                add_contents(ui)
                            })
                            .inner
                        })
                        .inner
                })
                .inner;
        } else {
            inner = Area::new(popup_id)
                .order(Order::Foreground)
                .fixed_pos(widget_response.rect.left_bottom())
                .show(ui.ctx(), |ui| {
                    // Note: we use a separate clip-rect for this area, so the popup can be outside the parent.
                    // See https://github.com/emilk/egui/issues/825
                    let frame = Frame::popup(ui.style());
                    let frame_margin = frame.inner_margin + frame.outer_margin;
                    frame
                        .show(ui, |ui| {
                            ui.with_layout(Layout::top_down_justified(Align::LEFT), |ui| {
                                ui.set_width(widget_response.rect.width() - frame_margin.sum().x);
                                add_contents(ui)
                            })
                            .inner
                        })
                        .inner
                })
                .inner;
        }

        if ui.input().key_pressed(Key::Escape) || widget_response.clicked_elsewhere() {
            ui.memory().close_popup();
        }
        Some(inner)
    } else {
        None
    }
}
