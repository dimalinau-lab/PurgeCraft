use std::path::{Path, PathBuf};
use std::process::Command;
use std::collections::HashSet;
#[cfg(windows)]
use std::os::windows::process::CommandExt;
use winreg::enums::*;
use winreg::RegKey;
use serde::Deserialize;

pub const CREATE_NO_WINDOW: u32 = 0x08000000;

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct InstalledApp {
    pub id: String,
    pub name: String,
    pub version: String,
    pub publisher: String,
    pub install_date: String,
    pub size_bytes: u64,
    pub install_location: String,
    pub uninstall_string: String,
    pub registry_path: String,
    pub is_64bit: bool,
    pub is_system_component: bool,
    pub is_uwp: bool,
    pub package_full_name: String,
    pub display_icon: String,
    pub is_new: bool,
    pub source_badge: String, // e.g. "[Steam]", "[Epic]", "[Store]", "[Winget]", ""
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct RunningProcessInfo {
    pub pid: u32,
    pub name: String,
    pub exe_path: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum StartupImpact {
    High,    // ⚡ Высокое (> 1-2 сек или тяжелый launcher/browser)
    Medium,  // 🟡 Среднее (0.3 - 1 сек, background sync, updater)
    Low,     // 🟢 Низкое (< 0.3 сек, легкие tray applets, native runners)
    None,    // ⚪ Не измерено
}

impl StartupImpact {
    #[allow(dead_code)]
    pub fn label(&self) -> &'static str {
        match self {
            StartupImpact::High => "⚡ Высокое",
            StartupImpact::Medium => "🟡 Среднее",
            StartupImpact::Low => "🟢 Низкое",
            StartupImpact::None => "⚪ Не измерено",
        }
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct StartupItem {
    pub id: String,
    pub name: String,
    pub command: String,
    pub source: String,           // e.g. "Реестр (Все пользователи)", "Реестр (Текущий)", "Меню Пуск", "Планировщик"
    pub source_type: String,      // "hklm_run", "hkcu_run", "startmenu", "taskscheduler"
    pub status: String,           // "enabled", "disabled"
    pub is_enabled: bool,
    pub run_as_admin: bool,
    pub raw_target: String,       // Registry val_name, full shortcut path, or task name
    pub impact: StartupImpact,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ShaderCacheLocation {
    pub id: String,
    pub name: String,
    pub vendor: String, // "DirectX", "NVIDIA", "AMD"
    pub path: String,
    pub file_count: usize,
    pub size_bytes: u64,
    pub selected: bool,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PathEntry {
    pub path: String,
    pub is_system: bool,
    pub exists: bool,
    pub is_duplicate: bool,
    pub selected: bool,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PortableApp {
    pub name: String,
    pub exe_path: String,
    pub folder_path: String,
    pub size_bytes: u64,
    pub file_count: usize,
    pub modified_date: String,
    pub selected: bool,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RuntimeItem {
    pub family: String,      // "Visual C++", "Java", ".NET Runtime"
    pub version: String,
    pub arch: String,        // "x64", "x86"
    pub display_name: String,
    pub app_id: String,
    pub is_deprecated: bool,
    pub uninstall_string: String,
    pub size_bytes: u64,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RuntimeFamilyGroup {
    pub family_name: String,
    pub items: Vec<RuntimeItem>,
    pub duplicate_count: usize,
    pub has_deprecated: bool,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct WingetAppItem {
    pub name: String,
    pub id: String,
    pub installed_version: String,
    pub available_version: Option<String>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct LeftoverTrace {
    pub id: String,
    pub app_id: String,
    pub trace_type: String, // "registry_key", "folder", "file", "appx_package"
    pub path: String,
    pub size_bytes: u64,
    pub confidence: u32,
    pub selected: bool,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct TracedApp {
    pub id: String,
    pub name: String,
    pub install_date: String,
    pub installer_path: String,
    pub traces: Vec<LeftoverTrace>,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct SystemSnapshot {
    pub timestamp: u64,
    pub uninstall_keys_hklm: HashSet<String>,
    pub uninstall_keys_hkcu: HashSet<String>,
    pub run_keys_hklm: HashSet<String>,
    pub run_keys_hkcu: HashSet<String>,
    pub software_subkeys_hklm: HashSet<String>,
    pub software_subkeys_hkcu: HashSet<String>,
    pub program_files_dirs: HashSet<String>,
    pub appdata_local_dirs: HashSet<String>,
    pub appdata_roaming_dirs: HashSet<String>,
    pub temp_items: HashSet<String>,
}

#[derive(Clone, Debug)]
pub struct TrackerLiveMsg {
    pub time: String,
    pub action: String,
    pub path: String,
}

#[derive(Deserialize)]
struct RawAppx {
    #[serde(default, rename = "Name")]
    name: String,
    #[serde(default, rename = "Version")]
    version: String,
    #[serde(default, rename = "Publisher")]
    publisher: String,
    #[serde(default, rename = "PackageFullName")]
    package_full_name: String,
    #[serde(default, rename = "InstallLocation")]
    install_location: String,
}

fn get_string_value(key: &RegKey, name: &str) -> String {
    key.get_value::<String, _>(name).unwrap_or_default()
}

fn get_u64_value(key: &RegKey, name: &str) -> u64 {
    key.get_value::<u32, _>(name).map(|v| v as u64).unwrap_or(0)
}

pub fn clean_uwp_package_name(raw_name: &str) -> String {
    let trimmed = raw_name.trim();
    if !trimmed.contains('.') {
        return trimmed.to_string();
    }

    let mut s = trimmed;
    // Strip numeric publisher ID prefix like "40459File-New-Project.EarTrumpet" or "21090PaddyXu.QuickLook"
    if let Some(dot_pos) = s.find('.') {
        let prefix = &s[..dot_pos];
        if prefix.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
            s = &s[dot_pos + 1..];
        }
    }

    let parts: Vec<&str> = s.split('.').filter(|p| !p.is_empty()).collect();
    if parts.is_empty() {
        return trimmed.to_string();
    }
    if parts.len() == 1 {
        return parts[0].to_string();
    }

    let last_is_numeric = parts.last().map(|p| p.chars().all(|c| c.is_ascii_digit())).unwrap_or(false);

    if parts[0].eq_ignore_ascii_case("Microsoft") || parts[0].eq_ignore_ascii_case("MicrosoftCorporationII") {
        parts.join(" ")
    } else if last_is_numeric {
        parts.join(" ")
    } else if parts.len() >= 2 {
        let last = parts.last().unwrap();
        if last.len() >= 3 {
            last.to_string()
        } else {
            parts.join(" ")
        }
    } else {
        parts.join(" ")
    }
}

pub fn scan_uwp_apps() -> Vec<InstalledApp> {
    let mut uwp_apps = Vec::new();

    // Query non-framework UWP packages via PowerShell JSON
    let cmd = r#"Get-AppxPackage | Where-Object { -not $_.IsFramework -and -not $_.NonRemovable } | Select-Object Name, Version, Publisher, PackageFullName, InstallLocation | ConvertTo-Json -Compress"#;
    let mut command = Command::new("powershell");
    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);
    let output = command
        .args(["-NoProfile", "-NonInteractive", "-Command", cmd])
        .output();

    if let Ok(out) = output {
        if out.status.success() {
            let json_str = String::from_utf8_lossy(&out.stdout);
            let trimmed = json_str.trim();
            if !trimmed.is_empty() {
                let list: Vec<RawAppx> = if trimmed.starts_with('[') {
                    serde_json::from_str(trimmed).unwrap_or_default()
                } else if trimmed.starts_with('{') {
                    serde_json::from_str::<RawAppx>(trimmed)
                        .map(|item| vec![item])
                        .unwrap_or_default()
                } else {
                    Vec::new()
                };

                for item in list {
                    if item.name.is_empty() || item.package_full_name.is_empty() {
                        continue;
                    }

                    // Clean human-friendly name
                    let clean_name = clean_uwp_package_name(&item.name);

                    // Clean publisher string: extract CN value if present
                    let clean_publisher = if item.publisher.contains("CN=") {
                        item.publisher
                            .split(',')
                            .find(|p| p.trim().starts_with("CN="))
                            .map(|p| p.trim().replace("CN=", ""))
                            .unwrap_or_else(|| item.publisher.clone())
                    } else {
                        item.publisher.clone()
                    };

                    let size_bytes = if !item.install_location.is_empty() {
                        get_folder_size(Path::new(&item.install_location))
                    } else {
                        0
                    };

                    uwp_apps.push(InstalledApp {
                        id: item.package_full_name.clone(),
                        name: clean_name,
                        version: item.version,
                        publisher: clean_publisher,
                        install_date: "Магазин Windows".to_string(),
                        size_bytes,
                        install_location: item.install_location,
                        uninstall_string: format!(
                            "powershell -NoProfile -Command \"Remove-AppxPackage -Package '{}'\"",
                            item.package_full_name
                        ),
                        registry_path: format!("AppX\\{}", item.package_full_name),
                        is_64bit: true,
                        is_system_component: false,
                        is_uwp: true,
                        package_full_name: item.package_full_name,
                        display_icon: String::new(),
                        is_new: false,
                        source_badge: "[Store]".to_string(),
                    });
                }
            }
        }
    }

    uwp_apps
}

pub fn get_steam_install_path() -> Option<String> {
    // Check HKCU\Software\Valve\Steam:SteamPath
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok(key) = hkcu.open_subkey(r"Software\Valve\Steam") {
        if let Ok(path) = key.get_value::<String, _>("SteamPath") {
            let p = path.replace('/', "\\");
            if Path::new(&p).exists() {
                return Some(p);
            }
        }
    }
    // Check HKLM\SOFTWARE\WOW6432Node\Valve\Steam:InstallPath
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    if let Ok(key) = hklm.open_subkey_with_flags(r"SOFTWARE\WOW6432Node\Valve\Steam", KEY_READ | KEY_WOW64_32KEY) {
        if let Ok(path) = key.get_value::<String, _>("InstallPath") {
            if Path::new(&path).exists() {
                return Some(path);
            }
        }
    }
    for cand in [r"C:\Program Files (x86)\Steam", r"C:\Steam", r"D:\Steam"] {
        if Path::new(cand).exists() {
            return Some(cand.to_string());
        }
    }
    None
}

#[derive(Clone, Debug)]
pub struct SteamGameDetails {
    pub install_location: String,
    pub size_bytes: u64,
    #[allow(dead_code)]
    pub acf_path: Option<PathBuf>,
}

pub fn get_all_steam_library_folders() -> Vec<PathBuf> {
    let steam_path = match get_steam_install_path() {
        Some(p) => p,
        None => return Vec::new(),
    };

    let mut library_dirs = Vec::new();
    let main_pb = PathBuf::from(&steam_path);
    library_dirs.push(main_pb);

    let vdf_path = Path::new(&steam_path).join("steamapps").join("libraryfolders.vdf");
    if vdf_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&vdf_path) {
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("\"path\"") {
                    let parts: Vec<&str> = trimmed.split('"').collect();
                    if parts.len() >= 4 {
                        let raw_p = parts[3].replace(r"\\", r"\");
                        let pb = PathBuf::from(&raw_p);
                        if pb.exists() && !library_dirs.contains(&pb) {
                            library_dirs.push(pb);
                        }
                    }
                }
            }
        }
    }
    library_dirs
}

pub fn find_steam_game_details(steam_id: &str) -> Option<SteamGameDetails> {
    let clean_id = steam_id.trim();
    if clean_id.is_empty() {
        return None;
    }
    let manifest_name = format!("appmanifest_{}.acf", clean_id);

    for lib_dir in get_all_steam_library_folders() {
        let steamapps = lib_dir.join("steamapps");
        let acf = steamapps.join(&manifest_name);
        if acf.is_file() {
            if let Ok(content) = std::fs::read_to_string(&acf) {
                let mut installdir = String::new();
                let mut size_on_disk = 0u64;

                for line in content.lines() {
                    let l = line.trim();
                    if l.starts_with("\"installdir\"") {
                        let parts: Vec<&str> = l.split('"').collect();
                        if parts.len() >= 4 {
                            installdir = parts[3].to_string();
                        }
                    } else if l.starts_with("\"SizeOnDisk\"") {
                        let parts: Vec<&str> = l.split('"').collect();
                        if parts.len() >= 4 {
                            size_on_disk = parts[3].trim().parse::<u64>().unwrap_or(0);
                        }
                    }
                }

                let common_dir = if !installdir.is_empty() {
                    steamapps.join("common").join(&installdir)
                } else {
                    steamapps.join("common")
                };

                let final_location = if common_dir.exists() {
                    common_dir.to_string_lossy().to_string()
                } else {
                    steamapps.to_string_lossy().to_string()
                };

                let final_size = if size_on_disk > 0 {
                    size_on_disk
                } else if common_dir.exists() {
                    get_folder_size(&common_dir)
                } else {
                    0
                };

                return Some(SteamGameDetails {
                    install_location: final_location,
                    size_bytes: final_size,
                    acf_path: Some(acf),
                });
            }
        }
    }
    None
}

pub fn scan_win32_apps() -> Vec<InstalledApp> {
    let mut apps = Vec::new();
    let mut seen_ids = HashSet::new();

    let hives = [
        (HKEY_LOCAL_MACHINE, r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall", KEY_WOW64_64KEY, true),
        (HKEY_LOCAL_MACHINE, r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall", KEY_WOW64_32KEY, false),
        (HKEY_CURRENT_USER, r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall", 0, true),
    ];

    for (root_hive, sub_path, flags, is_64) in hives {
        let root = RegKey::predef(root_hive);
        let open_flags = KEY_READ | flags;
        if let Ok(uninstall_key) = root.open_subkey_with_flags(sub_path, open_flags) {
            for subkey_name in uninstall_key.enum_keys().filter_map(|k| k.ok()) {
                if let Ok(app_key) = uninstall_key.open_subkey_with_flags(&subkey_name, open_flags) {
                    let name = get_string_value(&app_key, "DisplayName");
                    if name.trim().is_empty() {
                        continue;
                    }

                    if seen_ids.contains(&subkey_name) {
                        continue;
                    }
                    seen_ids.insert(subkey_name.clone());

                    let version = get_string_value(&app_key, "DisplayVersion");
                    let publisher = get_string_value(&app_key, "Publisher");
                    let install_date = get_string_value(&app_key, "InstallDate");
                    let location = get_string_value(&app_key, "InstallLocation");
                    let uninstall_str = get_string_value(&app_key, "UninstallString");
                    let size_kb = get_u64_value(&app_key, "EstimatedSize");
                    let sys_comp = get_u64_value(&app_key, "SystemComponent") == 1;

                    let display_icon = get_string_value(&app_key, "DisplayIcon");
                    let mut clean_icon = if !display_icon.is_empty() {
                        display_icon.trim().to_string()
                    } else if !location.is_empty() {
                        let p = Path::new(&location);
                        if p.is_file() {
                            location.trim().to_string()
                        } else if let Ok(entries) = std::fs::read_dir(p) {
                            entries.flatten()
                                .find(|e| e.path().extension().map(|ext| ext.eq_ignore_ascii_case("exe") || ext.eq_ignore_ascii_case("ico")).unwrap_or(false))
                                .map(|e| e.path().to_string_lossy().to_string())
                                .unwrap_or_default()
                        } else {
                            String::new()
                        }
                    } else {
                        String::new()
                    };

                    if clean_icon.is_empty() && !uninstall_str.is_empty() {
                        let trimmed = uninstall_str.trim().trim_matches('"');
                        let exe_candidate = if let Some(exe_idx) = trimmed.to_lowercase().find(".exe") {
                            &trimmed[..exe_idx + 4]
                        } else {
                            ""
                        };
                        if !exe_candidate.is_empty() && Path::new(exe_candidate).exists() {
                            clean_icon = exe_candidate.to_string();
                        }
                    }

                    let is_steam = subkey_name.starts_with("Steam App ") || uninstall_str.contains("steam://");
                    let steam_id = if subkey_name.starts_with("Steam App ") {
                        Some(subkey_name[10..].trim().to_string())
                    } else if let Some(pos) = uninstall_str.find("steam://uninstall/") {
                        let id_part = &uninstall_str[pos + 18..];
                        let clean_id: String = id_part.chars().take_while(|c| c.is_ascii_digit()).collect();
                        if !clean_id.is_empty() { Some(clean_id) } else { None }
                    } else {
                        None
                    };

                    let raw_loc = if !location.trim().is_empty() {
                        let loc = location.trim().to_string();
                        let loc_low = loc.to_lowercase();
                        if loc_low.ends_with(r"\steam\games") || loc_low.ends_with(r"/steam/games") {
                            String::new()
                        } else {
                            loc
                        }
                    } else if !clean_icon.is_empty() {
                        let f = clean_icon.split(',').next().unwrap_or(&clean_icon).trim().trim_matches('"');
                        let parent = Path::new(f).parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
                        let parent_low = parent.to_lowercase();
                        if parent_low.ends_with(r"\steam\games") || parent_low.ends_with(r"/steam/games") || (parent_low.contains("steam") && parent_low.ends_with(r"\games")) {
                            // C:\Program Files (x86)\Steam\steam\games is Steam's icon cache directory, NEVER a game folder!
                            String::new()
                        } else {
                            parent
                        }
                    } else if !uninstall_str.trim().is_empty() && !is_steam {
                        let clean = uninstall_str.trim().trim_matches('"');
                        let exe_candidate = if let Some(pos) = clean.to_lowercase().find(".exe") {
                            &clean[..pos + 4]
                        } else {
                            clean
                        };
                        Path::new(exe_candidate).parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_default()
                    } else {
                        String::new()
                    };

                    let mut resolved_location = if !raw_loc.is_empty() {
                        let p = Path::new(&raw_loc);
                        if let Some(file_name) = p.file_name().and_then(|n| n.to_str()) {
                            let lower = file_name.to_lowercase();
                            if lower == "bin" || lower == "cmd" || lower == "x64" || lower == "x86" || lower == "app" {
                                p.parent().map(|parent| parent.to_string_lossy().to_string()).unwrap_or(raw_loc)
                            } else {
                                raw_loc
                            }
                        } else {
                            raw_loc
                        }
                    } else {
                        String::new()
                    };

                    let mut size_bytes = size_kb * 1024;
                    if let Some(ref sid) = steam_id {
                        if let Some(details) = find_steam_game_details(sid) {
                            if resolved_location.is_empty()
                                || !Path::new(&resolved_location).exists()
                                || resolved_location.to_lowercase().contains(r"\steam\games")
                            {
                                resolved_location = details.install_location;
                            }
                            if details.size_bytes > 0 {
                                size_bytes = details.size_bytes;
                            }
                        }
                    }

                    let hive_str = if root_hive == HKEY_LOCAL_MACHINE { "HKLM" } else { "HKCU" };
                    let actual_sub_path = if flags == KEY_WOW64_32KEY && root_hive == HKEY_LOCAL_MACHINE {
                        r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall"
                    } else {
                        sub_path
                    };
                    let reg_path = format!("{}\\{}\\{}", hive_str, actual_sub_path, subkey_name);

                    let is_steam = subkey_name.starts_with("Steam App ") || uninstall_str.contains("steam://");
                    let is_epic = publisher.to_lowercase().contains("epic games") || uninstall_str.contains("com.epicgames.launcher") || reg_path.contains("EpicGames");
                    let source_badge = if is_steam {
                        "[Steam]".to_string()
                    } else if is_epic {
                        "[Epic]".to_string()
                    } else {
                        String::new()
                    };

                    apps.push(InstalledApp {
                        id: subkey_name,
                        name: name.trim().to_string(),
                        version: version.trim().to_string(),
                        publisher: publisher.trim().to_string(),
                        install_date: install_date.trim().to_string(),
                        size_bytes,
                        install_location: resolved_location,
                        uninstall_string: uninstall_str.trim().to_string(),
                        registry_path: reg_path,
                        is_64bit: is_64,
                        is_system_component: sys_comp,
                        is_uwp: false,
                        package_full_name: String::new(),
                        display_icon: clean_icon,
                        is_new: false,
                        source_badge,
                    });
                }
            }
        }
    }

    apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    apps
}

pub fn scan_steam_library_games(existing_apps: &[InstalledApp]) -> Vec<InstalledApp> {
    let mut new_steam_apps = Vec::new();
    let existing_names: HashSet<String> = existing_apps.iter().map(|a| a.name.to_lowercase()).collect();
    let existing_ids: HashSet<String> = existing_apps.iter().map(|a| a.id.to_lowercase()).collect();

    let steam_path = get_steam_install_path().unwrap_or_default();
    let library_dirs = get_all_steam_library_folders();
    if library_dirs.is_empty() {
        return new_steam_apps;
    }

    for lib_dir in library_dirs {
        let steamapps = lib_dir.join("steamapps");
        if !steamapps.exists() {
            continue;
        }

        if let Ok(entries) = std::fs::read_dir(&steamapps) {
            for entry in entries.flatten() {
                let p = entry.path();
                if let Some(file_name) = p.file_name().and_then(|n| n.to_str()) {
                    if file_name.starts_with("appmanifest_") && file_name.ends_with(".acf") {
                        if let Ok(acf_content) = std::fs::read_to_string(&p) {
                            let mut appid = String::new();
                            let mut name = String::new();
                            let mut installdir = String::new();
                            let mut size_bytes = 0u64;

                            for line in acf_content.lines() {
                                let l = line.trim();
                                if l.starts_with("\"appid\"") {
                                    let parts: Vec<&str> = l.split('"').collect();
                                    if parts.len() >= 4 { appid = parts[3].to_string(); }
                                } else if l.starts_with("\"name\"") {
                                    let parts: Vec<&str> = l.split('"').collect();
                                    if parts.len() >= 4 { name = parts[3].to_string(); }
                                } else if l.starts_with("\"installdir\"") {
                                    let parts: Vec<&str> = l.split('"').collect();
                                    if parts.len() >= 4 { installdir = parts[3].to_string(); }
                                } else if l.starts_with("\"SizeOnDisk\"") {
                                    let parts: Vec<&str> = l.split('"').collect();
                                    if parts.len() >= 4 {
                                        size_bytes = parts[3].trim().parse::<u64>().unwrap_or(0);
                                    }
                                }
                            }

                            if name.is_empty() || appid.is_empty() {
                                continue;
                            }

                            // Skip common steam redistributables
                            if appid == "228980" && name.contains("Steamworks") {
                                continue;
                            }

                            let app_id_str = format!("Steam App {}", appid);
                            if existing_ids.contains(&app_id_str.to_lowercase()) 
                                || existing_ids.contains(&format!("steam_{}", appid).to_lowercase())
                                || existing_names.contains(&name.to_lowercase()) 
                            {
                                continue;
                            }

                            let common_dir = steamapps.join("common").join(&installdir);
                            let location_str = if common_dir.exists() {
                                common_dir.to_string_lossy().to_string()
                            } else {
                                steamapps.to_string_lossy().to_string()
                            };

                            let icon = if common_dir.exists() {
                                if let Ok(dir_entries) = std::fs::read_dir(&common_dir) {
                                    dir_entries.flatten()
                                        .find(|e| e.path().extension().map(|ext| ext.eq_ignore_ascii_case("exe")).unwrap_or(false))
                                        .map(|e| e.path().to_string_lossy().to_string())
                                        .unwrap_or_else(|| format!(r"{}\steam.exe", steam_path))
                                } else {
                                    format!(r"{}\steam.exe", steam_path)
                                }
                            } else {
                                format!(r"{}\steam.exe", steam_path)
                            };

                            new_steam_apps.push(InstalledApp {
                                id: format!("steam_{}", appid),
                                name,
                                version: String::new(),
                                publisher: "Valve / Steam".to_string(),
                                install_date: "Steam".to_string(),
                                size_bytes,
                                install_location: location_str,
                                uninstall_string: format!("steam://uninstall/{}", appid),
                                registry_path: format!("Steam\\{}", appid),
                                is_64bit: true,
                                is_system_component: false,
                                is_uwp: false,
                                package_full_name: String::new(),
                                display_icon: icon,
                                is_new: false,
                                source_badge: "[Steam]".to_string(),
                            });
                        }
                    }
                }
            }
        }
    }

    new_steam_apps
}

pub fn scan_epic_games(existing_apps: &[InstalledApp]) -> Vec<InstalledApp> {
    let mut epic_apps = Vec::new();
    let existing_names: HashSet<String> = existing_apps.iter().map(|a| a.name.to_lowercase()).collect();
    let manifests_dir = Path::new(r"C:\ProgramData\Epic\EpicGamesLauncher\Data\Manifests");
    if !manifests_dir.exists() {
        return epic_apps;
    }

    if let Ok(entries) = std::fs::read_dir(manifests_dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.extension().map(|e| e.eq_ignore_ascii_case("item")).unwrap_or(false) {
                if let Ok(content) = std::fs::read_to_string(&p) {
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                        let name = val["DisplayName"].as_str().unwrap_or_default().to_string();
                        let loc = val["InstallLocation"].as_str().unwrap_or_default().to_string();
                        let size = val["InstallSize"].as_u64().unwrap_or(0);
                        let app_name = val["AppName"].as_str().unwrap_or_default().to_string();

                        if !name.is_empty() && !existing_names.contains(&name.to_lowercase()) {
                            epic_apps.push(InstalledApp {
                                id: format!("epic_{}", app_name),
                                name,
                                version: val["AppVersionString"].as_str().unwrap_or_default().to_string(),
                                publisher: "Epic Games".to_string(),
                                install_date: "Epic Games".to_string(),
                                size_bytes: size,
                                install_location: loc.clone(),
                                uninstall_string: format!("com.epicgames.launcher://apps/{}?action=uninstall", app_name),
                                registry_path: format!("Epic\\{}", app_name),
                                is_64bit: true,
                                is_system_component: false,
                                is_uwp: false,
                                package_full_name: String::new(),
                                display_icon: String::new(),
                                is_new: false,
                                source_badge: "[Epic]".to_string(),
                            });
                        }
                    }
                }
            }
        }
    }

    epic_apps
}

#[allow(dead_code)]
pub fn scan_installed_apps() -> Vec<InstalledApp> {
    let mut apps = scan_win32_apps();
    let mut uwp_apps = scan_uwp_apps();
    apps.append(&mut uwp_apps);

    // Deep library discovery: Steam libraries on all drives (C:, D:, E: ...)
    let mut steam_games = scan_steam_library_games(&apps);
    apps.append(&mut steam_games);

    // Deep library discovery: Epic Games manifests
    let mut epic_games = scan_epic_games(&apps);
    apps.append(&mut epic_games);

    apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    apps
}

#[derive(Deserialize)]
struct RawTask {
    #[serde(default, rename = "TaskName")]
    task_name: String,
    #[serde(default, rename = "TaskPath")]
    task_path: String,
    #[serde(default, rename = "State")]
    state: i32,
    #[serde(default, rename = "Command")]
    command: String,
}

fn is_registry_run_disabled(root_hive: isize, val_name: &str) -> bool {
    let root = RegKey::predef(root_hive);
    let app_path = r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run";
    if let Ok(key) = root.open_subkey_with_flags(app_path, KEY_READ) {
        if let Ok(raw) = key.get_raw_value(val_name) {
            if !raw.bytes.is_empty() && raw.bytes[0] == 0x03 {
                return true;
            }
        }
    }
    false
}

pub fn calculate_startup_impact(name: &str, command: &str) -> StartupImpact {
    let lower_name = name.to_lowercase();
    let lower_cmd = command.to_lowercase();

    // High impact launchers, browsers, communication platforms, heavy developer tooling
    if lower_name.contains("steam") || lower_cmd.contains("steam")
        || lower_name.contains("discord") || lower_cmd.contains("discord")
        || lower_name.contains("epic") || lower_cmd.contains("epicgames")
        || lower_name.contains("chrome") || lower_cmd.contains("chrome")
        || lower_name.contains("edge") || lower_cmd.contains("msedge")
        || lower_name.contains("browser") || lower_cmd.contains("browser")
        || lower_name.contains("spotify") || lower_cmd.contains("spotify")
        || lower_name.contains("telegram") || lower_cmd.contains("telegram")
        || lower_name.contains("onedrive") || lower_cmd.contains("onedrive")
        || lower_name.contains("docker") || lower_cmd.contains("docker")
        || lower_name.contains("jetbrains") || lower_cmd.contains("rider")
        || lower_name.contains("electron") || lower_cmd.contains("teams")
        || lower_name.contains("slack") || lower_cmd.contains("battlenet")
        || lower_name.contains("wallpaper") || lower_cmd.contains("wallpaper")
    {
        return StartupImpact::High;
    }

    // Low impact native drivers, hotkeys, small helpers (checked before generic 'service' keyword)
    if lower_name.contains("realtek") || lower_cmd.contains("rtk")
        || lower_name.contains("audio") || lower_name.contains("sound")
        || lower_name.contains("nvidia") || lower_cmd.contains("nv")
        || lower_name.contains("intel") || lower_name.contains("amd")
        || lower_name.contains("hotkey") || lower_name.contains("touchpad")
    {
        return StartupImpact::Low;
    }

    // Medium impact updaters and sync agents
    if lower_name.contains("update") || lower_cmd.contains("update")
        || lower_name.contains("helper") || lower_cmd.contains("helper")
        || lower_name.contains("sync") || lower_cmd.contains("sync")
        || lower_name.contains("service") || lower_cmd.contains("service")
        || lower_name.contains("client") || lower_cmd.contains("client")
        || lower_name.contains("tray") || lower_cmd.contains("agent")
    {
        return StartupImpact::Medium;
    }

    // Executable file size heuristic if path exists
    let clean_cmd = command.trim().trim_matches('"');
    let exe_path = if let Some(idx) = clean_cmd.to_lowercase().find(".exe") {
        &clean_cmd[..idx + 4]
    } else {
        clean_cmd
    };
    if let Ok(metadata) = std::fs::metadata(exe_path) {
        let size_mb = metadata.len() / (1024 * 1024);
        if size_mb >= 30 {
            return StartupImpact::High;
        } else if size_mb >= 5 {
            return StartupImpact::Medium;
        } else {
            return StartupImpact::Low;
        }
    }

    StartupImpact::Low
}

pub fn scan_startup_items() -> Vec<StartupItem> {
    let mut items = Vec::new();
    let mut seen_ids = HashSet::new();

    // 1. Registry Run & RunOnce targets
    let targets = [
        (HKEY_LOCAL_MACHINE, r"SOFTWARE\Microsoft\Windows\CurrentVersion\Run", "hklm_run", "Реестр (Все пользователи)", true),
        (HKEY_LOCAL_MACHINE, r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Run", "hklm_run", "Реестр (Все пользователи, 32-бит)", true),
        (HKEY_CURRENT_USER, r"SOFTWARE\Microsoft\Windows\CurrentVersion\Run", "hkcu_run", "Реестр (Текущий пользователь)", false),
        (HKEY_LOCAL_MACHINE, r"SOFTWARE\Microsoft\Windows\CurrentVersion\RunOnce", "hklm_runonce", "Реестр RunOnce", true),
        (HKEY_CURRENT_USER, r"SOFTWARE\Microsoft\Windows\CurrentVersion\RunOnce", "hkcu_runonce", "Реестр RunOnce", false),
    ];

    for (root_hive, sub_path, source_type, source_label, is_admin) in targets {
        let root = RegKey::predef(root_hive);
        if let Ok(key) = root.open_subkey_with_flags(sub_path, KEY_READ) {
            for (val_name, val_data) in key.enum_values().filter_map(|v| v.ok()) {
                let id = format!("{}_{}", source_type, val_name);
                if seen_ids.contains(&id) {
                    continue;
                }
                seen_ids.insert(id.clone());

                let disabled = is_registry_run_disabled(root_hive, &val_name);
                let cmd_str = val_data.to_string();
                let impact = calculate_startup_impact(&val_name, &cmd_str);
                items.push(StartupItem {
                    id,
                    name: val_name.clone(),
                    command: cmd_str,
                    source: source_label.to_string(),
                    source_type: source_type.to_string(),
                    status: if disabled { "disabled".to_string() } else { "enabled".to_string() },
                    is_enabled: !disabled,
                    run_as_admin: is_admin,
                    raw_target: val_name,
                    impact,
                });
            }
        }
    }

    // 2. Start Menu Startup folders
    let mut startup_dirs = Vec::new();
    if let Ok(appdata) = std::env::var("APPDATA") {
        startup_dirs.push((format!(r"{}\Microsoft\Windows\Start Menu\Programs\Startup", appdata), "Меню Пуск (Пользователь)"));
    }
    if let Ok(progdata) = std::env::var("PROGRAMDATA") {
        startup_dirs.push((format!(r"{}\Microsoft\Windows\Start Menu\Programs\Startup", progdata), "Меню Пуск (Все пользователи)"));
    }

    for (dir_path, label) in startup_dirs {
        let p = Path::new(&dir_path);
        if let Ok(entries) = std::fs::read_dir(p) {
            for entry in entries.flatten() {
                let file_path = entry.path();
                if let Some(file_name) = file_path.file_name().and_then(|n| n.to_str()) {
                    if file_name.eq_ignore_ascii_case("desktop.ini") {
                        continue;
                    }
                    let is_disabled = file_name.ends_with(".disabled");
                    let clean_name = if is_disabled {
                        file_name.trim_end_matches(".disabled").to_string()
                    } else if let Some(ext_idx) = file_name.rfind('.') {
                        file_name[..ext_idx].to_string()
                    } else {
                        file_name.to_string()
                    };

                    let full_p = file_path.to_string_lossy().to_string();
                    let id = format!("startmenu_{}", clean_name);
                    if seen_ids.contains(&id) {
                        continue;
                    }
                    seen_ids.insert(id.clone());

                    let impact = calculate_startup_impact(&clean_name, &full_p);
                    items.push(StartupItem {
                        id,
                        name: clean_name,
                        command: full_p.clone(),
                        source: label.to_string(),
                        source_type: "startmenu".to_string(),
                        status: if is_disabled { "disabled".to_string() } else { "enabled".to_string() },
                        is_enabled: !is_disabled,
                        run_as_admin: false,
                        raw_target: full_p,
                        impact,
                    });
                }
            }
        }
    }

    // 3. Task Scheduler (non-Windows third-party tasks)
    let ps_cmd = r#"Get-ScheduledTask | Where-Object { $_.TaskPath -notmatch 'Microsoft\\Windows' } | ForEach-Object { [PSCustomObject]@{ TaskName=$_.TaskName; TaskPath=$_.TaskPath; State=$_.State; Command=($_.Actions | ForEach-Object { $_.Execute + ' ' + $_.Arguments } | Out-String).Trim() } } | ConvertTo-Json -Compress"#;
    let mut command = Command::new("powershell");
    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);
    if let Ok(out) = command.args(["-NoProfile", "-NonInteractive", "-Command", ps_cmd]).output() {
        if out.status.success() {
            let json_str = String::from_utf8_lossy(&out.stdout);
            let trimmed = json_str.trim();
            if !trimmed.is_empty() {
                let task_list: Vec<RawTask> = if trimmed.starts_with('[') {
                    serde_json::from_str(trimmed).unwrap_or_default()
                } else if trimmed.starts_with('{') {
                    serde_json::from_str::<RawTask>(trimmed).map(|t| vec![t]).unwrap_or_default()
                } else {
                    Vec::new()
                };

                for t in task_list {
                    if t.task_name.is_empty() {
                        continue;
                    }
                    let full_task_name = format!("{}{}", t.task_path, t.task_name);
                    let id = format!("task_{}", full_task_name);
                    if seen_ids.contains(&id) {
                        continue;
                    }
                    seen_ids.insert(id.clone());

                    let is_enabled = t.state != 1; // 1 is Disabled in Task Scheduler
                    let cmd_str = if t.command.is_empty() { t.task_name.clone() } else { t.command };
                    let impact = calculate_startup_impact(&t.task_name, &cmd_str);

                    items.push(StartupItem {
                        id,
                        name: t.task_name,
                        command: cmd_str,
                        source: "Планировщик заданий".to_string(),
                        source_type: "taskscheduler".to_string(),
                        status: if is_enabled { "enabled".to_string() } else { "disabled".to_string() },
                        is_enabled,
                        run_as_admin: true,
                        raw_target: full_task_name,
                        impact,
                    });
                }
            }
        }
    }

    items.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    items
}

pub fn toggle_startup_item(item: &mut StartupItem) -> bool {
    let new_enabled = !item.is_enabled;
    match item.source_type.as_str() {
        "hklm_run" | "hkcu_run" => {
            let root_hive = if item.source_type == "hklm_run" { HKEY_LOCAL_MACHINE } else { HKEY_CURRENT_USER };
            let root = RegKey::predef(root_hive);
            let app_path = r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run";
            if let Ok((key, _)) = root.create_subkey(app_path) {
                let first_byte = if new_enabled { 0x02 } else { 0x03 };
                let data = vec![first_byte, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
                let reg_val = winreg::RegValue {
                    vtype: winreg::enums::REG_BINARY,
                    bytes: data,
                };
                if key.set_raw_value(&item.raw_target, &reg_val).is_ok() {
                    item.is_enabled = new_enabled;
                    item.status = if new_enabled { "enabled".to_string() } else { "disabled".to_string() };
                    return true;
                }
            }
        }
        "startmenu" => {
            let p = Path::new(&item.raw_target);
            if new_enabled {
                if item.raw_target.ends_with(".disabled") {
                    let new_path = item.raw_target.trim_end_matches(".disabled");
                    if std::fs::rename(p, new_path).is_ok() {
                        item.raw_target = new_path.to_string();
                        item.command = item.raw_target.clone();
                        item.is_enabled = true;
                        item.status = "enabled".to_string();
                        return true;
                    }
                }
            } else if !item.raw_target.ends_with(".disabled") {
                let new_path = format!("{}.disabled", item.raw_target);
                if std::fs::rename(p, &new_path).is_ok() {
                    item.raw_target = new_path.clone();
                    item.command = new_path;
                    item.is_enabled = false;
                    item.status = "disabled".to_string();
                    return true;
                }
            }
        }
        "taskscheduler" => {
            let action = if new_enabled { "/enable" } else { "/disable" };
            let mut cmd = Command::new("schtasks");
            #[cfg(windows)]
            cmd.creation_flags(CREATE_NO_WINDOW);
            if let Ok(out) = cmd.args(["/change", "/tn", &item.raw_target, action]).output() {
                if out.status.success() {
                    item.is_enabled = new_enabled;
                    item.status = if new_enabled { "enabled".to_string() } else { "disabled".to_string() };
                    return true;
                }
            }
        }
        _ => {}
    }
    false
}

pub fn delete_startup_item(item: &StartupItem) -> bool {
    match item.source_type.as_str() {
        "hklm_run" | "hkcu_run" | "hklm_runonce" | "hkcu_runonce" => {
            let sub_hive = if item.source_type.starts_with("hklm") { "HKLM" } else { "HKCU" };
            let key_name = if item.source_type.contains("runonce") { "RunOnce" } else { "Run" };
            let reg_path = format!(r"{}\SOFTWARE\Microsoft\Windows\CurrentVersion\{}", sub_hive, key_name);
            let mut cmd = Command::new("reg");
            #[cfg(windows)]
            cmd.creation_flags(CREATE_NO_WINDOW);
            cmd.args(["delete", &reg_path, "/v", &item.raw_target, "/f"]).output().map(|o| o.status.success()).unwrap_or(false)
        }
        "startmenu" => {
            std::fs::remove_file(&item.raw_target).is_ok()
        }
        "taskscheduler" => {
            let mut cmd = Command::new("schtasks");
            #[cfg(windows)]
            cmd.creation_flags(CREATE_NO_WINDOW);
            cmd.args(["/delete", "/tn", &item.raw_target, "/f"]).output().map(|o| o.status.success()).unwrap_or(false)
        }
        _ => false,
    }
}

pub fn add_startup_item(name: &str, path: &str, args: &str, hive_hklm: bool) -> bool {
    if name.trim().is_empty() || path.trim().is_empty() {
        return false;
    }
    let root_hive = if hive_hklm { HKEY_LOCAL_MACHINE } else { HKEY_CURRENT_USER };
    let root = RegKey::predef(root_hive);
    let run_path = r"SOFTWARE\Microsoft\Windows\CurrentVersion\Run";
    if let Ok((key, _)) = root.create_subkey(run_path) {
        let cmd = if args.trim().is_empty() {
            format!("\"{}\"", path.trim().trim_matches('"'))
        } else {
            format!("\"{}\" {}", path.trim().trim_matches('"'), args.trim())
        };
        return key.set_value(name.trim(), &cmd).is_ok();
    }
    false
}

pub fn pick_file_dialog(filter_desc: &str, filter_ext: &str) -> Option<String> {
    let script = format!(
        "Add-Type -AssemblyName System.Windows.Forms; $f = New-Object System.Windows.Forms.OpenFileDialog; $f.Filter = '{} ({})|{}|All files (*.*)|*.*'; if ($f.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) {{ $f.FileName }}",
        filter_desc, filter_ext, filter_ext
    );
    let mut cmd = Command::new("powershell");
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    let out = cmd.args(["-NoProfile", "-NonInteractive", "-Command", &script]).output().ok()?;
    if out.status.success() {
        let path = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !path.is_empty() && Path::new(&path).exists() {
            return Some(path);
        }
    }
    None
}

pub fn pick_folder_dialog() -> Option<String> {
    let script = "Add-Type -AssemblyName System.Windows.Forms; $f = New-Object System.Windows.Forms.FolderBrowserDialog; if ($f.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) { $f.SelectedPath }";
    let mut cmd = Command::new("powershell");
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    let out = cmd.args(["-NoProfile", "-NonInteractive", "-Command", script]).output().ok()?;
    if out.status.success() {
        let path = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !path.is_empty() && Path::new(&path).exists() {
            return Some(path);
        }
    }
    None
}

pub fn get_app_tokens(app: &InstalledApp) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut add_token = |t: &str| {
        let clean = t.trim().trim_matches('"').trim_matches('\'');
        if clean.len() >= 3 && !tokens.iter().any(|existing: &String| existing.eq_ignore_ascii_case(clean)) {
            tokens.push(clean.to_string());
        }
    };

    add_token(&app.id);
    add_token(&app.name);

    let safe_name: String = app.name.chars().filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-').collect();
    add_token(&safe_name);
    let safe_id: String = app.id.chars().filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-').collect();
    add_token(&safe_id);

    let stop_words = [
        "application", "app", "setup", "installer", "software", "suite", "version",
        "x64", "x86", "64-bit", "32-bit", "microsoft", "windows", "test", "demo",
        "free", "trial", "tool", "tools", "update", "client", "edition", "system", "package"
    ];
    for word in app.name.split_whitespace() {
        let clean_word: String = word.chars().filter(|c| c.is_alphanumeric()).collect();
        if clean_word.len() >= 4 && !stop_words.contains(&clean_word.to_lowercase().as_str()) {
            add_token(&clean_word);
        }
    }

    if !app.install_location.is_empty() {
        if let Some(leaf) = Path::new(&app.install_location).file_name().and_then(|n| n.to_str()) {
            add_token(leaf);
        }
    }

    tokens
}

pub fn build_app_leftover_targets(app: &InstalledApp, traced_apps: &[TracedApp]) -> Vec<(String, String)> {
    let mut targets: Vec<(String, String)> = Vec::new();
    let tokens = get_app_tokens(app);
    let safe_pub: String = app.publisher.chars().filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-').collect();

    // 1. UWP Packages
    if app.is_uwp && !app.package_full_name.is_empty() {
        targets.push(("appx_package".to_string(), app.package_full_name.clone()));
        if let Ok(local_appdata) = std::env::var("LOCALAPPDATA") {
            let pkg_dir = format!("{}\\Packages", local_appdata);
            targets.push(("folder".to_string(), pkg_dir));
        }
    }

    // 2. Registry inspection paths
    let reg_hives = [
        (HKEY_CURRENT_USER, "HKEY_CURRENT_USER", "Software"),
        (HKEY_LOCAL_MACHINE, "HKEY_LOCAL_MACHINE", "SOFTWARE"),
        (HKEY_LOCAL_MACHINE, "HKEY_LOCAL_MACHINE", "SOFTWARE\\WOW6432Node"),
    ];

    for (_, hive_name, base) in &reg_hives {
        for tok in &tokens {
            targets.push(("registry_key".to_string(), format!("{}\\{}\\{}", hive_name, base, tok)));
            if !app.publisher.is_empty() && !app.publisher.eq_ignore_ascii_case("microsoft") {
                targets.push(("registry_key".to_string(), format!("{}\\{}\\{}\\{}", hive_name, base, app.publisher, tok)));
            }
            if !safe_pub.is_empty() && !safe_pub.eq_ignore_ascii_case("microsoft") && safe_pub != app.publisher {
                targets.push(("registry_key".to_string(), format!("{}\\{}\\{}\\{}", hive_name, base, safe_pub, tok)));
            }
        }
    }

    // Search subkeys of HKCU\Software and HKLM\SOFTWARE matching tokens
    for (hive_val, hive_name) in [(HKEY_CURRENT_USER, "HKEY_CURRENT_USER"), (HKEY_LOCAL_MACHINE, "HKEY_LOCAL_MACHINE")] {
        if let Ok(key) = RegKey::predef(hive_val).open_subkey("Software") {
            for sub in key.enum_keys().flatten() {
                let sub_lower = sub.to_lowercase();
                if tokens.iter().any(|tok| tok.len() >= 4 && sub_lower.contains(&tok.to_lowercase())) {
                    targets.push(("registry_key".to_string(), format!("{}\\{}\\{}", hive_name, "Software", sub)));
                }
            }
        }
    }

    // Uninstall keys in registry
    for tok in &tokens {
        targets.push(("registry_key".to_string(), format!("HKEY_LOCAL_MACHINE\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\{}", tok)));
        targets.push(("registry_key".to_string(), format!("HKEY_LOCAL_MACHINE\\SOFTWARE\\WOW6432Node\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\{}", tok)));
        targets.push(("registry_key".to_string(), format!("HKEY_CURRENT_USER\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\{}", tok)));
    }
    if !app.registry_path.is_empty() {
        let full_reg = if app.registry_path.starts_with("HKLM\\") {
            format!("HKEY_LOCAL_MACHINE\\{}", &app.registry_path[5..])
        } else if app.registry_path.starts_with("HKCU\\") {
            format!("HKEY_CURRENT_USER\\{}", &app.registry_path[5..])
        } else {
            app.registry_path.clone()
        };
        targets.push(("registry_key".to_string(), full_reg));
    }

    // Run keys in registry
    for (hive_val, hive_name) in [(HKEY_CURRENT_USER, "HKEY_CURRENT_USER"), (HKEY_LOCAL_MACHINE, "HKEY_LOCAL_MACHINE")] {
        if let Ok(key) = RegKey::predef(hive_val).open_subkey(r"Software\Microsoft\Windows\CurrentVersion\Run") {
            for (val_name, _) in key.enum_values().flatten() {
                let val_lower = val_name.to_lowercase();
                if tokens.iter().any(|tok| val_lower.contains(&tok.to_lowercase())) {
                    targets.push(("registry_key".to_string(), format!(r"{}\Software\Microsoft\Windows\CurrentVersion\Run -> {}", hive_name, val_name)));
                }
            }
        }
    }

    // 3. Filesystem inspection paths
    let appdata = std::env::var("APPDATA").ok();
    let localappdata = std::env::var("LOCALAPPDATA").ok();
    let userprofile = std::env::var("USERPROFILE").ok();
    let locallow = userprofile.as_ref().map(|p| format!(r"{}\AppData\LocalLow", p));
    let progdata = std::env::var("ProgramData").ok();
    let progfiles = std::env::var("ProgramFiles").ok();
    let progfiles_x86 = std::env::var("ProgramFiles(x86)").ok();
    let temp_dir = std::env::var("TEMP").ok();
    let win_temp = Some(r"C:\Windows\Temp".to_string());

    let base_dirs = [appdata, localappdata, locallow, progdata, progfiles, progfiles_x86, temp_dir, win_temp];
    for base_dir in base_dirs.into_iter().flatten() {
        for tok in &tokens {
            targets.push(("folder".to_string(), format!("{}\\{}", base_dir, tok)));
            if !app.publisher.is_empty() && !app.publisher.eq_ignore_ascii_case("microsoft") {
                targets.push(("folder".to_string(), format!("{}\\{}\\{}", base_dir, app.publisher, tok)));
            }
            if !safe_pub.is_empty() && !safe_pub.eq_ignore_ascii_case("microsoft") && safe_pub != app.publisher {
                targets.push(("folder".to_string(), format!("{}\\{}\\{}", base_dir, safe_pub, tok)));
            }
        }

        // Subdirectories in base_dir for substring match
        if let Ok(entries) = std::fs::read_dir(&base_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                let name = entry.file_name().to_string_lossy().to_string();
                let name_lower = name.to_lowercase();
                if tokens.iter().any(|tok| tok.len() >= 4 && name_lower.contains(&tok.to_lowercase())) {
                    let is_dir = entry.metadata().map(|m| m.is_dir()).unwrap_or(false);
                    targets.push((if is_dir { "folder".to_string() } else { "file".to_string() }, p.to_string_lossy().to_string()));
                }
            }
        }
    }

    // Shortcuts: Desktop & Start Menu
    fn scan_shortcut_dir(dir: &Path, tokens: &[String], targets: &mut Vec<(String, String)>) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                let name = entry.file_name().to_string_lossy().to_string();
                let name_lower = name.to_lowercase();
                if tokens.iter().any(|tok| tok.len() >= 3 && name_lower.contains(&tok.to_lowercase())) {
                    let is_dir = entry.metadata().map(|m| m.is_dir()).unwrap_or(false);
                    targets.push((if is_dir { "folder".to_string() } else { "file".to_string() }, p.to_string_lossy().to_string()));
                }
            }
        }
    }

    if let Some(ref user_prof) = userprofile {
        let desktop = Path::new(user_prof).join("Desktop");
        scan_shortcut_dir(&desktop, &tokens, &mut targets);
        for tok in &tokens {
            let dot_folder = Path::new(user_prof).join(format!(".{}", tok));
            if dot_folder.exists() {
                targets.push(("folder".to_string(), dot_folder.to_string_lossy().to_string()));
            }
        }
    }
    if let Ok(public_dir) = std::env::var("PUBLIC") {
        let pub_desktop = Path::new(&public_dir).join("Desktop");
        scan_shortcut_dir(&pub_desktop, &tokens, &mut targets);
    }
    if let Ok(ad) = std::env::var("APPDATA") {
        let start_programs = Path::new(&ad).join(r"Microsoft\Windows\Start Menu\Programs");
        scan_shortcut_dir(&start_programs, &tokens, &mut targets);
    }
    if let Ok(pd) = std::env::var("ProgramData") {
        let start_programs = Path::new(&pd).join(r"Microsoft\Windows\Start Menu\Programs");
        scan_shortcut_dir(&start_programs, &tokens, &mut targets);
    }

    // Install location
    if !app.install_location.is_empty() {
        targets.push(("folder".to_string(), app.install_location.clone()));
    }

    // Traced app tracks
    if let Some(traced) = traced_apps.iter().find(|t| t.name.eq_ignore_ascii_case(&app.name) || t.id == app.id) {
        for t in &traced.traces {
            targets.push((t.trace_type.clone(), t.path.clone()));
        }
    }

    // Steam game appmanifest_<appid>.acf file
    if app.id.starts_with("Steam App ") || app.source_badge == "[Steam]" {
        let steam_id = if app.id.starts_with("Steam App ") {
            app.id.trim_start_matches("Steam App ").trim()
        } else {
            ""
        };
        if !steam_id.is_empty() {
            for lib_dir in get_all_steam_library_folders() {
                let m = lib_dir.join("steamapps").join(format!("appmanifest_{}.acf", steam_id));
                if m.exists() {
                    targets.push(("file".to_string(), m.to_string_lossy().to_string()));
                }
            }

            if !app.install_location.is_empty() {
                let p = Path::new(&app.install_location);
                if let Some(common_dir) = p.parent() {
                    if common_dir.file_name().map(|n| n.to_string_lossy().eq_ignore_ascii_case("common")).unwrap_or(false) {
                        if let Some(steamapps) = common_dir.parent() {
                            let m = steamapps.join(format!("appmanifest_{}.acf", steam_id));
                            if m.exists() {
                                targets.push(("file".to_string(), m.to_string_lossy().to_string()));
                            }
                        }
                    }
                }
            }
        }
    }

    // Deduplicate targets
    let mut seen = HashSet::new();
    targets.retain(|(_, p)| seen.insert(p.to_lowercase()));
    targets
}

fn check_target_exists(
    t_type: &str,
    p_str: &str,
    app: &InstalledApp,
    out: &mut Vec<LeftoverTrace>,
) -> Option<LeftoverTrace> {
    if t_type == "folder" || t_type == "file" {
        let p = Path::new(p_str);
        if p.exists() {
            let norm = p_str.to_lowercase();
            if !norm.ends_with("program files") && !norm.ends_with("windows") && !norm.ends_with("users") {
                let is_dir = p.is_dir();
                let sz = if is_dir { get_folder_size(p) } else { p.metadata().map(|m| m.len()).unwrap_or(0) };
                let trace = LeftoverTrace {
                    id: format!("fs_{}", p_str),
                    app_id: app.id.clone(),
                    trace_type: if is_dir { "folder".to_string() } else { "file".to_string() },
                    path: p_str.to_string(),
                    size_bytes: if sz > 0 { sz } else { app.size_bytes },
                    confidence: 96,
                    selected: true,
                };
                out.push(trace.clone());
                return Some(trace);
            }
        }
    } else if t_type == "registry_key" {
        if p_str.contains(" -> ") {
            let parts: Vec<&str> = p_str.split(" -> ").collect();
            if parts.len() == 2 {
                if let Some((hive_predef, subkey)) = parse_reg_hive_and_sub(parts[0].trim()) {
                    let root = RegKey::predef(hive_predef);
                    if let Ok(key) = root.open_subkey_with_flags(&subkey, KEY_READ) {
                        if key.get_raw_value(parts[1].trim()).is_ok() {
                            let trace = LeftoverTrace {
                                id: format!("reg_{}", p_str),
                                app_id: app.id.clone(),
                                trace_type: "registry_key".to_string(),
                                path: p_str.to_string(),
                                size_bytes: 0,
                                confidence: 98,
                                selected: true,
                            };
                            out.push(trace.clone());
                            return Some(trace);
                        }
                    }
                }
            }
        } else if let Some((hive_predef, subkey)) = parse_reg_hive_and_sub(p_str) {
            let root = RegKey::predef(hive_predef);
            if root.open_subkey_with_flags(&subkey, KEY_READ).is_ok() {
                let trace = LeftoverTrace {
                    id: format!("reg_{}", p_str),
                    app_id: app.id.clone(),
                    trace_type: "registry_key".to_string(),
                    path: p_str.to_string(),
                    size_bytes: 0,
                    confidence: 98,
                    selected: true,
                };
                out.push(trace.clone());
                return Some(trace);
            }
        }
    } else if t_type == "appx_package" {
        let trace = LeftoverTrace {
            id: format!("uwp_pkg_{}", p_str),
            app_id: app.id.clone(),
            trace_type: "appx_package".to_string(),
            path: p_str.to_string(),
            size_bytes: app.size_bytes,
            confidence: 100,
            selected: true,
        };
        out.push(trace.clone());
        return Some(trace);
    }
    None
}

pub fn scan_leftovers(app: &InstalledApp) -> Vec<LeftoverTrace> {
    let targets = build_app_leftover_targets(app, &[]);
    let mut traces = Vec::new();
    for (t_type, p_str) in &targets {
        check_target_exists(t_type, p_str, app, &mut traces);
    }
    let mut seen = HashSet::new();
    traces.retain(|t| seen.insert(t.path.to_lowercase()));
    traces
}

pub fn get_folder_size(path: &Path) -> u64 {
    get_folder_size_limited(path, 0)
}

fn get_folder_size_limited(path: &Path, depth: usize) -> u64 {
    if depth > 10 {
        return 0;
    }
    let mut total = 0;
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            if let Ok(meta) = entry.metadata() {
                if meta.is_symlink() {
                    continue;
                }
                if meta.is_dir() {
                    total += get_folder_size_limited(&entry.path(), depth + 1);
                } else {
                    total += meta.len();
                }
            }
        }
    }
    total
}

pub fn get_app_drive_letter(app: &InstalledApp) -> char {
    let loc = if !app.install_location.is_empty() {
        &app.install_location
    } else if !app.display_icon.is_empty() {
        app.display_icon.split(',').next().unwrap_or(&app.display_icon).trim().trim_matches('"')
    } else if !app.uninstall_string.is_empty() {
        app.uninstall_string.trim().trim_matches('"')
    } else {
        ""
    };

    let trimmed = loc.trim().trim_start_matches('"');
    let mut chars = trimmed.chars();
    if let (Some(c), Some(':')) = (chars.next(), chars.next()) {
        if c.is_ascii_alphabetic() {
            return c.to_ascii_uppercase();
        }
    }
    'C'
}

// --- GPU & DirectX Shader Cache Management ---

pub fn count_dir_files_and_size(dir: &Path) -> (usize, u64) {
    let mut count = 0;
    let mut total_size = 0;
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_file() {
                count += 1;
                total_size += entry.metadata().map(|m| m.len()).unwrap_or(0);
            } else if p.is_dir() {
                let (sub_count, sub_size) = count_dir_files_and_size(&p);
                count += sub_count;
                total_size += sub_size;
            }
        }
    }
    (count, total_size)
}

pub fn scan_shader_caches() -> Vec<ShaderCacheLocation> {
    let mut locations = Vec::new();
    let local = std::env::var("LOCALAPPDATA").unwrap_or_default();
    let appdata = std::env::var("APPDATA").unwrap_or_default();

    let targets = [
        ("nvidia_dxcache", "NVIDIA DirectX Shader Cache (DXCache)", "NVIDIA", format!(r"{}\NVIDIA\DXCache", local)),
        ("nvidia_glcache", "NVIDIA OpenGL Cache (GLCache)", "NVIDIA", format!(r"{}\NVIDIA\GLCache", local)),
        ("nvidia_compute", "NVIDIA Compute Cache", "NVIDIA", format!(r"{}\NVIDIA\ComputeCache", appdata)),
        ("dx_d3d", "DirectX D3D Shader Cache", "DirectX", format!(r"{}\D3DSCache", local)),
        ("dx_directx", "DirectX Shader Cache", "DirectX", format!(r"{}\DirectXShaderCache", local)),
        ("amd_dxcache", "AMD Radeon DirectX Cache", "AMD", format!(r"{}\AMD\DxCache", local)),
        ("amd_glcache", "AMD Radeon OpenGL Cache", "AMD", format!(r"{}\AMD\GLCache", local)),
    ];

    for (id, name, vendor, p_str) in targets {
        let p = Path::new(&p_str);
        if p.exists() && p.is_dir() {
            let (count, size) = count_dir_files_and_size(p);
            if count > 0 || size > 0 {
                locations.push(ShaderCacheLocation {
                    id: id.to_string(),
                    name: name.to_string(),
                    vendor: vendor.to_string(),
                    path: p_str,
                    file_count: count,
                    size_bytes: size,
                    selected: true,
                });
            }
        }
    }

    locations
}

pub fn delete_dir_contents_safe(dir: &Path, deleted_files: &mut usize, deleted_bytes: &mut u64) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_file() {
                let sz = entry.metadata().map(|m| m.len()).unwrap_or(0);
                if std::fs::remove_file(&p).is_ok() {
                    *deleted_files += 1;
                    *deleted_bytes += sz;
                }
            } else if p.is_dir() {
                delete_dir_contents_safe(&p, deleted_files, deleted_bytes);
                let _ = std::fs::remove_dir(&p);
            }
        }
    }
}

