use std::process::Command;
use eframe::egui::{self, Color32, RichText, Vec2, Stroke};
use crate::app::PurgeCraftApp;
use crate::types::{format_size_lang, open_system_folder};
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

    let lang = app.lang();
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

    let is_detached = app.detached_modals.contains("modal_shader");
    let outcome = super::show_detachable_window(
        ctx,
        "modal_shader",
        crate::i18n::t(lang, "Очиститель кэша шейдеров GPU", "GPU Shader Cache Cleaner"),
        None,
        egui::vec2(640.0, 440.0),
        egui::vec2(380.0, 280.0),
        true,
        is_detached,
        app.dark_mode,
        lang,
        |ui, _active_ctx| {
            ui.label(crate::i18n::t(
                lang,
                "Игры и графические приложения компилируют шейдеры в системные папки драйверов (DirectX, NVIDIA, AMD). Со временем эти кэши разрастаются до десятков гигабайтов и не удаляются автоматически.",
                "Games and graphics applications compile shaders into driver system directories (DirectX, NVIDIA, AMD). Over time, these caches grow to tens of gigabytes and are not automatically removed."
            ));
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
            ui.horizontal_wrapped(|ui| {
                if ui.button(crate::i18n::t(lang, "Выбрать все", "Select All")).clicked() {
                    for loc in &mut app.shader_locations {
                        loc.selected = true;
                    }
                }
                if ui.button(crate::i18n::t(lang, "Снять выбор", "Deselect All")).clicked() {
                    for loc in &mut app.shader_locations {
                        loc.selected = false;
                    }
                }
                if ui.button(crate::i18n::t(lang, "Пересканировать", "Rescan")).clicked() {
                    do_rescan = true;
                }
            });
            ui.add_space(6.0);

            // List of locations (stretches smoothly when window is resized)
            let list_bg = if app.dark_mode { Color32::from_rgb(18, 18, 20) } else { Color32::WHITE };
            egui::Frame::none()
                .fill(list_bg)
                .stroke(Stroke::new(1.0_f32, if app.dark_mode { Color32::from_rgb(39, 39, 42) } else { Color32::from_rgb(226, 232, 240) }))
                .rounding(4.0)
                .inner_margin(8.0)
                .show(ui, |ui| {
                    let list_h = super::fill_scroll_height(ui, 58.0, 90.0);
                    egui::ScrollArea::vertical().max_height(list_h).auto_shrink([false, false]).show(ui, |ui| {
                        if app.shader_locations.is_empty() {
                            ui.vertical_centered(|ui| {
                                ui.add_space(20.0);
                                ui.label(RichText::new(crate::i18n::t(lang, "Кэшей шейдеров не обнаружено или они уже пусты.", "No shader caches detected or they are already empty.")).color(Color32::from_rgb(148, 163, 184)));
                            });
                        } else {
                            for loc in &mut app.shader_locations {
                                super::split_row(
                                    ui,
                                    |ui| {
                                        ui.label(RichText::new(format_size_lang(loc.size_bytes, lang)).strong());
                                        ui.label(format!("({} {})", loc.file_count, crate::i18n::t(lang, "файлов", "files")));
                                    },
                                    |ui| {
                                        ui.horizontal(|ui| {
                                            ui.checkbox(&mut loc.selected, "");
                                            let vendor_color = match loc.vendor.as_str() {
                                                "NVIDIA" => Color32::from_rgb(34, 197, 94),
                                                "AMD" => Color32::from_rgb(239, 68, 68),
                                                _ => Color32::from_rgb(59, 130, 246),
                                            };
                                            ui.label(RichText::new(format!("[{}]", loc.vendor)).strong().color(vendor_color));
                                            ui.add(egui::Label::new(RichText::new(&loc.name).strong()).truncate())
                                                .on_hover_text(&loc.name);
                                        });
                                    },
                                );
                                ui.add(egui::Label::new(RichText::new(&loc.path).small().color(Color32::from_rgb(113, 113, 122))).truncate())
                                    .on_hover_text(&loc.path);
                                ui.separator();
                            }
                        }
                    });
                });

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(6.0);

            // Bottom action panel
            ui.horizontal(|ui| {
                let info_text = format!(
                    "{} {} ({} {})",
                    crate::i18n::t(lang, "Выбрано для очистки:", "Selected for cleanup:"),
                    format_size_lang(total_selected_bytes, lang),
                    total_selected_files,
                    crate::i18n::t(lang, "файлов", "files")
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(crate::i18n::t(lang, "Закрыть", "Close")).clicked() {
                        do_close = true;
                    }
                    let can_clean = total_selected_bytes > 0 && !app.shader_cleaning;
                    let clean_btn = egui::Button::new(
                        RichText::new(format!("{} ({})", crate::i18n::t(lang, "Очистить", "Clean"), format_size_lang(total_selected_bytes, lang)))
                            .strong()
                            .color(Color32::WHITE)
                    )
                    .fill(if can_clean { Color32::from_rgb(147, 51, 234) } else { Color32::from_rgb(100, 100, 100) })
                    .min_size(Vec2::new(140.0, 28.0));

                    if ui.add_enabled(can_clean, clean_btn).clicked() {
                        do_clean = true;
                    }
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        ui.add(egui::Label::new(RichText::new(info_text).strong().size(13.0)).truncate());
                    });
                });
            });
        },
    );

    if do_close {
        is_open = false;
    }
    super::sync_detachable_state("modal_shader", &mut app.detached_modals, &mut app.shader_modal_open, outcome);
    if !is_open {
        app.shader_modal_open = false;
        app.detached_modals.remove("modal_shader");
    }

    if do_rescan {
        app.shader_locations = scan_shader_caches();
        app.shader_clean_msg = None;
    }

    if do_clean {
        let (deleted_files, deleted_bytes) = clear_shader_cache_targets(&app.shader_locations);
        app.shader_clean_msg = Some(format!(
            "{} {} ({} {}).",
            crate::i18n::t(lang, "Очистка завершена! Освобождено", "Cleanup finished! Freed"),
            format_size_lang(deleted_bytes, lang),
            deleted_files,
            crate::i18n::t(lang, "файлов удалено", "files deleted")
        ));
        app.status_msg = format!(
            "{} {}.",
            crate::i18n::t(lang, "Кэш шейдеров очищен. Освобождено", "Shader cache cleaned. Freed"),
            format_size_lang(deleted_bytes, lang)
        );
        app.shader_locations = scan_shader_caches();
    }
}

