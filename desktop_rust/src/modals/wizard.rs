use std::process::Command;
use eframe::egui::{self, Color32, RichText};
use crate::app::PurgeCraftApp;
use crate::types::{WizardPhase, format_size, truncate_chars};
use crate::scanner::{export_traces_to_xml, spawn_delete_leftovers_stream};

pub fn render_wizard_modal(app: &mut PurgeCraftApp, ctx: &egui::Context) {
    if !app.wizard_open {
        return;
    }

    let mut close_wizard = false;

    egui::Window::new(format!("Удаление остатков {} — Мастер Удаления ™", app.wizard_app_title))
        .id(egui::Id::new("wizard_traces_modal"))
        .collapsible(false)
        .resizable(true)
        .pivot(egui::Align2::CENTER_CENTER)
        .default_pos(ctx.screen_rect().center())
        .show(ctx, |ui| {
            ui.set_width(620.0);
            ui.set_height(420.0);

            match app.wizard_phase {
                WizardPhase::Scanning => {
                    ui.heading(RichText::new("Сканирование оставшихся файлов/папок, элементов реестра...").size(15.0).strong());
                    ui.add_space(8.0);
                    ui.label(RichText::new(truncate_chars(&app.wizard_scan_current_path, 75)).monospace().small());
                    ui.add_space(12.0);

                    let pb = egui::ProgressBar::new(app.wizard_scan_progress)
                        .show_percentage()
                        .fill(Color32::from_rgb(34, 197, 94));
                    ui.add(pb);

                    ui.add_space(16.0);
                    ui.horizontal(|ui| {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("Отмена").clicked() {
                                close_wizard = true;
                            }
                        });
                    });
                }
                WizardPhase::Review => {
                    ui.label(RichText::new(format!("Принудительное удаление '{}'", app.wizard_app_title)).heading().strong());
                    ui.add_space(4.0);
                    ui.label("Uninstall Tool обнаружила, что следующие части данной программы все еще остались на вашем компьютере. Просмотрите внимательно выбранные элементы прежде, чем вы их решите удалить, нажав кнопку 'Удалить'.");
                    ui.add_space(10.0);

                    // Action buttons top row
                    ui.horizontal(|ui| {
                        if ui.button("📄 Экспортировать в XML").on_hover_text("Экспортировать список остатков в XML-файл").clicked() {
                            match export_traces_to_xml(&app.wizard_app_title, &app.wizard_traces) {
                                Ok(path) => {
                                    let _ = Command::new("notepad.exe").arg(&path).spawn();
                                    app.wizard_xml_status = Some("Экспорт завершен (открыт в Блокноте)".to_string());
                                }
                                Err(e) => {
                                    app.wizard_xml_status = Some(format!("Ошибка экспорта: {}", e));
                                }
                            }
                        }
                        if let Some(st) = &app.wizard_xml_status {
                            ui.label(RichText::new(st).small().color(Color32::from_rgb(34, 197, 94)));
                        }

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("Отмена").clicked() {
                                close_wizard = true;
                            }
                            let selected_count = app.wizard_traces.iter().filter(|t| t.selected).count();
                            let del_btn = egui::Button::new(RichText::new("Удалить").strong().color(Color32::WHITE))
                                .fill(Color32::from_rgb(220, 38, 38));
                            if ui.add_enabled(selected_count > 0, del_btn).clicked() {
                                let uninst_reg = app.wizard_target_app.as_ref().map(|a| a.registry_path.clone());
                                let selected_traces: Vec<_> = app.wizard_traces.iter().filter(|t| t.selected).cloned().collect();
                                app.wizard_phase = WizardPhase::Deleting;
                                app.wizard_delete_progress = 0.0;
                                app.wizard_delete_rx = Some(spawn_delete_leftovers_stream(app.wizard_app_title.clone(), selected_traces, uninst_reg));
                            }
                        });
                    });

                    ui.separator();

                    // Grouped tree list
                    let fs_traces: Vec<_> = app.wizard_traces.iter().filter(|t| t.trace_type != "registry_key").cloned().collect();
                    let reg_traces: Vec<_> = app.wizard_traces.iter().filter(|t| t.trace_type == "registry_key").cloned().collect();

                    egui::ScrollArea::vertical().max_height(240.0).show(ui, |ui| {
                        if app.wizard_traces.is_empty() {
                            ui.label("Оставшихся следов программы не обнаружено (чистая система).");
                        } else {
                            if !fs_traces.is_empty() {
                                let fs_size: u64 = fs_traces.iter().map(|t| t.size_bytes).sum();
                                ui.strong(format!("🗀 Файловая система ({} элементов, {})", fs_traces.len(), format_size(fs_size)));
                                for t in &mut app.wizard_traces {
                                    if t.trace_type != "registry_key" {
                                        ui.horizontal(|ui| {
                                            ui.add_space(14.0);
                                            ui.checkbox(&mut t.selected, "");
                                            let icon = if t.trace_type == "folder" { "📁" } else { "📄" };
                                            ui.label(format!("{} {}", icon, t.path));
                                        });
                                    }
                                }
                            }

                            if !reg_traces.is_empty() {
                                ui.add_space(6.0);
                                ui.strong(format!("🗁 Реестр ({} элементов)", reg_traces.len()));
                                for t in &mut app.wizard_traces {
                                    if t.trace_type == "registry_key" {
                                        ui.horizontal(|ui| {
                                            ui.add_space(14.0);
                                            ui.checkbox(&mut t.selected, "");
                                            ui.label(format!("🗄 {}", t.path));
                                        });
                                    }
                                }
                            }
                        }
                    });

                    ui.separator();

                    // Bottom selection & summary row
                    ui.horizontal(|ui| {
                        if ui.button("Выбрать все").clicked() {
                            for t in &mut app.wizard_traces {
                                t.selected = true;
                            }
                        }
                        if ui.button("Снять выбор").clicked() {
                            for t in &mut app.wizard_traces {
                                t.selected = false;
                            }
                        }

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let fs_count = app.wizard_traces.iter().filter(|t| t.trace_type != "registry_key").count();
                            let reg_count = app.wizard_traces.iter().filter(|t| t.trace_type == "registry_key").count();
                            let fs_size: u64 = app.wizard_traces.iter().filter(|t| t.trace_type != "registry_key").map(|t| t.size_bytes).sum();
                            ui.label(RichText::new(format!("{} файлов/папок ({}) и {} элементов реестра", fs_count, format_size(fs_size), reg_count)).small());
                        });
                    });
                }
                WizardPhase::Deleting => {
                    ui.heading(RichText::new("Удаление элементов...").size(15.0).strong());
                    ui.add_space(10.0);
                    ui.label("Идет очистка выбранных файлов, папок и веток реестра Windows...");
                    ui.add_space(12.0);

                    let pb = egui::ProgressBar::new(app.wizard_delete_progress)
                        .show_percentage()
                        .fill(Color32::from_rgb(34, 197, 94));
                    ui.add(pb);
                    ui.add_space(8.0);
                    let sub_color = if app.dark_mode { Color32::from_rgb(156, 163, 175) } else { Color32::from_rgb(107, 114, 128) };
                    ui.label(RichText::new(truncate_chars(&app.wizard_scan_current_path, 75)).size(11.0).monospace().color(sub_color));
                }
                WizardPhase::Finished => {
                    ui.heading(RichText::new("✔ Все остатки программы удалены").size(16.0).strong().color(Color32::from_rgb(34, 197, 94)));
                    ui.add_space(8.0);
                    ui.label(format!("Программа «{}» успешно удалена с вашего компьютера.", app.wizard_app_title));
                    if let Some(rep) = &app.wizard_report {
                        ui.add_space(4.0);
                        ui.label(RichText::new(rep).color(Color32::from_rgb(34, 197, 94)));
                    }
                    ui.add_space(10.0);

                    // Scrollable list of deleted items in bright green with checkmarks
                    egui::ScrollArea::vertical().max_height(140.0).show(ui, |ui| {
                        for trace in app.wizard_traces.iter().filter(|t| t.selected) {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("✔").color(Color32::from_rgb(34, 197, 94)).strong());
                                let icon = match trace.trace_type.as_str() {
                                    "registry_key" => "🗄",
                                    "folder" => "📁",
                                    _ => "📄",
                                };
                                ui.label(RichText::new(format!("{} {}", icon, trace.path)).color(Color32::from_rgb(34, 197, 94)).size(11.0));
                            });
                        }
                    });

                    ui.add_space(14.0);
                    ui.horizontal(|ui| {
                        if let Some(log_p) = &app.wizard_last_log_path {
                            let link_color = if app.dark_mode { Color32::from_rgb(96, 165, 250) } else { Color32::from_rgb(37, 99, 235) };
                            if ui.link(RichText::new("Лог удаления").underline().color(link_color)).clicked() {
                                let _ = Command::new("notepad.exe").arg(log_p).spawn();
                            }
                        }

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button(RichText::new("Закрыть").strong()).clicked() {
                                close_wizard = true;
                            }
                        });
                    });
                }
            }
        });

    if close_wizard {
        app.wizard_open = false;
        if let Some(target) = app.wizard_target_app.take() {
            let target_id = target.id.to_lowercase();
            let target_name = target.name.to_lowercase();
            app.recently_uninstalled.insert(target_id.clone());
            app.recently_uninstalled.insert(target_name.clone());
            app.apps.retain(|a| a.id.to_lowercase() != target_id && a.name.to_lowercase() != target_name);
        }
        app.wizard_app_title.clear();
        app.wizard_traces.clear();
        app.wizard_report = None;
        app.wizard_scan_rx = None;
        app.wizard_delete_rx = None;
        app.wizard_xml_status = None;
        let total_size: u64 = app.apps.iter().map(|a| a.size_bytes).sum();
        app.status_msg = format!("Программ: {} | Общий объем: {}", app.apps.len(), format_size(total_size));
        if let Some(idx) = app.selected_app_idx {
            if idx >= app.apps.len() {
                app.selected_app_idx = if !app.apps.is_empty() { Some(0) } else { None };
            }
        }
    }
}
