use eframe::egui::{self, Color32, RichText, Stroke};
use crate::app::PurgeCraftApp;
use crate::types::{format_size, jump_to_regedit, open_system_folder};
use crate::cleaner::{
    scan_windows_services, set_service_startup, delete_windows_service,
    ServiceFilter, ServiceStartupType,
};

// ============================================================================
// 1. System Deep Clean Modal
// ============================================================================

pub fn render_system_clean_modal(app: &mut PurgeCraftApp, ctx: &egui::Context) {
    if !app.system_clean_modal_open {
        return;
    }

    let mut is_open = true;
    let mut do_clean = false;
    let mut do_rescan = false;
    let mut do_close = false;

    let total_selected_bytes: u64 = app.system_clean_items
        .iter()
        .filter(|i| i.selected)
        .map(|i| i.size_bytes)
        .sum();
    let total_selected_files: usize = app.system_clean_items
        .iter()
        .filter(|i| i.selected)
        .map(|i| i.file_count)
        .sum();

    egui::Window::new("♻ Глубокая очистка системы Windows")
        .open(&mut is_open)
        .collapsible(false)
        .resizable(true)
        .pivot(egui::Align2::CENTER_CENTER)
        .default_pos(ctx.screen_rect().center())
        .show(ctx, |ui| {
            ui.set_width(720.0);
            ui.set_height(480.0);

            ui.horizontal(|ui| {
                ui.heading(RichText::new("♻ Очистка системного мусора и кэшей").strong());
                if app.system_clean_is_busy {
                    ui.add_space(8.0);
                    ui.spinner();
                    ui.label(RichText::new(&app.system_clean_busy_msg).color(Color32::from_rgb(59, 130, 246)));
                }
            });
            ui.add_space(4.0);
            ui.label("Безопасное удаление устаревших загрузок Windows Update, дампов падений WER, миниатюр Проводника и системных временных файлов.");
            ui.add_space(8.0);

            if let Some(msg) = &app.system_clean_msg {
                egui::Frame::none()
                    .fill(if app.dark_mode { Color32::from_rgb(20, 83, 45) } else { Color32::from_rgb(220, 252, 231) })
                    .rounding(4.0)
                    .inner_margin(8.0)
                    .show(ui, |ui| {
                        ui.label(RichText::new(msg).strong().color(Color32::from_rgb(22, 163, 74)));
                    });
                ui.add_space(8.0);
            }

            // Summary banner
            let banner_bg = if app.dark_mode { Color32::from_rgb(30, 41, 59) } else { Color32::from_rgb(241, 245, 249) };
            egui::Frame::none()
                .fill(banner_bg)
                .rounding(6.0)
                .inner_margin(10.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(format!("Выбрано: {} файлов • {}", total_selected_files, format_size(total_selected_bytes))).strong());
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.add_enabled(!app.system_clean_is_busy, egui::Button::new("Снять выбор")).clicked() {
                                for item in &mut app.system_clean_items {
                                    item.selected = false;
                                }
                            }
                            if ui.add_enabled(!app.system_clean_is_busy, egui::Button::new("Выбрать все")).clicked() {
                                for item in &mut app.system_clean_items {
                                    item.selected = true;
                                }
                            }
                        });
                    });
                });

            ui.add_space(8.0);

            // Table of items
            egui::ScrollArea::vertical().max_height(280.0).show(ui, |ui| {
                for item in &mut app.system_clean_items {
                    let card_bg = if app.dark_mode {
                        Color32::from_rgb(24, 24, 27)
                    } else {
                        Color32::from_rgb(255, 255, 255)
                    };
                    let stroke_col = if app.dark_mode { Color32::from_rgb(51, 65, 85) } else { Color32::from_rgb(226, 232, 240) };

                    egui::Frame::none()
                        .fill(card_bg)
                        .stroke(Stroke::new(1.0_f32, stroke_col))
                        .rounding(6.0)
                        .inner_margin(8.0)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.checkbox(&mut item.selected, "");
                                ui.vertical(|ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(RichText::new(&item.name).strong());
                                        // Category chip
                                        let chip_bg = if app.dark_mode { Color32::from_rgb(49, 46, 129) } else { Color32::from_rgb(224, 231, 255) };
                                        let chip_text = if app.dark_mode { Color32::from_rgb(199, 210, 254) } else { Color32::from_rgb(67, 56, 202) };
                                        egui::Frame::none()
                                            .fill(chip_bg)
                                            .rounding(4.0)
                                            .inner_margin(egui::Margin::symmetric(5.0, 2.0))
                                            .show(ui, |ui| {
                                                ui.label(RichText::new(&item.category).size(11.0).color(chip_text));
                                            });

                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                            ui.label(RichText::new(format_size(item.size_bytes)).strong().color(
                                                if item.size_bytes > 500 * 1024 * 1024 {
                                                    Color32::from_rgb(239, 68, 68)
                                                } else if item.size_bytes > 0 {
                                                    Color32::from_rgb(245, 158, 11)
                                                } else {
                                                    Color32::GRAY
                                                }
                                            ));
                                            ui.label(RichText::new(format!("{} файлов", item.file_count)).size(11.0).color(Color32::GRAY));
                                        });
                                    });
                                    ui.label(RichText::new(&item.description).size(11.0).color(Color32::GRAY));
                                });
                            });
                        });
                    ui.add_space(4.0);
                }
            });

            ui.add_space(10.0);
            ui.separator();
            ui.add_space(6.0);

            ui.horizontal(|ui| {
                if ui.add_enabled(!app.system_clean_is_busy, egui::Button::new("🔄 Пересканировать")).clicked() {
                    do_rescan = true;
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Закрыть").clicked() {
                        do_close = true;
                    }

                    let clean_label = format!("Очистить выбранное ({})", format_size(total_selected_bytes));
                    let clean_btn = egui::Button::new(RichText::new(clean_label).strong().color(Color32::WHITE))
                        .fill(if total_selected_bytes > 0 && !app.system_clean_is_busy { Color32::from_rgb(220, 38, 38) } else { Color32::from_rgb(100, 116, 139) });

                    if ui.add_enabled(total_selected_bytes > 0 && !app.system_clean_is_busy, clean_btn).clicked() {
                        do_clean = true;
                    }
                });
            });
        });

    if do_clean {
        app.start_system_clean_delete();
    }
    if do_rescan {
        app.start_system_clean_scan();
    }
    if do_close || !is_open {
        app.system_clean_modal_open = false;
    }
}