pub fn render_path_modal(app: &mut PurgeCraftApp, ctx: &egui::Context) {
    if !app.path_modal_open {
        return;
    }

    let lang = app.lang();
    let mut is_open = true;
    let mut do_clean = false;
    let mut do_rescan = false;
    let mut do_close = false;

    let total_selected = app.path_entries.iter().filter(|e| e.selected).count();
    let total_broken = app.path_entries.iter().filter(|e| !e.exists).count();
    let total_duplicates = app.path_entries.iter().filter(|e| e.is_duplicate).count();

    let is_detached = app.detached_modals.contains("modal_path");
    let outcome = super::show_detachable_window(
        ctx,
        "modal_path",
        crate::i18n::t(lang, "Санитайзер переменных PATH", "PATH Environment Variables Sanitizer"),
        None,
        egui::vec2(780.0, 460.0),
        egui::vec2(400.0, 280.0),
        true,
        is_detached,
        app.dark_mode,
        lang,
        |ui, _active_ctx| {
            ui.label(crate::i18n::t(
                lang,
                "Удаленные приложения часто оставляют «мертвые» ссылки на несуществующие каталоги в переменной PATH. Это замедляет работу командной строки и может вызывать конфликты библиотек.",
                "Uninstalled apps often leave dead links to missing folders in the PATH variable. This slows down CLI lookups and can cause DLL conflicts."
            ));
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

            // Action buttons (wraps cleanly on compact windows)
            ui.horizontal_wrapped(|ui| {
                if ui.button(crate::i18n::t(lang, "Битые и дубликаты", "Broken & Duplicates")).clicked() {
                    for e in &mut app.path_entries {
                        e.selected = !e.exists || e.is_duplicate;
                    }
                }
                if ui.button(crate::i18n::t(lang, "Все", "Select All")).clicked() {
                    for e in &mut app.path_entries {
                        e.selected = true;
                    }
                }
                if ui.button(crate::i18n::t(lang, "Снять", "Deselect All")).clicked() {
                    for e in &mut app.path_entries {
                        e.selected = false;
                    }
                }
                let stats = format!(
                    "{}: {}, {}: {}",
                    crate::i18n::t(lang, "Битых", "Broken"),
                    total_broken,
                    crate::i18n::t(lang, "Дубликатов", "Duplicates"),
                    total_duplicates
                );
                ui.label(RichText::new(stats).color(Color32::from_rgb(148, 163, 184)));
                if ui.button(crate::i18n::t(lang, "Обновить", "Refresh")).clicked() {
                    do_rescan = true;
                }
            });
            ui.add_space(6.0);

            let list_bg = if app.dark_mode { Color32::from_rgb(18, 18, 20) } else { Color32::WHITE };
            egui::Frame::none()
                .fill(list_bg)
                .stroke(Stroke::new(1.0_f32, if app.dark_mode { Color32::from_rgb(39, 39, 42) } else { Color32::from_rgb(226, 232, 240) }))
                .rounding(4.0)
                .inner_margin(8.0)
                .show(ui, |ui| {
                    let list_h = super::fill_scroll_height(ui, 56.0, 90.0);
                    egui::ScrollArea::vertical().max_height(list_h).auto_shrink([false, false]).show(ui, |ui| {
                        if app.path_entries.is_empty() {
                            ui.vertical_centered(|ui| {
                                ui.add_space(20.0);
                                ui.label(RichText::new(crate::i18n::t(lang, "Записей в PATH не обнаружено.", "No entries found in PATH.")).color(Color32::from_rgb(148, 163, 184)));
                            });
                        } else {
                            for entry in &mut app.path_entries {
                                ui.horizontal(|ui| {
                                    ui.checkbox(&mut entry.selected, "");

                                    // Hive badge
                                    let hive_label = if entry.is_system {
                                        crate::i18n::t(lang, "[Система]", "[System]")
                                    } else {
                                        crate::i18n::t(lang, "[Пользователь]", "[User]")
                                    };
                                    let hive_color = if entry.is_system { Color32::from_rgb(168, 85, 247) } else { Color32::from_rgb(59, 130, 246) };
                                    ui.label(RichText::new(hive_label).small().strong().color(hive_color));

                                    // Status badge
                                    if !entry.exists {
                                        egui::Frame::none()
                                            .fill(if app.dark_mode { Color32::from_rgb(69, 10, 10) } else { Color32::from_rgb(254, 226, 226) })
                                            .rounding(3.0)
                                            .inner_margin(egui::Margin::symmetric(6.0, 2.0))
                                            .show(ui, |ui| {
                                                ui.label(RichText::new(crate::i18n::t(lang, "Не существует", "Missing")).small().strong().color(Color32::from_rgb(239, 68, 68)));
                                            });
                                    } else if entry.is_duplicate {
                                        egui::Frame::none()
                                            .fill(if app.dark_mode { Color32::from_rgb(66, 32, 6) } else { Color32::from_rgb(254, 243, 199) })
                                            .rounding(3.0)
                                            .inner_margin(egui::Margin::symmetric(6.0, 2.0))
                                            .show(ui, |ui| {
                                                ui.label(RichText::new(crate::i18n::t(lang, "Дубликат", "Duplicate")).small().strong().color(Color32::from_rgb(217, 119, 6)));
                                            });
                                    } else {
                                        ui.label(RichText::new(crate::i18n::t(lang, "Существует", "Exists")).small().color(Color32::from_rgb(34, 197, 94)));
                                    }

                                    ui.add(egui::Label::new(RichText::new(&entry.path).monospace().size(12.0)).truncate())
                                        .on_hover_text(&entry.path);
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
                let count_text = format!(
                    "{} {} {}",
                    crate::i18n::t(lang, "Выбрано для удаления:", "Selected for removal:"),
                    total_selected,
                    crate::i18n::t(lang, "записей", "entries")
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(crate::i18n::t(lang, "Закрыть", "Close")).clicked() {
                        do_close = true;
                    }
                    let can_clean = total_selected > 0;
                    let clean_label = format!("{} ({})", crate::i18n::t(lang, "Очистить выбранные", "Clean Selected"), total_selected);
                    let clean_btn = egui::Button::new(
                        RichText::new(clean_label)
                            .strong()
                            .color(Color32::WHITE)
                    )
                    .fill(if can_clean { Color32::from_rgb(220, 38, 38) } else { Color32::from_rgb(100, 100, 100) })
                    .min_size(Vec2::new(165.0, 28.0));

                    if ui.add_enabled(can_clean, clean_btn).clicked() {
                        do_clean = true;
                    }
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        ui.add(egui::Label::new(RichText::new(count_text).strong()).truncate());
                    });
                });
            });
        },
    );

    if do_close {
        is_open = false;
    }
    super::sync_detachable_state("modal_path", &mut app.detached_modals, &mut app.path_modal_open, outcome);
    if !is_open {
        app.path_modal_open = false;
        app.detached_modals.remove("modal_path");
    }

    if do_rescan {
        app.path_entries = scan_path_entries();
        app.path_msg = None;
    }

    if do_clean {
        match clean_path_entries(&app.path_entries) {
            Ok((user_del, sys_del)) => {
                app.path_msg = Some(format!(
                    "{} {} ({} {}, {} {}). {}",
                    crate::i18n::t(lang, "Успешно очищено", "Successfully cleaned"),
                    user_del + sys_del,
                    crate::i18n::t(lang, "Пользовательских:", "User:"),
                    user_del,
                    crate::i18n::t(lang, "Системных:", "System:"),
                    sys_del,
                    crate::i18n::t(lang, "Переменные обновлены!", "Variables updated!")
                ));
                app.status_msg = format!(
                    "{} {} {}.",
                    crate::i18n::t(lang, "PATH очищен: удалено", "PATH cleaned: deleted"),
                    user_del + sys_del,
                    crate::i18n::t(lang, "битых путей", "broken paths")
                );
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

    let lang = app.lang();
    let mut is_open = true;
    let mut do_rescan = false;
    let mut do_close = false;
    let mut do_pick_folder = false;
    let mut app_to_delete: Option<PortableApp> = None;
    let mut app_to_launch: Option<String> = None;
    let mut app_to_open_folder: Option<String> = None;

    let total_portable_bytes: u64 = app.portable_apps.iter().map(|a| a.size_bytes).sum();

    let is_detached = app.detached_modals.contains("modal_portable");
    let outcome = super::show_detachable_window(
        ctx,
        "modal_portable",
        crate::i18n::t(lang, "Сканер Portable-программ", "Portable Apps Scanner"),
        None,
        egui::vec2(740.0, 460.0),
        egui::vec2(400.0, 280.0),
        true,
        is_detached,
        app.dark_mode,
        lang,
        |ui, _active_ctx| {
            ui.label(crate::i18n::t(
                lang,
                "Обнаруживает автономные программы и утилиты, которые запускаются без установки (на Рабочем столе, в Загрузках или в пользовательских папках).",
                "Detects standalone apps and utilities running without installation (on Desktop, Downloads, or user folders)."
            ));
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
            ui.horizontal_wrapped(|ui| {
                if ui.button(crate::i18n::t(lang, "Добавить папку для сканирования...", "Add folder to scan...")).clicked() {
                    do_pick_folder = true;
                }
                if ui.button(crate::i18n::t(lang, "Пересканировать", "Rescan")).clicked() {
                    do_rescan = true;
                }
                let found_text = format!(
                    "{}: {} {} ({})",
                    crate::i18n::t(lang, "Найдено:", "Found:"),
                    app.portable_apps.len(),
                    crate::i18n::t(lang, "программ", "apps"),
                    format_size_lang(total_portable_bytes, lang)
                );
                ui.label(found_text);
            });
            ui.add_space(6.0);

            let list_bg = if app.dark_mode { Color32::from_rgb(18, 18, 20) } else { Color32::WHITE };
            egui::Frame::none()
                .fill(list_bg)
                .stroke(Stroke::new(1.0_f32, if app.dark_mode { Color32::from_rgb(39, 39, 42) } else { Color32::from_rgb(226, 232, 240) }))
                .rounding(4.0)
                .inner_margin(8.0)
                .show(ui, |ui| {
                    let list_h = super::fill_scroll_height(ui, 54.0, 90.0);
                    egui::ScrollArea::vertical().max_height(list_h).auto_shrink([false, false]).show(ui, |ui| {
                        if app.portable_apps.is_empty() {
                            ui.vertical_centered(|ui| {
                                ui.add_space(20.0);
                                ui.label(RichText::new(crate::i18n::t(lang, "Portable-программ в стандартных каталогах не найдено.", "No portable applications found in standard directories.")).color(Color32::from_rgb(148, 163, 184)));
                            });
                        } else {
                            for p_app in &app.portable_apps {
                                super::split_row(
                                    ui,
                                    |ui| {
                                        if ui.button(crate::i18n::t(lang, "Удалить", "Delete")).clicked() {
                                            app_to_delete = Some(p_app.clone());
                                        }
                                        if ui.button(crate::i18n::t(lang, "Папка", "Folder")).clicked() {
                                            app_to_open_folder = Some(p_app.folder_path.clone());
                                        }
                                        if ui.button(crate::i18n::t(lang, "Запустить", "Launch")).clicked() {
                                            app_to_launch = Some(p_app.exe_path.clone());
                                        }
                                    },
                                    |ui| {
                                        ui.horizontal(|ui| {
                                            ui.vertical(|ui| {
                                                ui.add(egui::Label::new(RichText::new(&p_app.name).strong()).truncate())
                                                    .on_hover_text(&p_app.name);
                                                ui.horizontal_wrapped(|ui| {
                                                    ui.label(RichText::new(format!("({})", format_size_lang(p_app.size_bytes, lang))).small().color(Color32::from_rgb(34, 197, 94)));
                                                    let meta_text = format!(
                                                        "{} {} | {} {}",
                                                        crate::i18n::t(lang, "Файлов:", "Files:"),
                                                        p_app.file_count,
                                                        crate::i18n::t(lang, "Изменен:", "Modified:"),
                                                        p_app.modified_date
                                                    );
                                                    ui.label(RichText::new(meta_text).small().color(Color32::from_rgb(148, 163, 184)));
                                                });
                                                ui.add(egui::Label::new(RichText::new(&p_app.exe_path).small().color(Color32::from_rgb(113, 113, 122))).truncate())
                                                    .on_hover_text(&p_app.exe_path);
                                            });
                                        });
                                    },
                                );
                                ui.separator();
                            }
                        }
                    });
                });

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(6.0);

            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(crate::i18n::t(lang, "Закрыть", "Close")).clicked() {
                        do_close = true;
                    }
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        ui.add(egui::Label::new(RichText::new(crate::i18n::t(lang, "Portable-софт удаляется без следов в системных реестрах.", "Portable apps are removed without registry traces.")).small()).truncate());
                    });
                });
            });
        },
    );

    if do_close {
        is_open = false;
    }
    super::sync_detachable_state("modal_portable", &mut app.detached_modals, &mut app.portable_modal_open, outcome);
    if !is_open {
        app.portable_modal_open = false;
        app.detached_modals.remove("modal_portable");
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
                app.bump_apps_generation();
                app.portable_msg = Some(format!(
                    "{} '{}' {} {}.",
                    crate::i18n::t(lang, "Программа", "Application"),
                    del_app.name,
                    crate::i18n::t(lang, "успешно удалена! Освобождено", "successfully deleted! Freed"),
                    format_size_lang(bytes, lang)
                ));
                app.status_msg = format!(
                    "{} {}.",
                    crate::i18n::t(lang, "Удалена portable-программа", "Portable app deleted:"),
                    del_app.name
                );
                app.portable_apps = scan_portable_apps(&[], &app.apps);
            }
            Err(e) => {
                app.portable_msg = Some(format!("{}: {}", crate::i18n::t(lang, "Ошибка удаления", "Deletion error"), e));
            }
        }
    }
}

