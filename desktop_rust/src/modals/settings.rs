use eframe::egui::{self, Color32, RichText, Vec2, Stroke};
use crate::app::PurgeCraftApp;
use crate::types::SettingsTab;
use crate::settings::{save_settings, is_windows_dark_mode};
use crate::autoupdater;

pub fn render_settings_modal(app: &mut PurgeCraftApp, ctx: &egui::Context) {
    if !app.settings_open {
        return;
    }

    let mut is_open = true;
    let mut do_save = false;
    let mut do_cancel = false;

    egui::Window::new("⚙ Настройки")
        .open(&mut is_open)
        .collapsible(false)
        .resizable(false)
        .pivot(egui::Align2::CENTER_CENTER)
        .default_pos(ctx.screen_rect().center())
        .show(ctx, |ui| {
            ui.set_width(520.0);
            ui.set_height(410.0);

            // 1. Top Tab Bar
            ui.horizontal(|ui| {
                let tab_btn = |ui: &mut egui::Ui, is_sel: bool, icon: &str, label: &str| -> egui::Response {
                    let (fill, stroke, text_color) = if is_sel {
                        (
                            if app.dark_mode { Color32::from_rgb(45, 48, 56) } else { Color32::from_rgb(224, 242, 254) },
                            Stroke::new(1.0_f32, if app.dark_mode { Color32::from_rgb(96, 165, 250) } else { Color32::from_rgb(2, 132, 199) }),
                            if app.dark_mode { Color32::WHITE } else { Color32::from_rgb(2, 132, 199) },
                        )
                    } else {
                        (
                            if app.dark_mode { Color32::from_rgb(24, 24, 28) } else { Color32::from_rgb(241, 245, 249) },
                            Stroke::NONE,
                            if app.dark_mode { Color32::from_rgb(161, 161, 170) } else { Color32::from_rgb(71, 85, 105) },
                        )
                    };
                    let tab_w = ((ui.available_width() - 12.0) / 3.0).floor().max(100.0);
                    let btn = egui::Button::new(
                        RichText::new(format!("{} {}", icon, label)).strong().color(text_color)
                    )
                    .fill(fill)
                    .stroke(stroke)
                    .min_size(Vec2::new(tab_w, 34.0));
                    ui.add(btn)
                };

                if tab_btn(ui, app.settings_tab == SettingsTab::General, "📁", "Основные").clicked() {
                    app.settings_tab = SettingsTab::General;
                }
                if tab_btn(ui, app.settings_tab == SettingsTab::Interface, "🖥", "Интерфейс").clicked() {
                    app.settings_tab = SettingsTab::Interface;
                }
                if tab_btn(ui, app.settings_tab == SettingsTab::Uninstaller, "📦", "Деинсталлятор").clicked() {
                    app.settings_tab = SettingsTab::Uninstaller;
                }
            });

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);

            // 2. Tab Contents
            egui::ScrollArea::vertical().max_height(280.0).show(ui, |ui| {
                match app.settings_tab {
                    SettingsTab::General => {
                        ui.checkbox(
                            &mut app.temp_settings.skip_uac_task,
                            "Упростить запуск программы используя планировщик задач (избежать запроса Контроля учетных записей на разрешение вносить изменения)"
                        ).on_hover_text("Позволяет запускать PurgeCraft без всплывающего окна UAC Windows");

                        ui.add_space(6.0);
                        ui.checkbox(
                            &mut app.temp_settings.explorer_context_menu,
                            "Добавлять пункт 'Установить и следить' для установочных файлов (exe/msi) в контекстное меню Проводника Windows"
                        ).on_hover_text("Интегрирует пункт «Установить и следить (PurgeCraft)» в меню правой кнопки мыши");

                        ui.add_space(6.0);
                        ui.checkbox(
                            &mut app.temp_settings.check_updates,
                            "Проверять наличие новой версии при старте программы"
                        );

                        ui.horizontal(|ui| {
                            ui.add_space(20.0);
                            ui.add_enabled(
                                app.temp_settings.check_updates,
                                egui::Checkbox::new(&mut app.temp_settings.check_beta, "Проверять также наличие бета-версий")
                            );
                        });

                        ui.horizontal(|ui| {
                            ui.add_space(20.0);
                            ui.label("Репозиторий GitHub:");
                            ui.add(egui::TextEdit::singleline(&mut app.temp_settings.github_repo).desired_width(160.0));
                            if ui.button("🔍 Проверить сейчас").clicked() {
                                autoupdater::check_for_updates_async(
                                    app.temp_settings.github_repo.clone(),
                                    env!("CARGO_PKG_VERSION").to_string(),
                                    app.update_tx.clone(),
                                );
                                app.update_modal_open = true;
                            }
                        });

                        ui.add_space(6.0);
                        ui.checkbox(
                            &mut app.temp_settings.show_in_control_panel,
                            "Отображать значок PurgeCraft в Панели управления"
                        );

                        ui.add_space(6.0);
                        ui.checkbox(
                            &mut app.temp_settings.use_total_commander,
                            "Переходить в папку программы используя Total Commander"
                        );

                        ui.add_space(10.0);
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Язык интерфейса:").strong());
                            egui::ComboBox::from_id_salt("settings_lang")
                                .selected_text(&app.temp_settings.language)
                                .width(180.0)
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(&mut app.temp_settings.language, "Russian (Русский)".to_string(), "Russian (Русский)");
                                    ui.selectable_value(&mut app.temp_settings.language, "English".to_string(), "English");
                                });
                        });
                    }
                    SettingsTab::Interface => {
                        ui.checkbox(
                            &mut app.temp_settings.show_toolbar_labels,
                            "Показывать подписи к кнопкам в тулбаре"
                        );

                        ui.add_space(10.0);
                        ui.label(RichText::new("Список программ").heading().size(14.0));
                        ui.checkbox(&mut app.temp_settings.compact_list, "Компактный список программ");
                        ui.checkbox(&mut app.temp_settings.mark_32bit, "Помечать 32-битные приложения");

                        ui.add_space(12.0);
                        ui.label(RichText::new("Цветовая схема").heading().size(14.0));
                        let theme_label = match app.temp_settings.theme_mode.as_str() {
                            "system" => "Соответствовать цветовой схеме Windows",
                            "light" => "Светлая",
                            _ => "Темная",
                        };
                        egui::ComboBox::from_id_salt("settings_theme")
                            .selected_text(theme_label)
                            .width(260.0)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut app.temp_settings.theme_mode, "system".to_string(), "Соответствовать цветовой схеме Windows");
                                ui.selectable_value(&mut app.temp_settings.theme_mode, "dark".to_string(), "Темная");
                                ui.selectable_value(&mut app.temp_settings.theme_mode, "light".to_string(), "Светлая");
                            });

                        ui.add_space(12.0);
                        ui.label(RichText::new("Масштаб и размер текста").heading().size(14.0));
                        ui.horizontal(|ui| {
                            ui.label("Размер текста интерфейса:");
                            let scale_pct = (app.temp_settings.text_scale * 100.0).round() as i32;
                            egui::ComboBox::from_id_salt("settings_text_scale")
                                .selected_text(format!("{}%", scale_pct))
                                .width(180.0)
                                .show_ui(ui, |ui| {
                                    if ui.selectable_value(&mut app.temp_settings.text_scale, 1.0, "100% (Стандартный)").clicked() {
                                        ctx.set_zoom_factor(1.0);
                                    }
                                    if ui.selectable_value(&mut app.temp_settings.text_scale, 1.15, "115% (Рекомендуемый)").clicked() {
                                        ctx.set_zoom_factor(1.15);
                                    }
                                    if ui.selectable_value(&mut app.temp_settings.text_scale, 1.25, "125% (Увеличенный)").clicked() {
                                        ctx.set_zoom_factor(1.25);
                                    }
                                    if ui.selectable_value(&mut app.temp_settings.text_scale, 1.35, "135% (Крупный)").clicked() {
                                        ctx.set_zoom_factor(1.35);
                                    }
                                    if ui.selectable_value(&mut app.temp_settings.text_scale, 1.50, "150% (Очень крупный)").clicked() {
                                        ctx.set_zoom_factor(1.50);
                                    }
                                });
                        });

                        let mut scale_val = app.temp_settings.text_scale;
                        if ui.add(egui::Slider::new(&mut scale_val, 0.85..=1.75).step_by(0.05).text("Плавная настройка")).changed() {
                            app.temp_settings.text_scale = scale_val;
                            ctx.set_zoom_factor(scale_val);
                        }
                    }
                    SettingsTab::Uninstaller => {
                        ui.label(RichText::new("Древовидное отображение обновлений программ (дочерние элементы):").strong());
                        ui.horizontal(|ui| {
                            ui.label("Отображать обновления программ");
                            let upd_label = match app.temp_settings.tree_updates_mode.as_str() {
                                "expanded" => "Развернутыми",
                                "hidden" => "Не отображать",
                                _ => "Изначально свернутыми",
                            };
                            egui::ComboBox::from_id_salt("settings_updates")
                                .selected_text(upd_label)
                                .width(180.0)
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(&mut app.temp_settings.tree_updates_mode, "collapsed".to_string(), "Изначально свернутыми");
                                    ui.selectable_value(&mut app.temp_settings.tree_updates_mode, "expanded".to_string(), "Развернутыми");
                                    ui.selectable_value(&mut app.temp_settings.tree_updates_mode, "hidden".to_string(), "Не отображать");
                                });
                            ui.label("🛈").on_hover_text("Определяет группировку компонентов софта в основном списке");
                        });

                        ui.add_space(10.0);
                        ui.label(RichText::new("Недавно установленные программы:").strong());
                        ui.checkbox(&mut app.temp_settings.highlight_recent, "Подсвечивать недавно установленные программы");
                        ui.checkbox(&mut app.temp_settings.popup_recent, "Показывать всплывающее окно с недавно установленными программами");
                        ui.checkbox(&mut app.temp_settings.show_recent_in_toolbar, "Отображать в дополнительной панели инструментов");

                        ui.add_space(10.0);
                        ui.label(RichText::new("Мастер удаления").strong());
                        ui.label("Запуск Мастера удаления после выполнения штатной деинсталляции:");
                        let wiz_label = match app.temp_settings.wizard_auto_start.as_str() {
                            "ask" => "Спрашивать перед запуском",
                            "never" => "Никогда",
                            _ => "Автоматически",
                        };
                        egui::ComboBox::from_id_salt("settings_wiz_mode")
                            .selected_text(wiz_label)
                            .width(220.0)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut app.temp_settings.wizard_auto_start, "auto".to_string(), "Автоматически");
                                ui.selectable_value(&mut app.temp_settings.wizard_auto_start, "ask".to_string(), "Спрашивать перед запуском");
                                ui.selectable_value(&mut app.temp_settings.wizard_auto_start, "never".to_string(), "Никогда");
                            });
                    }
                }
            });

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(6.0);

            // 3. Bottom Button Row (Harmonious unified sizes)
            let btn_size = Vec2::new(92.0, 28.0);
            ui.horizontal(|ui| {
                let help_btn = egui::Button::new(RichText::new("❓ Помощь").strong())
                    .min_size(btn_size);
                if ui.add(help_btn).on_hover_text("Открыть документацию PurgeCraft на GitHub").clicked() {
                    let _ = open::that(format!("https://github.com/{}", app.settings.github_repo));
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let cancel_btn = egui::Button::new(RichText::new("Отмена").strong())
                        .min_size(btn_size);
                    if ui.add(cancel_btn).clicked() {
                        do_cancel = true;
                    }

                    let ok_btn = egui::Button::new(RichText::new("ОК").strong().color(Color32::WHITE))
                        .fill(if app.dark_mode { Color32::from_rgb(37, 99, 235) } else { Color32::from_rgb(2, 132, 199) })
                        .min_size(btn_size);
                    if ui.add(ok_btn).clicked() {
                        do_save = true;
                    }
                });
            });
        });

    if !is_open || do_cancel {
        ctx.set_zoom_factor(app.settings.text_scale);
        app.temp_settings = app.settings.clone();
        app.settings_open = false;
    }

    if do_save {
        app.settings = app.temp_settings.clone();
        save_settings(&app.settings);
        ctx.set_zoom_factor(app.settings.text_scale);
        app.dark_mode = match app.settings.theme_mode.as_str() {
            "light" => false,
            "system" => is_windows_dark_mode(),
            _ => true,
        };
        app.status_msg = "Настройки успешно сохранены.".to_string();
        app.settings_open = false;
    }
}
