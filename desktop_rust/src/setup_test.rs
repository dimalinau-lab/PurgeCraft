#![windows_subsystem = "windows"]

use std::path::{Path, PathBuf};
use winreg::enums::*;
use winreg::RegKey;
use eframe::egui::{self, Color32, RichText};

fn get_local_dir() -> PathBuf {
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        Path::new(&local).join("PurgeCraftTestApp")
    } else {
        PathBuf::from(r"C:\PurgeCraftTestApp")
    }
}

fn get_roaming_dir() -> PathBuf {
    if let Ok(appdata) = std::env::var("APPDATA") {
        Path::new(&appdata).join("PurgeCraftTestApp")
    } else {
        PathBuf::from(r"C:\PurgeCraftTestAppRoaming")
    }
}

struct AppStatus {
    is_uninstall_registered: bool,
    is_vendor_reg_present: bool,
    is_autorun_present: bool,
    local_dir_exists: bool,
    roaming_dir_exists: bool,
    local_size: u64,
}

fn check_status() -> AppStatus {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let is_uninstall_registered = hkcu.open_subkey(r"Software\Microsoft\Windows\CurrentVersion\Uninstall\PurgeCraftTestApp").is_ok();
    let is_vendor_reg_present = hkcu.open_subkey(r"Software\PurgeCraftTestApp").is_ok();
    let is_autorun_present = hkcu.open_subkey(r"Software\Microsoft\Windows\CurrentVersion\Run")
        .and_then(|k| k.get_value::<String, _>("PurgeCraftTestApp"))
        .is_ok();

    let local_p = get_local_dir();
    let roaming_p = get_roaming_dir();
    let local_dir_exists = local_p.exists();
    let roaming_dir_exists = roaming_p.exists();

    let mut local_size = 0;
    if local_dir_exists {
        if let Ok(entries) = std::fs::read_dir(&local_p) {
            for entry in entries.flatten() {
                if let Ok(m) = entry.metadata() {
                    local_size += m.len();
                }
            }
        }
    }

    AppStatus {
        is_uninstall_registered,
        is_vendor_reg_present,
        is_autorun_present,
        local_dir_exists,
        roaming_dir_exists,
        local_size,
    }
}