pub fn clear_shader_cache_targets(targets: &[ShaderCacheLocation]) -> (usize, u64) {
    let mut total_files = 0;
    let mut total_bytes = 0;
    for loc in targets {
        if loc.selected {
            let p = Path::new(&loc.path);
            if p.exists() && p.is_dir() {
                delete_dir_contents_safe(p, &mut total_files, &mut total_bytes);
            }
        }
    }
    (total_files, total_bytes)
}

// ============================================================================
// 1. Environment Variable PATH Sanitizer
// ============================================================================

#[cfg(windows)]
pub fn broadcast_environment_change() {
    use std::os::windows::ffi::OsStrExt;
    use win32_api::*;
    let env_str: Vec<u16> = std::ffi::OsStr::new("Environment")
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let mut result: usize = 0;
    unsafe {
        SendMessageTimeoutW(
            HWND_BROADCAST,
            WM_SETTINGCHANGE,
            0,
            env_str.as_ptr(),
            SMTO_ABORTIFHUNG,
            1000,
            &mut result,
        );
    }
}
#[cfg(not(windows))]
pub fn broadcast_environment_change() {}

fn expand_env_vars(raw: &str) -> String {
    let mut result = raw.to_string();
    let vars = [
        "USERPROFILE", "SystemRoot", "WINDIR", "LOCALAPPDATA",
        "APPDATA", "ProgramFiles", "ProgramFiles(x86)", "CommonProgramFiles", "PUBLIC"
    ];
    for v in vars {
        let pattern = format!("%{}%", v);
        if result.contains(&pattern) {
            if let Ok(val) = std::env::var(v) {
                result = result.replace(&pattern, &val);
            }
        }
    }
    result
}

