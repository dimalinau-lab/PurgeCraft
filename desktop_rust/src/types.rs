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
    let full_path = if let Some(rest) = registry_path.strip_prefix("HKLM") {
        format!("Computer\\HKEY_LOCAL_MACHINE{}", rest)
    } else if let Some(rest) = registry_path.strip_prefix("HKCU") {
        format!("Computer\\HKEY_CURRENT_USER{}", rest)
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
                if sz > 0 && tx.send((id, sz)).is_err() {
                    break;
                }
            }
        }
    });
}

pub fn format_size_lang(bytes: u64, lang: crate::i18n::Language) -> String {
    if bytes == 0 {
        return "—".to_string();
    }
    let mb = bytes as f64 / (1024.0 * 1024.0);
    let (mb_unit, gb_unit) = match lang {
        crate::i18n::Language::Russian => ("МБ", "ГБ"),
        crate::i18n::Language::English => ("MB", "GB"),
    };
    if mb < 1000.0 {
        format!("{:.1} {}", mb, mb_unit)
    } else {
        format!("{:.2} {}", mb / 1024.0, gb_unit)
    }
}

pub fn format_size(bytes: u64) -> String {
    format_size_lang(bytes, crate::i18n::Language::Russian)
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

/// Returns the prefix slice of `s` ending right after the first ASCII case-insensitive match of `needle_ascii`.
/// Guarantees valid UTF-8 char boundaries and performs zero heap allocations.
pub fn slice_through_ascii_ignore_case<'a>(s: &'a str, needle_ascii: &str) -> Option<&'a str> {
    debug_assert!(needle_ascii.is_ascii());
    let n_bytes = needle_ascii.as_bytes();
    if n_bytes.is_empty() || s.len() < n_bytes.len() {
        return None;
    }
    s.as_bytes()
        .windows(n_bytes.len())
        .position(|w| w.eq_ignore_ascii_case(n_bytes))
        .map(|pos| &s[..pos + n_bytes.len()])
}

/// Extracts the executable path prefix (up to `.exe`) from a command or uninstall string.
pub fn extract_exe_prefix(raw: &str) -> &str {
    let clean = raw.trim().trim_matches('"');
    slice_through_ascii_ignore_case(clean, ".exe").unwrap_or(clean)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_exe_prefix_handles_unicode_and_args() {
        assert_eq!(
            extract_exe_prefix(r#""C:\Program Files\Игры\İApp\unins000.EXE" /SILENT"#),
            r#"C:\Program Files\Игры\İApp\unins000.EXE"#
        );
        assert_eq!(
            extract_exe_prefix(r#"C:\Tools\app.exe --uninstall"#),
            r#"C:\Tools\app.exe"#
        );
        assert_eq!(
            extract_exe_prefix(r#"msiexec /X{1234}"#),
            r#"msiexec /X{1234}"#
        );
    }

    #[test]
    fn test_slice_through_ascii_ignore_case_edge_cases() {
        assert_eq!(slice_through_ascii_ignore_case("", ".exe"), None);
        assert_eq!(slice_through_ascii_ignore_case("app.exe", ""), None);
        assert_eq!(slice_through_ascii_ignore_case("a.e", ".exe"), None);
        assert_eq!(
            slice_through_ascii_ignore_case(r"C:\sys\driver.SYS /flag", ".sys "),
            Some(r"C:\sys\driver.SYS ")
        );
    }

    #[test]
    fn test_truncate_chars_unicode() {
        assert_eq!(truncate_chars("Привет мир!", 8), "Приве...");
        assert_eq!(truncate_chars("abc", 5), "abc");
    }
}
