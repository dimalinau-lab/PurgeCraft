use std::process::Command;
use eframe::egui::{self, Color32, RichText, Vec2, Stroke};
use crate::app::PurgeCraftApp;
use crate::types::{format_size, open_system_folder};
use crate::scanner::{
    scan_shader_caches, clear_shader_cache_targets,
    scan_path_entries, clean_path_entries,
    scan_portable_apps, delete_portable_app, pick_folder_dialog,
    scan_winget_packages, upgrade_winget_package, uninstall_winget_package,
    PortableApp,
};

pub fn render_shader_modal(app: &mut PurgeCraftApp, ctx: &egui::Context) {
    if !app.shader_modal_open {
        return;
    }

    let mut is_open = true;
    let mut do_clean = false;
    let mut do_rescan = false;
    let mut do_close = false;

    let total_selected_bytes: u64 = app.shader_locations
        .iter()
        .filter(|l| l.selected)
        .map(|l| l.size_bytes)
        .sum();
    let total_selected_files: usize = app.shader_locations
        .iter()
        .filter(|l| l.selected)
        .map(|l| l.file_count)
        .sum();

    egui::Window::new("🎮 Очиститель кэша шейдеров GPU и DirectX")
        .open(&mut is_open)
        .collapsible(false)
        .resizable(true)
        .pivot(egui::Align2::CENTER_CENTER)
        .default_pos(ctx.screen_rect().center())
        .show(ctx, |ui| {
            ui.set_width(640.0);
            ui.set_height(460.0);

            ui.horizontal(|ui| {
                ui.heading(RichText::new("🎮 Глубокая очистка кэшей GPU").strong());
            });
            ui.add_space(4.0);
            ui.label("Игры и графические приложения компилируют шейдеры в системные папки драйверов (DirectX, NVIDIA, AMD). Со временем эти кэши разрастаются до десятков гигабайтов и не удаляются автоматически.");
            ui.add_space(8.0);

            if let Some(msg) = &app.shader_clean_msg {
                egui::Frame::none()
                    .fill(if app.dark_mode { Color32::from_rgb(20, 83, 45) } else { Color32::from_rgb(220, 252, 231) })
                    .rounding(4.0)
                    .inner_margin(8.0)
                    .show(ui, |ui| {
                        ui.label(RichText::new(msg).strong().color(Color32::from_rgb(22, 163, 74)));
                    });
                ui.add_space(8.0);
            }

            // Selection buttons
            ui.horizontal(|ui| {
                if ui.button("☑ Выбрать все").clicked() {
                    for loc in &mut app.shader_locations {
                        loc.selected = true;
                    }
                }
                if ui.button("☐ Снять выбор").clicked() {
                    for loc in &mut app.shader_locations {
                        loc.selected = false;
                    }
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("🔄 Пересканировать").clicked() {
                        do_rescan = true;
                    }
                });
            });
            ui.add_space(6.0);

            // List of locations
            let list_bg = if app.dark_mode { Color32::from_rgb(18, 18, 20) } else { Color32::WHITE };
            egui::Frame::none()
                .fill(list_bg)
                .stroke(Stroke::new(1.0_f32, if app.dark_mode { Color32::from_rgb(39, 39, 42) } else { Color32::from_rgb(226, 232, 240) }))
                .rounding(4.0)
                .inner_margin(8.0)
                .show(ui, |ui| {
                    egui::ScrollArea::vertical().max_height(240.0).show(ui, |ui| {
                        if app.shader_locations.is_empty() {
                            ui.vertical_centered(|ui| {
                                ui.add_space(30.0);
                                ui.label(RichText::new("Кэшей шейдеров не обнаружено или они уже пусты.").color(Color32::from_rgb(148, 163, 184)));
                            });
                        } else {
                            for loc in &mut app.shader_locations {
                                ui.horizontal(|ui| {
                                    ui.checkbox(&mut loc.selected, "");
                                    let vendor_color = match loc.vendor.as_str() {
                                        "NVIDIA" => Color32::from_rgb(34, 197, 94),
                                        "AMD" => Color32::from_rgb(239, 68, 68),
                                        _ => Color32::from_rgb(59, 130, 246),
                                    };
                                    ui.label(RichText::new(format!("[{}]", loc.vendor)).strong().color(vendor_color));
                                    ui.label(RichText::new(&loc.name).strong());
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        ui.label(RichText::new(format_size(loc.size_bytes)).strong());
                                        ui.label(format!("({} файлов)", loc.file_count));
                                    });
                                });
                                ui.label(RichText::new(&loc.path).small().color(Color32::from_rgb(113, 113, 122)));
                                ui.separator();
                            }
                        }
                    });
                });

            ui.add_space(10.0);
            ui.separator();
            ui.add_space(8.0);

            // Bottom action panel
            ui.horizontal(|ui| {
                let info_text = format!("Выбрано для очистки: {} ({} файлов)", format_size(total_selected_bytes), total_selected_files);
                ui.label(RichText::new(info_text).strong().size(13.0));

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Закрыть").clicked() {
                        do_close = true;
                    }
                    let can_clean = total_selected_bytes > 0 && !app.shader_cleaning;
                    let clean_btn = egui::Button::new(
                        RichText::new(format!("♻ Очистить ({})", format_size(total_selected_bytes)))
                            .strong()
                            .color(Color32::WHITE)
                    )
                    .fill(if can_clean { Color32::from_rgb(147, 51, 234) } else { Color32::from_rgb(100, 100, 100) })
                    .min_size(Vec2::new(160.0, 28.0));

                    if ui.add_enabled(can_clean, clean_btn).clicked() {
                        do_clean = true;
                    }
                });
            });
        });

    if do_close || !is_open {
        app.shader_modal_open = false;
    }

    if do_rescan {
        app.shader_locations = scan_shader_caches();
        app.shader_clean_msg = None;
    }

    if do_clean {
        let (deleted_files, deleted_bytes) = clear_shader_cache_targets(&app.shader_locations);
        app.shader_clean_msg = Some(format!("Очистка завершена! Освобождено {} (удалено {} файлов).", format_size(deleted_bytes), deleted_files));
        app.status_msg = format!("Кэш шейдеров очищен. Освобождено {}.", format_size(deleted_bytes));
        app.shader_locations = scan_shader_caches();
    }
}