pub fn render_runtime_modal(app: &mut PurgeCraftApp, ctx: &egui::Context) {
    if !app.runtime_modal_open {
        return;
    }

    let lang = app.lang();
    let mut is_open = true;
    let mut do_close = false;
    let mut runtime_to_uninstall: Option<String> = None;

    let is_detached = app.detached_modals.contains("modal_runtime");
    let outcome = super::show_detachable_window(
        ctx,
        "modal_runtime",
        crate::i18n::t(lang, "Анализатор дубликатов версий (Visual C++, Java, .NET)", "Runtime Duplicate Analyzer (Visual C++, Java, .NET)"),
        None,
        egui::vec2(760.0, 460.0),
        egui::vec2(400.0, 280.0),
        true,
        is_detached,
        app.dark_mode,
        lang,
        |ui, _active_ctx| {
            ui.label(crate::i18n::t(
                lang,
                "Игры и приложения часто устанавливают избыточные и устаревшие сборки Microsoft Visual C++, Java и .NET. Здесь вы можете найти дубликаты и безопасно удалить их.",
                "Games and apps frequently install duplicate or outdated versions of MS Visual C++, Java, and .NET. You can safely detect and uninstall them here."
            ));
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
                    let list_h = super::fill_scroll_height(ui, 54.0, 90.0);
                    egui::ScrollArea::vertical().max_height(list_h).auto_shrink([false, false]).show(ui, |ui| {
                        if app.runtime_groups.is_empty() {
                            ui.vertical_centered(|ui| {
                                ui.add_space(20.0);
                                ui.label(RichText::new(crate::i18n::t(lang, "Установленных сред выполнения не обнаружено.", "No installed runtimes found.")).color(Color32::from_rgb(148, 163, 184)));
                            });
                        } else {
                            for group in &app.runtime_groups {
                                ui.horizontal_wrapped(|ui| {
                                    ui.heading(RichText::new(&group.family_name).size(15.0).strong());
                                    if group.duplicate_count > 0 {
                                        let dup_label = format!("{}: {}", crate::i18n::t(lang, "Дубликатов", "Duplicates"), group.duplicate_count);
                                        ui.label(RichText::new(dup_label).color(Color32::from_rgb(234, 179, 8)).strong());
                                    }
                                    if group.has_deprecated {
                                        ui.label(RichText::new(crate::i18n::t(lang, "Есть устаревшие (EOL)", "Outdated versions present (EOL)")).color(Color32::from_rgb(239, 68, 68)).strong());
                                    }
                                });
                                ui.add_space(2.0);

                                for item in &group.items {
                                    super::split_row(
                                        ui,
                                        |ui| {
                                            if !item.uninstall_string.is_empty()
                                                && ui.button(crate::i18n::t(lang, "Удалить", "Uninstall")).clicked()
                                            {
                                                runtime_to_uninstall = Some(item.uninstall_string.clone());
                                            }
                                            ui.label(RichText::new(format_size_lang(item.size_bytes, lang)).small().color(Color32::from_rgb(148, 163, 184)));
                                        },
                                        |ui| {
                                            ui.horizontal(|ui| {
                                                ui.label(RichText::new(format!("[{}]", item.arch)).strong().color(Color32::from_rgb(59, 130, 246)));

                                                if item.is_deprecated {
                                                    egui::Frame::none()
                                                        .fill(if app.dark_mode { Color32::from_rgb(69, 10, 10) } else { Color32::from_rgb(254, 226, 226) })
                                                        .rounding(3.0)
                                                        .inner_margin(egui::Margin::symmetric(6.0, 2.0))
                                                        .show(ui, |ui| {
                                                            ui.label(RichText::new(crate::i18n::t(lang, "Устарело (EOL)", "Deprecated (EOL)")).small().strong().color(Color32::from_rgb(239, 68, 68)));
                                                        });
                                                }

                                                ui.add(egui::Label::new(RichText::new(&item.display_name).strong()).truncate())
                                                    .on_hover_text(&item.display_name);
                                            });
                                        },
                                    );
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
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(crate::i18n::t(lang, "Закрыть", "Close")).clicked() {
                        do_close = true;
                    }
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        ui.add(egui::Label::new(RichText::new(crate::i18n::t(lang, "Совет: Visual C++ 2015–2022 является единой универсальной библиотекой.", "Tip: Visual C++ 2015-2022 is a unified binary-compatible runtime.")).small()).truncate());
                    });
                });
            });
        },
    );

    if do_close {
        is_open = false;
    }
    super::sync_detachable_state("modal_runtime", &mut app.detached_modals, &mut app.runtime_modal_open, outcome);
    if !is_open {
        app.runtime_modal_open = false;
        app.detached_modals.remove("modal_runtime");
    }

    if let Some(uninst_str) = runtime_to_uninstall {
        let _ = Command::new("cmd").args(["/C", &uninst_str]).spawn();
        app.runtime_msg = Some(crate::i18n::t(lang, "Запущен процесс деинсталляции выбранной среды выполнения.", "Uninstall process started for selected runtime.").to_string());
    }
}