pub fn scan_path_entries() -> Vec<PathEntry> {
    let mut entries = Vec::new();
    let mut seen_paths: HashSet<String> = HashSet::new();

    // 1. User PATH (HKCU\Environment)
    let user_path = if let Ok(key) = RegKey::predef(HKEY_CURRENT_USER).open_subkey("Environment") {
        key.get_value::<String, _>("Path").unwrap_or_default()
    } else {
        String::new()
    };

    // 2. System PATH (HKLM\SYSTEM\CurrentControlSet\Control\Session Manager\Environment)
    let sys_path = if let Ok(key) = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey(r"SYSTEM\CurrentControlSet\Control\Session Manager\Environment") {
        key.get_value::<String, _>("Path").unwrap_or_default()
    } else {
        String::new()
    };

    let mut parse_chunk = |raw_chunk: &str, is_sys: bool| {
        for part in raw_chunk.split(';') {
            let p_trimmed = part.trim().trim_matches('"').trim();
            if p_trimmed.is_empty() {
                continue;
            }
            let norm = p_trimmed.to_lowercase();
            let is_dup = seen_paths.contains(&norm);
            seen_paths.insert(norm);

            let expanded = expand_env_vars(p_trimmed);
            let exists = Path::new(&expanded).exists();
            let selected = !exists || is_dup;

            entries.push(PathEntry {
                path: p_trimmed.to_string(),
                is_system: is_sys,
                exists,
                is_duplicate: is_dup,
                selected,
            });
        }
    };

    parse_chunk(&user_path, false);
    parse_chunk(&sys_path, true);

    entries
}

