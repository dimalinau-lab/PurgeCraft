use eframe::egui::{self, Color32, RichText};
use crate::app::PurgeCraftApp;
use crate::autoupdater;
use crate::types::format_size_lang;

pub fn render_update_modal(app: &mut PurgeCraftApp, ctx: &egui::Context) {
    if !app.update_modal_open {
        return;
    }

    let lang = app.lang();
    let is_detached = app.detached_modals.contains("modal_update");
    let mut should_close = false;

    let outcome = super::show_detachable_window(
        ctx,
        "modal_update",
        crate::i18n::t(lang, "Обновление PurgeCraft", "PurgeCraft Update"),
        None,
        egui::vec2(480.0, 320.0),
        egui::vec2(320.0, 200.0),
        true,
        is_detached,
        app.dark_mode,
        lang,
        |ui, _active_ctx| {
            ui.add_space(4.0);

            match &app.update_status {
                autoupdater::UpdateStatus::Idle => {
                    ui.label(crate::i18n::t(lang, "Проверка обновлений не запущена.", "Update check has not started."));
                    ui.add_space(8.0);
                    if ui.button(crate::i18n::t(lang, "Проверить сейчас", "Check now")).clicked() {
                        autoupdater::check_for_updates_async(
                            app.settings.github_repo.clone(),
                            env!("CARGO_PKG_VERSION").to_string(),
                            app.update_tx.clone(),
                        );
                    }
                }
                autoupdater::UpdateStatus::Checking => {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(RichText::new(crate::i18n::t(lang, "Проверка наличия новой версии на GitHub...", "Checking for new version on GitHub...")).strong());
                    });
                    ui.add_space(6.0);
                    ui.label(format!("{}: github.com/{}", crate::i18n::t(lang, "Репозиторий", "Repository"), app.settings.github_repo));
                }
                autoupdater::UpdateStatus::UpToDate => {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(crate::i18n::t(lang, "Установлена последняя версия", "Latest version installed")).strong().color(Color32::from_rgb(34, 197, 94)));
                    });
                    ui.add_space(6.0);
                    ui.label(format!("{} v{} {}", crate::i18n::t(lang, "Текущая версия PurgeCraft", "Current PurgeCraft version"), env!("CARGO_PKG_VERSION"), crate::i18n::t(lang, "является актуальной.", "is up to date.")));
                    ui.add_space(10.0);
                    if ui.button(crate::i18n::t(lang, "Закрыть", "Close")).clicked() {
                        app.update_modal_open = false;
                    }
                }
                autoupdater::UpdateStatus::Available(info) => {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(crate::i18n::t(lang, "Доступна новая версия", "New version available")).strong().color(Color32::from_rgb(59, 130, 246)));
                        ui.label(RichText::new(format!("v{}", info.version)).strong());
                    });
                    ui.label(format!("{}: v{}  |  {}: v{}", crate::i18n::t(lang, "Текущая версия", "Current version"), env!("CARGO_PKG_VERSION"), crate::i18n::t(lang, "Новая", "New"), info.version));
                    ui.label(format!("{}: {}", crate::i18n::t(lang, "Размер файла", "File size"), format_size_lang(info.file_size, lang)));
                    ui.add_space(6.0);

                    ui.group(|ui| {
                        ui.label(RichText::new(crate::i18n::t(lang, "Описание изменений:", "Release notes:")).strong());
                        let list_h = super::fill_scroll_height(ui, 56.0, 80.0);
                        egui::ScrollArea::vertical().max_height(list_h).auto_shrink([false, false]).show(ui, |ui| {
                            ui.label(&info.release_notes);
                        });
                    });

                    ui.add_space(8.0);
                    ui.horizontal_wrapped(|ui| {
                        if ui.button(RichText::new(crate::i18n::t(lang, "Скачать и установить", "Download and install")).strong().color(Color32::WHITE))
                            .clicked()
                        {
                            autoupdater::download_and_apply_update_async(info.download_url.clone(), app.update_tx.clone());
                        }

                        if ui.button(crate::i18n::t(lang, "Открыть на GitHub", "Open on GitHub")).clicked() {
                            let _ = open::that(&info.html_url);
                        }

                        if ui.button(crate::i18n::t(lang, "Отмена", "Cancel")).clicked() {
                            app.update_modal_open = false;
                        }
                    });
                }
                autoupdater::UpdateStatus::Downloading { percent, downloaded, total } => {
                    ui.label(RichText::new(crate::i18n::t(lang, "Загрузка обновления...", "Downloading update...")).strong());
                    ui.add_space(4.0);
                    ui.add(egui::ProgressBar::new(*percent).show_percentage());
                    ui.add_space(4.0);
                    ui.label(format!("{}: {} {} {}", crate::i18n::t(lang, "Загружено", "Downloaded"), format_size_lang(*downloaded, lang), crate::i18n::t(lang, "из", "of"), format_size_lang(*total, lang)));
                    ui.label(RichText::new(crate::i18n::t(lang, "Пожалуйста, не закрывайте приложение, идет замена исполняемого файла.", "Please do not close the app, executable replacement in progress.")).color(Color32::from_rgb(234, 179, 8)));
                }
                autoupdater::UpdateStatus::ReadyToRestart => {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(crate::i18n::t(lang, "Обновление успешно установлено", "Update successfully installed")).strong().color(Color32::from_rgb(34, 197, 94)));
                    });
                    ui.add_space(6.0);
                    ui.label(crate::i18n::t(lang, "Исполняемый файл PurgeCraft был успешно заменен на новую версию.", "PurgeCraft executable was successfully replaced with the new version."));
                    ui.label(crate::i18n::t(lang, "Перезапустите приложение для перехода на обновленную версию.", "Restart the application to switch to the updated version."));
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        if ui.button(RichText::new(crate::i18n::t(lang, "Перезапустить", "Restart")).strong().color(Color32::WHITE)).clicked() {
                            if let Err(e) = autoupdater::restart_app() {
                                app.update_status = autoupdater::UpdateStatus::Error(e);
                            }
                        }
                        if ui.button(crate::i18n::t(lang, "Позже", "Later")).clicked() {
                            should_close = true;
                        }
                    });
                }
                autoupdater::UpdateStatus::Error(err) => {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(crate::i18n::t(lang, "Ошибка при обновлении", "Update error")).strong().color(Color32::from_rgb(239, 68, 68)));
                    });
                    ui.add_space(4.0);
                    ui.label(err);
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        if ui.button(crate::i18n::t(lang, "Повторить попытку", "Retry")).clicked() {
                            autoupdater::check_for_updates_async(
                                app.settings.github_repo.clone(),
                                env!("CARGO_PKG_VERSION").to_string(),
                                app.update_tx.clone(),
                            );
                        }
                        if ui.button(crate::i18n::t(lang, "Закрыть", "Close")).clicked() {
                            should_close = true;
                        }
                    });
                }
            }
        },
    );

    if should_close {
        app.update_modal_open = false;
    }
    super::sync_detachable_state("modal_update", &mut app.detached_modals, &mut app.update_modal_open, outcome);
}