pub fn render_path_modal(app: &mut PurgeCraftApp, ctx: &egui::Context) {
    if !app.path_modal_open {
        return;
    }

    let mut is_open = true;
    let mut do_clean = false;
    let mut do_rescan = false;
    let mut do_close = false;

    let total_selected = app.path_entries.iter().filter(|e| e.selected).count();
    let total_broken = app.path_entries.iter().filter(|e| !e.exists).count();
    let total_duplicates = app.path_entries.iter().filter(|e| e.is_duplicate).count();

    egui::Window::new("🗄 Санитайзер системного и пользовательского PATH")
        .open(&mut is_open)
        .collapsible(false)
        .resizable(true)
        .pivot(egui::Align2::CENTER_CENTER)
        .default_pos(ctx.screen_rect().center())
        .show(ctx, |ui| {
            ui.set_width(780.0);
            ui.set_height(480.0);

            ui.heading(RichText::new("🗄 Санитайзер переменных среды PATH").strong());
            ui.add_space(4.0);
            ui.label("Удаленные приложения часто оставляют «мертвые» ссылки на несуществующие каталоги в переменной PATH. Это замедляет работу командной строки и может вызывать конфликты библиотек.");
            ui.add_space(6.0);

            if let Some(msg) = &app.path_msg {
                egui::Frame::none()
                    .fill(if app.dark_mode { Color32::from_rgb(20, 83, 45) } else { Color32::from_rgb(220, 252, 231) })
                    .rounding(4.0)
                    .inner_margin(8.0)
                    .show(ui, |ui| {
                        ui.label(RichText::new(msg).strong().color(Color32::from_rgb(22, 163, 74)));
                    });
                ui.add_space(6.0);
            }

            // Action buttons
            ui.horizontal(|ui| {
                if ui.button("☑ Битые и дубликаты").clicked() {
                    for e in &mut app.path_entries {
                        e.selected = !e.exists || e.is_duplicate;
                    }
                }
                if ui.button("☑ Все").clicked() {
                    for e in &mut app.path_entries {
                        e.selected = true;
                    }
                }
                if ui.button("☐ Снять").clicked() {
                    for e in &mut app.path_entries {
                        e.selected = false;
                    }
                }
                ui.separator();
                ui.label(RichText::new(format!("Битых: {}  •  Дубликатов: {}", total_broken, total_duplicates)).color(Color32::from_rgb(148, 163, 184)));

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("🔄 Обновить").clicked() {
                        do_rescan = true;
                    }
                });
            });
            ui.add_space(6.0);

            let list_bg = if app.dark_mode { Color32::from_rgb(18, 18, 20) } else { Color32::WHITE };
            egui::Frame::none()
                .fill(list_bg)
                .stroke(Stroke::new(1.0_f32, if app.dark_mode { Color32::from_rgb(39, 39, 42) } else { Color32::from_rgb(226, 232, 240) }))
                .rounding(4.0)
                .inner_margin(8.0)
                .show(ui, |ui| {
                    egui::ScrollArea::vertical().max_height(270.0).show(ui, |ui| {
                        if app.path_entries.is_empty() {
                            ui.vertical_centered(|ui| {
                                ui.add_space(30.0);
                                ui.label(RichText::new("Записей в PATH не обнаружено.").color(Color32::from_rgb(148, 163, 184)));
                            });
                        } else {
                            for entry in &mut app.path_entries {
                                ui.horizontal(|ui| {
                                    ui.checkbox(&mut entry.selected, "");

                                    // Hive badge
                                    let hive_label = if entry.is_system { "[Система]" } else { "[Пользователь]" };
                                    let hive_color = if entry.is_system { Color32::from_rgb(168, 85, 247) } else { Color32::from_rgb(59, 130, 246) };
                                    ui.label(RichText::new(hive_label).small().strong().color(hive_color));

                                    // Status badge
                                    if !entry.exists {
                                        egui::Frame::none()
                                            .fill(if app.dark_mode { Color32::from_rgb(69, 10, 10) } else { Color32::from_rgb(254, 226, 226) })
                                            .rounding(3.0)
                                            .inner_margin(egui::Margin::symmetric(4.0, 1.0))
                                            .show(ui, |ui| {
                                                ui.label(RichText::new("🔴 МЕРТВЫЙ ПУТЬ").small().strong().color(Color32::from_rgb(239, 68, 68)));
                                            });
                                    } else if entry.is_duplicate {
                                        egui::Frame::none()
                                            .fill(if app.dark_mode { Color32::from_rgb(66, 32, 6) } else { Color32::from_rgb(254, 243, 199) })
                                            .rounding(3.0)
                                            .inner_margin(egui::Margin::symmetric(4.0, 1.0))
                                            .show(ui, |ui| {
                                                ui.label(RichText::new("🟡 ДУБЛИКАТ").small().strong().color(Color32::from_rgb(217, 119, 6)));
                                            });
                                    } else {
                                        ui.label(RichText::new("🟢 Существует").small().color(Color32::from_rgb(34, 197, 94)));
                                    }

                                    ui.label(RichText::new(&entry.path).monospace().size(12.0));
                                });
                                ui.separator();
                            }
                        }
                    });
                });

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(6.0);

            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("Выбрано для удаления: {} записей", total_selected)).strong());

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Закрыть").clicked() {
                        do_close = true;
                    }
                    let can_clean = total_selected > 0;
                    let clean_btn = egui::Button::new(
                        RichText::new(format!("♻ Очистить выбранные ({})", total_selected))
                            .strong()
                            .color(Color32::WHITE)
                    )
                    .fill(if can_clean { Color32::from_rgb(220, 38, 38) } else { Color32::from_rgb(100, 100, 100) })
                    .min_size(Vec2::new(180.0, 28.0));

                    if ui.add_enabled(can_clean, clean_btn).clicked() {
                        do_clean = true;
                    }
                });
            });
        });

    if do_close || !is_open {
        app.path_modal_open = false;
    }

    if do_rescan {
        app.path_entries = scan_path_entries();
        app.path_msg = None;
    }

    if do_clean {
        match clean_path_entries(&app.path_entries) {
            Ok((user_del, sys_del)) => {
                app.path_msg = Some(format!("Успешно очищено {} записей (Пользовательских: {}, Системных: {}). Переменные обновлены!", user_del + sys_del, user_del, sys_del));
                app.status_msg = format!("PATH очищен: удалено {} битых путей.", user_del + sys_del);
            }
            Err(err) => {
                app.path_msg = Some(err);
            }
        }
        app.path_entries = scan_path_entries();
    }
}