pub fn clean_path_entries(current_entries: &[PathEntry]) -> Result<(usize, usize), String> {
    let mut user_survivors = Vec::new();
    let mut sys_survivors = Vec::new();
    let mut user_removed = 0;
    let mut sys_removed = 0;

    for entry in current_entries {
        if entry.selected {
            if entry.is_system {
                sys_removed += 1;
            } else {
                user_removed += 1;
            }
        } else {
            if entry.is_system {
                sys_survivors.push(entry.path.as_str());
            } else {
                user_survivors.push(entry.path.as_str());
            }
        }
    }

    // Write back user PATH
    if let Ok((user_key, _)) = RegKey::predef(HKEY_CURRENT_USER).create_subkey("Environment") {
        let new_user_path = user_survivors.join(";");
        let _ = user_key.set_value("Path", &new_user_path);
    }

    // Write back system PATH if changes exist
    if sys_removed > 0 {
        match RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey_with_flags(
            r"SYSTEM\CurrentControlSet\Control\Session Manager\Environment",
            KEY_SET_VALUE,
        ) {
            Ok(sys_key) => {
                let new_sys_path = sys_survivors.join(";");
                let _ = sys_key.set_value("Path", &new_sys_path);
            }
            Err(_) => {
                broadcast_environment_change();
                return Err(format!("Пользовательский PATH очищен (удалено {} записей), но для очистки системного PATH ({} записей) запустите PurgeCraft от имени Администратора.", user_removed, sys_removed));
            }
        }
    }

    broadcast_environment_change();
    Ok((user_removed, sys_removed))
}

// ============================================================================
// 2. Portable Applications Scanner
// ============================================================================

pub fn scan_portable_apps(custom_folders: &[String], installed_apps: &[InstalledApp]) -> Vec<PortableApp> {
    let mut known_dirs: HashSet<String> = HashSet::new();
    for app in installed_apps {
        if !app.install_location.is_empty() {
            known_dirs.insert(app.install_location.to_lowercase());
        }
        if !app.uninstall_string.is_empty() {
            if let Some(parent) = Path::new(&app.uninstall_string).parent() {
                known_dirs.insert(parent.to_string_lossy().to_lowercase());
            }
        }
    }

    let mut search_dirs: Vec<PathBuf> = Vec::new();

    // User Desktop
    if let Ok(desktop) = std::env::var("USERPROFILE").map(|u| PathBuf::from(u).join("Desktop")) {
        if desktop.is_dir() {
            search_dirs.push(desktop);
        }
    }
    // User Downloads
    if let Ok(downloads) = std::env::var("USERPROFILE").map(|u| PathBuf::from(u).join("Downloads")) {
        if downloads.is_dir() {
            search_dirs.push(downloads);
        }
    }
    // Standard Portable locations
    for cand in [r"C:\Portable", r"C:\PortableApps", r"D:\Portable", r"D:\PortableApps", r"D:\Soft"] {
        let p = PathBuf::from(cand);
        if p.is_dir() {
            search_dirs.push(p);
        }
    }
    // Custom folders
    for custom in custom_folders {
        let p = PathBuf::from(custom);
        if p.is_dir() && !search_dirs.contains(&p) {
            search_dirs.push(p);
        }
    }

    let excluded_names = [
        "setup.exe", "installer.exe", "install.exe", "unins000.exe", "uninstall.exe",
        "update.exe", "updater.exe", "crashpad_handler.exe", "elevation_service.exe",
        "chrome_proxy.exe", "vc_redist.x64.exe", "vc_redist.x86.exe"
    ];

    let mut found = Vec::new();
    let mut seen_exes: HashSet<String> = HashSet::new();

    for base in search_dirs {
        scan_portable_dir(&base, 0, &known_dirs, &excluded_names, &mut seen_exes, &mut found);
    }

    found
}

