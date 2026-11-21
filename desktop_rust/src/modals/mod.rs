pub mod dialogs;
pub mod wizard;
pub mod tracker;
pub mod settings;
pub mod tools;
pub mod cleaners;
pub mod update;

use std::collections::HashSet;
use eframe::egui;
use crate::app::PurgeCraftApp;

/// Result returned by [`show_detachable_window`] for syncing open/detached state.
pub struct WindowHostOutcome {
    pub still_open: bool,
    pub detached: bool,
}

/// Computes responsive modal inner width clamped strictly inside the application viewport.
pub fn responsive_modal_width(ctx: &egui::Context, ideal_w: f32) -> f32 {
    clamp_dimension(ctx.screen_rect().width(), ideal_w, 32.0, 260.0)
}

/// Computes list height inside a resizable container, reserving `footer_h` at the bottom.
pub fn fill_scroll_height(ui: &egui::Ui, footer_h: f32, min_h: f32) -> f32 {
    (ui.available_height() - footer_h).max(min_h)
}

/// Renders a row where `add_right` controls stay on the right edge without overlap
/// and `add_left` content is strictly clipped and constrained to the remaining left width.
pub fn split_row(
    ui: &mut egui::Ui,
    add_right: impl FnOnce(&mut egui::Ui),
    add_left: impl FnOnce(&mut egui::Ui),
) {
    ui.horizontal(|ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            add_right(ui);
            ui.add_space(8.0);
            let left_w = ui.available_width().max(24.0);
            ui.allocate_ui_with_layout(
                egui::vec2(left_w, 0.0),
                egui::Layout::top_down(egui::Align::Min),
                |left_ui| {
                    let bounds = left_ui.max_rect();
                    let clip = left_ui.clip_rect().intersect(egui::Rect::from_min_max(
                        egui::pos2(bounds.left(), -1e6),
                        egui::pos2(bounds.right(), 1e6),
                    ));
                    left_ui.set_clip_rect(clip);
                    add_left(left_ui);
                },
            );
        });
    });
}

fn clamp_dimension(viewport_dim: f32, ideal: f32, margin: f32, min_dim: f32) -> f32 {
    let max_dim = (viewport_dim - margin).max(min_dim.min(viewport_dim.max(120.0)));
    ideal.min(max_dim)
}

fn build_docked_window<'a>(
    key: &'static str,
    title: impl Into<egui::WidgetText>,
    ctx: &egui::Context,
    ideal_size: egui::Vec2,
    min_size: egui::Vec2,
) -> egui::Window<'a> {
    let screen = ctx.screen_rect();
    let max_w = (screen.width() - 20.0).max(240.0);
    let max_h = (screen.height() - 24.0).max(160.0);
    let init_w = ideal_size.x.min(max_w);
    let init_h = ideal_size.y.min(max_h);
    let eff_min = egui::vec2(min_size.x.min(max_w), min_size.y.min(max_h));
    let start_pos = screen.center() - egui::vec2(init_w * 0.5, init_h * 0.5);

    egui::Window::new(title)
        .id(egui::Id::new(key))
        .collapsible(false)
        .movable(true)
        .resizable(true)
        .constrain_to(screen.shrink(6.0))
        .default_pos(start_pos)
        .default_size(egui::vec2(init_w, init_h))
        .min_size(eff_min)
        .max_size(egui::vec2(max_w, max_h))
}

/// Renders an inner-only confirmation/dialog window that users can freely drag and resize.
pub fn show_confirm_window(
    ctx: &egui::Context,
    key: &'static str,
    title: impl Into<egui::WidgetText>,
    ideal_size: egui::Vec2,
    min_size: egui::Vec2,
    add_contents: impl FnOnce(&mut egui::Ui),
) {
    build_docked_window(key, title, ctx, ideal_size, min_size).show(ctx, add_contents);
}

fn render_detach_header(
    ui: &mut egui::Ui,
    heading: Option<&str>,
    is_detached: bool,
    dark_mode: bool,
    lang: crate::i18n::Language,
) -> bool {
    let mut toggle_clicked = false;
    let btn_label = if is_detached {
        crate::i18n::t(lang, "Закрепить", "Dock")
    } else {
        crate::i18n::t(lang, "Открепить", "Pop out")
    };
    let btn_tip = if is_detached {
        crate::i18n::t(lang, "Вернуть окно внутрь главного окна", "Dock window back inside main window")
    } else {
        crate::i18n::t(lang, "Вынести в отдельное окно", "Open in separate window")
    };

    ui.horizontal(|ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let btn = egui::Button::new(egui::RichText::new(btn_label).small().strong())
                .fill(if dark_mode {
                    egui::Color32::from_rgb(39, 43, 52)
                } else {
                    egui::Color32::from_rgb(226, 232, 240)
                });
            if ui.add(btn).on_hover_text(btn_tip).clicked() {
                toggle_clicked = true;
            }
            if let Some(h) = heading {
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.add(egui::Label::new(egui::RichText::new(h).heading()).truncate());
                });
            }
        });
    });
    toggle_clicked
}