// ============================================================================
// 2. Developer Environments Cleaner Modal
// ============================================================================

pub fn render_dev_clean_modal(app: &mut PurgeCraftApp, ctx: &egui::Context) {
    if !app.dev_clean_modal_open {
        return;
    }

    let mut is_open = true;
    let mut do_clean = false;
    let mut do_rescan = false;
    let mut do_close = false;

    let total_selected_bytes: u64 = app.dev_clean_items
        .iter()
        .filter(|i| i.selected)
        .map(|i| i.size_bytes)
        .sum();
    let total_selected_files: usize = app.dev_clean_items
        .iter()
        .filter(|i| i.selected)
        .map(|i| i.file_count)
        .sum();

    egui::Window::new("🛠 Очистка кэшей разработчика (Dev Environments)")
        .open(&mut is_open)
        .collapsible(false)
        .resizable(true)
        .pivot(egui::Align2::CENTER_CENTER)
        .default_pos(ctx.screen_rect().center())
        .show(ctx, |ui| {
            ui.set_width(740.0);
            ui.set_height(490.0);

            ui.horizontal(|ui| {
                ui.heading(RichText::new("🛠 Кэши сред разработки и пакетных менеджеров").strong());
                if app.dev_clean_is_busy {
                    ui.add_space(8.0);
                    ui.spinner();
                    ui.label(RichText::new(&app.dev_clean_busy_msg).color(Color32::from_rgb(59, 130, 246)));
                }
            });
            ui.add_space(4.0);
            ui.label("Очистка накопленных архивов библиотек и промежуточных сборок Node.js, Python, Rust, Go, .NET, Gradle, Maven и JetBrains.");
            ui.add_space(8.0);

            if let Some(msg) = &app.dev_clean_msg {
                egui::Frame::none()
                    .fill(if app.dark_mode { Color32::from_rgb(20, 83, 45) } else { Color32::from_rgb(220, 252, 231) })
                    .rounding(4.0)
                    .inner_margin(8.0)
                    .show(ui, |ui| {
                        ui.label(RichText::new(msg).strong().color(Color32::from_rgb(22, 163, 74)));
                    });
                ui.add_space(8.0);
            }

            // Summary banner
            let banner_bg = if app.dark_mode { Color32::from_rgb(30, 41, 59) } else { Color32::from_rgb(241, 245, 249) };
            egui::Frame::none()
                .fill(banner_bg)
                .rounding(6.0)
                .inner_margin(10.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(format!("Выбрано: {} файлов • {}", total_selected_files, format_size(total_selected_bytes))).strong());
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.add_enabled(!app.dev_clean_is_busy, egui::Button::new("Снять выбор")).clicked() {
                                for item in &mut app.dev_clean_items {
                                    item.selected = false;
                                }
                            }
                            if ui.add_enabled(!app.dev_clean_is_busy, egui::Button::new("Выбрать все")).clicked() {
                                for item in &mut app.dev_clean_items {
                                    item.selected = true;
                                }
                            }
                        });
                    });
                });

            ui.add_space(8.0);

            // Table of items
            egui::ScrollArea::vertical().max_height(280.0).show(ui, |ui| {
                for item in &mut app.dev_clean_items {
                    let card_bg = if app.dark_mode {
                        Color32::from_rgb(24, 24, 27)
                    } else {
                        Color32::from_rgb(255, 255, 255)
                    };
                    let stroke_col = if app.dark_mode { Color32::from_rgb(51, 65, 85) } else { Color32::from_rgb(226, 232, 240) };

                    egui::Frame::none()
                        .fill(card_bg)
                        .stroke(Stroke::new(1.0_f32, stroke_col))
                        .rounding(6.0)
                        .inner_margin(8.0)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.checkbox(&mut item.selected, "");
                                ui.vertical(|ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(RichText::new(&item.name).strong());
                                        // Ecosystem badge
                                        let (chip_bg, chip_text) = match item.ecosystem.as_str() {
                                            "Rust" => (Color32::from_rgb(180, 83, 9), Color32::WHITE),
                                            "JavaScript" => (Color32::from_rgb(234, 179, 8), Color32::BLACK),
                                            "Python" => (Color32::from_rgb(2, 132, 199), Color32::WHITE),
                                            "Go" => (Color32::from_rgb(14, 165, 233), Color32::WHITE),
                                            ".NET" => (Color32::from_rgb(124, 58, 237), Color32::WHITE),
                                            "Java" => (Color32::from_rgb(225, 29, 72), Color32::WHITE),
                                            _ => (Color32::from_rgb(71, 85, 105), Color32::WHITE),
                                        };
                                        egui::Frame::none()
                                            .fill(chip_bg)
                                            .rounding(4.0)
                                            .inner_margin(egui::Margin::symmetric(5.0, 2.0))
                                            .show(ui, |ui| {
                                                ui.label(RichText::new(&item.ecosystem).size(10.0).color(chip_text).strong());
                                            });

                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                            ui.label(RichText::new(format_size(item.size_bytes)).strong().color(
                                                if item.size_bytes > 1024 * 1024 * 1024 {
                                                    Color32::from_rgb(239, 68, 68)
                                                } else if item.size_bytes > 0 {
                                                    Color32::from_rgb(245, 158, 11)
                                                } else {
                                                    Color32::GRAY
                                                }
                                            ));
                                            ui.label(RichText::new(format!("{} файлов", item.file_count)).size(11.0).color(Color32::GRAY));
                                        });
                                    });
                                    ui.label(RichText::new(&item.description).size(11.0).color(Color32::GRAY));
                                });
                            });
                        });
                    ui.add_space(4.0);
                }
            });

            ui.add_space(10.0);
            ui.separator();
            ui.add_space(6.0);

            ui.horizontal(|ui| {
                if ui.add_enabled(!app.dev_clean_is_busy, egui::Button::new("🔄 Пересканировать")).clicked() {
                    do_rescan = true;
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Закрыть").clicked() {
                        do_close = true;
                    }

                    let clean_label = format!("Очистить выбранные кэши ({})", format_size(total_selected_bytes));
                    let clean_btn = egui::Button::new(RichText::new(clean_label).strong().color(Color32::WHITE))
                        .fill(if total_selected_bytes > 0 && !app.dev_clean_is_busy { Color32::from_rgb(16, 185, 129) } else { Color32::from_rgb(100, 116, 139) });

                    if ui.add_enabled(total_selected_bytes > 0 && !app.dev_clean_is_busy, clean_btn).clicked() {
                        do_clean = true;
                    }
                });
            });
        });

    if do_clean {
        app.start_dev_clean_delete();
    }
    if do_rescan {
        app.start_dev_clean_scan();
    }
    if do_close || !is_open {
        app.dev_clean_modal_open = false;
    }
}

