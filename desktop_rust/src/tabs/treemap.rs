use eframe::egui::{self, Color32, RichText, Vec2, Stroke};
use crate::app::PurgeCraftApp;
use crate::types::{format_size, truncate_chars, open_system_folder};
use crate::scanner;

pub fn render_disk_treemap(app: &mut PurgeCraftApp, ui: &mut egui::Ui, filtered_indices: &[usize]) {
    let mut sized_apps: Vec<(usize, u64)> = filtered_indices
        .iter()
        .map(|&idx| (idx, app.apps[idx].size_bytes))
        .filter(|&(_, sz)| sz > 0)
        .collect();

    sized_apps.sort_by(|a, b| b.1.cmp(&a.1));

    let total_bytes: u64 = sized_apps.iter().map(|(_, sz)| sz).sum();
    let max_bytes: u64 = sized_apps.first().map(|(_, sz)| *sz).unwrap_or(1);

    let is_dark = app.dark_mode;
    let card_bg = if is_dark { Color32::from_rgb(24, 24, 27) } else { Color32::from_rgb(255, 255, 255) };
    let border_color = if is_dark { Color32::from_rgb(39, 39, 42) } else { Color32::from_rgb(226, 232, 240) };
    let selected_border = Color32::from_rgb(59, 130, 246);

    ui.add_space(6.0);

    // 1. Top Summary Banner
    let banner_bg = if is_dark { Color32::from_rgb(30, 41, 59) } else { Color32::from_rgb(241, 245, 249) };
    egui::Frame::none()
        .fill(banner_bg)
        .rounding(6.0)
        .inner_margin(10.0)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("📊 Анализ дискового пространства").heading().strong());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new(format!("Всего занято: {}", format_size(total_bytes))).strong().color(Color32::from_rgb(16, 185, 129)));
                    ui.label(format!("Программ с известным размером: {} •", sized_apps.len()));
                });
            });

            ui.add_space(8.0);

            // Segmented usage bar for top 5 apps
            let avail_w = ui.available_width().max(200.0);
            let bar_h = 12.0;
            let (rect, _resp) = ui.allocate_exact_size(Vec2::new(avail_w, bar_h), egui::Sense::hover());
            let painter = ui.painter();

            // Background bar
            painter.rect_filled(rect, 4.0, if is_dark { Color32::from_rgb(51, 65, 85) } else { Color32::from_rgb(203, 213, 225) });

            let colors = [
                Color32::from_rgb(147, 51, 234), // purple
                Color32::from_rgb(59, 130, 246),  // blue
                Color32::from_rgb(16, 185, 129),  // emerald
                Color32::from_rgb(245, 158, 11),  // amber
                Color32::from_rgb(239, 68, 68),   // red
            ];

            if total_bytes > 0 {
                let mut current_x = rect.left();
                for (i, &(_, sz)) in sized_apps.iter().take(5).enumerate() {
                    let seg_w = ((sz as f32 / total_bytes as f32) * avail_w).max(2.0);
                    let seg_rect = egui::Rect::from_min_size(egui::pos2(current_x, rect.top()), Vec2::new(seg_w, bar_h));
                    painter.rect_filled(seg_rect, 0.0, colors[i % colors.len()]);
                    current_x += seg_w;
                    if current_x >= rect.right() {
                        break;
                    }
                }
            }
        });

    ui.add_space(8.0);

    if sized_apps.is_empty() {
        ui.vertical_centered(|ui| {
            ui.add_space(40.0);
            ui.label(RichText::new("Нет программ с подсчитанным размером").size(15.0).color(Color32::GRAY));
            ui.label("Идет расчет размеров в фоновом режиме или выберите другую категорию.");
        });
        return;
    }

    // 2. Bento Grid of Largest Apps
    let mut app_to_uninstall = None;

    egui::ScrollArea::vertical().show(ui, |ui| {
        let avail_w = ui.available_width();
        let cols = if avail_w > 900.0 { 3 } else if avail_w > 550.0 { 2 } else { 1 };
        let card_w = (avail_w - (cols as f32 - 1.0) * 10.0) / cols as f32;

        let rows = (sized_apps.len() + cols - 1) / cols;

        for r in 0..rows {
            ui.horizontal(|ui| {
                for c in 0..cols {
                    let idx_in_sized = r * cols + c;
                    if idx_in_sized >= sized_apps.len() {
                        break;
                    }

                    let (real_idx, sz) = sized_apps[idx_in_sized];
                    let a = &app.apps[real_idx];

                    let is_selected = app.selected_app_idx == Some(real_idx);
                    let stroke = if is_selected {
                        Stroke::new(2.0_f32, selected_border)
                    } else {
                        Stroke::new(1.0_f32, border_color)
                    };

                    let percent = if total_bytes > 0 { (sz as f64 / total_bytes as f64) * 100.0 } else { 0.0 };
                    let relative_to_max = (sz as f32 / max_bytes as f32).clamp(0.02, 1.0);

                    // Category accent color
                    let accent_color = if a.source_badge == "[Steam]" || a.name.to_lowercase().contains("game") {
                        Color32::from_rgb(147, 51, 234) // Purple
                    } else if a.is_uwp {
                        Color32::from_rgb(6, 182, 212) // Cyan
                    } else if a.source_badge == "[Portable]" {
                        Color32::from_rgb(249, 115, 22) // Orange
                    } else if a.is_system_component {
                        Color32::from_rgb(100, 116, 139) // Slate
                    } else {
                        Color32::from_rgb(59, 130, 246) // Blue
                    };

                    egui::Frame::none()
                        .fill(card_bg)
                        .stroke(stroke)
                        .rounding(8.0)
                        .inner_margin(10.0)
                        .show(ui, |ui| {
                            ui.set_width(card_w - 4.0);

                            ui.vertical(|ui| {
                                // Title & Badge
                                ui.horizontal(|ui| {
                                    // Accent dot
                                    ui.painter().circle_filled(
                                        ui.cursor().min + egui::vec2(6.0, 8.0),
                                        4.0,
                                        accent_color
                                    );
                                    ui.add_space(10.0);

                                    let title = truncate_chars(&a.name, 26);
                                    let resp = ui.selectable_label(is_selected, RichText::new(title).strong().size(13.5));
                                    if resp.clicked() {
                                        app.selected_app_idx = Some(real_idx);
                                    }

                                    if !a.source_badge.is_empty() {
                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                            ui.label(RichText::new(&a.source_badge).size(10.0).color(accent_color));
                                        });
                                    }
                                });

                                ui.add_space(4.0);

                                // Big Size & Percentage
                                ui.horizontal(|ui| {
                                    let size_str = format_size(sz);
                                    ui.label(RichText::new(size_str).size(18.0).strong().color(
                                        if sz > 10 * 1024 * 1024 * 1024 {
                                            Color32::from_rgb(239, 68, 68)
                                        } else if sz > 1024 * 1024 * 1024 {
                                            Color32::from_rgb(245, 158, 11)
                                        } else {
                                            Color32::from_rgb(16, 185, 129)
                                        }
                                    ));

                                    ui.label(RichText::new(format!("({:.1}%)", percent)).size(11.0).color(Color32::GRAY));

                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        let drive = scanner::get_app_drive_letter(a);
                                        ui.label(RichText::new(format!("Диск {}:", drive)).size(11.0).color(Color32::GRAY));
                                    });
                                });

                                ui.add_space(2.0);

                                // Relative bar
                                let bar_w = ui.available_width();
                                let (bar_rect, _) = ui.allocate_exact_size(Vec2::new(bar_w, 4.0), egui::Sense::hover());
                                ui.painter().rect_filled(bar_rect, 2.0, if is_dark { Color32::from_rgb(40, 40, 45) } else { Color32::from_rgb(226, 232, 240) });
                                let filled_rect = egui::Rect::from_min_size(bar_rect.min, Vec2::new(bar_w * relative_to_max, 4.0));
                                ui.painter().rect_filled(filled_rect, 2.0, accent_color);

                                ui.add_space(4.0);

                                // Publisher & Quick action buttons
                                ui.horizontal(|ui| {
                                    let pub_name = if !a.publisher.is_empty() { truncate_chars(&a.publisher, 18) } else { "—".to_string() };
                                    ui.label(RichText::new(pub_name).size(10.5).color(Color32::GRAY));

                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        if ui.small_button(RichText::new("🗑 Удалить").size(10.5)).clicked() {
                                            app_to_uninstall = Some(a.clone());
                                        }
                                        if !a.install_location.is_empty() && ui.small_button(RichText::new("📁").size(10.5)).on_hover_text("Открыть папку программы").clicked() {
                                            open_system_folder(&a.install_location, app.settings.use_total_commander);
                                        }
                                    });
                                });
                            });
                        });

                    ui.add_space(6.0);
                }
            });
            ui.add_space(8.0);
        }
    });

    if let Some(app_to_remove) = app_to_uninstall {
        app.start_uninstall(app_to_remove);
    }
}