/// Renders a working tool/settings window either inside PurgeCraft (movable + resizable)
/// or detached into its own native OS viewport window that can be dragged outside the app.
#[allow(clippy::too_many_arguments)]
pub fn show_detachable_window(
    ctx: &egui::Context,
    key: &'static str,
    title: impl Into<String>,
    heading: Option<&str>,
    ideal_size: egui::Vec2,
    min_size: egui::Vec2,
    closable: bool,
    is_detached: bool,
    dark_mode: bool,
    lang: crate::i18n::Language,
    mut add_contents: impl FnMut(&mut egui::Ui, &egui::Context),
) -> WindowHostOutcome {
    let title_str = title.into();
    let mut still_open = true;
    let mut next_detached = is_detached;

    if is_detached {
        let vp_id = egui::ViewportId::from_hash_of(key);
        let builder = egui::ViewportBuilder::default()
            .with_title(title_str)
            .with_inner_size(ideal_size)
            .with_min_inner_size(min_size)
            .with_resizable(true)
            .with_close_button(closable);

        ctx.show_viewport_immediate(vp_id, builder, |vp_ctx, _class| {
            if closable && vp_ctx.input(|i| i.viewport().close_requested()) {
                still_open = false;
            }
            egui::CentralPanel::default().show(vp_ctx, |ui| {
                if render_detach_header(ui, heading, true, dark_mode, lang) {
                    next_detached = false;
                }
                if heading.is_some() {
                    ui.add_space(4.0);
                }
                add_contents(ui, vp_ctx);
            });
        });
    } else {
        let mut win = build_docked_window(key, &title_str, ctx, ideal_size, min_size);
        if closable {
            win = win.open(&mut still_open);
        }
        win.show(ctx, |ui| {
            if render_detach_header(ui, heading, false, dark_mode, lang) {
                next_detached = true;
            }
            if heading.is_some() {
                ui.add_space(4.0);
            }
            add_contents(ui, ctx);
        });
    }

    WindowHostOutcome {
        still_open,
        detached: next_detached && still_open,
    }
}

/// Applies [`WindowHostOutcome`] cleanly to the open flag and detached set.
pub fn sync_detachable_state(
    key: &'static str,
    detached_set: &mut HashSet<&'static str>,
    open_flag: &mut bool,
    outcome: WindowHostOutcome,
) {
    if !outcome.still_open {
        *open_flag = false;
    }
    if *open_flag && outcome.detached {
        detached_set.insert(key);
    } else {
        detached_set.remove(key);
    }
}

pub fn render_all_modals(app: &mut PurgeCraftApp, ctx: &egui::Context) {
    dialogs::render_dialogs(app, ctx);
    wizard::render_wizard_modal(app, ctx);
    tracker::render_tracker_modal(app, ctx);
    settings::render_settings_modal(app, ctx);
    tools::render_shader_modal(app, ctx);
    tools::render_path_modal(app, ctx);
    tools::render_portable_modal(app, ctx);
    tools::render_runtime_modal(app, ctx);
    tools::render_winget_modal(app, ctx);
    cleaners::render_system_clean_modal(app, ctx);
    cleaners::render_dev_clean_modal(app, ctx);
    cleaners::render_services_modal(app, ctx);
    update::render_update_modal(app, ctx);
}

#[cfg(test)]
mod tests {
    use super::clamp_dimension;

    #[test]
    fn test_responsive_modal_clamp_shrinks_and_caps() {
        // Large window keeps ideal size
        assert!((clamp_dimension(1280.0, 780.0, 32.0, 260.0) - 780.0).abs() < f32::EPSILON);
        // Narrow window (like resized PurgeCraft with zoom) shrinks cleanly inside margin
        assert!((clamp_dimension(680.0, 780.0, 32.0, 260.0) - 648.0).abs() < f32::EPSILON);
        // Very compact viewport enforces safe minimum instead of going negative
        assert!((clamp_dimension(200.0, 500.0, 260.0, 96.0) - 96.0).abs() < f32::EPSILON);
    }
}
