use std::process::Command;
use crate::scanner::{self, InstalledApp};

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum WizardPhase {
    Scanning,
    Review,
    Deleting,
    Finished,
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum ActiveTab {
    Uninstaller,
    Startup,
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum UninstallerViewMode {
    List,
    DiskTreemap,
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum SettingsTab {
    General,
    Interface,
    Uninstaller,
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum AppCategory {
    All,
    UserApps,
    System,
    StoreApps,
    Portable,
    Arch64,
    Arch32,
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum StartupFilter {
    All,
    Registry,
    StartMenu,
    TaskScheduler,
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum TrackerStep {
    SelectInstaller,
    Installing,
    ReviewTraces,
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum SortColumn {
    Name,
    Size,
    Date,
    Publisher,
    AppType,
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum SortOrder {
    Ascending,
    Descending,
}

pub fn jump_to_regedit(registry_path: &str) {
    if registry_path.is_empty() {
        let _ = Command::new("regedit").spawn();
        return;
    }
    let full_path = if registry_path.starts_with("HKLM") {
        format!("Computer\\HKEY_LOCAL_MACHINE{}", &registry_path[4..])
    } else if registry_path.starts_with("HKCU") {
        format!("Computer\\HKEY_CURRENT_USER{}", &registry_path[4..])
    } else {
        format!("Computer\\{}", registry_path)
    };

    let hkcu = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER);
    if let Ok((key, _)) = hkcu.create_subkey(r"Software\Microsoft\Windows\CurrentVersion\Applets\Regedit") {
        let _ = key.set_value("LastKey", &full_path);
    }
    let _ = Command::new("regedit").spawn();
}

pub fn open_system_folder(path: &str, use_total_commander: bool) {
    if path.is_empty() {
        return;
    }
    let p = std::path::Path::new(path);
    let target = if p.is_file() {
        p.parent().unwrap_or(p)
    } else {
        p
    };
    let target_str = target.to_string_lossy();

    if use_total_commander {
        let tc_candidates = [
            r"C:\totalcmd\TOTALCMD64.EXE",
            r"C:\totalcmd\TOTALCMD.EXE",
            r"C:\Program Files\totalcmd\TOTALCMD64.EXE",
            r"C:\Program Files\totalcmd\TOTALCMD.EXE",
            r"C:\Program Files (x86)\totalcmd\TOTALCMD.EXE",
        ];
        for tc in tc_candidates {
            if std::path::Path::new(tc).exists() {
                let _ = std::process::Command::new(tc)
                    .args(["/O", "/T", &format!("/L={}", target_str)])
                    .spawn();
                return;
            }
        }
    }
    let _ = open::that(target);
}

pub fn spawn_size_calculator(apps: &[InstalledApp], tx: std::sync::mpsc::Sender<(String, u64)>) {
    let targets: Vec<(String, String)> = apps.iter()
        .filter(|a| a.size_bytes == 0 && !a.install_location.is_empty())
        .map(|a| (a.id.clone(), a.install_location.clone()))
        .collect();

    if targets.is_empty() {
        return;
    }

    std::thread::spawn(move || {
        for (id, loc) in targets {
            let p = std::path::Path::new(&loc);
            let dir = if p.is_file() { p.parent().unwrap_or(p) } else { p };
            if scanner::is_safe_app_dir(dir) {
                let sz = scanner::get_folder_size(dir);
                if sz > 0 {
                    if tx.send((id, sz)).is_err() {
                        break;
                    }
                }
            }
        }
    });
}

pub fn format_size(bytes: u64) -> String {
    if bytes == 0 {
        return "—".to_string();
    }
    let mb = bytes as f64 / (1024.0 * 1024.0);
    if mb < 1000.0 {
        format!("{:.1} МБ", mb)
    } else {
        format!("{:.2} ГБ", mb / 1024.0)
    }
}

pub fn truncate_chars(s: &str, max_chars: usize) -> String {
    let char_count = s.chars().count();
    if char_count > max_chars {
        let truncated: String = s.chars().take(max_chars.saturating_sub(3)).collect();
        format!("{}...", truncated)
    } else {
        s.to_string()
    }
}
