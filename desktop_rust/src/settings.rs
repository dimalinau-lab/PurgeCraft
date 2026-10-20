use std::path::PathBuf;
use winreg::enums::*;
use winreg::RegKey;
use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppSettings {
    // 1. Основные
    pub skip_uac_task: bool,
    pub explorer_context_menu: bool,
    pub check_updates: bool,
    pub check_beta: bool,
    #[serde(default = "default_github_repo")]
    pub github_repo: String,
    pub show_in_control_panel: bool,
    pub use_total_commander: bool,
    pub language: String,

    // 2. Интерфейс
    pub show_toolbar_labels: bool,
    pub compact_list: bool,
    pub mark_32bit: bool,
    pub theme_mode: String, // "system", "dark", "light"
    #[serde(default = "default_text_scale")]
    pub text_scale: f32,

    // 3. Деинсталлятор
    pub tree_updates_mode: String, // "collapsed", "expanded", "hidden"
    pub highlight_recent: bool,
    pub popup_recent: bool,
    pub show_recent_in_toolbar: bool,
    pub wizard_auto_start: String, // "auto", "ask", "never"
}

fn default_text_scale() -> f32 {
    1.15
}

fn default_github_repo() -> String {
    "PurgeCraft/PurgeCraft".to_string()
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            skip_uac_task: true,
            explorer_context_menu: false,
            check_updates: true,
            check_beta: false,
            github_repo: default_github_repo(),
            show_in_control_panel: true,
            use_total_commander: false,
            language: "Russian (Русский)".to_string(),

            show_toolbar_labels: true,
            compact_list: false,
            mark_32bit: true,
            theme_mode: "dark".to_string(),
            text_scale: 1.15,

            tree_updates_mode: "collapsed".to_string(),
            highlight_recent: true,
            popup_recent: true,
            show_recent_in_toolbar: true,
            wizard_auto_start: "auto".to_string(),
        }
    }
}

pub fn get_settings_path() -> PathBuf {
    if let Ok(mut base) = std::env::current_exe() {
        base.pop();
        return base.join("settings.json");
    }
    PathBuf::from("settings.json")
}

pub fn load_settings() -> AppSettings {
    let p = get_settings_path();
    if let Ok(content) = std::fs::read_to_string(&p) {
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        AppSettings::default()
    }
}

pub fn save_settings(settings: &AppSettings) {
    let p = get_settings_path();
    if let Ok(content) = serde_json::to_string_pretty(settings) {
        let _ = std::fs::write(&p, content);
    }
    apply_explorer_context_menu(settings.explorer_context_menu);
    apply_skip_uac_task(settings.skip_uac_task);
    apply_control_panel_icon(settings.show_in_control_panel);
}

pub fn apply_skip_uac_task(enable: bool) {
    let exe_path = match std::env::current_exe() {
        Ok(p) => p.to_string_lossy().to_string(),
        Err(_) => return,
    };
    #[cfg(windows)]
    use std::os::windows::process::CommandExt;

    if enable {
        let mut cmd = std::process::Command::new("schtasks");
        #[cfg(windows)]
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
        let _ = cmd.args([
            "/create", "/tn", "PurgeCraftSkipUAC",
            "/tr", &format!("\"{}\"", exe_path),
            "/sc", "onlogon",
            "/rl", "highest",
            "/f"
        ]).output();
    } else {
        let mut cmd = std::process::Command::new("schtasks");
        #[cfg(windows)]
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
        let _ = cmd.args(["/delete", "/tn", "PurgeCraftSkipUAC", "/f"]).output();
    }
}

pub fn apply_control_panel_icon(enable: bool) {
    let exe_path = match std::env::current_exe() {
        Ok(p) => p.to_string_lossy().to_string(),
        Err(_) => return,
    };
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let path = r"Software\Microsoft\Windows\CurrentVersion\Uninstall\PurgeCraft";
    if enable {
        if let Ok((key, _)) = hkcu.create_subkey(path) {
            let _ = key.set_value("DisplayName", &"PurgeCraft");
            let _ = key.set_value("DisplayIcon", &format!("\"{}\",0", exe_path));
            let _ = key.set_value("DisplayVersion", &"3.8.0");
            let _ = key.set_value("Publisher", &"PurgeCraft Systems");
            let _ = key.set_value("UninstallString", &format!("\"{}\" --uninstall", exe_path));
        }
    } else {
        let parent = r"Software\Microsoft\Windows\CurrentVersion\Uninstall";
        if let Ok(uninst) = hkcu.open_subkey_with_flags(parent, KEY_ALL_ACCESS) {
            let _ = uninst.delete_subkey_all("PurgeCraft");
        }
    }
}

pub fn apply_explorer_context_menu(enable: bool) {
    let exe_path = match std::env::current_exe() {
        Ok(p) => p.to_string_lossy().to_string(),
        Err(_) => return,
    };

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let targets = [
        r"Software\Classes\exefile\shell",
        r"Software\Classes\Msi.Package\shell",
    ];

    for parent in targets {
        if let Ok((shell_key, _)) = hkcu.create_subkey(parent) {
            if enable {
                if let Ok((track_key, _)) = shell_key.create_subkey("PurgeCraftTrack") {
                    let _ = track_key.set_value("", &"Установить и следить (PurgeCraft)");
                    let _ = track_key.set_value("Icon", &format!("\"{}\",0", exe_path));
                    if let Ok((cmd_key, _)) = track_key.create_subkey("command") {
                        let _ = cmd_key.set_value("", &format!("\"{}\" --track \"%1\"", exe_path));
                    }
                }
            } else {
                let _ = shell_key.delete_subkey_all("PurgeCraftTrack");
            }
        }
    }
}

pub fn is_windows_dark_mode() -> bool {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok(key) = hkcu.open_subkey(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize") {
        if let Ok(val) = key.get_value::<u32, _>("AppsUseLightTheme") {
            return val == 0;
        }
    }
    true
}