pub fn render_portable_modal(app: &mut PurgeCraftApp, ctx: &egui::Context) {
    if !app.portable_modal_open {
        return;
    }

    let mut is_open = true;
    let mut do_rescan = false;
    let mut do_close = false;
    let mut do_pick_folder = false;
    let mut app_to_delete: Option<PortableApp> = None;
    let mut app_to_launch: Option<String> = None;
    let mut app_to_open_folder: Option<String> = None;

    let total_portable_bytes: u64 = app.portable_apps.iter().map(|a| a.size_bytes).sum();

    egui::Window::new("📦 Сканер Portable-программ")
        .open(&mut is_open)
        .collapsible(false)
        .resizable(true)
        .pivot(egui::Align2::CENTER_CENTER)
        .default_pos(ctx.screen_rect().center())
        .show(ctx, |ui| {
            ui.set_width(740.0);
            ui.set_height(480.0);

            ui.heading(RichText::new("📦 Сканер Portable-софта").strong());
            ui.add_space(4.0);
            ui.label("Обнаруживает автономные программы и утилиты, которые запускаются без установки (на Рабочем столе, в Загрузках или в пользовательских папках).");
            ui.add_space(6.0);

            if let Some(msg) = &app.portable_msg {
                egui::Frame::none()
                    .fill(if app.dark_mode { Color32::from_rgb(20, 83, 45) } else { Color32::from_rgb(220, 252, 231) })
                    .rounding(4.0)
                    .inner_margin(8.0)
                    .show(ui, |ui| {
                        ui.label(RichText::new(msg).strong().color(Color32::from_rgb(22, 163, 74)));
                    });
                ui.add_space(6.0);
            }

            // Action buttons
            ui.horizontal(|ui| {
                if ui.button("📂 Добавить папку для сканирования...").clicked() {
                    do_pick_folder = true;
                }
                if ui.button("🔄 Пересканировать").clicked() {
                    do_rescan = true;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(format!("Найдено: {} программ ({})", app.portable_apps.len(), format_size(total_portable_bytes)));
                });
            });
            ui.add_space(6.0);

            let list_bg = if app.dark_mode { Color32::from_rgb(18, 18, 20) } else { Color32::WHITE };
            egui::Frame::none()
                .fill(list_bg)
                .stroke(Stroke::new(1.0_f32, if app.dark_mode { Color32::from_rgb(39, 39, 42) } else { Color32::from_rgb(226, 232, 240) }))
                .rounding(4.0)
                .inner_margin(8.0)
                .show(ui, |ui| {
                    egui::ScrollArea::vertical().max_height(270.0).show(ui, |ui| {
                        if app.portable_apps.is_empty() {
                            ui.vertical_centered(|ui| {
                                ui.add_space(30.0);
                                ui.label(RichText::new("Portable-программ в стандартных каталогах не найдено.").color(Color32::from_rgb(148, 163, 184)));
                            });
                        } else {
                            for p_app in &app.portable_apps {
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new("📦").size(18.0));
                                    ui.vertical(|ui| {
                                        ui.horizontal(|ui| {
                                            ui.label(RichText::new(&p_app.name).strong());
                                            ui.label(RichText::new(format!("({})", format_size(p_app.size_bytes))).color(Color32::from_rgb(34, 197, 94)));
                                            ui.label(RichText::new(format!("Файлов: {} | Изменен: {}", p_app.file_count, p_app.modified_date)).small().color(Color32::from_rgb(148, 163, 184)));
                                        });
                                        ui.label(RichText::new(&p_app.exe_path).small().color(Color32::from_rgb(113, 113, 122)));
                                    });

                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        if ui.button("🗑 Удалить").clicked() {
                                            app_to_delete = Some(p_app.clone());
                                        }
                                        if ui.button("📁 Папка").clicked() {
                                            app_to_open_folder = Some(p_app.folder_path.clone());
                                        }
                                        if ui.button("▶ Запустить").clicked() {
                                            app_to_launch = Some(p_app.exe_path.clone());
                                        }
                                    });
                                });
                                ui.separator();
                            }
                        }
                    });
                });

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(6.0);

            ui.horizontal(|ui| {
                ui.label(RichText::new("Portable-софт удаляется без следов в системных реестрах.").small());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Закрыть").clicked() {
                        do_close = true;
                    }
                });
            });
        });

    if do_close || !is_open {
        app.portable_modal_open = false;
    }

    if do_pick_folder {
        if let Some(folder) = pick_folder_dialog() {
            let scanned = scan_portable_apps(&[folder], &app.apps);
            app.portable_apps = scanned;
        }
    }

    if do_rescan {
        app.portable_apps = scan_portable_apps(&[], &app.apps);
        app.portable_msg = None;
    }

    if let Some(exe) = app_to_launch {
        let _ = open::that(&exe);
    }

    if let Some(folder) = app_to_open_folder {
        open_system_folder(&folder, app.settings.use_total_commander);
    }

    if let Some(del_app) = app_to_delete {
        match delete_portable_app(&del_app) {
            Ok(bytes) => {
                let id = format!("portable_{}", del_app.exe_path.to_lowercase());
                app.recently_uninstalled.insert(id);
                app.recently_uninstalled.insert(del_app.name.to_lowercase());
                app.apps.retain(|a| !app.recently_uninstalled.contains(&a.id.to_lowercase()) && !app.recently_uninstalled.contains(&a.name.to_lowercase()));
                app.portable_msg = Some(format!("Программа '{}' успешно удалена! Освобождено {}.", del_app.name, format_size(bytes)));
                app.status_msg = format!("Удалена portable-программа {}.", del_app.name);
                app.portable_apps = scan_portable_apps(&[], &app.apps);
            }
            Err(e) => {
                app.portable_msg = Some(format!("Ошибка удаления: {}", e));
            }
        }
    }
}

