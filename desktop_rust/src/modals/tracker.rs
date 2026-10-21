use eframe::egui::{self, Color32, RichText, Vec2, Stroke};
use crate::app::PurgeCraftApp;
use crate::types::TrackerStep;
use crate::scanner::{self, take_system_snapshot, diff_system_snapshot, save_traced_apps, pick_file_dialog, TracedApp};

pub fn render_tracker_modal(app: &mut PurgeCraftApp, ctx: &egui::Context) {
    if !app.tracker_modal_open {
        return;
    }

    let mut is_open = true;
    egui::Window::new("Установить и следить за программой")
        .open(&mut is_open)
        .collapsible(false)
        .resizable(true)
        .pivot(egui::Align2::CENTER_CENTER)
        .default_pos(ctx.screen_rect().center())
        .show(ctx, |ui| {
            ui.set_width(580.0);
            ui.set_height(420.0);

            match app.tracker_step {
                TrackerStep::SelectInstaller => {
                    ui.heading("Установка программы с отслеживанием");
                    ui.add_space(6.0);
                    ui.label("PurgeCraft зафиксирует снимок реестра и файловой системы до установки, запустит инсталлятор, а затем определит все добавленные ключи, ветки и папки. Это гарантирует 100% чистое удаление в будущем.");
                    ui.add_space(12.0);

                    ui.label("Путь к инсталлятору (.exe, .msi):");
                    ui.horizontal(|ui| {
                        ui.add(egui::TextEdit::singleline(&mut app.tracker_installer_path).desired_width(420.0));
                        if ui.button("📁 Обзор...").clicked() {
                            if let Some(p) = pick_file_dialog("Инсталляторы", "*.exe;*.msi") {
                                if app.tracker_app_name.is_empty() {
                                    if let Some(stem) = std::path::Path::new(&p).file_stem() {
                                        app.tracker_app_name = stem.to_string_lossy().to_string();
                                    }
                                }
                                app.tracker_installer_path = p;
                            }
                        }
                    });

                    ui.add_space(8.0);
                    ui.label("Название программы:");
                    ui.add(egui::TextEdit::singleline(&mut app.tracker_app_name).desired_width(420.0));

                    ui.add_space(16.0);
                    ui.horizontal(|ui| {
                        if ui.button("Отмена").clicked() {
                            app.tracker_modal_open = false;
                        }
                        let can_start = !app.tracker_installer_path.trim().is_empty() && std::path::Path::new(&app.tracker_installer_path).exists();
                        if ui.add_enabled(can_start, egui::Button::new(RichText::new("▶ Начать снимок и запустить установку").strong().color(Color32::WHITE))).clicked() {
                            let snap = take_system_snapshot();
                            app.tracker_snapshot_before = Some(snap);
                            app.tracker_live_log.clear();
                            app.tracker_process_finished = false;
                            match scanner::spawn_install_tracker(app.tracker_installer_path.clone()) {
                                Ok((rx, stop_tx, child)) => {
                                    app.tracker_rx = Some(rx);
                                    app.tracker_stop_tx = Some(stop_tx);
                                    app.tracker_child = Some(child);
                                    app.tracker_step = TrackerStep::Installing;
                                }
                                Err(e) => {
                                    app.status_msg = format!("Ошибка запуска: {}", e);
                                }
                            }
                        }
                    });
                }
                TrackerStep::Installing => {
                    let mut finish_tracking = false;
                    let mut abort_tracking = false;

                    ui.horizontal(|ui| {
                        ui.label(RichText::new("📦").size(22.0));
                        ui.heading(RichText::new("Установить и следить за программой").size(15.0).strong());
                    });
                    ui.add_space(6.0);
                    ui.label(RichText::new("Прогресс установки:").strong().size(12.5));
                    ui.add_space(4.0);

                    // Dark Terminal Log Box (exact 1:1 match with Screenshot 2)
                    let log_bg = if app.dark_mode { Color32::from_rgb(15, 17, 23) } else { Color32::from_rgb(15, 23, 42) };
                    let border_color = if app.dark_mode { Color32::from_rgb(60, 64, 75) } else { Color32::from_rgb(51, 65, 85) };

                    egui::Frame::none()
                        .fill(log_bg)
                        .stroke(Stroke::new(1.0_f32, border_color))
                        .inner_margin(8.0)
                        .show(ui, |ui| {
                            ui.set_min_width(540.0);
                            ui.set_height(250.0);

                            egui::ScrollArea::vertical()
                                .stick_to_bottom(true)
                                .max_height(245.0)
                                .show(ui, |ui| {
                                    for entry in &app.tracker_live_log {
                                        ui.horizontal(|ui| {
                                            // Col 1: Time (width 70)
                                            ui.allocate_ui_with_layout(Vec2::new(70.0, 16.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                                                ui.label(RichText::new(&entry.time).monospace().size(11.5).color(Color32::from_rgb(203, 213, 225)));
                                            });

                                            // Col 2: Action (width 110)
                                            ui.allocate_ui_with_layout(Vec2::new(110.0, 16.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                                                if !entry.action.is_empty() {
                                                    let act_color = match entry.action.as_str() {
                                                        "Создана папка" => Color32::from_rgb(234, 179, 8),
                                                        "Создан ключ реестра" | "Запись реестра" => Color32::from_rgb(168, 85, 247),
                                                        _ => Color32::from_rgb(56, 189, 248),
                                                    };
                                                    ui.label(RichText::new(&entry.action).monospace().size(11.5).color(act_color));
                                                }
                                            });

                                            // Col 3: Path / Message
                                            let path_color = if entry.action.is_empty() {
                                                Color32::from_rgb(241, 245, 249)
                                            } else {
                                                Color32::from_rgb(226, 232, 240)
                                            };
                                            ui.label(RichText::new(&entry.path).monospace().size(11.5).color(path_color));
                                        });
                                    }
                                });
                        });

                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let stop_btn = egui::Button::new(RichText::new("Стоп").size(12.5))
                                .min_size(Vec2::new(75.0, 26.0));
                            if ui.add(stop_btn).on_hover_text("Остановить монитор установки").clicked() {
                                abort_tracking = true;
                            }

                            let finish_btn = egui::Button::new(
                                RichText::new("Установить и следить").size(12.5).strong().color(Color32::WHITE)
                            )
                            .fill(if app.dark_mode { Color32::from_rgb(45, 52, 65) } else { Color32::from_rgb(30, 41, 59) })
                            .stroke(Stroke::new(1.0_f32, Color32::from_rgb(75, 85, 99)))
                            .min_size(Vec2::new(175.0, 26.0));

                            if ui.add(finish_btn).on_hover_text("Завершить отслеживание и зафиксировать созданные следы").clicked() {
                                finish_tracking = true;
                            }
                        });
                    });

                    if abort_tracking {
                        if let Some(stop) = app.tracker_stop_tx.take() {
                            let _ = stop.send(());
                        }
                        app.tracker_rx = None;
                        app.tracker_child = None;
                        app.tracker_modal_open = false;
                        app.tracker_step = TrackerStep::SelectInstaller;
                    }

                    if finish_tracking {
                        if let Some(stop) = app.tracker_stop_tx.take() {
                            let _ = stop.send(());
                        }
                        app.tracker_rx = None;
                        app.tracker_child = None;
                        if let Some(before) = &app.tracker_snapshot_before {
                            let traces = diff_system_snapshot(before, &app.tracker_app_name);
                            app.tracker_traces = traces;
                            app.tracker_step = TrackerStep::ReviewTraces;
                        }
                    }
                }
                TrackerStep::ReviewTraces => {
                    ui.heading(format!("Следы установки «{}»", app.tracker_app_name));
                    ui.add_space(4.0);
                    ui.label(format!("Зафиксировано новых элементов в системе: {}", app.tracker_traces.len()));
                    ui.separator();

                    egui::ScrollArea::vertical().max_height(240.0).show(ui, |ui| {
                        if app.tracker_traces.is_empty() {
                            ui.label("Новых файлов или ключей реестра не зафиксировано.");
                        } else {
                            for t in &app.tracker_traces {
                                let icon = if t.trace_type == "registry_key" { "🗄" } else { "📁" };
                                ui.label(format!("{} {}", icon, t.path));
                            }
                        }
                    });

                    ui.separator();
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.button("Отмена").clicked() {
                            app.tracker_modal_open = false;
                        }
                        if ui.button(RichText::new("💾 Сохранить в базу отслеженных программ").strong().color(Color32::WHITE)).clicked() {
                            let now = chrono::Local::now().format("%Y-%m-%d").to_string();
                            let traced = TracedApp {
                                id: format!("traced_{}_{}", app.tracker_app_name, app.tracker_traces.len()),
                                name: app.tracker_app_name.clone(),
                                install_date: now,
                                installer_path: app.tracker_installer_path.clone(),
                                traces: app.tracker_traces.clone(),
                            };
                            app.traced_apps.push(traced);
                            save_traced_apps(&app.traced_apps);
                            app.status_msg = format!("Программа «{}» успешно зарегистрирована как отслеженная ({} следов).", app.tracker_app_name, app.tracker_traces.len());
                            app.tracker_modal_open = false;
                            app.refresh_data();
                        }
                    });
                }
            }
        });

    if !is_open {
        if let Some(stop) = app.tracker_stop_tx.take() {
            let _ = stop.send(());
        }
        app.tracker_rx = None;
        app.tracker_child = None;
        app.tracker_modal_open = false;
        app.tracker_step = TrackerStep::SelectInstaller;
    }
}
