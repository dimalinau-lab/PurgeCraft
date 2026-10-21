use eframe::egui::{self, Color32, RichText, Vec2, Stroke};
use crate::app::PurgeCraftApp;
use crate::types::SettingsTab;
use crate::settings::{save_settings, is_windows_dark_mode};
use crate::autoupdater;

pub fn render_settings_modal(app: &mut PurgeCraftApp, ctx: &egui::Context) {
    if !app.settings_open {
        return;
    }

    let lang = crate::i18n::Language::from_str(&app.temp_settings.language);
    let mut is_open = true;
    let mut do_save = false;
    let mut do_cancel = false;

    egui::Window::new(crate::i18n::t(lang, "⚙ Настройки", "⚙ Settings"))
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

                let tab_general = crate::i18n::t(lang, "Основные", "General");
                let tab_interface = crate::i18n::t(lang, "Интерфейс", "Interface");
                let tab_uninstaller = crate::i18n::t(lang, "Деинсталлятор", "Uninstaller");

                if tab_btn(ui, app.settings_tab == SettingsTab::General, "📁", tab_general).clicked() {
                    app.settings_tab = SettingsTab::General;
                }
                if tab_btn(ui, app.settings_tab == SettingsTab::Interface, "🖥", tab_interface).clicked() {
                    app.settings_tab = SettingsTab::Interface;
                }
                if tab_btn(ui, app.settings_tab == SettingsTab::Uninstaller, "📦", tab_uninstaller).clicked() {
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
                        let uac_label = crate::i18n::t(
                            lang,
                            "Упростить запуск программы используя планировщик задач (избежать запроса Контроля учетных записей на разрешение вносить изменения)",
                            "Run without UAC prompt using Task Scheduler (avoid User Account Control prompt)"
                        );
                        let uac_tip = crate::i18n::t(
                            lang,
                            "Позволяет запускать PurgeCraft без всплывающего окна UAC Windows",
                            "Allows launching PurgeCraft without the Windows UAC confirmation dialog"
                        );
                        ui.checkbox(&mut app.temp_settings.skip_uac_task, uac_label).on_hover_text(uac_tip);

                        ui.add_space(6.0);
                        let context_label = crate::i18n::t(
                            lang,
                            "Добавлять пункт 'Установить и следить' для установочных файлов (exe/msi) в контекстное меню Проводника Windows",
                            "Add 'Install and Track' for installer files (exe/msi) to Windows Explorer context menu"
                        );
                        let context_tip = crate::i18n::t(
                            lang,
                            "Интегрирует пункт «Установить и следить (PurgeCraft)» в меню правой кнопки мыши",
                            "Integrates 'Install and Track (PurgeCraft)' into the right-click menu"
                        );
                        ui.checkbox(&mut app.temp_settings.explorer_context_menu, context_label).on_hover_text(context_tip);

                        ui.add_space(6.0);
                        let updates_label = crate::i18n::t(
                            lang,
                            "Проверять наличие новой версии при старте программы",
                            "Check for new versions on application startup"
                        );
                        ui.checkbox(&mut app.temp_settings.check_updates, updates_label);

                        ui.horizontal(|ui| {
                            ui.add_space(20.0);
                            let beta_label = crate::i18n::t(lang, "Проверять также наличие бета-версий", "Also check for beta releases");
                            ui.add_enabled(
                                app.temp_settings.check_updates,
                                egui::Checkbox::new(&mut app.temp_settings.check_beta, beta_label)
                            );
                        });

                        ui.horizontal(|ui| {
                            ui.add_space(20.0);
                            let repo_label = crate::i18n::t(lang, "Репозиторий GitHub:", "GitHub Repository:");
                            ui.label(repo_label);
                            ui.add(egui::TextEdit::singleline(&mut app.temp_settings.github_repo).desired_width(160.0));
                            let check_btn_label = crate::i18n::t(lang, "🔍 Проверить сейчас", "🔍 Check now");
                            if ui.button(check_btn_label).clicked() {
                                autoupdater::check_for_updates_async(
                                    app.temp_settings.github_repo.clone(),
                                    env!("CARGO_PKG_VERSION").to_string(),
                                    app.update_tx.clone(),
                                );
                                app.update_modal_open = true;
                            }
                        });

                        ui.add_space(6.0);
                        let cpl_label = crate::i18n::t(
                            lang,
                            "Отображать значок PurgeCraft в Панели управления",
                            "Display PurgeCraft icon in Windows Control Panel"
                        );
                        ui.checkbox(&mut app.temp_settings.show_in_control_panel, cpl_label);

                        ui.add_space(6.0);
                        let tc_label = crate::i18n::t(
                            lang,
                            "Переходить в папку программы используя Total Commander",
                            "Open application folders using Total Commander"
                        );
                        ui.checkbox(&mut app.temp_settings.use_total_commander, tc_label);

                        ui.add_space(10.0);
                        ui.horizontal(|ui| {
                            let lang_label = crate::i18n::t(lang, "Язык интерфейса:", "Interface Language:");
                            ui.label(RichText::new(lang_label).strong());
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
                        let toolbar_label = crate::i18n::t(
                            lang,
                            "Показывать подписи к кнопкам в тулбаре",
                            "Show button labels in toolbar"
                        );
                        ui.checkbox(&mut app.temp_settings.show_toolbar_labels, toolbar_label);

                        ui.add_space(10.0);
                        ui.label(RichText::new(crate::i18n::t(lang, "Список программ", "Programs List")).heading().size(14.0));
                        ui.checkbox(&mut app.temp_settings.compact_list, crate::i18n::t(lang, "Компактный список программ", "Compact programs list"));
                        ui.checkbox(&mut app.temp_settings.mark_32bit, crate::i18n::t(lang, "Помечать 32-битные приложения", "Mark 32-bit applications"));

                        ui.add_space(12.0);
                        ui.label(RichText::new(crate::i18n::t(lang, "Цветовая схема", "Color Scheme")).heading().size(14.0));
                        let theme_label = match app.temp_settings.theme_mode.as_str() {
                            "system" => crate::i18n::t(lang, "Соответствовать цветовой схеме Windows", "Match Windows theme"),
                            "light" => crate::i18n::t(lang, "Светлая", "Light"),
                            _ => crate::i18n::t(lang, "Темная", "Dark"),
                        };
                        egui::ComboBox::from_id_salt("settings_theme")
                            .selected_text(theme_label)
                            .width(260.0)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut app.temp_settings.theme_mode, "system".to_string(), crate::i18n::t(lang, "Соответствовать цветовой схеме Windows", "Match Windows theme"));
                                ui.selectable_value(&mut app.temp_settings.theme_mode, "dark".to_string(), crate::i18n::t(lang, "Темная", "Dark"));
                                ui.selectable_value(&mut app.temp_settings.theme_mode, "light".to_string(), crate::i18n::t(lang, "Светлая", "Light"));
                            });

                        ui.add_space(12.0);
                        ui.label(RichText::new(crate::i18n::t(lang, "Масштаб и размер текста", "Text Scale and Size")).heading().size(14.0));
                        ui.horizontal(|ui| {
                            ui.label(crate::i18n::t(lang, "Размер текста интерфейса:", "UI text size:"));
                            let scale_pct = (app.temp_settings.text_scale * 100.0).round() as i32;
                            egui::ComboBox::from_id_salt("settings_text_scale")
                                .selected_text(format!("{}%", scale_pct))
                                .width(180.0)
                                .show_ui(ui, |ui| {
                                    if ui.selectable_value(&mut app.temp_settings.text_scale, 1.0, crate::i18n::t(lang, "100% (Стандартный)", "100% (Default)")).clicked() {
                                        ctx.set_zoom_factor(1.0);
                                    }
                                    if ui.selectable_value(&mut app.temp_settings.text_scale, 1.15, crate::i18n::t(lang, "115% (Рекомендуемый)", "115% (Recommended)")).clicked() {
                                        ctx.set_zoom_factor(1.15);
                                    }
                                    if ui.selectable_value(&mut app.temp_settings.text_scale, 1.25, crate::i18n::t(lang, "125% (Увеличенный)", "125% (Large)")).clicked() {
                                        ctx.set_zoom_factor(1.25);
                                    }
                                    if ui.selectable_value(&mut app.temp_settings.text_scale, 1.35, crate::i18n::t(lang, "135% (Крупный)", "135% (Larger)")).clicked() {
                                        ctx.set_zoom_factor(1.35);
                                    }
                                    if ui.selectable_value(&mut app.temp_settings.text_scale, 1.50, crate::i18n::t(lang, "150% (Очень крупный)", "150% (Huge)")).clicked() {
                                        ctx.set_zoom_factor(1.50);
                                    }
                                });
                        });

                        let mut scale_val = app.temp_settings.text_scale;
                        let smooth_text = crate::i18n::t(lang, "Плавная настройка", "Smooth adjustment");
                        if ui.add(egui::Slider::new(&mut scale_val, 0.85..=1.75).step_by(0.05).text(smooth_text)).changed() {
                            app.temp_settings.text_scale = scale_val;
                            ctx.set_zoom_factor(scale_val);
                        }
                    }
                    SettingsTab::Uninstaller => {
                        let updates_hdr = crate::i18n::t(
                            lang,
                            "Древовидное отображение обновлений программ (дочерние элементы):",
                            "Tree display of program updates (child items):"
                        );
                        ui.label(RichText::new(updates_hdr).strong());
                        ui.horizontal(|ui| {
                            ui.label(crate::i18n::t(lang, "Отображать обновления программ", "Display program updates"));
                            let upd_label = match app.temp_settings.tree_updates_mode.as_str() {
                                "expanded" => crate::i18n::t(lang, "Развернутыми", "Expanded"),
                                "hidden" => crate::i18n::t(lang, "Не отображать", "Do not display"),
                                _ => crate::i18n::t(lang, "Изначально свернутыми", "Collapsed by default"),
                            };
                            egui::ComboBox::from_id_salt("settings_updates")
                                .selected_text(upd_label)
                                .width(180.0)
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(&mut app.temp_settings.tree_updates_mode, "collapsed".to_string(), crate::i18n::t(lang, "Изначально свернутыми", "Collapsed by default"));
                                    ui.selectable_value(&mut app.temp_settings.tree_updates_mode, "expanded".to_string(), crate::i18n::t(lang, "Развернутыми", "Expanded"));
                                    ui.selectable_value(&mut app.temp_settings.tree_updates_mode, "hidden".to_string(), crate::i18n::t(lang, "Не отображать", "Do not display"));
                                });
                            let group_tip = crate::i18n::t(lang, "Определяет группировку компонентов софта в основном списке", "Controls grouping of software components in the main list");
                            ui.label("🛈").on_hover_text(group_tip);
                        });

                        ui.add_space(10.0);
                        ui.label(RichText::new(crate::i18n::t(lang, "Недавно установленные программы:", "Recently installed programs:")).strong());
                        ui.checkbox(&mut app.temp_settings.highlight_recent, crate::i18n::t(lang, "Подсвечивать недавно установленные программы", "Highlight recently installed programs"));
                        ui.checkbox(&mut app.temp_settings.popup_recent, crate::i18n::t(lang, "Показывать всплывающее окно с недавно установленными программами", "Show popup window for recently installed programs"));
                        ui.checkbox(&mut app.temp_settings.show_recent_in_toolbar, crate::i18n::t(lang, "Отображать в дополнительной панели инструментов", "Display in secondary toolbar"));

                        ui.add_space(10.0);
                        ui.label(RichText::new(crate::i18n::t(lang, "Мастер удаления", "Removal Wizard")).strong());
                        ui.label(crate::i18n::t(lang, "Запуск Мастера удаления после выполнения штатной деинсталляции:", "Launch Removal Wizard after standard uninstaller finishes:"));
                        let wiz_label = match app.temp_settings.wizard_auto_start.as_str() {
                            "ask" => crate::i18n::t(lang, "Спрашивать перед запуском", "Ask before launch"),
                            "never" => crate::i18n::t(lang, "Никогда", "Never"),
                            _ => crate::i18n::t(lang, "Автоматически", "Automatically"),
                        };
                        egui::ComboBox::from_id_salt("settings_wiz_mode")
                            .selected_text(wiz_label)
                            .width(220.0)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut app.temp_settings.wizard_auto_start, "auto".to_string(), crate::i18n::t(lang, "Автоматически", "Automatically"));
                                ui.selectable_value(&mut app.temp_settings.wizard_auto_start, "ask".to_string(), crate::i18n::t(lang, "Спрашивать перед запуском", "Ask before launch"));
                                ui.selectable_value(&mut app.temp_settings.wizard_auto_start, "never".to_string(), crate::i18n::t(lang, "Никогда", "Never"));
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
                let help_btn_text = crate::i18n::t(lang, "❓ Помощь", "❓ Help");
                let help_btn_tip = crate::i18n::t(lang, "Открыть документацию PurgeCraft на GitHub", "Open PurgeCraft documentation on GitHub");
                let help_btn = egui::Button::new(RichText::new(help_btn_text).strong())
                    .min_size(btn_size);
                if ui.add(help_btn).on_hover_text(help_btn_tip).clicked() {
                    let _ = open::that(format!("https://github.com/{}", app.settings.github_repo));
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let cancel_btn = egui::Button::new(RichText::new(crate::i18n::t(lang, "Отмена", "Cancel")).strong())
                        .min_size(btn_size);
                    if ui.add(cancel_btn).clicked() {
                        do_cancel = true;
                    }

                    let ok_btn = egui::Button::new(RichText::new(crate::i18n::t(lang, "ОК", "OK")).strong().color(Color32::WHITE))
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
        app.status_msg = crate::i18n::t(lang, "Настройки успешно сохранены.", "Settings saved successfully.").to_string();
        app.settings_open = false;
    }
}