pub fn render_runtime_modal(app: &mut PurgeCraftApp, ctx: &egui::Context) {
    if !app.runtime_modal_open {
        return;
    }

    let mut is_open = true;
    let mut do_close = false;
    let mut runtime_to_uninstall: Option<String> = None;

    egui::Window::new("⚖ Анализатор дубликатов версий (Visual C++, Java, .NET)")
        .open(&mut is_open)
        .collapsible(false)
        .resizable(true)
        .pivot(egui::Align2::CENTER_CENTER)
        .default_pos(ctx.screen_rect().center())
        .show(ctx, |ui| {
            ui.set_width(760.0);
            ui.set_height(480.0);

            ui.heading(RichText::new("⚖ Анализатор дубликатов системных сред выполнения").strong());
            ui.add_space(4.0);
            ui.label("Игры и приложения часто устанавливают избыточные и устаревшие сборки Microsoft Visual C++, Java и .NET. Здесь вы можете найти дубликаты и безопасно удалить их.");
            ui.add_space(6.0);

            if let Some(msg) = &app.runtime_msg {
                egui::Frame::none()
                    .fill(if app.dark_mode { Color32::from_rgb(20, 83, 45) } else { Color32::from_rgb(220, 252, 231) })
                    .rounding(4.0)
                    .inner_margin(8.0)
                    .show(ui, |ui| {
                        ui.label(RichText::new(msg).strong().color(Color32::from_rgb(22, 163, 74)));
                    });
                ui.add_space(6.0);
            }

            let list_bg = if app.dark_mode { Color32::from_rgb(18, 18, 20) } else { Color32::WHITE };
            egui::Frame::none()
                .fill(list_bg)
                .stroke(Stroke::new(1.0_f32, if app.dark_mode { Color32::from_rgb(39, 39, 42) } else { Color32::from_rgb(226, 232, 240) }))
                .rounding(4.0)
                .inner_margin(8.0)
                .show(ui, |ui| {
                    egui::ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
                        if app.runtime_groups.is_empty() {
                            ui.vertical_centered(|ui| {
                                ui.add_space(30.0);
                                ui.label(RichText::new("Установленных сред выполнения не обнаружено.").color(Color32::from_rgb(148, 163, 184)));
                            });
                        } else {
                            for group in &app.runtime_groups {
                                ui.horizontal(|ui| {
                                    ui.heading(RichText::new(&group.family_name).size(15.0).strong());
                                    if group.duplicate_count > 0 {
                                        ui.label(RichText::new(format!("⚠ Дубликатов: {}", group.duplicate_count)).color(Color32::from_rgb(234, 179, 8)).strong());
                                    }
                                    if group.has_deprecated {
                                        ui.label(RichText::new("🛑 Есть устаревшие (EOL)").color(Color32::from_rgb(239, 68, 68)).strong());
                                    }
                                });
                                ui.add_space(2.0);

                                for item in &group.items {
                                    ui.horizontal(|ui| {
                                        ui.label(RichText::new(format!("[{}]", item.arch)).strong().color(Color32::from_rgb(59, 130, 246)));
                                        ui.label(RichText::new(&item.display_name).strong());

                                        if item.is_deprecated {
                                            egui::Frame::none()
                                                .fill(if app.dark_mode { Color32::from_rgb(69, 10, 10) } else { Color32::from_rgb(254, 226, 226) })
                                                .rounding(3.0)
                                                .inner_margin(egui::Margin::symmetric(4.0, 1.0))
                                                .show(ui, |ui| {
                                                    ui.label(RichText::new("УСТАРЕЛО (EOL)").small().strong().color(Color32::from_rgb(239, 68, 68)));
                                                });
                                        }

                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                            if !item.uninstall_string.is_empty() {
                                                if ui.button("🗑 Удалить").clicked() {
                                                    runtime_to_uninstall = Some(item.uninstall_string.clone());
                                                }
                                            }
                                            ui.label(RichText::new(format_size(item.size_bytes)).small().color(Color32::from_rgb(148, 163, 184)));
                                        });
                                    });
                                    ui.separator();
                                }
                                ui.add_space(8.0);
                            }
                        }
                    });
                });

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(6.0);

            ui.horizontal(|ui| {
                ui.label(RichText::new("Совет: Visual C++ 2015–2022 является единой универсальной библиотекой.").small());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Закрыть").clicked() {
                        do_close = true;
                    }
                });
            });
        });

    if do_close || !is_open {
        app.runtime_modal_open = false;
    }

    if let Some(uninst_str) = runtime_to_uninstall {
        let _ = Command::new("cmd").args(["/C", &uninst_str]).spawn();
        app.runtime_msg = Some("Запущен процесс деинсталляции выбранной среды выполнения.".to_string());
    }
}