fn install_test_app() -> Result<String, String> {
    let local_dir = get_local_dir();
    let roaming_dir = get_roaming_dir();

    std::fs::create_dir_all(&local_dir).map_err(|e| format!("Не удалось создать папку Local: {}", e))?;
    std::fs::create_dir_all(&roaming_dir).map_err(|e| format!("Не удалось создать папку Roaming: {}", e))?;
    let cache_dir = local_dir.join("cache");
    let _ = std::fs::create_dir_all(&cache_dir);

    let current_exe = std::env::current_exe().map_err(|e| format!("Ошибка пути exe: {}", e))?;
    let test_app_exe = local_dir.join("test_app.exe");
    let uninst_exe = local_dir.join("uninstall.exe");

    let _ = std::fs::copy(&current_exe, &test_app_exe);
    let _ = std::fs::copy(&current_exe, &uninst_exe);

    // Dummy application files
    let _ = std::fs::write(local_dir.join("config.json"), r#"{"app":"PurgeCraftTestApp","version":"1.0.0","channel":"qa"}"#);
    let _ = std::fs::write(local_dir.join("userData_leftover.dat"), "Важные кэшированные данные пользователя (должны остаться для мастера).");
    let _ = std::fs::write(cache_dir.join("cache_chunk_01.bin"), vec![0xAB; 512 * 1024]);
    let _ = std::fs::write(roaming_dir.join("settings.json"), r#"{"theme":"dark","last_sync":"2026-10-20T14:00:00"}"#);
    let _ = std::fs::write(roaming_dir.join("user_telemetry.log"), "2026-10-20 Session started successfully\n2026-10-20 Ping ok\n");

    // Realistic payload file ~15 MB
    let dummy_data = local_dir.join("data_package.bin");
    if let Ok(mut f) = std::fs::File::create(&dummy_data) {
        use std::io::Write;
        let chunk = vec![0x55; 1024 * 1024];
        for _ in 0..15 {
            let _ = f.write_all(&chunk);
        }
    }

    // Registry Entries
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);

    // 1. Uninstall key
    let uninst_path = r"Software\Microsoft\Windows\CurrentVersion\Uninstall\PurgeCraftTestApp";
    if let Ok((key, _)) = hkcu.create_subkey(uninst_path) {
        let _ = key.set_value("DisplayName", &"PurgeCraft Test Application");
        let _ = key.set_value("DisplayVersion", &"1.0.0");
        let _ = key.set_value("Publisher", &"PurgeCraft QA Lab");
        let _ = key.set_value("InstallDate", &"20261020");
        let _ = key.set_value("InstallLocation", &local_dir.to_string_lossy().as_ref());
        let _ = key.set_value("UninstallString", &format!("\"{}\" --uninstall", uninst_exe.display()));
        let _ = key.set_value("DisplayIcon", &format!("\"{}\",0", test_app_exe.display()));
        let _ = key.set_value("EstimatedSize", &16384u32); // 16 MB
    }

    // 2. Vendor settings key
    let vendor_path = r"Software\PurgeCraftTestApp";
    if let Ok((key, _)) = hkcu.create_subkey(vendor_path) {
        let _ = key.set_value("Installed", &1u32);
        let _ = key.set_value("LicenseKey", &"PURGECRAFT-DEMO-2026-KEY-991");
        let _ = key.set_value("InstallPath", &local_dir.to_string_lossy().as_ref());
        let _ = key.set_value("TelemetryToken", &"d3b07384d113edec49eaa6238ad5ff00");
    }

    // 3. Autorun
    let run_path = r"Software\Microsoft\Windows\CurrentVersion\Run";
    if let Ok((key, _)) = hkcu.create_subkey(run_path) {
        let _ = key.set_value("PurgeCraftTestApp", &format!("\"{}\" --app", test_app_exe.display()));
    }

    Ok("Тестовое приложение успешно установлено!".to_string())
}

fn realistic_uninstall() {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);

    // 1. Remove Uninstall key
    if let Ok(uninst) = hkcu.open_subkey_with_flags(r"Software\Microsoft\Windows\CurrentVersion\Uninstall", KEY_ALL_ACCESS) {
        let _ = uninst.delete_subkey_all("PurgeCraftTestApp");
    }

    // 2. Remove Run key
    if let Ok(run) = hkcu.open_subkey_with_flags(r"Software\Microsoft\Windows\CurrentVersion\Run", KEY_ALL_ACCESS) {
        let _ = run.delete_value("PurgeCraftTestApp");
    }

    // 3. Remove main exe and main data package
    let local_dir = get_local_dir();
    let _ = std::fs::remove_file(local_dir.join("test_app.exe"));
    let _ = std::fs::remove_file(local_dir.join("data_package.bin"));

    // INTENTIONALLY LEAVING BEHIND:
    // - local_dir\cache\cache_chunk_01.bin
    // - local_dir\userData_leftover.dat
    // - roaming_dir\settings.json
    // - roaming_dir\user_telemetry.log
    // - HKCU\Software\PurgeCraftTestApp (Vendor registry key)
    // PurgeCraft Leftovers Wizard will detect and clean all of them!
}

fn full_cleanup() {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok(uninst) = hkcu.open_subkey_with_flags(r"Software\Microsoft\Windows\CurrentVersion\Uninstall", KEY_ALL_ACCESS) {
        let _ = uninst.delete_subkey_all("PurgeCraftTestApp");
    }
    if let Ok(vendor) = hkcu.open_subkey_with_flags(r"Software", KEY_ALL_ACCESS) {
        let _ = vendor.delete_subkey_all("PurgeCraftTestApp");
    }
    if let Ok(run) = hkcu.open_subkey_with_flags(r"Software\Microsoft\Windows\CurrentVersion\Run", KEY_ALL_ACCESS) {
        let _ = run.delete_value("PurgeCraftTestApp");
    }
    let _ = std::fs::remove_dir_all(get_local_dir());
    let _ = std::fs::remove_dir_all(get_roaming_dir());
}

// ---------------------------------------------------------------------------------
// GUI 1: Native Uninstaller Simulation (Run when --uninstall is passed)
// ---------------------------------------------------------------------------------

struct UninstallerApp {
    phase: UninstallerPhase,
    progress: f32,
}

enum UninstallerPhase {
    Confirm,
    Uninstalling,
    Done,
}