fn scan_portable_dir(
    dir: &Path,
    depth: usize,
    known_dirs: &HashSet<String>,
    excluded_names: &[&str],
    seen_exes: &mut HashSet<String>,
    found: &mut Vec<PortableApp>,
) {
    if depth > 3 {
        return;
    }
    let dir_lower = dir.to_string_lossy().to_lowercase();
    if known_dirs.contains(&dir_lower) {
        return;
    }

    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                let mut folder_exes = Vec::new();
                if let Ok(sub_entries) = std::fs::read_dir(&p) {
                    for sub in sub_entries.flatten() {
                        let sub_p = sub.path();
                        if sub_p.is_file() {
                            if let Some(ext) = sub_p.extension() {
                                if ext.eq_ignore_ascii_case("exe") {
                                    let fname = sub_p.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
                                    if !excluded_names.contains(&fname.as_str()) {
                                        folder_exes.push(sub_p);
                                    }
                                }
                            }
                        }
                    }
                }

                if !folder_exes.is_empty() {
                    let primary_exe = folder_exes[0].clone();
                    let exe_str = primary_exe.to_string_lossy().to_string();
                    let exe_lower = exe_str.to_lowercase();

                    if !seen_exes.contains(&exe_lower) {
                        seen_exes.insert(exe_lower);
                        let (count, size) = count_dir_files_and_size(&p);
                        let mod_date = entry.metadata()
                            .and_then(|m| m.modified())
                            .ok()
                            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                            .map(|d| {
                                let secs = d.as_secs() as i64;
                                chrono::DateTime::from_timestamp(secs, 0)
                                    .map(|dt| dt.format("%Y-%m-%d").to_string())
                                    .unwrap_or_else(|| "Неизвестно".to_string())
                            })
                            .unwrap_or_else(|| "Неизвестно".to_string());

                        let name = p.file_name().unwrap_or_default().to_string_lossy().to_string();

                        found.push(PortableApp {
                            name,
                            exe_path: exe_str,
                            folder_path: p.to_string_lossy().to_string(),
                            size_bytes: size,
                            file_count: count,
                            modified_date: mod_date,
                            selected: false,
                        });
                    }
                } else {
                    scan_portable_dir(&p, depth + 1, known_dirs, excluded_names, seen_exes, found);
                }
            } else if p.is_file() && depth == 0 {
                if let Some(ext) = p.extension() {
                    if ext.eq_ignore_ascii_case("exe") {
                        let fname = p.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
                        if !excluded_names.contains(&fname.as_str()) {
                            let exe_str = p.to_string_lossy().to_string();
                            let exe_lower = exe_str.to_lowercase();
                            if !seen_exes.contains(&exe_lower) {
                                seen_exes.insert(exe_lower);
                                let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                                if size > 1_000_000 {
                                    let mod_date = entry.metadata()
                                        .and_then(|m| m.modified())
                                        .ok()
                                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                                        .map(|d| {
                                            let secs = d.as_secs() as i64;
                                            chrono::DateTime::from_timestamp(secs, 0)
                                                .map(|dt| dt.format("%Y-%m-%d").to_string())
                                                .unwrap_or_else(|| "Неизвестно".to_string())
                                        })
                                        .unwrap_or_else(|| "Неизвестно".to_string());

                                    let name = p.file_stem().unwrap_or_default().to_string_lossy().to_string();
                                    found.push(PortableApp {
                                        name,
                                        exe_path: exe_str.clone(),
                                        folder_path: exe_str,
                                        size_bytes: size,
                                        file_count: 1,
                                        modified_date: mod_date,
                                        selected: false,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

pub fn delete_portable_app(app: &PortableApp) -> Result<u64, String> {
    let p = Path::new(&app.folder_path);
    if !p.exists() {
        return Err("Файл или папка не существует".to_string());
    }
    let mut files_del = 0;
    let mut bytes_del = 0;
    if p.is_dir() {
        delete_dir_contents_safe(p, &mut files_del, &mut bytes_del);
        std::fs::remove_dir_all(p).map_err(|e| e.to_string())?;
        Ok(bytes_del)
    } else {
        let sz = p.metadata().map(|m| m.len()).unwrap_or(0);
        std::fs::remove_file(p).map_err(|e| e.to_string())?;
        Ok(sz)
    }
}

// ============================================================================
// 3. Duplicate Runtime Detector (Visual C++, Java, .NET)
// ============================================================================

pub fn detect_runtime_duplicates(installed_apps: &[InstalledApp]) -> Vec<RuntimeFamilyGroup> {
    let mut vc_items = Vec::new();
    let mut java_items = Vec::new();
    let mut dotnet_items = Vec::new();

    for app in installed_apps {
        let name_lower = app.name.to_lowercase();

        // 1. Visual C++
        if name_lower.contains("visual c++") || name_lower.contains("vcredist") {
            let arch = if name_lower.contains("x64") || name_lower.contains("64-bit") {
                "x64"
            } else if name_lower.contains("x86") || name_lower.contains("32-bit") {
                "x86"
            } else if app.is_64bit {
                "x64"
            } else {
                "x86"
            };

            let version = if name_lower.contains("2005") {
                "2005"
            } else if name_lower.contains("2008") {
                "2008"
            } else if name_lower.contains("2010") {
                "2010"
            } else if name_lower.contains("2012") {
                "2012"
            } else if name_lower.contains("2013") {
                "2013"
            } else if name_lower.contains("2015-2022") || name_lower.contains("2015+") || name_lower.contains("v14") {
                "2015-2022"
            } else {
                "Другая"
            };

            let is_dep = version == "2005" || version == "2008";

            vc_items.push(RuntimeItem {
                family: "Microsoft Visual C++".to_string(),
                version: version.to_string(),
                arch: arch.to_string(),
                display_name: app.name.clone(),
                app_id: app.id.clone(),
                is_deprecated: is_dep,
                uninstall_string: app.uninstall_string.clone(),
                size_bytes: app.size_bytes,
            });
        }
        // 2. Java
        else if (name_lower.contains("java") || name_lower.contains("jdk") || name_lower.contains("jre"))
            && !name_lower.contains("javascript")
        {
            let arch = if name_lower.contains("64-bit") || app.is_64bit { "x64" } else { "x86" };
            let is_dep = name_lower.contains("java 6") || name_lower.contains("java 7") || name_lower.contains("jre 6") || name_lower.contains("jre 7");

            java_items.push(RuntimeItem {
                family: "Java Runtime / JDK".to_string(),
                version: if app.version.is_empty() { "Не указана".to_string() } else { app.version.clone() },
                arch: arch.to_string(),
                display_name: app.name.clone(),
                app_id: app.id.clone(),
                is_deprecated: is_dep,
                uninstall_string: app.uninstall_string.clone(),
                size_bytes: app.size_bytes,
            });
        }
        // 3. .NET
        else if name_lower.contains(".net") || name_lower.contains("dotnet") || name_lower.contains("desktop runtime") {
            let arch = if name_lower.contains("x64") || app.is_64bit { "x64" } else { "x86" };
            let is_dep = name_lower.contains("3.1") || name_lower.contains("5.0") || name_lower.contains("7.0");

            dotnet_items.push(RuntimeItem {
                family: "Microsoft .NET Runtime".to_string(),
                version: if app.version.is_empty() { "Не указана".to_string() } else { app.version.clone() },
                arch: arch.to_string(),
                display_name: app.name.clone(),
                app_id: app.id.clone(),
                is_deprecated: is_dep,
                uninstall_string: app.uninstall_string.clone(),
                size_bytes: app.size_bytes,
            });
        }
    }

    let mut groups = Vec::new();

    let build_group = |name: &str, items: Vec<RuntimeItem>| -> Option<RuntimeFamilyGroup> {
        if items.is_empty() {
            return None;
        }
        let mut key_counts: std::collections::HashMap<(String, String), usize> = std::collections::HashMap::new();
        for it in &items {
            *key_counts.entry((it.version.clone(), it.arch.clone())).or_insert(0) += 1;
        }
        let dup_count = key_counts.values().filter(|&&c| c > 1).sum::<usize>();
        let has_dep = items.iter().any(|i| i.is_deprecated);

        Some(RuntimeFamilyGroup {
            family_name: name.to_string(),
            items,
            duplicate_count: dup_count,
            has_deprecated: has_dep,
        })
    };

    if let Some(g) = build_group("Microsoft Visual C++ Redistributable", vc_items) {
        groups.push(g);
    }
    if let Some(g) = build_group("Java Development Kit / Runtime", java_items) {
        groups.push(g);
    }
    if let Some(g) = build_group("Microsoft .NET & Windows App Runtimes", dotnet_items) {
        groups.push(g);
    }

    groups
}

// ============================================================================
// 4. Winget CLI Integration
// ============================================================================

#[allow(dead_code)]
pub fn is_winget_available() -> bool {
    let mut cmd = Command::new("winget");
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd.arg("--version").output().map(|o| o.status.success()).unwrap_or(false)
}

pub fn parse_winget_list_output(text: &str) -> Vec<WingetAppItem> {
    let mut results = Vec::new();
    let mut past_header = false;
    let mut id_col = 0;
    let mut ver_col = 0;
    let mut avail_col = 0;

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if !past_header {
            if line.contains("---") {
                past_header = true;
                continue;
            }
            // Detect column positions in characters from header row
            let lower_chars: Vec<char> = line.to_lowercase().chars().collect();
            if let Some(pos) = lower_chars.windows(2).position(|w| w == ['и', 'д'] || w == ['i', 'd']) {
                id_col = pos;
            }
            if let Some(pos) = lower_chars.windows(6).position(|w| w == ['в', 'е', 'р', 'с', 'и', 'я'])
                .or_else(|| lower_chars.windows(7).position(|w| w == ['v', 'e', 'r', 's', 'i', 'o', 'n'])) {
                ver_col = pos;
            }
            if let Some(pos) = lower_chars.windows(8).position(|w| w == ['д', 'о', 'с', 'т', 'у', 'п', 'н', 'о'])
                .or_else(|| lower_chars.windows(9).position(|w| w == ['a', 'v', 'a', 'i', 'l', 'a', 'b', 'l', 'e'])) {
                avail_col = pos;
            }
            continue;
        }

        if id_col == 0 {
            id_col = 60;
            ver_col = 96;
            avail_col = 113;
        }

        let chars: Vec<char> = line.chars().collect();
        if chars.len() <= id_col {
            continue;
        }

        let name: String = chars[..id_col.min(chars.len())].iter().collect();
        let name = name.trim().to_string();

        let id: String = if chars.len() > id_col {
            let end = if ver_col > id_col { ver_col.min(chars.len()) } else { chars.len() };
            chars[id_col..end].iter().collect()
        } else {
            String::new()
        };
        let id = id.trim().to_string();
        if id.is_empty() {
            continue;
        }

        let version: String = if ver_col > 0 && chars.len() > ver_col {
            let end = if avail_col > ver_col { avail_col.min(chars.len()) } else { chars.len() };
            chars[ver_col..end].iter().collect()
        } else {
            String::new()
        };
        let version = version.trim().to_string();

        let available: String = if avail_col > 0 && chars.len() > avail_col {
            chars[avail_col..].iter().collect()
        } else {
            String::new()
        };
        let available = available.trim();
        let available_opt = if !available.is_empty() && available != "<" && !available.starts_with('<') {
            Some(available.to_string())
        } else {
            None
        };

        results.push(WingetAppItem {
            name: if name.is_empty() { id.clone() } else { name },
            id,
            installed_version: version,
            available_version: available_opt,
        });
    }
    results
}

pub fn scan_winget_packages() -> Vec<WingetAppItem> {
    let mut cmd = Command::new("winget");
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    let output = cmd.args(["list", "--source", "winget"]).output();

    if let Ok(out) = output {
        if out.status.success() {
            let text = String::from_utf8_lossy(&out.stdout);
            return parse_winget_list_output(&text);
        }
    }
    Vec::new()
}

pub fn uninstall_winget_package(package_id: &str) -> Result<String, String> {
    let mut cmd = Command::new("winget");
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    let output = cmd.args([
        "uninstall",
        "--id", package_id,
        "-e",
        "--silent",
        "--accept-source-agreements"
    ])
    .output()
    .map_err(|e| e.to_string())?;

    if output.status.success() {
        Ok(format!("Пакет {} успешно удален через Winget.", package_id))
    } else {
        let err = String::from_utf8_lossy(&output.stderr);
        Err(if err.trim().is_empty() {
            String::from_utf8_lossy(&output.stdout).to_string()
        } else {
            err.to_string()
        })
    }
}

pub fn upgrade_winget_package(package_id: &str) -> Result<String, String> {
    let mut cmd = Command::new("winget");
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    let output = cmd.args([
        "upgrade",
        "--id", package_id,
        "-e",
        "--include-unknown",
        "--accept-source-agreements",
        "--accept-package-agreements",
        "--disable-interactivity"
    ])
    .output()
    .map_err(|e| e.to_string())?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if output.status.success() {
        Ok(format!("Пакет {} успешно обновлен через Winget.", package_id))
    } else {
        let err = if !stderr.trim().is_empty() {
            stderr.to_string()
        } else if !stdout.trim().is_empty() {
            stdout.to_string()
        } else {
            format!("Команда winget завершилась с кодом {}", output.status.code().unwrap_or(-1))
        };
        Err(err)
    }
}

#[cfg(windows)]
pub mod win32_api {
    use std::ffi::c_void;

    pub type HANDLE = *mut c_void;
    pub type BOOL = i32;
    pub type DWORD = u32;
    pub type WCHAR = u16;

    pub const INVALID_HANDLE_VALUE: HANDLE = -1isize as HANDLE;
    pub const TH32CS_SNAPPROCESS: DWORD = 0x00000002;
    pub const PROCESS_TERMINATE: DWORD = 0x0001;
    pub const PROCESS_QUERY_LIMITED_INFORMATION: DWORD = 0x1000;
    pub const MOVEFILE_DELAY_UNTIL_REBOOT: DWORD = 0x00000004;

    #[repr(C)]
    #[allow(non_snake_case)]
    pub struct PROCESSENTRY32W {
        pub dwSize: DWORD,
        pub cntUsage: DWORD,
        pub th32ProcessID: DWORD,
        pub th32DefaultHeapID: usize,
        pub th32ModuleID: DWORD,
        pub cntThreads: DWORD,
        pub th32ParentProcessID: DWORD,
        pub pcPriClassBase: i32,
        pub dwFlags: DWORD,
        pub szExeFile: [WCHAR; 260],
    }

    #[link(name = "kernel32")]
    extern "system" {
        pub fn CreateToolhelp32Snapshot(dwFlags: DWORD, th32ProcessID: DWORD) -> HANDLE;
        pub fn Process32FirstW(hSnapshot: HANDLE, lppe: *mut PROCESSENTRY32W) -> BOOL;
        pub fn Process32NextW(hSnapshot: HANDLE, lppe: *mut PROCESSENTRY32W) -> BOOL;
        pub fn OpenProcess(dwDesiredAccess: DWORD, bInheritHandle: BOOL, dwProcessId: DWORD) -> HANDLE;
        pub fn TerminateProcess(hProcess: HANDLE, uExitCode: u32) -> BOOL;
        pub fn CloseHandle(hObject: HANDLE) -> BOOL;
        pub fn QueryFullProcessImageNameW(hProcess: HANDLE, dwFlags: DWORD, lpExeName: *mut WCHAR, lpdwSize: *mut DWORD) -> BOOL;
        pub fn MoveFileExW(lpExistingFileName: *const WCHAR, lpNewFileName: *const WCHAR, dwFlags: DWORD) -> BOOL;
    }

    pub const HWND_BROADCAST: HANDLE = 0xffff as HANDLE;
    pub const WM_SETTINGCHANGE: u32 = 0x001A;
    pub const SMTO_ABORTIFHUNG: u32 = 0x0002;

    #[link(name = "user32")]
    extern "system" {
        pub fn SendMessageTimeoutW(
            hWnd: HANDLE,
            Msg: u32,
            wParam: usize,
            lParam: *const u16,
            fuFlags: u32,
            uTimeout: u32,
            lpdwResult: *mut usize,
        ) -> isize;
    }

    #[link(name = "advapi32")]
    extern "system" {
        pub fn RegDeleteTreeW(hKey: HANDLE, lpSubKey: *const WCHAR) -> i32;
        pub fn RegDeleteKeyExW(hKey: HANDLE, lpSubKey: *const WCHAR, samDesired: DWORD, Reserved: DWORD) -> i32;
        pub fn RegDeleteKeyW(hKey: HANDLE, lpSubKey: *const WCHAR) -> i32;
        pub fn RegOpenKeyExW(hKey: HANDLE, lpSubKey: *const WCHAR, ulOptions: DWORD, samDesired: DWORD, phkResult: *mut HANDLE) -> i32;
        pub fn RegCloseKey(hKey: HANDLE) -> i32;
        #[allow(dead_code)]
        pub fn RegDeleteValueW(hKey: HANDLE, lpValueName: *const WCHAR) -> i32;
    }
}

#[cfg(windows)]
pub fn native_delete_reg_tree(hive: winreg::HKEY, subkey: &str) -> bool {
    use std::os::windows::ffi::OsStrExt;
    use win32_api::*;

    let wide_subkey: Vec<u16> = std::ffi::OsStr::new(subkey).encode_wide().chain(std::iter::once(0)).collect();

    unsafe {
        let hive_h = hive as HANDLE;
        // 1. RegDeleteTreeW recursively deletes all subkeys & values
        let _ = RegDeleteTreeW(hive_h, wide_subkey.as_ptr());

        // 2. Ensure root subkey itself is gone (both 64-bit and 32-bit views)
        let _ = RegDeleteKeyExW(hive_h, wide_subkey.as_ptr(), KEY_WOW64_64KEY, 0);
        let _ = RegDeleteKeyExW(hive_h, wide_subkey.as_ptr(), KEY_WOW64_32KEY, 0);
        let _ = RegDeleteKeyW(hive_h, wide_subkey.as_ptr());

        // 3. Check if key is now gone
        let mut h_res: HANDLE = std::ptr::null_mut();
        let open_res = RegOpenKeyExW(hive_h, wide_subkey.as_ptr(), 0, KEY_READ, &mut h_res);
        if open_res == 0 {
            let _ = RegCloseKey(h_res);
            false
        } else {
            true
        }
    }
}
#[cfg(not(windows))]
pub fn native_delete_reg_tree(_hive: winreg::HKEY, _subkey: &str) -> bool {
    false
}

pub fn clear_readonly_recursive(p: &Path) {
    if let Ok(meta) = std::fs::symlink_metadata(p) {
        if meta.permissions().readonly() {
            let mut perms = meta.permissions();
            perms.set_readonly(false);
            let _ = std::fs::set_permissions(p, perms);
        }
        if meta.is_dir() {
            if let Ok(entries) = std::fs::read_dir(p) {
                for entry in entries.flatten() {
                    clear_readonly_recursive(&entry.path());
                }
            }
        }
    }
}

pub fn fast_delete_folder(p: &Path, original_path: &str) -> (bool, bool) {
    if !p.exists() {
        return (true, false);
    }

    // Step 1: Standard remove_dir_all
    if std::fs::remove_dir_all(p).is_ok() {
        return (true, false);
    }

    // Step 2: Clear read-only flags recursively and retry
    clear_readonly_recursive(p);
    if std::fs::remove_dir_all(p).is_ok() {
        return (true, false);
    }

    // Step 3: Windows shell fast directory removal (cmd /C rd /s /q)
    #[cfg(windows)]
    {
        let mut cmd = Command::new("cmd");
        cmd.args(["/C", "rd", "/s", "/q", original_path]);
        cmd.creation_flags(CREATE_NO_WINDOW);
        if let Ok(status) = cmd.status() {
            if status.success() && !p.exists() {
                return (true, false);
            }
        }
    }

    // Step 4: If stubborn locked files remain, schedule delayed removal on reboot
    let mut any_scheduled = false;
    if let Ok(entries) = std::fs::read_dir(p) {
        for entry in entries.flatten() {
            let child_p = entry.path();
            if child_p.exists() {
                let p_str = child_p.to_string_lossy();
                if schedule_delayed_removal_on_reboot(&p_str) {
                    any_scheduled = true;
                }
            }
        }
    }
    let folder_scheduled = schedule_delayed_removal_on_reboot(original_path);
    (any_scheduled || folder_scheduled, any_scheduled || folder_scheduled)
}

#[cfg(windows)]
pub fn schedule_delayed_removal_on_reboot(path: &str) -> bool {
    use std::os::windows::ffi::OsStrExt;
    use win32_api::*;

    let wide_path: Vec<u16> = std::ffi::OsStr::new(path).encode_wide().chain(std::iter::once(0)).collect();
    unsafe {
        MoveFileExW(wide_path.as_ptr(), std::ptr::null(), MOVEFILE_DELAY_UNTIL_REBOOT) != 0
    }
}
#[cfg(not(windows))]
pub fn schedule_delayed_removal_on_reboot(_path: &str) -> bool {
    false
}

#[cfg(windows)]
pub fn terminate_process_by_pid(pid: u32) -> bool {
    use win32_api::*;
    unsafe {
        let handle = OpenProcess(PROCESS_TERMINATE, 0, pid);
        if handle.is_null() || handle == INVALID_HANDLE_VALUE {
            return false;
        }
        let ok = TerminateProcess(handle, 1) != 0;
        CloseHandle(handle);
        ok
    }
}
#[cfg(not(windows))]
pub fn terminate_process_by_pid(_pid: u32) -> bool {
    false
}

#[cfg(windows)]
pub fn find_running_processes_for_app(app: &InstalledApp) -> Vec<RunningProcessInfo> {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    use win32_api::*;

    let mut result = Vec::new();

    let system_critical = [
        "system", "smss.exe", "csrss.exe", "wininit.exe", "services.exe", "lsass.exe",
        "svchost.exe", "fontdrvhost.exe", "dwm.exe", "explorer.exe", "taskhostw.exe",
        "sihost.exe", "ctfmon.exe", "conhost.exe", "purgecraft.exe", "cmd.exe", "powershell.exe", "cargo.exe"
    ];

    let current_pid = std::process::id();

    let mut target_names: Vec<String> = Vec::new();
    let mut install_loc = app.install_location.trim().trim_matches('"').to_lowercase();
    if install_loc.ends_with('\\') {
        install_loc.pop();
    }

    if !app.uninstall_string.is_empty() {
        let uninst_clean = app.uninstall_string.trim_matches('"');
        if let Some(exe_idx) = uninst_clean.to_lowercase().find(".exe") {
            let sub = &uninst_clean[..exe_idx + 4];
            if let Some(name) = Path::new(sub).file_name().and_then(|n| n.to_str()) {
                target_names.push(name.to_lowercase());
            }
        }
    }

    if !app.display_icon.is_empty() {
        let icon_clean = app.display_icon.split(',').next().unwrap_or("").trim_matches('"');
        if let Some(name) = Path::new(icon_clean).file_name().and_then(|n| n.to_str()) {
            if name.to_lowercase().ends_with(".exe") {
                target_names.push(name.to_lowercase());
            }
        }
    }

    let app_id_clean = app.id.to_lowercase();
    if app_id_clean.ends_with(".exe") {
        target_names.push(app_id_clean);
    }
    let app_name_token: String = app.name.chars().filter(|c| c.is_alphanumeric()).collect::<String>().to_lowercase();

    unsafe {
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snap == INVALID_HANDLE_VALUE || snap.is_null() {
            return result;
        }

        let mut entry: PROCESSENTRY32W = std::mem::zeroed();
        entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;

        if Process32FirstW(snap, &mut entry) != 0 {
            loop {
                let pid = entry.th32ProcessID;
                if pid != 0 && pid != current_pid {
                    let end_idx = entry.szExeFile.iter().position(|&c| c == 0).unwrap_or(entry.szExeFile.len());
                    let exe_name = OsString::from_wide(&entry.szExeFile[..end_idx]).to_string_lossy().to_string();
                    let exe_name_lower = exe_name.to_lowercase();

                    if !system_critical.contains(&exe_name_lower.as_str()) {
                        let mut full_path = String::new();
                        let h_proc = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
                        if !h_proc.is_null() && h_proc != INVALID_HANDLE_VALUE {
                            let mut path_buf = [0u16; 1024];
                            let mut path_len = path_buf.len() as u32;
                            if QueryFullProcessImageNameW(h_proc, 0, path_buf.as_mut_ptr(), &mut path_len) != 0 {
                                full_path = OsString::from_wide(&path_buf[..path_len as usize]).to_string_lossy().to_string();
                            }
                            CloseHandle(h_proc);
                        }

                        let full_path_lower = full_path.to_lowercase();
                        let mut matched = false;

                        if !install_loc.is_empty() && is_safe_app_dir(Path::new(&install_loc)) {
                            if full_path_lower.starts_with(&install_loc) {
                                matched = true;
                            }
                        }

                        if !matched {
                            for target in &target_names {
                                if &exe_name_lower == target {
                                    matched = true;
                                    break;
                                }
                            }
                        }

                        if !matched && app_name_token.len() >= 6 {
                            let proc_token: String = exe_name.chars().filter(|c| c.is_alphanumeric()).collect::<String>().to_lowercase();
                            if proc_token.contains(&app_name_token) || app_name_token.contains(&proc_token) {
                                matched = true;
                            }
                        }

                        if matched {
                            result.push(RunningProcessInfo {
                                pid,
                                name: exe_name,
                                exe_path: if full_path.is_empty() { exe_name_lower } else { full_path },
                            });
                        }
                    }
                }

                if Process32NextW(snap, &mut entry) == 0 {
                    break;
                }
            }
        }

        CloseHandle(snap);
    }

    result
}

#[cfg(not(windows))]
pub fn find_running_processes_for_app(_app: &InstalledApp) -> Vec<RunningProcessInfo> {
    Vec::new()
}

/// Deletes a leftover trace item.
/// Returns (success_or_scheduled: bool, scheduled_for_reboot: bool).
pub fn delete_trace(trace: &LeftoverTrace) -> (bool, bool) {
    if trace.trace_type == "folder" {
        let p = Path::new(&trace.path);
        fast_delete_folder(p, &trace.path)
    } else if trace.trace_type == "file" {
        let p = Path::new(&trace.path);
        if !p.exists() {
            return (true, false);
        }
        if std::fs::remove_file(p).is_ok() {
            return (true, false);
        }
        // If file removal failed (e.g. read-only attribute), clear read-only and retry
        if let Ok(meta) = p.metadata() {
            if meta.permissions().readonly() {
                let mut perms = meta.permissions();
                perms.set_readonly(false);
                let _ = std::fs::set_permissions(p, perms);
            }
        }
        if std::fs::remove_file(p).is_ok() {
            (true, false)
        } else {
            let scheduled = schedule_delayed_removal_on_reboot(&trace.path);
            (scheduled, scheduled)
        }
    } else if trace.trace_type == "registry_key" {
        if trace.path.contains(" -> ") {
            let parts: Vec<&str> = trace.path.split(" -> ").collect();
            if parts.len() == 2 {
                let key_path = parts[0].trim();
                let val_name = parts[1].trim();
                if let Some((hive, subkey)) = parse_reg_hive_and_sub(key_path) {
                    let root = RegKey::predef(hive);
                    match root.open_subkey_with_flags(&subkey, KEY_SET_VALUE) {
                        Ok(key) => {
                            let _ = key.delete_value(val_name);
                            return (true, false);
                        }
                        Err(_) => {
                            // Parent key doesn't even exist, so value is already deleted!
                            return (true, false);
                        }
                    }
                }
                let mut reg_cmd = Command::new("reg");
                #[cfg(windows)]
                reg_cmd.creation_flags(CREATE_NO_WINDOW);
                let ok = reg_cmd
                    .args(["delete", key_path, "/v", val_name, "/f"])
                    .output()
                    .map(|o| o.status.success())
                    .unwrap_or(false);
                return (ok, false);
            }
        }

        if let Some((hive, subkey)) = parse_reg_hive_and_sub(&trace.path) {
            let root = RegKey::predef(hive);
            // 1. If the key is already gone, return success immediately
            if root.open_subkey_with_flags(&subkey, KEY_READ).is_err() {
                return (true, false);
            }
            // 2. Fast native Win32 deletion (RegDeleteTreeW recursively removes all subkeys & values in <1ms)
            #[cfg(windows)]
            {
                if native_delete_reg_tree(hive, &subkey) {
                    return (true, false);
                }
            }
            // 3. Fallback to winreg delete_subkey_all
            if root.delete_subkey_all(&subkey).is_ok() {
                return (true, false);
            }
            // 4. Verify if it's gone
            if root.open_subkey_with_flags(&subkey, KEY_READ).is_err() {
                return (true, false);
            }
        }

        let mut reg_cmd = Command::new("reg");
        #[cfg(windows)]
        reg_cmd.creation_flags(CREATE_NO_WINDOW);
        let ok = reg_cmd
            .args(["delete", &trace.path, "/f"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        (ok, false)
    } else if trace.trace_type == "appx_package" {
        let mut ps_cmd = Command::new("powershell");
        #[cfg(windows)]
        ps_cmd.creation_flags(CREATE_NO_WINDOW);
        let ok = ps_cmd
            .args(["-NoProfile", "-NonInteractive", "-Command", &format!("Remove-AppxPackage -Package '{}'", trace.path)])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        (ok, false)
    } else {
        (false, false)
    }
}

pub fn is_safe_app_dir(path: &Path) -> bool {
    if !path.is_dir() {
        return false;
    }
    if path.components().count() < 2 {
        return false;
    }
    let p_str = path.to_string_lossy().to_lowercase();
    if p_str.ends_with(r"\steam\games") || p_str.ends_with(r"/steam/games") || p_str.contains(r"\steam\steam\games") {
        return false;
    }
    let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_lowercase();
    let dangerous_names = [
        "windows", "system32", "syswow64", "program files", "program files (x86)",
        "appdata", "local", "locallow", "roaming", "temp", "users", "common files", "microsoft"
    ];
    if dangerous_names.contains(&file_name.as_str()) {
        return false;
    }
    if p_str == "c:\\" || p_str == "d:\\" || p_str == "e:\\" || p_str.ends_with(":\\") {
        return false;
    }
    true
}

pub fn take_system_snapshot() -> SystemSnapshot {
    let mut snapshot = SystemSnapshot::default();
    snapshot.timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    // 1. Uninstall keys
    let uninstall_paths = [
        (HKEY_LOCAL_MACHINE, r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall", KEY_READ | KEY_WOW64_64KEY, true),
        (HKEY_LOCAL_MACHINE, r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall", KEY_READ | KEY_WOW64_32KEY, true),
        (HKEY_CURRENT_USER, r"Software\Microsoft\Windows\CurrentVersion\Uninstall", KEY_READ, false),
    ];
    for (hive, path, flags, is_hklm) in uninstall_paths {
        let root = RegKey::predef(hive);
        if let Ok(key) = root.open_subkey_with_flags(path, flags) {
            for sub in key.enum_keys().flatten() {
                if is_hklm {
                    snapshot.uninstall_keys_hklm.insert(sub);
                } else {
                    snapshot.uninstall_keys_hkcu.insert(sub);
                }
            }
        }
    }

    // 2. Run keys
    let run_paths = [
        (HKEY_LOCAL_MACHINE, r"SOFTWARE\Microsoft\Windows\CurrentVersion\Run", true),
        (HKEY_CURRENT_USER, r"Software\Microsoft\Windows\CurrentVersion\Run", false),
    ];
    for (hive, path, is_hklm) in run_paths {
        let root = RegKey::predef(hive);
        if let Ok(key) = root.open_subkey(path) {
            for (val_name, _) in key.enum_values().flatten() {
                if is_hklm {
                    snapshot.run_keys_hklm.insert(val_name);
                } else {
                    snapshot.run_keys_hkcu.insert(val_name);
                }
            }
        }
    }

    // 3. Software subkeys
    if let Ok(key) = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey("SOFTWARE") {
        for sub in key.enum_keys().flatten() {
            snapshot.software_subkeys_hklm.insert(sub);
        }
    }
    if let Ok(key) = RegKey::predef(HKEY_CURRENT_USER).open_subkey("Software") {
        for sub in key.enum_keys().flatten() {
            snapshot.software_subkeys_hkcu.insert(sub);
        }
    }

    // 4. Directory children
    let scan_dir = |dir_path: &str, set: &mut HashSet<String>| {
        if let Ok(entries) = std::fs::read_dir(dir_path) {
            for entry in entries.flatten() {
                set.insert(entry.path().to_string_lossy().to_string());
            }
        }
    };

    if let Ok(pf) = std::env::var("ProgramFiles") {
        scan_dir(&pf, &mut snapshot.program_files_dirs);
    }
    if let Ok(pf86) = std::env::var("ProgramFiles(x86)") {
        scan_dir(&pf86, &mut snapshot.program_files_dirs);
    }
    if let Ok(pd) = std::env::var("ProgramData") {
        scan_dir(&pd, &mut snapshot.program_files_dirs);
    }
    if let Ok(la) = std::env::var("LOCALAPPDATA") {
        scan_dir(&la, &mut snapshot.appdata_local_dirs);
    }
    if let Ok(ap) = std::env::var("APPDATA") {
        scan_dir(&ap, &mut snapshot.appdata_roaming_dirs);
    }
    if let Ok(temp) = std::env::var("TEMP") {
        scan_dir(&temp, &mut snapshot.temp_items);
    }
    scan_dir(r"C:\Windows\Temp", &mut snapshot.temp_items);

    snapshot
}

#[allow(dead_code)]
fn collect_all_files_in_dir(dir: &Path, app_name: &str, out: &mut Vec<LeftoverTrace>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if let Ok(meta) = entry.metadata() {
                if meta.is_dir() {
                    collect_all_files_in_dir(&p, app_name, out);
                } else {
                    out.push(LeftoverTrace {
                        id: format!("snap_file_{}", p.to_string_lossy()),
                        app_id: app_name.to_string(),
                        trace_type: "file".to_string(),
                        path: p.to_string_lossy().to_string(),
                        size_bytes: meta.len(),
                        confidence: 96,
                        selected: true,
                    });
                }
            }
        }
    }
}

pub fn diff_system_snapshot(before: &SystemSnapshot, app_name: &str) -> Vec<LeftoverTrace> {
    let current = take_system_snapshot();
    let mut traces = Vec::new();

    // Newly added Uninstall keys
    for key in current.uninstall_keys_hklm.difference(&before.uninstall_keys_hklm) {
        traces.push(LeftoverTrace {
            id: format!("snap_reg_hklm_uninst_{}", key),
            app_id: app_name.to_string(),
            trace_type: "registry_key".to_string(),
            path: format!(r"HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\{}", key),
            size_bytes: 0,
            confidence: 100,
            selected: true,
        });
    }
    for key in current.uninstall_keys_hkcu.difference(&before.uninstall_keys_hkcu) {
        traces.push(LeftoverTrace {
            id: format!("snap_reg_hkcu_uninst_{}", key),
            app_id: app_name.to_string(),
            trace_type: "registry_key".to_string(),
            path: format!(r"HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\{}", key),
            size_bytes: 0,
            confidence: 100,
            selected: true,
        });
    }

    // Newly added Run keys
    for key in current.run_keys_hklm.difference(&before.run_keys_hklm) {
        traces.push(LeftoverTrace {
            id: format!("snap_reg_hklm_run_{}", key),
            app_id: app_name.to_string(),
            trace_type: "registry_key".to_string(),
            path: format!(r"HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Run -> {}", key),
            size_bytes: 0,
            confidence: 95,
            selected: true,
        });
    }
    for key in current.run_keys_hkcu.difference(&before.run_keys_hkcu) {
        traces.push(LeftoverTrace {
            id: format!("snap_reg_hkcu_run_{}", key),
            app_id: app_name.to_string(),
            trace_type: "registry_key".to_string(),
            path: format!(r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run -> {}", key),
            size_bytes: 0,
            confidence: 95,
            selected: true,
        });
    }

    // Newly added software vendor keys
    for key in current.software_subkeys_hklm.difference(&before.software_subkeys_hklm) {
        traces.push(LeftoverTrace {
            id: format!("snap_reg_hklm_soft_{}", key),
            app_id: app_name.to_string(),
            trace_type: "registry_key".to_string(),
            path: format!(r"HKLM\SOFTWARE\{}", key),
            size_bytes: 0,
            confidence: 98,
            selected: true,
        });
    }
    for key in current.software_subkeys_hkcu.difference(&before.software_subkeys_hkcu) {
        traces.push(LeftoverTrace {
            id: format!("snap_reg_hkcu_soft_{}", key),
            app_id: app_name.to_string(),
            trace_type: "registry_key".to_string(),
            path: format!(r"HKCU\Software\{}", key),
            size_bytes: 0,
            confidence: 98,
            selected: true,
        });
    }

    // Newly added folders (and recursively enumerate files inside)
    let diff_folders = |curr_set: &HashSet<String>, prev_set: &HashSet<String>, traces_vec: &mut Vec<LeftoverTrace>, prefix: &str| {
        for folder in curr_set.difference(prev_set) {
            let p = Path::new(folder);
            if p.exists() {
                let is_dir = p.is_dir();
                let sz = if is_dir { get_folder_size(p) } else { p.metadata().map(|m| m.len()).unwrap_or(0) };
                traces_vec.push(LeftoverTrace {
                    id: format!("snap_fs_{}_{}", prefix, folder),
                    app_id: app_name.to_string(),
                    trace_type: if is_dir { "folder".to_string() } else { "file".to_string() },
                    path: folder.clone(),
                    size_bytes: sz,
                    confidence: 98,
                    selected: true,
                });
            }
        }
    };

    diff_folders(&current.program_files_dirs, &before.program_files_dirs, &mut traces, "pf");
    diff_folders(&current.appdata_local_dirs, &before.appdata_local_dirs, &mut traces, "local");
    diff_folders(&current.appdata_roaming_dirs, &before.appdata_roaming_dirs, &mut traces, "roam");
    diff_folders(&current.temp_items, &before.temp_items, &mut traces, "temp");

    traces
}

fn scan_and_emit_fs(
    dir: &Path,
    seen_paths: &mut HashSet<String>,
    tx: &std::sync::mpsc::Sender<TrackerLiveMsg>,
    depth: usize,
    max_depth: usize,
) {
    if depth > max_depth {
        return;
    }
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            let p_str = p.to_string_lossy().to_string();
            let p_lower = p_str.to_lowercase();
            let is_dir = entry.metadata().map(|m| m.is_dir()).unwrap_or(false);
            if !seen_paths.contains(&p_lower) {
                seen_paths.insert(p_lower);
                let action = if is_dir { "Создана папка".to_string() } else { "Создан файл".to_string() };
                let _ = tx.send(TrackerLiveMsg {
                    time: chrono::Local::now().format("%H:%M:%S").to_string(),
                    action,
                    path: p_str,
                });
            }
            if is_dir {
                scan_and_emit_fs(&p, seen_paths, tx, depth + 1, max_depth);
            }
        }
    }
}

fn record_existing_fs(dir: &Path, seen_paths: &mut HashSet<String>, depth: usize, max_depth: usize) {
    if depth > max_depth {
        return;
    }
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            seen_paths.insert(entry.path().to_string_lossy().to_lowercase());
            if entry.metadata().map(|m| m.is_dir()).unwrap_or(false) {
                record_existing_fs(&entry.path(), seen_paths, depth + 1, max_depth);
            }
        }
    }
}

pub fn spawn_install_tracker(
    installer_path: String,
) -> Result<(std::sync::mpsc::Receiver<TrackerLiveMsg>, std::sync::mpsc::Sender<()>, std::process::Child), String> {
    let mut cmd = Command::new(&installer_path);
    let child = cmd.spawn().map_err(|e| format!("Не удалось запустить инсталлятор: {}", e))?;

    let (tx, rx) = std::sync::mpsc::channel();
    let (stop_tx, stop_rx) = std::sync::mpsc::channel();

    let exe_name = Path::new(&installer_path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| installer_path.clone());

    std::thread::spawn(move || {
        let now = || chrono::Local::now().format("%H:%M:%S").to_string();

        let _ = tx.send(TrackerLiveMsg {
            time: now(),
            action: String::new(),
            path: "Запускается сервис Монитора Установки...".to_string(),
        });
        std::thread::sleep(std::time::Duration::from_millis(50));
        let _ = tx.send(TrackerLiveMsg {
            time: now(),
            action: String::new(),
            path: "Версия сервиса: 0x105".to_string(),
        });
        std::thread::sleep(std::time::Duration::from_millis(50));
        let _ = tx.send(TrackerLiveMsg {
            time: now(),
            action: String::new(),
            path: format!("Запускаем '{}'...", exe_name),
        });
        std::thread::sleep(std::time::Duration::from_millis(50));
        let _ = tx.send(TrackerLiveMsg {
            time: now(),
            action: String::new(),
            path: "Монитор установки запущен".to_string(),
        });

        let mut seen_paths = HashSet::new();
        let mut seen_reg_keys = HashSet::new();

        if let Ok(temp) = std::env::var("TEMP") {
            record_existing_fs(Path::new(&temp), &mut seen_paths, 0, 3);
        }
        record_existing_fs(Path::new(r"C:\Windows\Temp"), &mut seen_paths, 0, 2);
        if let Ok(local) = std::env::var("LOCALAPPDATA") {
            record_existing_fs(Path::new(&local), &mut seen_paths, 0, 2);
        }
        if let Ok(roam) = std::env::var("APPDATA") {
            record_existing_fs(Path::new(&roam), &mut seen_paths, 0, 2);
        }
        if let Ok(pf) = std::env::var("ProgramFiles") {
            record_existing_fs(Path::new(&pf), &mut seen_paths, 0, 2);
        }
        if let Ok(pf86) = std::env::var("ProgramFiles(x86)") {
            record_existing_fs(Path::new(&pf86), &mut seen_paths, 0, 2);
        }
        if let Ok(pd) = std::env::var("ProgramData") {
            record_existing_fs(Path::new(&pd), &mut seen_paths, 0, 2);
        }

        // Snapshot existing registry
        for (hive, name) in [(HKEY_CURRENT_USER, "HKCU"), (HKEY_LOCAL_MACHINE, "HKLM")] {
            if let Ok(key) = RegKey::predef(hive).open_subkey("Software") {
                for sub in key.enum_keys().flatten() {
                    seen_reg_keys.insert(format!(r"{}\Software\{}", name, sub).to_lowercase());
                }
            }
            if let Ok(key) = RegKey::predef(hive).open_subkey(r"Software\Microsoft\Windows\CurrentVersion\Uninstall") {
                for sub in key.enum_keys().flatten() {
                    seen_reg_keys.insert(format!(r"{}\Software\Microsoft\Windows\CurrentVersion\Uninstall\{}", name, sub).to_lowercase());
                }
            }
            if let Ok(key) = RegKey::predef(hive).open_subkey(r"Software\Microsoft\Windows\CurrentVersion\Run") {
                for (val, _) in key.enum_values().flatten() {
                    seen_reg_keys.insert(format!(r"{}\Software\Microsoft\Windows\CurrentVersion\Run -> {}", name, val).to_lowercase());
                }
            }
        }

        while stop_rx.try_recv().is_err() {
            std::thread::sleep(std::time::Duration::from_millis(120));

            // 1. Filesystem scan
            if let Ok(temp) = std::env::var("TEMP") {
                scan_and_emit_fs(Path::new(&temp), &mut seen_paths, &tx, 0, 4);
            }
            scan_and_emit_fs(Path::new(r"C:\Windows\Temp"), &mut seen_paths, &tx, 0, 3);
            if let Ok(local) = std::env::var("LOCALAPPDATA") {
                scan_and_emit_fs(Path::new(&local), &mut seen_paths, &tx, 0, 3);
            }
            if let Ok(roam) = std::env::var("APPDATA") {
                scan_and_emit_fs(Path::new(&roam), &mut seen_paths, &tx, 0, 3);
            }
            if let Ok(pf) = std::env::var("ProgramFiles") {
                scan_and_emit_fs(Path::new(&pf), &mut seen_paths, &tx, 0, 3);
            }
            if let Ok(pf86) = std::env::var("ProgramFiles(x86)") {
                scan_and_emit_fs(Path::new(&pf86), &mut seen_paths, &tx, 0, 3);
            }
            if let Ok(pd) = std::env::var("ProgramData") {
                scan_and_emit_fs(Path::new(&pd), &mut seen_paths, &tx, 0, 3);
            }

            // 2. Registry scan
            for (hive, name) in [(HKEY_CURRENT_USER, "HKCU"), (HKEY_LOCAL_MACHINE, "HKLM")] {
                if let Ok(key) = RegKey::predef(hive).open_subkey("Software") {
                    for sub in key.enum_keys().flatten() {
                        let full = format!(r"{}\Software\{}", name, sub);
                        let full_lower = full.to_lowercase();
                        if !seen_reg_keys.contains(&full_lower) {
                            seen_reg_keys.insert(full_lower);
                            let _ = tx.send(TrackerLiveMsg {
                                time: now(),
                                action: "Создан ключ реестра".to_string(),
                                path: full,
                            });
                        }
                    }
                }
                if let Ok(key) = RegKey::predef(hive).open_subkey(r"Software\Microsoft\Windows\CurrentVersion\Uninstall") {
                    for sub in key.enum_keys().flatten() {
                        let full = format!(r"{}\Software\Microsoft\Windows\CurrentVersion\Uninstall\{}", name, sub);
                        let full_lower = full.to_lowercase();
                        if !seen_reg_keys.contains(&full_lower) {
                            seen_reg_keys.insert(full_lower);
                            let _ = tx.send(TrackerLiveMsg {
                                time: now(),
                                action: "Создан ключ реестра".to_string(),
                                path: full,
                            });
                        }
                    }
                }
                if let Ok(key) = RegKey::predef(hive).open_subkey(r"Software\Microsoft\Windows\CurrentVersion\Run") {
                    for (val, _) in key.enum_values().flatten() {
                        let full = format!(r"{}\Software\Microsoft\Windows\CurrentVersion\Run -> {}", name, val);
                        let full_lower = full.to_lowercase();
                        if !seen_reg_keys.contains(&full_lower) {
                            seen_reg_keys.insert(full_lower);
                            let _ = tx.send(TrackerLiveMsg {
                                time: now(),
                                action: "Запись реестра".to_string(),
                                path: full,
                            });
                        }
                    }
                }
            }
        }
    });

    Ok((rx, stop_tx, child))
}

pub fn get_traced_apps_path() -> std::path::PathBuf {
    if let Ok(mut base) = std::env::current_exe() {
        base.pop();
        return base.join("traced_apps.json");
    }
    std::path::PathBuf::from("traced_apps.json")
}

pub fn load_traced_apps() -> Vec<TracedApp> {
    let p = get_traced_apps_path();
    if let Ok(content) = std::fs::read_to_string(&p) {
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        Vec::new()
    }
}

pub fn save_traced_apps(apps: &[TracedApp]) {
    let p = get_traced_apps_path();
    if let Ok(content) = serde_json::to_string_pretty(apps) {
        let _ = std::fs::write(&p, content);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_uwp_package_name() {
        assert_eq!(clean_uwp_package_name("40459File-New-Project.EarTrumpet"), "EarTrumpet");
        assert_eq!(clean_uwp_package_name("21090PaddyXu.QuickLook"), "QuickLook");
        assert_eq!(clean_uwp_package_name("Microsoft.WindowsAppRuntime.2"), "Microsoft WindowsAppRuntime 2");
        assert_eq!(clean_uwp_package_name("Microsoft.UI.Xaml.2.8"), "Microsoft UI Xaml 2 8");
        assert_eq!(clean_uwp_package_name("AppUp.IntelGraphicsExperience"), "IntelGraphicsExperience");
        assert_eq!(clean_uwp_package_name("NVIDIACorp.NVIDIAControlPanel"), "NVIDIAControlPanel");
        assert_eq!(clean_uwp_package_name("RealtekSemiconductorCorp.RealtekAudioControl"), "RealtekAudioControl");
        assert_eq!(clean_uwp_package_name("DevHome"), "DevHome");
    }

    #[test]
    fn test_parse_winget_list_output() {
        let sample = "\
Имя                                                          ИД                                  Версия           Доступно
-------------------------------------------------------------------------------------------------------------------------------
7-Zip 26.03 (x64)                                            7zip.7zip                           26.03            26.04
Antares 0.7.35                                               Fabio286.antares                    0.7.35           
Antigravity 2.19.1                                           Google.Antigravity                  2.19.1           2.24.0
";
        let parsed = parse_winget_list_output(sample);
        assert_eq!(parsed.len(), 3);
        assert_eq!(parsed[0].name, "7-Zip 26.03 (x64)");
        assert_eq!(parsed[0].id, "7zip.7zip");
        assert_eq!(parsed[0].installed_version, "26.03");
        assert_eq!(parsed[0].available_version, Some("26.04".to_string()));

        assert_eq!(parsed[1].name, "Antares 0.7.35");
        assert_eq!(parsed[1].id, "Fabio286.antares");
        assert_eq!(parsed[1].installed_version, "0.7.35");
        assert_eq!(parsed[1].available_version, None);

        assert_eq!(parsed[2].name, "Antigravity 2.19.1");
        assert_eq!(parsed[2].id, "Google.Antigravity");
        assert_eq!(parsed[2].available_version, Some("2.24.0".to_string()));
    }

    #[test]
    fn test_scan_startup_items() {
        let items = scan_startup_items();
        println!("Found {} startup items", items.len());
        for it in &items {
            println!("Startup item: [{}] {} (Source: {}, Enabled: {})", it.source_type, it.name, it.source, it.is_enabled);
        }
        assert!(!items.is_empty(), "Should discover startup items on Windows");
        let has_reg = items.iter().any(|i| i.source_type.contains("run"));
        println!("Has registry autorun items: {}", has_reg);
    }

    #[test]
    fn test_system_snapshot() {
        let snap = take_system_snapshot();
        println!("HKLM Uninstall keys: {}", snap.uninstall_keys_hklm.len());
        println!("HKCU Uninstall keys: {}", snap.uninstall_keys_hkcu.len());
        println!("Program Files dirs: {}", snap.program_files_dirs.len());
        assert!(!snap.uninstall_keys_hklm.is_empty(), "HKLM uninstall keys must not be empty");
        assert!(!snap.program_files_dirs.is_empty(), "Program Files dirs must not be empty");

        let diff = diff_system_snapshot(&snap, "TestApp");
        println!("Immediate diff against self: {} traces", diff.len());
    }

    #[test]
    fn test_traced_apps_persistence() {
        let test_app = TracedApp {
            id: "test_traced_1".to_string(),
            name: "Test Traced Program".to_string(),
            install_date: "2026-10-20".to_string(),
            installer_path: "C:\\test\\setup.exe".to_string(),
            traces: vec![
                LeftoverTrace {
                    id: "trace_1".to_string(),
                    app_id: "test_traced_1".to_string(),
                    trace_type: "folder".to_string(),
                    path: "C:\\Program Files\\TestApp".to_string(),
                    size_bytes: 1024,
                    confidence: 100,
                    selected: true,
                }
            ],
        };
        let mut apps = load_traced_apps();
        apps.retain(|a| a.id != "test_traced_1");
        apps.push(test_app);
        save_traced_apps(&apps);

        let reloaded = load_traced_apps();
        assert!(reloaded.iter().any(|a| a.id == "test_traced_1"));

        // Clean up test app
        let cleaned: Vec<_> = reloaded.into_iter().filter(|a| a.id != "test_traced_1").collect();
        save_traced_apps(&cleaned);
    }

    #[test]
    fn test_scan_leftovers_test_app() {
        let apps = scan_win32_apps();
        if let Some(test_app) = apps.iter().find(|a| a.id == "PurgeCraftTestApp" || a.name.contains("PurgeCraft Test")) {
            let traces = scan_leftovers(test_app);
            println!("Found {} leftover traces for PurgeCraft Test App", traces.len());
            for t in &traces {
                println!("Trace: [{}] {}", t.trace_type, t.path);
            }
            assert!(traces.iter().any(|t| t.path.contains("PurgeCraftTestApp")), "Must find PurgeCraftTestApp leftovers");
        }
    }

    #[test]
    fn test_running_processes_scan() {
        let dummy = InstalledApp {
            id: "explorer".to_string(),
            name: "Windows Explorer".to_string(),
            version: "1.0".to_string(),
            publisher: "Microsoft".to_string(),
            install_date: "20261020".to_string(),
            size_bytes: 1000,
            install_location: r"C:\NonExistentTestFolder123".to_string(),
            uninstall_string: "".to_string(),
            registry_path: "".to_string(),
            is_64bit: true,
            is_system_component: false,
            is_uwp: false,
            package_full_name: "".to_string(),
            display_icon: "".to_string(),
            is_new: false,
            source_badge: String::new(),
        };
        // Explorer is system critical so it should be excluded by safety check
        let procs = find_running_processes_for_app(&dummy);
        assert!(procs.is_empty(), "System-critical processes must not be targeted");
    }

    #[test]
    fn test_delayed_removal_reboot() {
        // Test that MoveFileExW can be called without panic
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("purgecraft_test_locked.tmp");
        let _ = std::fs::write(&test_file, b"test delayed lock");
        let scheduled = schedule_delayed_removal_on_reboot(&test_file.to_string_lossy());
        println!("MoveFileExW scheduled: {}", scheduled);
        let _ = std::fs::remove_file(&test_file);
    }

    #[test]
    fn test_get_app_drive_letter() {
        let app_d = InstalledApp {
            id: "witcher3".to_string(),
            name: "The Witcher 3".to_string(),
            version: "1.32".to_string(),
            publisher: "CD PROJEKT RED".to_string(),
            install_date: "20261020".to_string(),
            size_bytes: 70_000_000_000,
            install_location: r#"D:\Games\The Witcher 3"#.to_string(),
            uninstall_string: r#""D:\Games\The Witcher 3\uninstall.exe""#.to_string(),
            registry_path: "".to_string(),
            is_64bit: true,
            is_system_component: false,
            is_uwp: false,
            package_full_name: "".to_string(),
            display_icon: r#"D:\Games\The Witcher 3\bin\x64\witcher3.exe,0"#.to_string(),
            is_new: false,
            source_badge: "[Steam]".to_string(),
        };
        assert_eq!(get_app_drive_letter(&app_d), 'D');

        let app_c = InstalledApp {
            id: "office".to_string(),
            name: "Microsoft Office".to_string(),
            version: "16.0".to_string(),
            publisher: "Microsoft".to_string(),
            install_date: "20261020".to_string(),
            size_bytes: 5_000_000_000,
            install_location: r#"C:\Program Files\Microsoft Office"#.to_string(),
            uninstall_string: "".to_string(),
            registry_path: "".to_string(),
            is_64bit: true,
            is_system_component: false,
            is_uwp: false,
            package_full_name: "".to_string(),
            display_icon: "".to_string(),
            is_new: false,
            source_badge: String::new(),
        };
        assert_eq!(get_app_drive_letter(&app_c), 'C');
    }

    #[test]
    fn test_scan_shader_caches() {
        let caches = scan_shader_caches();
        println!("Found {} shader cache locations", caches.len());
        for c in &caches {
            println!("Cache [{}]: {} ({} files, {:.2} MB) at {}", c.vendor, c.name, c.file_count, c.size_bytes as f64 / (1024.0 * 1024.0), c.path);
        }
    }

    #[test]
    fn test_steam_games_scan() {
        let win32_apps = scan_win32_apps();
        if let Some(hitler) = win32_apps.iter().find(|a| a.name.contains("Hitler Hates Anime")) {
            println!("Hitler Hates Anime: Location='{}', Size={} bytes ({:.2} MB)", hitler.install_location, hitler.size_bytes, hitler.size_bytes as f64 / (1024.0 * 1024.0));
            assert!(hitler.install_location.to_lowercase().contains("hitler hates anime"));
            assert!(!hitler.install_location.to_lowercase().contains(r"steam\games"));
            assert!(hitler.size_bytes > 800_000_000);
        }

        let steam_apps = scan_steam_library_games(&[]);
        println!("Discovered {} Steam games across library folders", steam_apps.len());
        for g in &steam_apps {
            println!("Steam Game: {} (Drive {}, Size {:.2} GB)", g.name, get_app_drive_letter(g), g.size_bytes as f64 / (1024.0 * 1024.0 * 1024.0));
        }
    }

    #[test]
    fn test_startup_impact() {
        assert_eq!(calculate_startup_impact("Steam Client Bootstrapper", "C:\\Steam\\steam.exe"), StartupImpact::High);
        assert_eq!(calculate_startup_impact("Discord", "C:\\Users\\user\\AppData\\Local\\Discord\\app.exe"), StartupImpact::High);
        assert_eq!(calculate_startup_impact("Realtek Audio Service", "C:\\Program Files\\Realtek\\RtkAud.exe"), StartupImpact::Low);
    }

    #[test]
    fn test_path_entries_scan() {
        let entries = scan_path_entries();
        println!("Scanned {} PATH entries across User and System", entries.len());
        assert!(!entries.is_empty(), "PATH entries should not be empty on Windows");
        let broken: Vec<_> = entries.iter().filter(|e| !e.exists).collect();
        println!("Found {} broken/dead PATH entries:", broken.len());
        for b in &broken {
            println!("  [DEAD PATH] (Sys: {}) {}", b.is_system, b.path);
        }
    }

    #[test]
    fn test_detect_runtime_duplicates() {
        let mock_apps = vec![
            InstalledApp {
                id: "vc2010_1".to_string(),
                name: "Microsoft Visual C++ 2010  x64 Redistributable - 10.0.40219".to_string(),
                version: "10.0.40219".to_string(),
                publisher: "Microsoft".to_string(),
                install_date: "20261020".to_string(),
                size_bytes: 20_000_000,
                install_location: "".to_string(),
                uninstall_string: "".to_string(),
                registry_path: "".to_string(),
                is_64bit: true,
                is_system_component: false,
                is_uwp: false,
                package_full_name: "".to_string(),
                display_icon: "".to_string(),
                is_new: false,
                source_badge: "".to_string(),
            },
            InstalledApp {
                id: "vc2010_2".to_string(),
                name: "Microsoft Visual C++ 2010  x64 Redistributable - 10.0.30319".to_string(),
                version: "10.0.30319".to_string(),
                publisher: "Microsoft".to_string(),
                install_date: "20261020".to_string(),
                size_bytes: 19_000_000,
                install_location: "".to_string(),
                uninstall_string: "".to_string(),
                registry_path: "".to_string(),
                is_64bit: true,
                is_system_component: false,
                is_uwp: false,
                package_full_name: "".to_string(),
                display_icon: "".to_string(),
                is_new: false,
                source_badge: "".to_string(),
            },
            InstalledApp {
                id: "vc2005".to_string(),
                name: "Microsoft Visual C++ 2005 Redistributable".to_string(),
                version: "8.0".to_string(),
                publisher: "Microsoft".to_string(),
                install_date: "20261020".to_string(),
                size_bytes: 10_000_000,
                install_location: "".to_string(),
                uninstall_string: "".to_string(),
                registry_path: "".to_string(),
                is_64bit: false,
                is_system_component: false,
                is_uwp: false,
                package_full_name: "".to_string(),
                display_icon: "".to_string(),
                is_new: false,
                source_badge: "".to_string(),
            },
        ];

        let groups = detect_runtime_duplicates(&mock_apps);
        assert!(!groups.is_empty(), "Should group runtimes");
        let vc_group = groups.iter().find(|g| g.family_name.contains("Visual C++")).expect("Must find VC++ group");
        assert_eq!(vc_group.items.len(), 3);
        assert!(vc_group.duplicate_count >= 2, "Must detect 2010 x64 duplicate");
        assert!(vc_group.has_deprecated, "Must flag VC++ 2005 as deprecated/EOL");
    }

    #[test]
    fn test_portable_apps_scan() {
        let portables = scan_portable_apps(&[], &[]);
        println!("Discovered {} portable apps in default directories", portables.len());
        for p in &portables {
            println!("  [Portable] {} ({} files, {:.2} MB) -> {}", p.name, p.file_count, p.size_bytes as f64 / (1024.0 * 1024.0), p.exe_path);
        }
    }

    #[test]
    fn test_print_apps() {
        let mut win32 = scan_win32_apps();
        let uwp = scan_uwp_apps();
        println!("=== WIN32 APPS ({}) ===", win32.len());
        for (i, a) in win32.iter().take(15).enumerate() {
            println!("  Win32 [{}]: id='{}', name='{}', badge='{}'", i, a.id, a.name, a.source_badge);
        }
        println!("=== UWP APPS ({}) ===", uwp.len());
        for (i, a) in uwp.iter().take(15).enumerate() {
            println!("  UWP [{}]: id='{}', name='{}', badge='{}'", i, a.id, a.name, a.source_badge);
        }
        win32.extend(uwp);
        println!("=== SORTED COMBINED (take 20) ===");
        win32.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        for (i, a) in win32.iter().take(20).enumerate() {
            println!("  [{}] name='{}' (len={}), badge='{}'", i, a.name, a.name.len(), a.source_badge);
        }
    }
}

// --- Known Apps Cache for Highlighting Newly Installed Programs ---

pub fn get_known_apps_path() -> std::path::PathBuf {
    if let Ok(mut base) = std::env::current_exe() {
        base.pop();
        return base.join("known_apps.json");
    }
    std::path::PathBuf::from("known_apps.json")
}

pub fn load_known_apps() -> (HashSet<String>, bool) {
    let p = get_known_apps_path();
    if let Ok(content) = std::fs::read_to_string(&p) {
        if let Ok(set) = serde_json::from_str::<HashSet<String>>(&content) {
            return (set, true);
        }
    }
    (HashSet::new(), false)
}

pub fn save_known_apps(apps: &[InstalledApp]) {
    let p = get_known_apps_path();
    let mut set: HashSet<String> = HashSet::new();
    for a in apps {
        set.insert(a.name.clone());
        set.insert(a.id.clone());
    }
    if let Ok(content) = serde_json::to_string_pretty(&set) {
        let _ = std::fs::write(&p, content);
    }
}

pub fn apply_known_apps(apps: &mut [InstalledApp]) -> usize {
    let (known_set, exists) = load_known_apps();
    if !exists {
        save_known_apps(apps);
        return 0;
    }
    let mut new_count = 0;
    for app in apps.iter_mut() {
        if !known_set.contains(&app.name) && !known_set.contains(&app.id) {
            app.is_new = true;
            new_count += 1;
        }
    }
    new_count
}

// --- Realtime Streaming Leftovers Scanner & Cleaner ---

#[derive(Clone, Debug)]
pub enum ScanProgressMsg {
    Path { path: String, progress: f32 },
    TraceFound(LeftoverTrace),
    Done(Vec<LeftoverTrace>),
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub enum DeleteProgressMsg {
    ItemDeleted {
        path: String,
        success: bool,
        log_line: String,
        progress: f32,
    },
    Done {
        deleted_count: usize,
        bytes_freed: u64,
        log_path: std::path::PathBuf,
        log_content: String,
        scheduled_reboot_items: Vec<String>,
    },
}

pub fn parse_reg_hive_and_sub(full: &str) -> Option<(winreg::HKEY, String)> {
    let trimmed = full.trim();
    if let Some(rest) = trimmed.strip_prefix("HKEY_CURRENT_USER\\").or_else(|| trimmed.strip_prefix("HKCU\\")) {
        Some((HKEY_CURRENT_USER, rest.to_string()))
    } else if let Some(rest) = trimmed.strip_prefix("HKEY_LOCAL_MACHINE\\").or_else(|| trimmed.strip_prefix("HKLM\\")) {
        Some((HKEY_LOCAL_MACHINE, rest.to_string()))
    } else if let Some(rest) = trimmed.strip_prefix("HKEY_CLASSES_ROOT\\").or_else(|| trimmed.strip_prefix("HKCR\\")) {
        Some((HKEY_CLASSES_ROOT, rest.to_string()))
    } else if let Some(rest) = trimmed.strip_prefix("HKEY_USERS\\").or_else(|| trimmed.strip_prefix("HKU\\")) {
        Some((HKEY_USERS, rest.to_string()))
    } else if let Some(rest) = trimmed.strip_prefix("HKEY_CURRENT_CONFIG\\").or_else(|| trimmed.strip_prefix("HKCC\\")) {
        Some((HKEY_CURRENT_CONFIG, rest.to_string()))
    } else {
        None
    }
}

pub fn delete_registry_key_by_path(path: &str) -> bool {
    if let Some((hive, subkey)) = parse_reg_hive_and_sub(path) {
        let root = RegKey::predef(hive);
        // 1. If key is already gone, return true immediately!
        if root.open_subkey_with_flags(&subkey, KEY_READ).is_err() {
            return true;
        }
        // 2. Fast native Win32 deletion (<1ms)
        #[cfg(windows)]
        {
            if native_delete_reg_tree(hive, &subkey) {
                return true;
            }
        }
        // 3. Fallback to winreg delete_subkey_all
        if root.delete_subkey_all(&subkey).is_ok() {
            return true;
        }
        // 4. Verify if it's gone
        if root.open_subkey_with_flags(&subkey, KEY_READ).is_err() {
            return true;
        }
    }
    let mut cmd = Command::new("reg");
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd.args(["delete", path, "/f"]).output().map(|o| o.status.success()).unwrap_or(false)
}

pub fn spawn_scan_leftovers_stream(
    app: InstalledApp,
    traced_apps: Vec<TracedApp>,
) -> std::sync::mpsc::Receiver<ScanProgressMsg> {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let targets = build_app_leftover_targets(&app, &traced_apps);
        let mut found_traces = Vec::new();
        let total = targets.len().max(1);
        let mut last_ui_update = std::time::Instant::now();

        for (idx, (t_type, p_str)) in targets.iter().enumerate() {
            let progress = (idx as f32) / (total as f32);
            let now = std::time::Instant::now();
            if now.duration_since(last_ui_update) >= std::time::Duration::from_millis(16) || idx == total - 1 {
                let _ = tx.send(ScanProgressMsg::Path {
                    path: p_str.clone(),
                    progress,
                });
                last_ui_update = now;
            }
            if let Some(trace) = check_target_exists(t_type, p_str, &app, &mut found_traces) {
                let _ = tx.send(ScanProgressMsg::TraceFound(trace));
            }
        }
        let mut seen = HashSet::new();
        found_traces.retain(|t| seen.insert(t.path.to_lowercase()));

        let _ = tx.send(ScanProgressMsg::Path {
            path: "Сканирование завершено".to_string(),
            progress: 1.0,
        });
        let _ = tx.send(ScanProgressMsg::Done(found_traces));
    });
    rx
}

pub fn spawn_delete_leftovers_stream(
    app_name: String,
    traces: Vec<LeftoverTrace>,
    uninstall_reg_path: Option<String>,
) -> std::sync::mpsc::Receiver<DeleteProgressMsg> {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut log_lines = Vec::new();
        let mut deleted_count = 0;
        let mut bytes_freed = 0;
        let mut scheduled_reboot_items = Vec::new();

        // Track already removed folders and registry keys to skip thousands of child entries instantly
        let mut deleted_folders: Vec<String> = Vec::new();
        let mut deleted_reg_prefixes: Vec<String> = Vec::new();

        // Sort traces parent-first:
        // 1. Folders first (by shortest path length so parent folders delete before child folders)
        // 2. Registry keys (without ->) next (by shortest path length)
        // 3. Registry values (with ->) next
        // 4. Files next
        // 5. Appx packages last
        let mut sorted_traces = traces;
        sorted_traces.sort_by(|a, b| {
            let type_rank = |t: &LeftoverTrace| match t.trace_type.as_str() {
                "folder" => 0,
                "registry_key" if !t.path.contains(" -> ") => 1,
                "registry_key" => 2,
                "file" => 3,
                _ => 4,
            };
            let r_a = type_rank(a);
            let r_b = type_rank(b);
            if r_a != r_b {
                r_a.cmp(&r_b)
            } else {
                a.path.len().cmp(&b.path.len())
            }
        });

        let total = sorted_traces.len().max(1);
        let mut last_ui_update = std::time::Instant::now();

        for (i, trace) in sorted_traces.iter().enumerate() {
            let progress = (i as f32) / (total as f32);
            let norm = trace.path.to_lowercase().replace('/', "\\");

            let already_handled = if trace.trace_type == "file" || trace.trace_type == "folder" {
                deleted_folders.iter().any(|f| norm == *f || norm.starts_with(f))
            } else if trace.trace_type == "registry_key" {
                let key_part = if let Some(idx) = norm.find(" -> ") {
                    norm[..idx].trim()
                } else {
                    norm.as_str()
                };
                deleted_reg_prefixes.iter().any(|r| key_part == r || key_part.starts_with(&format!("{}\\", r)))
            } else {
                false
            };

            let (success, scheduled_reboot) = if already_handled {
                (true, false)
            } else {
                let res = delete_trace(trace);
                if res.0 && !res.1 {
                    if trace.trace_type == "folder" {
                        let mut f = norm.clone();
                        if !f.ends_with('\\') {
                            f.push('\\');
                        }
                        deleted_folders.push(f);
                    } else if trace.trace_type == "registry_key" && !trace.path.contains(" -> ") {
                        let r = norm.trim_end_matches('\\').to_string();
                        deleted_reg_prefixes.push(r);
                    }
                }
                res
            };

            let time_str = chrono::Local::now().format("%Y-%m-%d - %H:%M:%S").to_string();
            let log_line = if scheduled_reboot {
                format!("{} [ОТЛОЖЕНО] Запланировано удаление при перезагрузке (MoveFileEx): {}", time_str, trace.path)
            } else {
                format!("{} Removing {}", time_str, trace.path)
            };
            log_lines.push(log_line.clone());

            if success {
                deleted_count += 1;
                bytes_freed += trace.size_bytes;
            }
            if scheduled_reboot {
                scheduled_reboot_items.push(trace.path.clone());
            }

            let now = std::time::Instant::now();
            if now.duration_since(last_ui_update) >= std::time::Duration::from_millis(30) || i == total - 1 {
                let _ = tx.send(DeleteProgressMsg::ItemDeleted {
                    path: trace.path.clone(),
                    success,
                    log_line,
                    progress,
                });
                last_ui_update = now;
            }
        }

        // Clean registry uninstall key if specified
        if let Some(ref reg_p) = uninstall_reg_path {
            if !reg_p.is_empty() {
                let time_str = chrono::Local::now().format("%Y-%m-%d - %H:%M:%S").to_string();
                let _ = delete_registry_key_by_path(reg_p);
                log_lines.push(format!("{} Removing Uninstall registry key {}", time_str, reg_p));
            }
        }

        // Save log file in logs/ directory
        let log_dir = if let Ok(mut base) = std::env::current_exe() {
            base.pop();
            let d = base.join("logs");
            let _ = std::fs::create_dir_all(&d);
            d
        } else {
            let _ = std::fs::create_dir_all("logs");
            std::path::PathBuf::from("logs")
        };

        let safe_title: String = app_name.chars().filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-').collect();
        let log_filename = format!("uninstall_{}_{}.log", safe_title, chrono::Local::now().format("%Y%m%d_%H%M%S"));
        let log_path = log_dir.join(&log_filename);
        let log_content = log_lines.join("\r\n");
        let _ = std::fs::write(&log_path, &log_content);

        let _ = tx.send(DeleteProgressMsg::Done {
            deleted_count,
            bytes_freed,
            log_path,
            log_content,
            scheduled_reboot_items,
        });
    });
    rx
}

pub fn export_traces_to_xml(app_name: &str, traces: &[LeftoverTrace]) -> Result<std::path::PathBuf, String> {
    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n");
    xml.push_str(&format!("<UninstallToolLog application=\"{}\" date=\"{}\">\n", app_name, chrono::Local::now().format("%Y-%m-%d %H:%M:%S")));
    xml.push_str("  <FileSystem>\n");
    for t in traces.iter().filter(|t| t.trace_type != "registry_key") {
        xml.push_str(&format!("    <Item path=\"{}\" size_bytes=\"{}\" />\n", t.path, t.size_bytes));
    }
    xml.push_str("  </FileSystem>\n");
    xml.push_str("  <Registry>\n");
    for t in traces.iter().filter(|t| t.trace_type == "registry_key") {
        xml.push_str(&format!("    <Item path=\"{}\" />\n", t.path));
    }
    xml.push_str("  </Registry>\n");
    xml.push_str("</UninstallToolLog>\n");

    let export_dir = if let Ok(mut base) = std::env::current_exe() {
        base.pop();
        let d = base.join("exports");
        let _ = std::fs::create_dir_all(&d);
        d
    } else {
        let _ = std::fs::create_dir_all("exports");
        std::path::PathBuf::from("exports")
    };

    let safe_title: String = app_name.chars().filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-').collect();
    let export_path = export_dir.join(format!("{}_leftovers_{}.xml", safe_title, chrono::Local::now().format("%Y%m%d_%H%M%S")));
    std::fs::write(&export_path, xml).map_err(|e| e.to_string())?;
    Ok(export_path)
}



