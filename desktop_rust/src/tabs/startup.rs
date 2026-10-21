use eframe::egui::{self, Color32, RichText, Vec2};
use egui_extras::{TableBuilder, Column};
use crate::app::PurgeCraftApp;
use crate::scanner::{toggle_startup_item, delete_startup_item, StartupImpact};
use crate::types::{StartupFilter, jump_to_regedit, truncate_chars};

pub fn render_startup_tab(app: &mut PurgeCraftApp, ui: &mut egui::Ui) {
    let lang = app.lang();
    let table_bg = if app.dark_mode {
        Color32::from_rgb(18, 18, 20)
    } else {
        Color32::WHITE
    };

    let q = app.search_query.to_lowercase();
    let filter = app.startup_filter;
    let filtered_indices: Vec<usize> = app.startup_items
        .iter()
        .enumerate()
        .filter(|(_, item)| {
            let matches_filter = match filter {
                StartupFilter::All => true,
                StartupFilter::Registry => item.source_type == "hklm_run" || item.source_type == "hkcu_run" || item.source_type == "hklm_runonce" || item.source_type == "hkcu_runonce",
                StartupFilter::StartMenu => item.source_type == "startmenu",
                StartupFilter::TaskScheduler => item.source_type == "taskscheduler",
            };
            if !matches_filter {
                return false;
            }
            if q.is_empty() {
                true
            } else {
                item.name.to_lowercase().contains(&q) || item.command.to_lowercase().contains(&q) || item.source.to_lowercase().contains(&q)
            }
        })
        .map(|(i, _)| i)
        .collect();

    egui::Frame::none()
        .fill(table_bg)
        .inner_margin(4.0)
        .show(ui, |ui| {
            if filtered_indices.is_empty() {
                ui.vertical_centered(|ui| {
                    ui.add_space(60.0);
                    ui.label(RichText::new(crate::i18n::t(lang, "Записей автозагрузки не найдено", "No startup items found")).size(16.0).color(Color32::from_rgb(100, 116, 139)));
                    ui.label(crate::i18n::not_found_hint(lang));
                });
                return;
            }

            let mut to_toggle = None;
            let mut to_delete = None;
            let count = filtered_indices.len();

            TableBuilder::new(ui)
                .striped(true)
                .resizable(true)
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                .column(Column::exact(86.0))                   // State Toggle (✔ Вкл / ○ Откл)
                .column(Column::initial(220.0).at_least(160.0)) // Name
                .column(Column::exact(130.0))                  // Startup Impact
                .column(Column::exact(170.0))                  // Source
                .column(Column::remainder().at_least(220.0))   // Command / Path
                .column(Column::exact(90.0))                   // Action
                .header(28.0, |mut header| {
                    header.col(|ui| { ui.strong(crate::i18n::startup_col_state(lang)); });
                    header.col(|ui| { ui.strong(crate::i18n::startup_col_name(lang)); });
                    header.col(|ui| { ui.strong(crate::i18n::startup_col_impact(lang)); });
                    header.col(|ui| { ui.strong(crate::i18n::startup_col_source(lang)); });
                    header.col(|ui| { ui.strong(crate::i18n::startup_col_command(lang)); });
                    header.col(|ui| { ui.strong(crate::i18n::startup_col_action(lang)); });
                })
                .body(|body| {
                    body.rows(28.0, count, |mut row| {
                        let row_idx = row.index();
                        let orig_idx = filtered_indices[row_idx];
                        let item = &app.startup_items[orig_idx];

                        // Column 0: Toggle Switch (✔ Вкл / ○ Откл)
                        row.col(|ui| {
                            let (label_text, btn_fill, btn_color) = if item.is_enabled {
                                (crate::i18n::startup_status_on(lang), if app.dark_mode { Color32::from_rgb(20, 83, 45) } else { Color32::from_rgb(220, 252, 231) }, Color32::from_rgb(34, 197, 94))
                            } else {
                                (crate::i18n::startup_status_off(lang), if app.dark_mode { Color32::from_rgb(39, 39, 42) } else { Color32::from_rgb(244, 244, 245) }, Color32::from_rgb(148, 163, 184))
                            };
                            let toggle_btn = egui::Button::new(RichText::new(label_text).color(btn_color).strong().size(12.0))
                                .fill(btn_fill)
                                .min_size(Vec2::new(72.0, 22.0));
                            let toggle_tip = crate::i18n::t(lang, "Нажмите для включения/отключения автозапуска", "Click to enable/disable startup");
                            if ui.add(toggle_btn).on_hover_text(toggle_tip).clicked() {
                                to_toggle = Some(orig_idx);
                            }
                        });

                        // Column 1: Name with Context Menu
                        row.col(|ui| {
                            let resp = ui.label(RichText::new(&item.name).strong());
                            resp.context_menu(|ui| {
                                let toggle_menu_title = if item.is_enabled {
                                    crate::i18n::t(lang, "○ Отключить автозапуск", "○ Disable startup")
                                } else {
                                    crate::i18n::t(lang, "✔ Включить автозапуск", "✔ Enable startup")
                                };
                                if ui.button(toggle_menu_title).clicked() {
                                    to_toggle = Some(orig_idx);
                                    ui.close_menu();
                                }
                                ui.separator();
                                if ui.button(crate::i18n::t(lang, "📁 Открыть расположение файла", "📁 Open file location")).clicked() {
                                    let cmd_clean = item.command.trim().trim_matches('"');
                                    let exe_part = if let Some(pos) = cmd_clean.to_lowercase().find(".exe") {
                                        &cmd_clean[..pos + 4]
                                    } else {
                                        cmd_clean
                                    };
                                    if let Some(parent) = std::path::Path::new(exe_part).parent() {
                                        let _ = open::that(parent);
                                    }
                                    ui.close_menu();
                                }
                                if item.source_type.contains("run") {
                                    if ui.button(crate::i18n::t(lang, "🗄 Открыть ключ в реестре", "🗄 Open registry key")).clicked() {
                                        let sub_hive = if item.source_type.starts_with("hklm") { "HKLM" } else { "HKCU" };
                                        let key_name = if item.source_type.contains("runonce") { "RunOnce" } else { "Run" };
                                        let reg_path = format!("{}\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\{}", sub_hive, key_name);
                                        jump_to_regedit(&reg_path);
                                        ui.close_menu();
                                    }
                                }
                                ui.separator();
                                if ui.button(crate::i18n::t(lang, "❌ Удалить из автозагрузки", "❌ Remove from startup")).clicked() {
                                    to_delete = Some(orig_idx);
                                    ui.close_menu();
                                }
                            });
                        });

                        // Column 2: Startup Impact
                        row.col(|ui| {
                            let (label_text, badge_color, badge_bg) = match item.impact {
                                StartupImpact::High => {
                                    (crate::i18n::startup_impact_high(lang), Color32::from_rgb(239, 68, 68), if app.dark_mode { Color32::from_rgb(69, 10, 10) } else { Color32::from_rgb(254, 226, 226) })
                                }
                                StartupImpact::Medium => {
                                    (crate::i18n::startup_impact_medium(lang), Color32::from_rgb(234, 179, 8), if app.dark_mode { Color32::from_rgb(66, 32, 6) } else { Color32::from_rgb(254, 243, 199) })
                                }
                                StartupImpact::Low => {
                                    (crate::i18n::startup_impact_low(lang), Color32::from_rgb(34, 197, 94), if app.dark_mode { Color32::from_rgb(20, 83, 45) } else { Color32::from_rgb(220, 252, 231) })
                                }
                                StartupImpact::None => {
                                    (crate::i18n::startup_impact_none(lang), Color32::from_rgb(148, 163, 184), if app.dark_mode { Color32::from_rgb(39, 39, 42) } else { Color32::from_rgb(241, 245, 249) })
                                }
                            };
                            egui::Frame::none()
                                .fill(badge_bg)
                                .rounding(4.0)
                                .inner_margin(egui::Margin::symmetric(6.0, 2.0))
                                .show(ui, |ui| {
                                    ui.label(RichText::new(label_text).size(12.0).strong().color(badge_color));
                                });
                        });

                        // Column 2: Source
                        row.col(|ui| {
                            ui.label(&item.source);
                        });

                        // Column 3: Command
                        row.col(|ui| {
                            ui.label(truncate_chars(&item.command, 55)).on_hover_text(&item.command);
                        });

                        // Column 4: Delete button
                        row.col(|ui| {
                            let del_label = crate::i18n::t(lang, "Удалить", "Delete");
                            let del_tip = crate::i18n::t(lang, "Полностью удалить запись из автозапуска", "Permanently remove entry from startup");
                            if ui.button(del_label).on_hover_text(del_tip).clicked() {
                                to_delete = Some(orig_idx);
                            }
                        });
                    });
                });

            if let Some(toggle_idx) = to_toggle {
                let item = &mut app.startup_items[toggle_idx];
                let old_name = item.name.clone();
                if toggle_startup_item(item) {
                    let st = if item.is_enabled { crate::i18n::t(lang, "включен", "enabled") } else { crate::i18n::t(lang, "отключен", "disabled") };
                    app.status_msg = format!("{} «{}»: {}.", crate::i18n::t(lang, "Автозапуск для", "Startup for"), old_name, st);
                } else {
                    app.status_msg = format!("{} «{}».", crate::i18n::t(lang, "Не удалось изменить статус автозапуска для", "Failed to update startup status for"), old_name);
                }
            }

            if let Some(del_idx) = to_delete {
                let item = &app.startup_items[del_idx];
                let name = item.name.clone();
                if delete_startup_item(item) {
                    app.startup_items.remove(del_idx);
                    app.status_msg = format!("{} «{}».", crate::i18n::t(lang, "Запись автозагрузки успешно удалена:", "Startup entry successfully deleted:"), name);
                } else {
                    app.status_msg = format!("{} «{}».", crate::i18n::t(lang, "Не удалось удалить запись", "Failed to delete entry"), name);
                }
            }
        });
}