impl eframe::App for UninstallerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.set_width(460.0);
            match self.phase {
                UninstallerPhase::Confirm => {
                    ui.heading(RichText::new("Мастер удаления PurgeCraft Test Application").strong().size(16.0));
                    ui.add_space(8.0);
                    ui.label("Вас приветствует мастер удаления программы.");
                    ui.label("Вы действительно хотите удалить «PurgeCraft Test Application» и все ее компоненты с вашего компьютера?");
                    ui.add_space(16.0);
                    ui.separator();
                    ui.add_space(12.0);

                    ui.horizontal(|ui| {
                        if ui.button(RichText::new("Отмена").size(13.0)).clicked() {
                            std::process::exit(1);
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let del_btn = egui::Button::new(RichText::new("Удалить").strong().size(13.0).color(Color32::WHITE))
                                .fill(Color32::from_rgb(220, 38, 38));
                            if ui.add(del_btn).clicked() {
                                self.phase = UninstallerPhase::Uninstalling;
                            }
                        });
                    });
                }
                UninstallerPhase::Uninstalling => {
                    ui.heading(RichText::new("Удаление компонентов программы...").size(15.0));
                    ui.add_space(10.0);
                    ui.label("Идет удаление исполняемых файлов и ключей регистрации...");
                    ui.add_space(12.0);

                    self.progress += 0.05;
                    let pb = egui::ProgressBar::new(self.progress.min(1.0))
                        .show_percentage()
                        .fill(Color32::from_rgb(34, 197, 94));
                    ui.add(pb);

                    if self.progress >= 1.0 {
                        realistic_uninstall();
                        self.phase = UninstallerPhase::Done;
                    }
                    ctx.request_repaint_after(std::time::Duration::from_millis(50));
                }
                UninstallerPhase::Done => {
                    ui.heading(RichText::new("✔ Деинсталляция завершена").size(16.0).color(Color32::from_rgb(34, 197, 94)).strong());
                    ui.add_space(8.0);
                    ui.label("Штатный деинсталлятор программы завершил свою работу.");
                    ui.add_space(6.0);
                    ui.label(RichText::new("Обратите внимание: штатный деинсталлятор удалил только основной exe, оставив пользовательские файлы в AppData и ключ настроек в HKCU\\Software!").small().color(Color32::from_rgb(234, 179, 8)));
                    ui.label("Теперь PurgeCraft автоматически запустит «Мастер Удаления ™» для их обнаружения.");
                    ui.add_space(18.0);

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(RichText::new("Готово").strong().size(14.0)).clicked() {
                            std::process::exit(0);
                        }
                    });
                }
            }
        });
    }
}

// ---------------------------------------------------------------------------------
// GUI 2: Test Lab Controller (When run directly)
// ---------------------------------------------------------------------------------

struct SetupTestLabApp {
    status: AppStatus,
    msg: Option<(String, bool)>,
}

impl SetupTestLabApp {
    fn new() -> Self {
        Self {
            status: check_status(),
            msg: None,
        }
    }
}