pub fn render_winget_modal(app: &mut PurgeCraftApp, ctx: &egui::Context) {
    if !app.winget_modal_open {
        return;
    }

    let mut is_open = true;
    let mut do_rescan = false;
    let mut do_close = false;
    let mut pkg_to_upgrade: Option<String> = None;
    let mut pkg_to_uninstall: Option<String> = None;

    egui::Window::new("🚀 Менеджер пакетов и обновлений Winget")
        .open(&mut is_open)
        .collapsible(false)
        .resizable(true)
        .pivot(egui::Align2::CENTER_CENTER)
        .default_pos(ctx.screen_rect().center())
        .show(ctx, |ui| {
            ui.set_width(760.0);
            ui.set_height(480.0);

            ui.heading(RichText::new("🚀 Диспетчер пакетов Windows (Winget CLI)").strong());
            ui.add_space(4.0);
            ui.label("Интеграция с репозиторием Winget. Позволяет проверять наличие официальных обновлений установленных программ и производить их тихое удаление без открытия сторонних мастеров.");
            ui.add_space(6.0);

            if let Some(msg) = &app.winget_msg {
                egui::Frame::none()
                    .fill(if app.dark_mode { Color32::from_rgb(20, 83, 45) } else { Color32::from_rgb(220, 252, 231) })
                    .rounding(4.0)
                    .inner_margin(8.0)
                    .show(ui, |ui| {
                        ui.label(RichText::new(msg).strong().color(Color32::from_rgb(22, 163, 74)));
                    });
                ui.add_space(6.0);
            }

            // Top Action Bar
            ui.horizontal(|ui| {
                let refresh_btn = egui::Button::new("🔄 Обновить список пакетов Winget");
                if ui.add_enabled(!app.winget_is_busy, refresh_btn).clicked() {
                    do_rescan = true;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let updatable_count = app.winget_packages.iter().filter(|p| p.available_version.is_some()).count();
                    ui.label(RichText::new(format!("Всего: {} | Доступно обновлений: {}", app.winget_packages.len(), updatable_count)).strong());
                });
            });
            ui.add_space(6.0);

            if app.winget_is_busy {
                egui::Frame::none()
                    .fill(if app.dark_mode { Color32::from_rgb(30, 27, 75) } else { Color32::from_rgb(238, 242, 255) })
                    .rounding(4.0)
                    .stroke(Stroke::new(1.0_f32, if app.dark_mode { Color32::from_rgb(99, 102, 241) } else { Color32::from_rgb(165, 180, 252) }))
                    .inner_margin(8.0)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label(RichText::new(&app.winget_busy_msg).strong().color(Color32::from_rgb(99, 102, 241)));
                        });
                    });
                ui.add_space(6.0);
                ctx.request_repaint_after(std::time::Duration::from_millis(100));
            }

            let list_bg = if app.dark_mode { Color32::from_rgb(18, 18, 20) } else { Color32::WHITE };
            egui::Frame::none()
                .fill(list_bg)
                .stroke(Stroke::new(1.0_f32, if app.dark_mode { Color32::from_rgb(39, 39, 42) } else { Color32::from_rgb(226, 232, 240) }))
                .rounding(4.0)
                .inner_margin(8.0)
                .show(ui, |ui| {
                    egui::ScrollArea::vertical().max_height(280.0).show(ui, |ui| {
                        if app.winget_packages.is_empty() {
                            ui.vertical_centered(|ui| {
                                ui.add_space(30.0);
                                ui.label(RichText::new("Пакеты Winget загружаются или Winget недоступен на этом ПК.").color(Color32::from_rgb(148, 163, 184)));
                            });
                        } else {
                            for pkg in &app.winget_packages {
                                ui.horizontal(|ui| {
                                    ui.vertical(|ui| {
                                        ui.horizontal(|ui| {
                                            ui.label(RichText::new(&pkg.name).strong());
                                            ui.label(RichText::new(format!("v{}", pkg.installed_version)).small().color(Color32::from_rgb(148, 163, 184)));

                                            if let Some(avail) = &pkg.available_version {
                                                egui::Frame::none()
                                                    .fill(if app.dark_mode { Color32::from_rgb(20, 83, 45) } else { Color32::from_rgb(220, 252, 231) })
                                                    .rounding(3.0)
                                                    .inner_margin(egui::Margin::symmetric(4.0, 1.0))
                                                    .show(ui, |ui| {
                                                        ui.label(RichText::new(format!("⬆ Доступна v{}", avail)).small().strong().color(Color32::from_rgb(22, 163, 74)));
                                                    });
                                            }
                                        });
                                        ui.label(RichText::new(&pkg.id).small().color(Color32::from_rgb(113, 113, 122)));
                                    });

                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        let uninst_btn = egui::Button::new("🗑 Удалить (Winget)");
                                        if ui.add_enabled(!app.winget_is_busy, uninst_btn).clicked() {
                                            pkg_to_uninstall = Some(pkg.id.clone());
                                        }
                                        if pkg.available_version.is_some() {
                                            let up_btn = egui::Button::new(RichText::new("⬆ Обновить").color(Color32::WHITE).strong())
                                                .fill(Color32::from_rgb(22, 163, 74));
                                            if ui.add_enabled(!app.winget_is_busy, up_btn).clicked() {
                                                pkg_to_upgrade = Some(pkg.id.clone());
                                            }
                                        }
                                    });
                                });
                                ui.separator();
                            }
                        }
                    });
                });

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(6.0);

            ui.horizontal(|ui| {
                ui.label(RichText::new("Winget выполняет установку и удаление из проверенных манифестов.").small());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Закрыть").clicked() {
                        do_close = true;
                    }
                });
            });
        });

    if do_close || !is_open {
        app.winget_modal_open = false;
    }

    if do_rescan && !app.winget_is_busy {
        app.start_winget_scan();
    }

    if let Some(id) = pkg_to_upgrade {
        if !app.winget_is_busy {
            app.winget_is_busy = true;
            app.winget_busy_msg = format!("Обновление пакета {} через Winget (загрузка и установка)...", id);
            app.winget_msg = Some(format!("Запущен процесс обновления пакета {}...", id));
            let (tx, rx) = std::sync::mpsc::channel();
            app.winget_task_rx = Some(rx);
            let id_clone = id.clone();
            std::thread::spawn(move || {
                let res = upgrade_winget_package(&id_clone);
                let pkgs = scan_winget_packages();
                let _ = tx.send((res, Some(pkgs)));
            });
        }
    }

    if let Some(id) = pkg_to_uninstall {
        if !app.winget_is_busy {
            app.winget_is_busy = true;
            app.winget_busy_msg = format!("Удаление пакета {} через Winget...", id);
            app.winget_msg = Some(format!("Запущен процесс удаления пакета {}...", id));
            let (tx, rx) = std::sync::mpsc::channel();
            app.winget_task_rx = Some(rx);
            let id_clone = id.clone();
            std::thread::spawn(move || {
                let res = uninstall_winget_package(&id_clone);
                let pkgs = scan_winget_packages();
                let _ = tx.send((res, Some(pkgs)));
            });
        }
    }
}