// ============================================================================
// 3. Windows Services & Zombie Hunter Modal
// ============================================================================

pub fn render_services_modal(app: &mut PurgeCraftApp, ctx: &egui::Context) {
    if !app.services_modal_open {
        return;
    }

    let mut is_open = true;
    let mut do_rescan = false;
    let mut do_close = false;
    let mut service_to_toggle: Option<(String, String)> = None; // (name, new_startup)
    let mut service_to_del: Option<String> = None;

    let zombie_count = app.services_items.iter().filter(|s| s.is_zombie).count();
    let third_party_count = app.services_items.iter().filter(|s| s.is_third_party && !s.is_zombie).count();

    egui::Window::new("⚙ Менеджер служб Windows и Охотник за зомби")
        .open(&mut is_open)
        .collapsible(false)
        .resizable(true)
        .pivot(egui::Align2::CENTER_CENTER)
        .default_pos(ctx.screen_rect().center())
        .show(ctx, |ui| {
            ui.set_width(860.0);
            ui.set_height(540.0);

            ui.horizontal(|ui| {
                ui.heading(RichText::new("⚙ Службы Windows и Детектор «зомби-сервисов»").strong());
            });
            ui.add_space(4.0);
            ui.label("Обнаружение служб, чьи исполняемые файлы были удалены («зомби»), а также управление запуском сторонних служб Windows.");
            ui.add_space(8.0);

            if let Some(msg) = &app.services_msg {
                egui::Frame::none()
                    .fill(if app.dark_mode { Color32::from_rgb(30, 41, 59) } else { Color32::from_rgb(241, 245, 249) })
                    .rounding(4.0)
                    .inner_margin(8.0)
                    .show(ui, |ui| {
                        ui.label(RichText::new(msg).strong());
                    });
                ui.add_space(6.0);
            }

            // Filter bar and search
            ui.horizontal(|ui| {
                ui.selectable_value(&mut app.services_filter, ServiceFilter::All, "Все службы");

                let zombie_chip = format!("🧟 Службы-зомби ({})", zombie_count);
                let z_btn = ui.selectable_value(&mut app.services_filter, ServiceFilter::Zombies, zombie_chip);
                if zombie_count > 0 {
                    z_btn.on_hover_text("Службы, исполняемый файл которых отсутствует на диске!");
                }

                let third_chip = format!("Сторонние ({})", third_party_count);
                ui.selectable_value(&mut app.services_filter, ServiceFilter::ThirdParty, third_chip);
                ui.selectable_value(&mut app.services_filter, ServiceFilter::Drivers, "Драйверы ядра");

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add(egui::TextEdit::singleline(&mut app.services_search).hint_text("🔍 Поиск по имени или пути..."));
                });
            });

            ui.add_space(8.0);

            // Filtered services
            let search_lower = app.services_search.to_lowercase();
            let filtered: Vec<usize> = app.services_items
                .iter()
                .enumerate()
                .filter(|(_, s)| {
                    // Filter chip
                    let matches_chip = match app.services_filter {
                        ServiceFilter::All => true,
                        ServiceFilter::Zombies => s.is_zombie,
                        ServiceFilter::ThirdParty => s.is_third_party && !s.is_driver,
                        ServiceFilter::Drivers => s.is_driver,
                    };
                    if !matches_chip {
                        return false;
                    }
                    // Search
                    if !search_lower.is_empty() {
                        let matches_name = s.name.to_lowercase().contains(&search_lower);
                        let matches_display = s.display_name.to_lowercase().contains(&search_lower);
                        let matches_img = s.image_path.to_lowercase().contains(&search_lower);
                        matches_name || matches_display || matches_img
                    } else {
                        true
                    }
                })
                .map(|(idx, _)| idx)
                .collect();

            // Services list
            egui::ScrollArea::vertical().max_height(330.0).show(ui, |ui| {
                if filtered.is_empty() {
                    ui.add_space(20.0);
                    ui.vertical_centered(|ui| {
                        ui.label(RichText::new("Службы по заданному фильтру не найдены").color(Color32::GRAY));
                    });
                }

                for &idx in &filtered {
                    let s = &app.services_items[idx];
                    let card_bg = if s.is_zombie {
                        if app.dark_mode { Color32::from_rgb(69, 10, 10) } else { Color32::from_rgb(254, 242, 242) }
                    } else if app.dark_mode {
                        Color32::from_rgb(24, 24, 27)
                    } else {
                        Color32::from_rgb(255, 255, 255)
                    };

                    let stroke_col = if s.is_zombie {
                        Color32::from_rgb(239, 68, 68)
                    } else if app.dark_mode {
                        Color32::from_rgb(51, 65, 85)
                    } else {
                        Color32::from_rgb(226, 232, 240)
                    };

                    egui::Frame::none()
                        .fill(card_bg)
                        .stroke(Stroke::new(1.0_f32, stroke_col))
                        .rounding(6.0)
                        .inner_margin(8.0)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                // Status badge
                                if s.is_zombie {
                                    egui::Frame::none()
                                        .fill(Color32::from_rgb(220, 38, 38))
                                        .rounding(4.0)
                                        .inner_margin(egui::Margin::symmetric(6.0, 2.0))
                                        .show(ui, |ui| {
                                            ui.label(RichText::new("🧟 ЗОМБИ").color(Color32::WHITE).strong().size(11.0));
                                        });
                                } else if s.is_third_party {
                                    egui::Frame::none()
                                        .fill(if app.dark_mode { Color32::from_rgb(30, 58, 138) } else { Color32::from_rgb(219, 234, 254) })
                                        .rounding(4.0)
                                        .inner_margin(egui::Margin::symmetric(5.0, 2.0))
                                        .show(ui, |ui| {
                                            ui.label(RichText::new("3rd Party").color(if app.dark_mode { Color32::from_rgb(191, 219, 254) } else { Color32::from_rgb(30, 64, 175) }).size(11.0));
                                        });
                                } else if s.is_driver {
                                    egui::Frame::none()
                                        .fill(if app.dark_mode { Color32::from_rgb(63, 63, 70) } else { Color32::from_rgb(243, 244, 246) })
                                        .rounding(4.0)
                                        .inner_margin(egui::Margin::symmetric(5.0, 2.0))
                                        .show(ui, |ui| {
                                            ui.label(RichText::new("Driver").color(Color32::GRAY).size(11.0));
                                        });
                                }

                                ui.vertical(|ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(RichText::new(&s.display_name).strong());
                                        ui.label(RichText::new(format!("({})", s.name)).size(11.0).color(Color32::GRAY));

                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                            // Action buttons
                                            if s.is_zombie {
                                                if ui.button(RichText::new("🗑 Удалить зомби").color(Color32::WHITE)).clicked() {
                                                    service_to_del = Some(s.name.clone());
                                                }
                                            } else {
                                                match s.startup_type {
                                                    ServiceStartupType::Disabled => {
                                                        if ui.button("Включить (Авто)").clicked() {
                                                            service_to_toggle = Some((s.name.clone(), "auto".to_string()));
                                                        }
                                                    }
                                                    _ => {
                                                        if ui.button("Отключить").clicked() {
                                                            service_to_toggle = Some((s.name.clone(), "disabled".to_string()));
                                                        }
                                                    }
                                                }
                                            }

                                            if ui.small_button("Реестр").clicked() {
                                                jump_to_regedit(&format!(r"HKLM\SYSTEM\CurrentControlSet\Services\{}", s.name));
                                            }

                                            let st_color = match s.startup_type {
                                                ServiceStartupType::Automatic => Color32::from_rgb(34, 197, 94),
                                                ServiceStartupType::Manual => Color32::from_rgb(59, 130, 246),
                                                ServiceStartupType::Disabled => Color32::from_rgb(239, 68, 68),
                                                _ => Color32::GRAY,
                                            };
                                            ui.label(RichText::new(s.startup_type.label()).size(11.0).color(st_color).strong());
                                        });
                                    });

                                    // Binary path / status
                                    if let Some(bin) = &s.binary_path {
                                        ui.horizontal(|ui| {
                                            let path_col = if s.is_zombie { Color32::from_rgb(239, 68, 68) } else { Color32::GRAY };
                                            let icon = if s.is_zombie { "❌ Файл не найден: " } else { "📁 " };
                                            ui.label(RichText::new(format!("{}{}", icon, bin)).size(11.0).color(path_col));
                                            if !s.is_zombie && ui.small_button("Папка").clicked() {
                                                open_system_folder(bin, app.settings.use_total_commander);
                                            }
                                        });
                                    }
                                });
                            });
                        });
                    ui.add_space(4.0);
                }
            });

            ui.add_space(10.0);
            ui.separator();
            ui.add_space(6.0);

            ui.horizontal(|ui| {
                if ui.button("🔄 Обновить службы").clicked() {
                    do_rescan = true;
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Закрыть").clicked() {
                        do_close = true;
                    }
                });
            });
        });

    if let Some((name, mode)) = service_to_toggle {
        match set_service_startup(&name, &mode) {
            Ok(_) => {
                app.services_msg = Some(format!("Режим запуска службы '{}' изменен на '{}'", name, mode));
                app.services_items = scan_windows_services();
            }
            Err(e) => {
                app.services_msg = Some(format!("Ошибка: {}", e));
            }
        }
    }

    if let Some(name) = service_to_del {
        match delete_windows_service(&name) {
            Ok(_) => {
                app.services_msg = Some(format!("Зомби-служба '{}' успешно удалена из системы", name));
                app.services_items = scan_windows_services();
            }
            Err(e) => {
                app.services_msg = Some(format!("Ошибка удаления службы: {}", e));
            }
        }
    }

    if do_rescan {
        app.services_items = scan_windows_services();
        app.services_msg = Some("Список служб обновлен".to_string());
    }
    if do_close || !is_open {
        app.services_modal_open = false;
    }
}