impl eframe::App for SetupTestLabApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.set_width(540.0);

            ui.heading(RichText::new("🧪 PurgeCraft QA TestLab — Тестовое Приложение").size(18.0).strong());
            ui.label("Утилита для полной проверки жизненного цикла программы в PurgeCraft.");
            ui.add_space(10.0);
            ui.separator();
            ui.add_space(8.0);

            // System Status Card
            ui.group(|ui| {
                ui.strong("Текущий статус в системе Windows:");
                ui.add_space(6.0);

                let render_indicator = |ui: &mut egui::Ui, name: &str, active: bool, detail: &str| {
                    ui.horizontal(|ui| {
                        let icon = if active { "✔" } else { "✖" };
                        let color = if active { Color32::from_rgb(34, 197, 94) } else { Color32::from_rgb(156, 163, 175) };
                        ui.label(RichText::new(icon).color(color).strong());
                        ui.label(name);
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(RichText::new(detail).small().color(color));
                        });
                    });
                };

                render_indicator(ui, "Запись деинсталляции в Windows (Uninstall)", self.status.is_uninstall_registered, if self.status.is_uninstall_registered { "Зарегистрирована" } else { "Отсутствует" });
                render_indicator(ui, "Ветка настроек вендора (HKCU\\Software\\PurgeCraftTestApp)", self.status.is_vendor_reg_present, if self.status.is_vendor_reg_present { "Присутствует" } else { "Отсутствует" });
                render_indicator(ui, "Пункт автозагрузки (HKCU\\Run)", self.status.is_autorun_present, if self.status.is_autorun_present { "Включен" } else { "Отключен" });
                render_indicator(ui, "Каталог данных %LOCALAPPDATA%\\PurgeCraftTestApp", self.status.local_dir_exists, &format!("Размер: {:.2} МБ", self.status.local_size as f64 / (1024.0 * 1024.0)));
                render_indicator(ui, "Каталог настроек %APPDATA%\\PurgeCraftTestApp", self.status.roaming_dir_exists, if self.status.roaming_dir_exists { "Файлы созданы" } else { "Отсутствует" });
            });

            ui.add_space(12.0);

            // Action Buttons
            ui.horizontal(|ui| {
                let install_btn = egui::Button::new(RichText::new("🚀 1. Установить приложение").strong().size(13.0).color(Color32::WHITE))
                    .fill(Color32::from_rgb(37, 99, 235));
                if ui.add(install_btn).clicked() {
                    match install_test_app() {
                        Ok(m) => {
                            self.msg = Some((m, true));
                            self.status = check_status();
                        }
                        Err(e) => {
                            self.msg = Some((e, false));
                        }
                    }
                }

                let uninst_enabled = self.status.is_uninstall_registered || self.status.local_dir_exists;
                let uninst_btn = egui::Button::new(RichText::new("🗑 2. Штатный деинсталлятор").size(13.0));
                if ui.add_enabled(uninst_enabled, uninst_btn).clicked() {
                    let local = get_local_dir();
                    let uninst_exe = local.join("uninstall.exe");
                    let _ = std::process::Command::new(uninst_exe).arg("--uninstall").spawn();
                }

                if ui.button(RichText::new("♻ Очистить всё").size(13.0)).on_hover_text("Полная очистка реестра и папок без следов").clicked() {
                    full_cleanup();
                    self.status = check_status();
                    self.msg = Some(("Все файлы и ветки реестра полностью удалены.".to_string(), true));
                }
            });

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button("📁 Открыть папку LocalAppData").clicked() {
                    let _ = open::that(get_local_dir());
                }
                if ui.button("📁 Открыть папку Roaming").clicked() {
                    let _ = open::that(get_roaming_dir());
                }
                if ui.button("🗄 Открыть Regedit").clicked() {
                    let _ = std::process::Command::new("regedit").spawn();
                }
            });

            if let Some((msg, success)) = &self.msg {
                ui.add_space(10.0);
                let col = if *success { Color32::from_rgb(34, 197, 94) } else { Color32::from_rgb(239, 68, 68) };
                ui.label(RichText::new(msg).color(col).strong());
            }

            ui.add_space(14.0);
            ui.group(|ui| {
                ui.strong("Сценарий проверки в PurgeCraft:");
                ui.label("1. Нажмите «🚀 1. Установить приложение» выше.");
                ui.label("2. Запустите PurgeCraft — программа появится в самом верху с бейджем ✨ НОВОЕ.");
                ui.label("3. Выберите «Деинсталляция» — PurgeCraft запустит штатный деинсталлятор и перейдет в режим ожидания.");
                ui.label("4. В деинсталляторе нажмите «Удалить» и «Готово».");
                ui.label("5. PurgeCraft автоматически откроет Мастер удаления остатков, покажет зеленую шкалу поиска, найдет оставшиеся ветки реестра и файлы AppData, и подчистую вычистит их!");
            });
        });
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.iter().any(|a| a == "--install") {
        match install_test_app() {
            Ok(m) => println!("{}", m),
            Err(e) => eprintln!("Error: {}", e),
        }
        return;
    }

    if args.iter().any(|a| a == "--clean" || a == "--cleanup") {
        full_cleanup();
        println!("Cleanup completed.");
        return;
    }

    if args.iter().any(|a| a == "--uninstall") {
        let options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([480.0, 240.0])
                .with_resizable(false)
                .with_title("Мастер удаления PurgeCraft Test Application"),
            ..Default::default()
        };
        let _ = eframe::run_native(
            "PurgeCraft Test Application Uninstaller",
            options,
            Box::new(|_cc| Ok(Box::new(UninstallerApp {
                phase: UninstallerPhase::Confirm,
                progress: 0.0,
            }))),
        );
        return;
    }

    if args.iter().any(|a| a == "--app") {
        let options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([420.0, 180.0])
                .with_resizable(false)
                .with_title("PurgeCraft Test Application"),
            ..Default::default()
        };
        let _ = eframe::run_native(
            "PurgeCraft Test Application",
            options,
            Box::new(|_cc| Ok(Box::new(UninstallerApp {
                phase: UninstallerPhase::Done,
                progress: 1.0,
            }))),
        );
        return;
    }

    // Default: TestLab Control Center
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([580.0, 480.0])
            .with_resizable(true)
            .with_title("PurgeCraft QA TestLab"),
        ..Default::default()
    };
    let _ = eframe::run_native(
        "PurgeCraft QA TestLab",
        options,
        Box::new(|_cc| Ok(Box::new(SetupTestLabApp::new()))),
    );
}
