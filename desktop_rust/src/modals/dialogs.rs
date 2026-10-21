use std::process::Command;
#[cfg(windows)]
use std::os::windows::process::CommandExt;
use eframe::egui::{self, Color32, RichText, Vec2, Stroke};
use crate::app::PurgeCraftApp;
use crate::scanner::{self, scan_leftovers, add_startup_item, scan_startup_items, pick_file_dialog};
use crate::types::{format_size_lang, truncate_chars};

pub fn render_dialogs(app: &mut PurgeCraftApp, ctx: &egui::Context) {
    let lang = app.lang();

    // 0. Single Uninstall Confirmation Modal
    if app.uninstall_confirm_open {
        if let Some(target_app) = app.uninstall_target_app.clone() {
            egui::Window::new(crate::i18n::t(lang, "Подтверждение деинсталляции", "Uninstall Confirmation"))
                .collapsible(false)
                .resizable(false)
                .pivot(egui::Align2::CENTER_CENTER)
                .default_pos(ctx.screen_rect().center())
                .show(ctx, |ui| {
                    ui.set_width(450.0);
                    ui.heading(crate::i18n::t(lang, "Деинсталляция программы", "Program Uninstallation"));
                    ui.add_space(8.0);
                    let confirm_msg = format!(
                        "{} «{}»?",
                        crate::i18n::t(lang, "Вы действительно хотите удалить программу", "Are you sure you want to uninstall"),
                        target_app.name
                    );
                    ui.label(confirm_msg);
                    ui.label(crate::i18n::t(
                        lang,
                        "После выполнения удаления PurgeCraft проверит систему на оставшиеся файлы, папки и записи реестра.",
                        "After uninstallation completes, PurgeCraft will scan the system for leftover files, folders, and registry keys."
                    ));
                    ui.add_space(16.0);

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add(egui::Button::new(crate::i18n::t(lang, "Отмена", "Cancel")).min_size(Vec2::new(85.0, 26.0))).clicked() {
                            app.uninstall_confirm_open = false;
                            app.uninstall_target_app = None;
                        }
                        let uninst_btn = egui::Button::new(
                            RichText::new(crate::i18n::t(lang, "Деинсталлировать", "Uninstall")).strong().color(Color32::WHITE),
                        )
                        .fill(Color32::from_rgb(37, 99, 235))
                        .min_size(Vec2::new(145.0, 26.0));

                        if ui.add(uninst_btn).clicked() {
                            app.uninstall_confirm_open = false;
                            app.uninstall_target_app = None;
                            let running = scanner::find_running_processes_for_app(&target_app);
                            if !running.is_empty() {
                                app.running_processes = running;
                                app.running_processes_target_app = Some(target_app);
                                app.running_processes_modal_open = true;
                            } else {
                                app.execute_uninstall(target_app);
                            }
                        }
                    });
                });
        }
    }

    // 0a. Running Processes Termination Modal
    if app.running_processes_modal_open {
        if let Some(target_app) = app.running_processes_target_app.clone() {
            egui::Window::new(crate::i18n::t(lang, "Принудительно завершить запущенные процессы?", "Terminate Running Processes?"))
                .collapsible(false)
                .resizable(false)
                .pivot(egui::Align2::CENTER_CENTER)
                .default_pos(ctx.screen_rect().center())
                .show(ctx, |ui| {
                    ui.set_width(520.0);

                    ui.horizontal(|ui| {
                        ui.label(RichText::new("⛔").size(24.0));
                        ui.vertical(|ui| {
                            ui.heading(RichText::new(crate::i18n::t(lang, "Принудительно завершить запущенные процессы?", "Terminate Running Processes?")).strong());
                            ui.label(RichText::new(crate::i18n::t(lang, "Следующие процессы были обнаружены:", "The following active processes were detected:")).color(Color32::from_rgb(156, 163, 175)));
                        });
                    });

                    ui.add_space(8.0);

                    egui::Frame::none()
                        .fill(if app.dark_mode { Color32::from_rgb(24, 24, 27) } else { Color32::from_rgb(243, 244, 246) })
                        .rounding(4.0)
                        .stroke(Stroke::new(1.0_f32, if app.dark_mode { Color32::from_rgb(63, 63, 70) } else { Color32::from_rgb(229, 231, 235) }))
                        .inner_margin(8.0)
                        .show(ui, |ui| {
                            egui::ScrollArea::vertical().max_height(140.0).show(ui, |ui| {
                                for proc in &app.running_processes {
                                    ui.horizontal(|ui| {
                                        ui.label(RichText::new(format!("[PID {}]", proc.pid)).monospace().color(Color32::from_rgb(245, 158, 11)));
                                        ui.label(RichText::new(&proc.name).strong());
                                        ui.label(RichText::new(format!("({})", truncate_chars(&proc.exe_path, 40))).color(Color32::GRAY));
                                    });
                                }
                            });
                        });

                    ui.add_space(10.0);
                    ui.label(RichText::new(crate::i18n::t(
                        lang,
                        "Рекомендуется завершать процессы вручную (если это возможно) перед удалением программы, однако Мастер Удаления это может сделать принудительно.",
                        "It is recommended to close running apps before uninstalling, but the Removal Wizard can terminate them forcibly."
                    ))
                        .small()
                        .color(if app.dark_mode { Color32::from_rgb(209, 213, 219) } else { Color32::from_rgb(55, 65, 81) }));

                    ui.add_space(14.0);

                    ui.horizontal(|ui| {
                        if ui.button(RichText::new(crate::i18n::t(lang, "⛔ Завершить процессы", "⛔ Terminate Processes")).strong().color(Color32::WHITE)).clicked() {
                            for proc in &app.running_processes {
                                let _ = scanner::terminate_process_by_pid(proc.pid);
                            }
                            app.running_processes_modal_open = false;
                            app.running_processes.clear();
                            app.running_processes_target_app = None;
                            app.execute_uninstall(target_app.clone());
                        }

                        if ui.button(crate::i18n::t(lang, "⏩ Пропустить", "⏩ Skip")).clicked() {
                            app.running_processes_modal_open = false;
                            app.running_processes.clear();
                            app.running_processes_target_app = None;
                            app.execute_uninstall(target_app);
                        }

                        if ui.button(crate::i18n::t(lang, "✖ Отмена", "✖ Cancel")).clicked() {
                            app.running_processes_modal_open = false;
                            app.running_processes.clear();
                            app.running_processes_target_app = None;
                        }
                    });
                });
        }
    }

    // 0d. Delayed Files Removal On Reboot Modal
    if app.delayed_removal_modal_open {
        egui::Window::new(crate::i18n::t(lang, "Некоторые файлы не удалены", "Some files could not be deleted"))
            .collapsible(false)
            .resizable(false)
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(ctx.screen_rect().center())
            .show(ctx, |ui| {
                ui.set_width(540.0);

                ui.horizontal(|ui| {
                    ui.label(RichText::new("🔒").size(24.0));
                    ui.vertical(|ui| {
                        ui.heading(RichText::new(crate::i18n::t(lang, "Некоторые файлы не удалены", "Some files could not be deleted")).strong());
                        ui.label(RichText::new(crate::i18n::t(
                            lang,
                            "Следующие файлы на данный момент используются системой и не могут быть удалены:",
                            "The following files are currently in use by the system and cannot be deleted:"
                        )).color(Color32::from_rgb(156, 163, 175)));
                    });
                });

                ui.add_space(8.0);

                egui::Frame::none()
                    .fill(if app.dark_mode { Color32::from_rgb(24, 24, 27) } else { Color32::from_rgb(243, 244, 246) })
                    .rounding(4.0)
                    .stroke(Stroke::new(1.0_f32, if app.dark_mode { Color32::from_rgb(63, 63, 70) } else { Color32::from_rgb(229, 231, 235) }))
                    .inner_margin(8.0)
                    .show(ui, |ui| {
                        egui::ScrollArea::vertical().max_height(140.0).show(ui, |ui| {
                            for file in &app.delayed_removal_files {
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new("⏳").color(Color32::from_rgb(245, 158, 11)));
                                    ui.label(RichText::new(file).monospace().color(if app.dark_mode { Color32::from_rgb(229, 231, 235) } else { Color32::from_rgb(31, 41, 55) }));
                                });
                            }
                        });
                    });

                ui.add_space(10.0);
                ui.label(RichText::new(crate::i18n::t(
                    lang,
                    "Эти файлы будут удалены автоматически при следующем запуске компьютера.",
                    "These files will be deleted automatically upon next system restart."
                ))
                    .small()
                    .strong()
                    .color(Color32::from_rgb(34, 197, 94)));

                ui.add_space(14.0);

                ui.horizontal(|ui| {
                    if ui.button(RichText::new(crate::i18n::t(lang, "🔄 Перезагрузить", "🔄 Restart")).strong().color(Color32::WHITE)).clicked() {
                        app.delayed_removal_modal_open = false;
                        app.delayed_removal_files.clear();
                        let mut cmd = Command::new("shutdown");
                        #[cfg(windows)]
                        cmd.creation_flags(scanner::CREATE_NO_WINDOW);
                        let _ = cmd.args(["/r", "/t", "5"]).spawn();
                    }

                    if ui.button(crate::i18n::t(lang, "Позже", "Later")).clicked() {
                        app.delayed_removal_modal_open = false;
                        app.delayed_removal_files.clear();
                    }
                });
            });
    }

    // 0b. Waiting for Native Uninstaller Completion Modal
    if app.uninstall_waiting_open {
        if let Some(target_app) = app.uninstall_waiting_app.clone() {
            egui::Window::new(format!("{} — {}", crate::i18n::t(lang, "Деинсталляция", "Uninstallation"), target_app.name))
                .id(egui::Id::new("single_uninstall_progress_dialog"))
                .collapsible(false)
                .resizable(false)
                .pivot(egui::Align2::CENTER_CENTER)
                .default_pos(ctx.screen_rect().center())
                .show(ctx, |ui| {
                    ui.set_width(480.0);
                    ui.heading(RichText::new(crate::i18n::t(lang, "Выполняется штатная деинсталляция...", "Standard uninstaller in progress...")).color(Color32::from_rgb(96, 165, 250)));
                    ui.add_space(8.0);
                    ui.label(crate::i18n::t(lang, "Запущен штатный деинсталлятор программы.", "Standard program uninstaller has been launched."));
                    ui.label(crate::i18n::t(lang, "Пожалуйста, завершите процесс удаления в окне программы.", "Please complete the uninstallation in the program's window."));
                    ui.label(crate::i18n::t(lang, "Как только процесс завершится, PurgeCraft автоматически запустит поиск оставшихся следов.", "Once the process finishes, PurgeCraft will automatically scan for leftover traces."));
                    ui.add_space(14.0);
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(crate::i18n::t(lang, "Ожидание закрытия деинсталлятора...", "Waiting for uninstaller to exit..."));
                    });
                    ui.add_space(12.0);
                    if ui.button(RichText::new(crate::i18n::t(lang, "⚡ Перейти к поиску остатков", "⚡ Proceed to leftover scan")).strong()).clicked() {
                        app.uninstall_waiting_open = false;
                        app.uninstall_child = None;
                        app.start_wizard_scan(target_app);
                    }
                });
        }
    }

    // 1. Single Force Removal Confirmation
    if app.force_confirm_open {
        if let Some(target_app) = app.force_confirm_app.clone() {
            egui::Window::new(crate::i18n::t(lang, "Принудительное удаление программы", "Force Program Removal"))
                .collapsible(false)
                .resizable(false)
                .pivot(egui::Align2::CENTER_CENTER)
                .default_pos(ctx.screen_rect().center())
                .show(ctx, |ui| {
                    ui.set_width(450.0);
                    ui.label(RichText::new(crate::i18n::t(lang, "Внимание!", "Warning!")).color(Color32::from_rgb(239, 68, 68)).heading());
                    ui.add_space(8.0);
                    ui.label(format!("{} «{}»?", crate::i18n::t(lang, "Вы действительно хотите принудительно удалить", "Are you sure you want to forcibly remove"), target_app.name));
                    ui.label(crate::i18n::t(
                        lang,
                        "Штатный деинсталлятор запускаться не будет. Система напрямую выполнит поиск и зачистку всех связанных записей реестра и папок установки.",
                        "Standard uninstaller will not be launched. The system will directly scan and clean all associated registry entries and install directories."
                    ));
                    ui.add_space(16.0);

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add(egui::Button::new(crate::i18n::t(lang, "Отмена", "Cancel")).min_size(Vec2::new(85.0, 26.0))).clicked() {
                            app.force_confirm_open = false;
                            app.force_confirm_app = None;
                        }
                        let btn = egui::Button::new(RichText::new(crate::i18n::t(lang, "Начать удаление", "Start Removal")).strong().color(Color32::WHITE))
                            .fill(Color32::from_rgb(220, 38, 38))
                            .min_size(Vec2::new(135.0, 26.0));
                        if ui.add(btn).clicked() {
                            app.force_confirm_open = false;
                            app.force_confirm_app = None;
                            app.start_wizard_scan(target_app);
                        }
                    });
                });
        }
    }

    // 1b. Remove Registry Entry Confirmation Modal
    if app.remove_entry_confirm_open {
        if let Some(target_app) = app.remove_entry_confirm_app.clone() {
            egui::Window::new(crate::i18n::t(lang, "Удаление записи из реестра", "Remove Registry Entry"))
                .collapsible(false)
                .resizable(false)
                .pivot(egui::Align2::CENTER_CENTER)
                .default_pos(ctx.screen_rect().center())
                .show(ctx, |ui| {
                    ui.set_width(450.0);
                    ui.label(RichText::new(crate::i18n::t(lang, "Удалить запись из списка?", "Remove entry from list?")).heading());
                    ui.add_space(8.0);
                    ui.label(format!("{} «{}»?", crate::i18n::t(lang, "Вы действительно хотите удалить запись", "Are you sure you want to remove entry"), target_app.name));
                    ui.label(crate::i18n::t(
                        lang,
                        "Штатный деинсталлятор запускаться не будет, а файлы программы не будут затронуты. Это действие удаляет только запись из реестра Windows (полезно для уже удаленных вручную программ).",
                        "The standard uninstaller will not run and program files will remain untouched. This action only removes the entry from the Windows registry (useful for manually deleted apps)."
                    ));
                    ui.add_space(16.0);

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add(egui::Button::new(crate::i18n::t(lang, "Отмена", "Cancel")).min_size(Vec2::new(85.0, 26.0))).clicked() {
                            app.remove_entry_confirm_open = false;
                            app.remove_entry_confirm_app = None;
                        }
                        let btn = egui::Button::new(RichText::new(crate::i18n::t(lang, "Удалить запись", "Remove Entry")).strong().color(Color32::WHITE))
                            .fill(Color32::from_rgb(220, 38, 38))
                            .min_size(Vec2::new(135.0, 26.0));
                        if ui.add(btn).clicked() {
                            app.remove_entry_confirm_open = false;
                            app.remove_entry_confirm_app = None;
                            app.remove_registry_entry(&target_app);
                        }
                    });
                });
        }
    }

    // 2. Batch Uninstall Queue Modal
    if app.batch_queue_open {
        let total = app.batch_queue_apps.len();
        let curr = app.batch_queue_idx;

        egui::Window::new(format!("{} ({}/{})", crate::i18n::t(lang, "Очередь пакетной деинсталляции", "Batch Uninstall Queue"), (curr + 1).min(total), total))
            .id(egui::Id::new("batch_uninstall_queue_dialog"))
            .collapsible(false)
            .resizable(true)
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(ctx.screen_rect().center())
            .show(ctx, |ui| {
                ui.set_width(540.0);
                ui.set_height(380.0);

                ui.heading(crate::i18n::t(lang, "Процесс пакетного удаления программ", "Batch Program Uninstallation Process"));
                ui.add_space(4.0);
                ui.label(crate::i18n::t(lang, "Программы удаляются по очереди для исключения конфликтов деинсталляторов.", "Programs are uninstalled sequentially to avoid uninstaller conflicts."));
                ui.separator();

                egui::ScrollArea::vertical().max_height(220.0).show(ui, |ui| {
                    for (i, a) in app.batch_queue_apps.iter().enumerate() {
                        ui.horizontal(|ui| {
                            let raw_st = &app.batch_queue_statuses[i];
                            let (label_st, color) = match raw_st.as_str() {
                                "Выполняется..." => (crate::i18n::t(lang, "Выполняется...", "In progress..."), Color32::from_rgb(56, 189, 248)),
                                "Завершено" => (crate::i18n::t(lang, "Завершено", "Completed"), Color32::from_rgb(34, 197, 94)),
                                "Пропущено" => (crate::i18n::t(lang, "Пропущено", "Skipped"), Color32::from_rgb(148, 163, 184)),
                                _ => (raw_st.as_str(), Color32::from_rgb(100, 116, 139)),
                            };
                            ui.label(RichText::new(format!("[{}]", label_st)).color(color).strong());
                            ui.label(RichText::new(&a.name).strong());
                            ui.label(format!("({})", format_size_lang(a.size_bytes, lang)));
                        });
                    }
                });

                ui.separator();
                ui.add_space(8.0);

                if curr < total {
                    let app_name = app.batch_queue_apps[curr].name.clone();
                    ui.label(format!("{}: «{}»", crate::i18n::t(lang, "Текущая", "Current"), app_name));
                    ui.horizontal(|ui| {
                        if ui.button(crate::i18n::t(lang, "⏩ Следующая программа", "⏩ Next program")).clicked() {
                            app.batch_queue_statuses[curr] = "Завершено".to_string();
                            app.batch_queue_idx += 1;
                            app.launch_current_batch_app();
                        }
                        if ui.button(crate::i18n::t(lang, "Пропустить", "Skip")).clicked() {
                            app.batch_queue_statuses[curr] = "Пропущено".to_string();
                            app.batch_queue_idx += 1;
                            app.launch_current_batch_app();
                        }
                        if ui.button(crate::i18n::t(lang, "Прервать очередь", "Abort queue")).clicked() {
                            app.batch_queue_open = false;
                        }
                    });
                } else {
                    ui.label(RichText::new(crate::i18n::t(lang, "Все программы из очереди обработаны!", "All queue programs have been processed!")).color(Color32::from_rgb(34, 197, 94)).strong());
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.button(crate::i18n::t(lang, "♻ Запустить поиск остатков для всех", "♻ Scan leftovers for all")).clicked() {
                            let mut combined_traces = Vec::new();
                            for a in &app.batch_queue_apps {
                                combined_traces.extend(scan_leftovers(a));
                            }
                            app.wizard_traces = combined_traces;
                            app.wizard_app_title = format!("{} {} {}", crate::i18n::t(lang, "Пакет из", "Batch of"), app.batch_queue_apps.len(), crate::i18n::t(lang, "программ", "programs"));
                            app.wizard_report = None;
                            app.batch_queue_open = false;
                            app.wizard_open = true;
                        }
                        if ui.button(crate::i18n::t(lang, "Закрыть", "Close")).clicked() {
                            app.batch_queue_open = false;
                            app.batch_selected.clear();
                            app.refresh_data();
                        }
                    });
                }
            });
    }

    // 3. Batch Force Removal Confirmation Modal
    if app.batch_force_open {
        let count = app.batch_force_apps.len();
        egui::Window::new(format!("{} ({} {})", crate::i18n::t(lang, "Пакетное принудительное удаление", "Batch Force Removal"), count, crate::i18n::t(lang, "программ", "programs")))
            .id(egui::Id::new("batch_force_modal"))
            .collapsible(false)
            .resizable(false)
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(ctx.screen_rect().center())
            .show(ctx, |ui| {
                ui.set_width(480.0);
                ui.label(RichText::new(crate::i18n::t(lang, "Внимание! Пакетная зачистка следов", "Warning! Batch leftover purge")).color(Color32::from_rgb(239, 68, 68)).heading());
                ui.add_space(8.0);
                ui.label(format!("{} {} {}", crate::i18n::t(lang, "Вы выбрали", "You selected"), count, crate::i18n::t(lang, "программ для одновременного принудительного удаления.", "programs for concurrent force removal.")));
                ui.label(crate::i18n::t(
                    lang,
                    "Система соберет все следы (реестр, папки, пакеты) воедино и полностью удалит их без запуска штатных деинсталляторов.",
                    "The system will aggregate all traces (registry, folders, packages) and completely remove them without running standard uninstallers."
                ));
                ui.add_space(16.0);

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.add(egui::Button::new(crate::i18n::t(lang, "Отмена", "Cancel")).min_size(Vec2::new(85.0, 26.0))).clicked() {
                        app.batch_force_open = false;
                    }
                    let btn = egui::Button::new(RichText::new(crate::i18n::t(lang, "Собрать остатки и удалить", "Collect leftovers and delete")).strong().color(Color32::WHITE))
                        .fill(Color32::from_rgb(220, 38, 38))
                        .min_size(Vec2::new(195.0, 26.0));
                    if ui.add(btn).clicked() {
                        let mut combined_traces = Vec::new();
                        for a in &app.batch_force_apps {
                            combined_traces.extend(scan_leftovers(a));
                        }
                        app.wizard_traces = combined_traces;
                        app.wizard_app_title = format!("{} {} {}", crate::i18n::t(lang, "Пакет из", "Batch of"), count, crate::i18n::t(lang, "программ", "programs"));
                        app.wizard_report = None;
                        app.batch_force_open = false;
                        app.wizard_open = true;
                    }
                });
            });
    }

    // 5. Add Startup Item Modal
    if app.add_startup_modal_open {
        egui::Window::new(crate::i18n::t(lang, "➕ Добавить программу в автозапуск", "➕ Add Program to Startup"))
            .collapsible(false)
            .resizable(false)
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(ctx.screen_rect().center())
            .show(ctx, |ui| {
                ui.set_width(460.0);
                ui.heading(crate::i18n::t(lang, "Новая запись автозагрузки", "New Startup Entry"));
                ui.add_space(8.0);

                ui.label(crate::i18n::t(lang, "Имя программы:", "Program Name:"));
                ui.text_edit_singleline(&mut app.new_startup_name);

                ui.add_space(6.0);
                ui.label(crate::i18n::t(lang, "Путь к исполняемому файлу (.exe):", "Executable Path (.exe):"));
                ui.horizontal(|ui| {
                    ui.add(egui::TextEdit::singleline(&mut app.new_startup_path).desired_width(360.0));
                    if ui.button(crate::i18n::t(lang, "📁 Обзор...", "📁 Browse...")).clicked() {
                        if let Some(p) = pick_file_dialog(crate::i18n::t(lang, "Исполняемые файлы", "Executables"), "*.exe") {
                            if app.new_startup_name.is_empty() {
                                if let Some(stem) = std::path::Path::new(&p).file_stem() {
                                    app.new_startup_name = stem.to_string_lossy().to_string();
                                }
                            }
                            app.new_startup_path = p;
                        }
                    }
                });

                ui.add_space(6.0);
                ui.label(crate::i18n::t(lang, "Аргументы запуска (необязательно):", "Launch Arguments (optional):"));
                ui.text_edit_singleline(&mut app.new_startup_args);

                ui.add_space(8.0);
                ui.checkbox(&mut app.new_startup_hklm, crate::i18n::t(lang, "Для всех пользователей (HKLM - требует прав администратора)", "For all users (HKLM - requires admin rights)"));

                if let Some(err) = &app.new_startup_error {
                    ui.add_space(4.0);
                    ui.label(RichText::new(err).color(Color32::from_rgb(239, 68, 68)));
                }

                ui.add_space(14.0);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.add(egui::Button::new(crate::i18n::t(lang, "Отмена", "Cancel")).min_size(Vec2::new(85.0, 26.0))).clicked() {
                        app.add_startup_modal_open = false;
                    }
                    let add_btn = egui::Button::new(RichText::new(crate::i18n::t(lang, "Добавить", "Add")).strong().color(Color32::WHITE))
                        .fill(Color32::from_rgb(37, 99, 235))
                        .min_size(Vec2::new(95.0, 26.0));
                    if ui.add(add_btn).clicked() {
                        if app.new_startup_name.trim().is_empty() {
                            app.new_startup_error = Some(crate::i18n::t(lang, "Укажите имя программы", "Specify program name").to_string());
                        } else if app.new_startup_path.trim().is_empty() {
                            app.new_startup_error = Some(crate::i18n::t(lang, "Укажите путь к исполняемому файлу", "Specify executable path").to_string());
                        } else {
                            if add_startup_item(&app.new_startup_name, &app.new_startup_path, &app.new_startup_args, app.new_startup_hklm) {
                                app.startup_items = scan_startup_items();
                                app.status_msg = format!("{} «{}» {}", crate::i18n::t(lang, "Программа", "Program"), app.new_startup_name, crate::i18n::t(lang, "добавлена в автозапуск.", "added to startup."));
                                app.add_startup_modal_open = false;
                            } else {
                                app.new_startup_error = Some(crate::i18n::t(lang, "Ошибка добавления в реестр. Возможно, требуются права администратора.", "Failed to add to registry. Admin privileges may be required.").to_string());
                            }
                        }
                    }
                });
            });
    }
}