pub fn render_winget_modal(app: &mut PurgeCraftApp, ctx: &egui::Context) {
    if !app.winget_modal_open {
        return;
    }

    let lang = app.lang();
    let mut is_open = true;
    let mut do_rescan = false;
    let mut do_close = false;
    let mut pkg_to_upgrade: Option<String> = None;
    let mut pkg_to_uninstall: Option<String> = None;

    let is_detached = app.detached_modals.contains("modal_winget");
    let outcome = super::show_detachable_window(
        ctx,
        "modal_winget",
        crate::i18n::t(lang, "Менеджер пакетов Winget", "Winget Package Manager"),
        None,
        egui::vec2(760.0, 480.0),
        egui::vec2(420.0, 300.0),
        true,
        is_detached,
        app.dark_mode,
        lang,
        |ui, active_ctx| {
            ui.label(crate::i18n::t(
                lang,
                "Интеграция с репозиторием Winget. Проверка официальных обновлений установленных программ и их тихое удаление.",
                "Integration with Winget repository. Check official updates for installed software and perform silent removal."
            ));
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
            ui.horizontal_wrapped(|ui| {
                let refresh_btn = egui::Button::new(crate::i18n::t(lang, "Обновить список", "Refresh list"));
                if ui.add_enabled(!app.winget_is_busy, refresh_btn).clicked() {
                    do_rescan = true;
                }
                let updatable_count = app.winget_packages.iter().filter(|p| p.available_version.is_some()).count();
                let stats_text = format!(
                    "{}: {} | {}: {}",
                    crate::i18n::t(lang, "Всего", "Total"),
                    app.winget_packages.len(),
                    crate::i18n::t(lang, "Доступно обновлений", "Updates available"),
                    updatable_count
                );
                ui.label(RichText::new(stats_text).strong());
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
                active_ctx.request_repaint_after(std::time::Duration::from_millis(100));
            }

            let list_bg = if app.dark_mode { Color32::from_rgb(18, 18, 20) } else { Color32::WHITE };
            egui::Frame::none()
                .fill(list_bg)
                .stroke(Stroke::new(1.0_f32, if app.dark_mode { Color32::from_rgb(39, 39, 42) } else { Color32::from_rgb(226, 232, 240) }))
                .rounding(4.0)
                .inner_margin(8.0)
                .show(ui, |ui| {
                    let list_h = super::fill_scroll_height(ui, 54.0, 90.0);
                    egui::ScrollArea::vertical().max_height(list_h).auto_shrink([false, false]).show(ui, |ui| {
                        if app.winget_packages.is_empty() {
                            ui.vertical_centered(|ui| {
                                ui.add_space(20.0);
                                ui.label(RichText::new(crate::i18n::t(lang, "Пакеты Winget загружаются или Winget недоступен на этом ПК.", "Winget packages are loading or Winget is unavailable on this PC.")).color(Color32::from_rgb(148, 163, 184)));
                            });
                        } else {
                            for pkg in &app.winget_packages {
                                super::split_row(
                                    ui,
                                    |ui| {
                                        let uninst_btn = egui::Button::new(crate::i18n::t(lang, "Удалить (Winget)", "Uninstall (Winget)"));
                                        if ui.add_enabled(!app.winget_is_busy, uninst_btn).clicked() {
                                            pkg_to_uninstall = Some(pkg.id.clone());
                                        }
                                        if pkg.available_version.is_some() {
                                            let up_btn = egui::Button::new(RichText::new(crate::i18n::t(lang, "Обновить", "Upgrade")).color(Color32::WHITE).strong())
                                                .fill(Color32::from_rgb(22, 163, 74));
                                            if ui.add_enabled(!app.winget_is_busy, up_btn).clicked() {
                                                pkg_to_upgrade = Some(pkg.id.clone());
                                            }
                                        }
                                    },
                                    |ui| {
                                        ui.add(egui::Label::new(RichText::new(&pkg.name).strong()).truncate())
                                            .on_hover_text(&pkg.name);
                                        ui.horizontal_wrapped(|ui| {
                                            ui.label(RichText::new(&pkg.id).small().color(Color32::from_rgb(113, 113, 122)));
                                            ui.label(RichText::new(format!("• v{}", pkg.installed_version)).small().color(Color32::from_rgb(148, 163, 184)));
                                            if let Some(avail) = &pkg.available_version {
                                                egui::Frame::none()
                                                    .fill(if app.dark_mode { Color32::from_rgb(20, 83, 45) } else { Color32::from_rgb(220, 252, 231) })
                                                    .rounding(3.0)
                                                    .inner_margin(egui::Margin::symmetric(6.0, 2.0))
                                                    .show(ui, |ui| {
                                                        let avail_text = format!("{} v{}", crate::i18n::t(lang, "Доступна", "Available"), avail);
                                                        ui.label(RichText::new(avail_text).small().strong().color(Color32::from_rgb(22, 163, 74)));
                                                    });
                                            }
                                        });
                                    },
                                );
                                ui.separator();
                            }
                        }
                    });
                });

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(6.0);

            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(crate::i18n::t(lang, "Закрыть", "Close")).clicked() {
                        do_close = true;
                    }
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        ui.add(egui::Label::new(RichText::new(crate::i18n::t(lang, "Winget выполняет установку и удаление из проверенных манифестов.", "Winget installs and uninstalls packages from verified manifests.")).small()).truncate());
                    });
                });
            });
        },
    );

    if do_close {
        is_open = false;
    }
    super::sync_detachable_state("modal_winget", &mut app.detached_modals, &mut app.winget_modal_open, outcome);
    if !is_open {
        app.winget_modal_open = false;
        app.detached_modals.remove("modal_winget");
    }

    if do_rescan && !app.winget_is_busy {
        app.start_winget_scan();
    }

    if let Some(id) = pkg_to_upgrade {
        if !app.winget_is_busy {
            app.winget_is_busy = true;
            app.winget_busy_msg = format!(
                "{} {} ({})...",
                crate::i18n::t(lang, "Обновление пакета", "Upgrading package"),
                id,
                crate::i18n::t(lang, "загрузка и установка", "downloading & installing")
            );
            app.winget_msg = Some(format!(
                "{} {}...",
                crate::i18n::t(lang, "Запущен процесс обновления пакета", "Started upgrade process for package"),
                id
            ));
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
            app.winget_busy_msg = format!(
                "{} {}...",
                crate::i18n::t(lang, "Удаление пакета", "Uninstalling package"),
                id
            );
            app.winget_msg = Some(format!(
                "{} {}...",
                crate::i18n::t(lang, "Запущен процесс удаления пакета", "Started removal process for package"),
                id
            ));
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
