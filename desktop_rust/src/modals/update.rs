use eframe::egui::{self, Color32, RichText};
use crate::app::PurgeCraftApp;
use crate::autoupdater;
use crate::types::format_size;

pub fn render_update_modal(app: &mut PurgeCraftApp, ctx: &egui::Context) {
    if !app.update_modal_open {
        return;
    }

    let mut is_open = true;
    egui::Window::new("🔄 Обновление PurgeCraft")
        .open(&mut is_open)
        .collapsible(false)
        .resizable(false)
        .pivot(egui::Align2::CENTER_CENTER)
        .default_pos(ctx.screen_rect().center())
        .show(ctx, |ui| {
            ui.set_width(480.0);
            ui.add_space(4.0);

            match &app.update_status {
                autoupdater::UpdateStatus::Idle => {
                    ui.label("Проверка обновлений не запущена.");
                    ui.add_space(8.0);
                    if ui.button("🔍 Проверить сейчас").clicked() {
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
                        ui.label(RichText::new("Проверка наличия новой версии на GitHub...").strong());
                    });
                    ui.add_space(6.0);
                    ui.label(format!("Репозиторий: github.com/{}", app.settings.github_repo));
                }
                autoupdater::UpdateStatus::UpToDate => {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("✅ Установлена последняя версия").strong().color(Color32::from_rgb(34, 197, 94)));
                    });
                    ui.add_space(6.0);
                    ui.label(format!("Текущая версия PurgeCraft v{} является актуальной.", env!("CARGO_PKG_VERSION")));
                    ui.add_space(10.0);
                    if ui.button("Закрыть").clicked() {
                        app.update_modal_open = false;
                    }
                }
                autoupdater::UpdateStatus::Available(info) => {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("✨ Доступна новая версия!").strong().color(Color32::from_rgb(59, 130, 246)));
                        ui.label(RichText::new(format!("v{}", info.version)).strong());
                    });
                    ui.label(format!("Текущая версия: v{}  |  Новая: v{}", env!("CARGO_PKG_VERSION"), info.version));
                    ui.label(format!("Размер файла: {}", format_size(info.file_size)));
                    ui.add_space(6.0);

                    ui.group(|ui| {
                        ui.label(RichText::new("Описание изменений:").strong());
                        egui::ScrollArea::vertical().max_height(140.0).show(ui, |ui| {
                            ui.label(&info.release_notes);
                        });
                    });

                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.button(RichText::new("📥 Скачать и установить сейчас").strong().color(Color32::WHITE))
                            .on_hover_text("Автоматически загрузить и обновить текущий исполняемый файл")
                            .clicked()
                        {
                            autoupdater::download_and_apply_update_async(info.download_url.clone(), app.update_tx.clone());
                        }

                        if ui.button("🌐 Открыть на GitHub").clicked() {
                            let _ = open::that(&info.html_url);
                        }

                        if ui.button("Отмена").clicked() {
                            app.update_modal_open = false;
                        }
                    });
                }
                autoupdater::UpdateStatus::Downloading { percent, downloaded, total } => {
                    ui.label(RichText::new("Загрузка обновления...").strong());
                    ui.add_space(4.0);
                    ui.add(egui::ProgressBar::new(*percent).show_percentage());
                    ui.add_space(4.0);
                    ui.label(format!("Загружено: {} из {}", format_size(*downloaded), format_size(*total)));
                    ui.label(RichText::new("Пожалуйста, не закрывайте приложение — идет замена исполняемого файла.").color(Color32::from_rgb(234, 179, 8)));
                }
                autoupdater::UpdateStatus::ReadyToRestart => {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("🎉 Обновление успешно установлено!").strong().color(Color32::from_rgb(34, 197, 94)));
                    });
                    ui.add_space(6.0);
                    ui.label("Исполняемый файл PurgeCraft был успешно заменен на новую версию.");
                    ui.label("Перезапустите приложение для перехода на обновленную версию.");
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        if ui.button(RichText::new("🚀 Перезапустить сейчас").strong().color(Color32::WHITE)).clicked() {
                            if let Err(e) = autoupdater::restart_app() {
                                app.update_status = autoupdater::UpdateStatus::Error(e);
                            }
                        }
                        if ui.button("Позже").clicked() {
                            app.update_modal_open = false;
                        }
                    });
                }
                autoupdater::UpdateStatus::Error(err) => {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("❌ Ошибка при обновлении").strong().color(Color32::from_rgb(239, 68, 68)));
                    });
                    ui.add_space(4.0);
                    ui.label(err);
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        if ui.button("🔄 Повторить попытку").clicked() {
                            autoupdater::check_for_updates_async(
                                app.settings.github_repo.clone(),
                                env!("CARGO_PKG_VERSION").to_string(),
                                app.update_tx.clone(),
                            );
                        }
                        if ui.button("Закрыть").clicked() {
                            app.update_modal_open = false;
                        }
                    });
                }
            }
        });

    if !is_open {
        app.update_modal_open = false;
    }
}
