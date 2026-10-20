#![windows_subsystem = "windows"]

mod scanner;
mod icons;
mod settings;
mod autoupdater;

use eframe::egui::{self, Color32, RichText, Vec2, Stroke};
use egui_extras::{TableBuilder, Column};
use scanner::{
    scan_win32_apps, scan_uwp_apps, scan_startup_items, scan_leftovers,
    toggle_startup_item, delete_startup_item, add_startup_item, pick_file_dialog, pick_folder_dialog,
    take_system_snapshot, diff_system_snapshot, load_traced_apps, save_traced_apps,
    apply_known_apps, save_known_apps, spawn_scan_leftovers_stream, spawn_delete_leftovers_stream,
    export_traces_to_xml, ScanProgressMsg, DeleteProgressMsg,
    InstalledApp, StartupItem, StartupImpact, LeftoverTrace, TracedApp, SystemSnapshot,
    ShaderCacheLocation, scan_shader_caches, clear_shader_cache_targets,
    PathEntry, scan_path_entries, clean_path_entries,
    PortableApp, scan_portable_apps, delete_portable_app,
    RuntimeFamilyGroup, detect_runtime_duplicates,
    WingetAppItem, scan_winget_packages, uninstall_winget_package, upgrade_winget_package,
};
use settings::{AppSettings, load_settings, save_settings, is_windows_dark_mode};

#[derive(PartialEq, Clone, Copy, Debug)]
enum WizardPhase {
    Scanning,
    Review,
    Deleting,
    Finished,
}
use std::process::Command;
use std::collections::{HashSet, HashMap};
#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[derive(PartialEq, Clone, Copy)]
enum ActiveTab {
    Uninstaller,
    Startup,
}

#[derive(PartialEq, Clone, Copy)]
enum SettingsTab {
    General,
    Interface,
    Uninstaller,
}

#[derive(PartialEq, Clone, Copy)]
enum AppCategory {
    All,
    UserApps,
    System,
    StoreApps,
    Portable,
    Arch64,
    Arch32,
}

#[derive(PartialEq, Clone, Copy)]
enum StartupFilter {
    All,
    Registry,
    StartMenu,
    TaskScheduler,
}

#[derive(PartialEq, Clone, Copy)]
enum TrackerStep {
    SelectInstaller,
    Installing,
    ReviewTraces,
}

#[derive(PartialEq, Clone, Copy)]
enum SortColumn {
    Name,
    Size,
    Date,
    Publisher,
    AppType,
}

#[derive(PartialEq, Clone, Copy)]
enum SortOrder {
    Ascending,
    Descending,
}

struct PurgeCraftApp {
    active_tab: ActiveTab,
    category: AppCategory,
    drive_filter: Option<char>,
    startup_filter: StartupFilter,
    apps: Vec<InstalledApp>,
    startup_items: Vec<StartupItem>,
    selected_app_idx: Option<usize>,
    search_query: String,
    status_msg: String,
    dark_mode: bool,

    // Background size calculation stream
    size_rx: std::sync::mpsc::Receiver<(String, u64)>,
    size_tx: std::sync::mpsc::Sender<(String, u64)>,

    // Sorting
    sort_column: SortColumn,
    sort_order: SortOrder,

    // Real App Icon texture cache
    icon_textures: HashMap<String, Option<egui::TextureHandle>>,
    brand_logo_tex: Option<egui::TextureHandle>,

    // Batch operations
    batch_selected: HashSet<String>, // set of app.id
    batch_queue_open: bool,
    batch_queue_apps: Vec<InstalledApp>,
    batch_queue_idx: usize,
    batch_queue_statuses: Vec<String>,
    batch_force_open: bool,
    batch_force_apps: Vec<InstalledApp>,

    // Leftovers wizard (1:1 with Uninstall Tool 4-step wizard)
    wizard_open: bool,
    wizard_phase: WizardPhase,
    wizard_app_title: String,
    wizard_target_app: Option<InstalledApp>,
    wizard_scan_current_path: String,
    wizard_scan_progress: f32,
    wizard_scan_rx: Option<std::sync::mpsc::Receiver<ScanProgressMsg>>,
    wizard_traces: Vec<LeftoverTrace>,
    wizard_delete_progress: f32,
    wizard_delete_rx: Option<std::sync::mpsc::Receiver<DeleteProgressMsg>>,
    wizard_last_log_path: Option<std::path::PathBuf>,
    wizard_last_log_content: String,
    wizard_xml_status: Option<String>,
    wizard_report: Option<String>,

    // Uninstaller Execution & Waiting
    uninstall_confirm_open: bool,
    uninstall_target_app: Option<InstalledApp>,
    uninstall_waiting_open: bool,
    uninstall_waiting_app: Option<InstalledApp>,
    uninstall_child: Option<std::process::Child>,

    // Force removal confirm
    force_confirm_open: bool,
    force_confirm_app: Option<InstalledApp>,

    // Running processes detection & termination modal (S09 RunningProcessesDialog)
    running_processes_modal_open: bool,
    running_processes: Vec<scanner::RunningProcessInfo>,
    running_processes_target_app: Option<InstalledApp>,

    // Delayed reboot files removal modal (S10 DelayedFilesRemovalDialog)
    delayed_removal_modal_open: bool,
    delayed_removal_files: Vec<String>,

    // New Apps Notification
    new_apps_count: usize,

    // Remove entry confirm
    remove_entry_confirm_open: bool,
    remove_entry_confirm_app: Option<InstalledApp>,
    focus_search_requested: bool,
    uwp_rx: std::sync::mpsc::Receiver<Vec<InstalledApp>>,
    uwp_tx: std::sync::mpsc::Sender<Vec<InstalledApp>>,

    // Add Startup Item Dialog
    add_startup_modal_open: bool,
    new_startup_name: String,
    new_startup_path: String,
    new_startup_args: String,
    new_startup_hklm: bool,
    new_startup_error: Option<String>,

    // Install Tracker
    tracker_modal_open: bool,
    tracker_step: TrackerStep,
    tracker_installer_path: String,
    tracker_app_name: String,
    tracker_snapshot_before: Option<SystemSnapshot>,
    tracker_traces: Vec<LeftoverTrace>,
    traced_apps: Vec<TracedApp>,
    tracker_live_log: Vec<scanner::TrackerLiveMsg>,
    tracker_rx: Option<std::sync::mpsc::Receiver<scanner::TrackerLiveMsg>>,
    tracker_stop_tx: Option<std::sync::mpsc::Sender<()>>,
    tracker_child: Option<std::process::Child>,
    tracker_process_finished: bool,

    // Settings Window
    settings_open: bool,
    settings_tab: SettingsTab,
    settings: AppSettings,
    temp_settings: AppSettings,

    // Shader Cache Cleaner
    shader_modal_open: bool,
    shader_locations: Vec<ShaderCacheLocation>,
    shader_clean_msg: Option<String>,
    shader_cleaning: bool,

    // PATH Sanitizer
    path_modal_open: bool,
    path_entries: Vec<PathEntry>,
    path_msg: Option<String>,

    // Portable Apps Scanner
    portable_modal_open: bool,
    portable_apps: Vec<PortableApp>,
    portable_msg: Option<String>,

    // Duplicate Runtime Detector
    runtime_modal_open: bool,
    runtime_groups: Vec<RuntimeFamilyGroup>,
    runtime_msg: Option<String>,

    // Winget Manager
    winget_modal_open: bool,
    winget_packages: Vec<WingetAppItem>,
    winget_msg: Option<String>,
    winget_task_rx: Option<std::sync::mpsc::Receiver<(Result<String, String>, Option<Vec<WingetAppItem>>)>>,
    winget_is_busy: bool,
    winget_busy_msg: String,

    // Async Startup & Portable Scan
    startup_rx: Option<std::sync::mpsc::Receiver<Vec<StartupItem>>>,
    portable_rx: Option<std::sync::mpsc::Receiver<Vec<PortableApp>>>,
    is_refreshing: bool,

    // Blacklist of uninstalled apps during this session
    recently_uninstalled: HashSet<String>,

    // GitHub Auto-updater
    update_status: autoupdater::UpdateStatus,
    update_rx: std::sync::mpsc::Receiver<autoupdater::UpdateStatus>,
    update_tx: std::sync::mpsc::Sender<autoupdater::UpdateStatus>,
    update_modal_open: bool,
}

fn jump_to_regedit(registry_path: &str) {
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

fn open_system_folder(path: &str, use_total_commander: bool) {
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

fn spawn_size_calculator(apps: &[InstalledApp], tx: std::sync::mpsc::Sender<(String, u64)>) {
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

fn render_sort_header(
    ui: &mut egui::Ui,
    text: &str,
    is_active: bool,
    order: SortOrder,
    dark_mode: bool,
) -> egui::Response {
    let desired_size = Vec2::new(ui.available_width(), 26.0);
    let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::click());

    if ui.is_rect_visible(rect) {
        let bg_fill = if is_active {
            if dark_mode { Color32::from_rgb(45, 48, 56) } else { Color32::from_rgb(224, 242, 254) }
        } else if response.hovered() {
            if dark_mode { Color32::from_rgb(38, 38, 45) } else { Color32::from_rgb(241, 245, 249) }
        } else {
            Color32::TRANSPARENT
        };

        let text_color = if is_active {
            if dark_mode { Color32::WHITE } else { Color32::from_rgb(2, 132, 199) }
        } else if response.hovered() {
            if dark_mode { Color32::from_rgb(244, 244, 245) } else { Color32::from_rgb(15, 23, 42) }
        } else {
            if dark_mode { Color32::from_rgb(212, 212, 216) } else { Color32::from_rgb(71, 85, 105) }
        };

        if bg_fill != Color32::TRANSPARENT {
            ui.painter().rect_filled(rect, egui::Rounding::same(4.0), bg_fill);
            if is_active {
                let stroke_color = if dark_mode { Color32::from_rgb(75, 80, 92) } else { Color32::from_rgb(186, 230, 253) };
                ui.painter().rect_stroke(rect, egui::Rounding::same(4.0), Stroke::new(1.0_f32, stroke_color));
            }
        }

        let font_id = egui::FontId::proportional(13.5);
        let text_pos = egui::pos2(rect.min.x + 8.0, rect.center().y);
        let text_rect = ui.painter().text(text_pos, egui::Align2::LEFT_CENTER, text, font_id, text_color);

        if is_active {
            let arrow_x = text_rect.right() + 6.0;
            let arrow_y = rect.center().y;
            let sz = 3.5;
            let points = match order {
                SortOrder::Ascending => vec![
                    egui::pos2(arrow_x - sz, arrow_y + 2.0),
                    egui::pos2(arrow_x + sz, arrow_y + 2.0),
                    egui::pos2(arrow_x, arrow_y - 2.5),
                ],
                SortOrder::Descending => vec![
                    egui::pos2(arrow_x - sz, arrow_y - 2.0),
                    egui::pos2(arrow_x + sz, arrow_y - 2.0),
                    egui::pos2(arrow_x, arrow_y + 2.5),
                ],
            };
            let arrow_color = if dark_mode { Color32::from_rgb(96, 165, 250) } else { Color32::from_rgb(2, 132, 199) };
            ui.painter().add(egui::Shape::convex_polygon(points, arrow_color, Stroke::NONE));
        }
    }

    response
}

impl Default for PurgeCraftApp {
    fn default() -> Self {
        let mut apps = scan_win32_apps();
        let new_apps_count = apply_known_apps(&mut apps);
        let total_size: u64 = apps.iter().map(|a| a.size_bytes).sum();
        let size_str = format_size(total_size);
        let count = apps.len();

        let (size_tx, size_rx) = std::sync::mpsc::channel();
        spawn_size_calculator(&apps, size_tx.clone());

        let (uwp_tx, uwp_rx) = std::sync::mpsc::channel();
        let u_tx = uwp_tx.clone();
        let s_tx = size_tx.clone();
        std::thread::spawn(move || {
            let uwp = scan_uwp_apps();
            spawn_size_calculator(&uwp, s_tx);
            let _ = u_tx.send(uwp);
        });

        let (startup_tx, startup_rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let startup = scan_startup_items();
            let _ = startup_tx.send(startup);
        });

        let (portable_tx, portable_rx) = std::sync::mpsc::channel();
        let apps_snap = apps.clone();
        std::thread::spawn(move || {
            let portables = scan_portable_apps(&[], &apps_snap);
            let _ = portable_tx.send(portables);
        });

        let traced_apps = load_traced_apps();
        let settings = load_settings();
        let temp_settings = settings.clone();
        let dark_mode = match settings.theme_mode.as_str() {
            "light" => false,
            "system" => is_windows_dark_mode(),
            _ => true,
        };

        let (update_tx, update_rx) = std::sync::mpsc::channel();
        if settings.check_updates {
            autoupdater::check_for_updates_async(
                settings.github_repo.clone(),
                env!("CARGO_PKG_VERSION").to_string(),
                update_tx.clone(),
            );
        }

        Self {
            active_tab: ActiveTab::Uninstaller,
            category: AppCategory::All,
            drive_filter: None,
            startup_filter: StartupFilter::All,
            apps,
            startup_items: Vec::new(),
            selected_app_idx: if count > 0 { Some(0) } else { None },
            search_query: String::new(),
            status_msg: format!("Программ: {} | Общий объем: {}", count, size_str),
            dark_mode,
            sort_column: SortColumn::Name,
            sort_order: SortOrder::Ascending,
            icon_textures: HashMap::new(),
            brand_logo_tex: None,
            size_rx,
            size_tx,
            uwp_rx,
            uwp_tx,
            batch_selected: HashSet::new(),
            batch_queue_open: false,
            batch_queue_apps: Vec::new(),
            batch_queue_idx: 0,
            batch_queue_statuses: Vec::new(),
            batch_force_open: false,
            batch_force_apps: Vec::new(),
            wizard_open: false,
            wizard_phase: WizardPhase::Scanning,
            wizard_app_title: String::new(),
            wizard_target_app: None,
            wizard_scan_current_path: String::new(),
            wizard_scan_progress: 0.0,
            wizard_scan_rx: None,
            wizard_traces: Vec::new(),
            wizard_delete_progress: 0.0,
            wizard_delete_rx: None,
            wizard_last_log_path: None,
            wizard_last_log_content: String::new(),
            wizard_xml_status: None,
            wizard_report: None,
            uninstall_confirm_open: false,
            uninstall_target_app: None,
            uninstall_waiting_open: false,
            uninstall_waiting_app: None,
            uninstall_child: None,
            force_confirm_open: false,
            force_confirm_app: None,
            running_processes_modal_open: false,
            running_processes: Vec::new(),
            running_processes_target_app: None,
            delayed_removal_modal_open: false,
            delayed_removal_files: Vec::new(),
            new_apps_count,
            remove_entry_confirm_open: false,
            remove_entry_confirm_app: None,
            focus_search_requested: false,
            add_startup_modal_open: false,
            new_startup_name: String::new(),
            new_startup_path: String::new(),
            new_startup_args: String::new(),
            new_startup_hklm: false,
            new_startup_error: None,
            tracker_modal_open: false,
            tracker_step: TrackerStep::SelectInstaller,
            tracker_installer_path: String::new(),
            tracker_app_name: String::new(),
            tracker_snapshot_before: None,
            tracker_traces: Vec::new(),
            traced_apps,
            tracker_live_log: Vec::new(),
            tracker_rx: None,
            tracker_stop_tx: None,
            tracker_child: None,
            tracker_process_finished: false,
            settings_open: false,
            settings_tab: SettingsTab::General,
            settings,
            temp_settings,
            shader_modal_open: false,
            shader_locations: Vec::new(),
            shader_clean_msg: None,
            shader_cleaning: false,
            path_modal_open: false,
            path_entries: Vec::new(),
            path_msg: None,
            portable_modal_open: false,
            portable_apps: Vec::new(),
            portable_msg: None,
            runtime_modal_open: false,
            runtime_groups: Vec::new(),
            runtime_msg: None,
            winget_modal_open: false,
            winget_packages: Vec::new(),
            winget_msg: None,
            winget_task_rx: None,
            winget_is_busy: false,
            winget_busy_msg: String::new(),
            startup_rx: Some(startup_rx),
            portable_rx: Some(portable_rx),
            is_refreshing: false,
            recently_uninstalled: HashSet::new(),
            update_status: autoupdater::UpdateStatus::Idle,
            update_rx,
            update_tx,
            update_modal_open: false,
        }
    }
}

fn format_size(bytes: u64) -> String {
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

fn truncate_chars(s: &str, max_chars: usize) -> String {
    let char_count = s.chars().count();
    if char_count > max_chars {
        let truncated: String = s.chars().take(max_chars.saturating_sub(3)).collect();
        format!("{}...", truncated)
    } else {
        s.to_string()
    }
}

impl eframe::App for PurgeCraftApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Apply text zoom factor from settings
        ctx.set_zoom_factor(self.settings.text_scale);

        if self.brand_logo_tex.is_none() {
            let brand_bytes = include_bytes!("icon_64.rgba");
            let img = egui::ColorImage::from_rgba_unmultiplied([64, 64], brand_bytes);
            self.brand_logo_tex = Some(ctx.load_texture("brand_logo", img, egui::TextureOptions::LINEAR));
        }

        // Drain asynchronous background size calculations
        let mut size_updated = false;
        while let Ok((app_id, size)) = self.size_rx.try_recv() {
            if let Some(app) = self.apps.iter_mut().find(|a| a.id == app_id) {
                if app.size_bytes == 0 {
                    app.size_bytes = size;
                    size_updated = true;
                }
            }
        }
        if size_updated {
            let total_size: u64 = self.apps.iter().map(|a| a.size_bytes).sum();
            self.status_msg = format!("Программ: {} | Общий объем: {}", self.apps.len(), format_size(total_size));
            ctx.request_repaint();
        }

        // Drain asynchronous background UWP app scanning
        if let Ok(uwp_apps) = self.uwp_rx.try_recv() {
            for uwp in uwp_apps {
                let id_low = uwp.id.to_lowercase();
                let name_low = uwp.name.to_lowercase();
                if self.recently_uninstalled.contains(&id_low) || self.recently_uninstalled.contains(&name_low) {
                    continue;
                }
                if !self.apps.iter().any(|a| a.id == uwp.id) {
                    self.apps.push(uwp);
                }
            }
            self.new_apps_count = apply_known_apps(&mut self.apps);
            self.apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
            let total_size: u64 = self.apps.iter().map(|a| a.size_bytes).sum();
            self.status_msg = format!("Программ: {} | Общий объем: {}", self.apps.len(), format_size(total_size));
            ctx.request_repaint();
        }

        // Drain asynchronous background startup scan
        if let Some(rx) = &self.startup_rx {
            if let Ok(items) = rx.try_recv() {
                self.startup_items = items;
                self.startup_rx = None;
                ctx.request_repaint();
            }
        }

        // Drain asynchronous background portable apps scan
        if let Some(rx) = &self.portable_rx {
            if let Ok(portables) = rx.try_recv() {
                for p in portables {
                    let port_app = InstalledApp {
                        id: format!("portable_{}", p.exe_path.to_lowercase()),
                        name: p.name,
                        version: String::new(),
                        publisher: "Portable".to_string(),
                        install_date: p.modified_date,
                        size_bytes: p.size_bytes,
                        install_location: p.folder_path.clone(),
                        uninstall_string: String::new(),
                        registry_path: String::new(),
                        is_64bit: true,
                        is_system_component: false,
                        is_uwp: false,
                        package_full_name: String::new(),
                        display_icon: p.exe_path,
                        is_new: false,
                        source_badge: "[Portable]".to_string(),
                    };
                    let id_low = port_app.id.to_lowercase();
                    let name_low = port_app.name.to_lowercase();
                    if self.recently_uninstalled.contains(&id_low) || self.recently_uninstalled.contains(&name_low) {
                        continue;
                    }
                    if !self.apps.iter().any(|a| a.id == port_app.id) {
                        self.apps.push(port_app);
                    }
                }
                self.portable_rx = None;
                self.is_refreshing = false;
                let total_size: u64 = self.apps.iter().map(|a| a.size_bytes).sum();
                self.status_msg = format!("Программ: {} | Общий объем: {}", self.apps.len(), format_size(total_size));
                ctx.request_repaint();
            }
        }

        // Drain asynchronous Winget task
        if let Some(rx) = &self.winget_task_rx {
            if let Ok((res, pkgs)) = rx.try_recv() {
                self.winget_is_busy = false;
                self.winget_task_rx = None;
                if let Some(pkgs) = pkgs {
                    self.winget_packages = pkgs;
                }
                match res {
                    Ok(msg) => {
                        self.winget_msg = Some(msg);
                    }
                    Err(e) => {
                        self.winget_msg = Some(format!("Ошибка: {}", e));
                    }
                }
                self.refresh_data();
                ctx.request_repaint();
            }
        }

        // Drain asynchronous GitHub auto-update messages
        while let Ok(msg) = self.update_rx.try_recv() {
            match &msg {
                autoupdater::UpdateStatus::Available(_) => {
                    self.update_modal_open = true;
                }
                autoupdater::UpdateStatus::Downloading { .. } => {
                    ctx.request_repaint();
                }
                _ => {}
            }
            self.update_status = msg;
            ctx.request_repaint();
        }

        // Poll native uninstaller process completion
        if self.uninstall_waiting_open {
            if let Some(child) = &mut self.uninstall_child {
                match child.try_wait() {
                    Ok(Some(_status)) => {
                        self.uninstall_child = None;
                        self.uninstall_waiting_open = false;
                        if let Some(app) = self.uninstall_waiting_app.take() {
                            let id_low = app.id.to_lowercase();
                            let name_low = app.name.to_lowercase();
                            self.recently_uninstalled.insert(id_low.clone());
                            self.recently_uninstalled.insert(name_low.clone());
                            self.apps.retain(|a| a.id.to_lowercase() != id_low && a.name.to_lowercase() != name_low);
                            self.start_wizard_scan(app);
                        }
                    }
                    _ => {}
                }
            }
            ctx.request_repaint_after(std::time::Duration::from_millis(200));
        }

        // Drain Realtime Leftovers Scan Stream
        if let Some(rx) = &self.wizard_scan_rx {
            while let Ok(msg) = rx.try_recv() {
                match msg {
                    ScanProgressMsg::Path { path, progress } => {
                        self.wizard_scan_current_path = path;
                        self.wizard_scan_progress = progress;
                    }
                    ScanProgressMsg::TraceFound(trace) => {
                        if !self.wizard_traces.iter().any(|t| t.path.eq_ignore_ascii_case(&trace.path)) {
                            self.wizard_traces.push(trace);
                        }
                    }
                    ScanProgressMsg::Done(traces) => {
                        self.wizard_traces = traces;
                        self.wizard_phase = WizardPhase::Review;
                        self.wizard_scan_rx = None;
                        break;
                    }
                }
            }
            ctx.request_repaint_after(std::time::Duration::from_millis(30));
        }

        // Drain Realtime Leftovers Delete Stream
        if let Some(rx) = &self.wizard_delete_rx {
            while let Ok(msg) = rx.try_recv() {
                match msg {
                    DeleteProgressMsg::ItemDeleted { path, success: _, log_line: _, progress } => {
                        self.wizard_delete_progress = progress;
                        self.wizard_scan_current_path = path;
                    }
                    DeleteProgressMsg::Done { deleted_count, bytes_freed, log_path, log_content, scheduled_reboot_items } => {
                        self.wizard_last_log_path = Some(log_path);
                        self.wizard_last_log_content = log_content;
                        self.wizard_report = Some(format!(
                            "Успешно удалено элементов: {}\nОсвобождено дискового пространства: {}",
                            deleted_count,
                            format_size(bytes_freed)
                        ));
                        self.wizard_phase = WizardPhase::Finished;
                        self.wizard_delete_rx = None;
                        self.batch_selected.clear();
                        if let Some(app) = &self.wizard_target_app {
                            let id_low = app.id.to_lowercase();
                            let name_low = app.name.to_lowercase();
                            self.recently_uninstalled.insert(id_low.clone());
                            self.recently_uninstalled.insert(name_low.clone());
                            self.apps.retain(|a| a.id.to_lowercase() != id_low && a.name.to_lowercase() != name_low);
                        }
                        self.refresh_data();

                        if !scheduled_reboot_items.is_empty() {
                            self.delayed_removal_files = scheduled_reboot_items;
                            self.delayed_removal_modal_open = true;
                        }
                        break;
                    }
                }
            }
            ctx.request_repaint_after(std::time::Duration::from_millis(30));
        }

        // Drain Realtime Install Tracker Log Stream
        if let Some(rx) = &self.tracker_rx {
            while let Ok(msg) = rx.try_recv() {
                self.tracker_live_log.push(msg);
            }
            if let Some(child) = &mut self.tracker_child {
                match child.try_wait() {
                    Ok(Some(_status)) => {
                        self.tracker_child = None;
                        self.tracker_process_finished = true;
                        self.tracker_live_log.push(scanner::TrackerLiveMsg {
                            time: chrono::Local::now().format("%H:%M:%S").to_string(),
                            action: String::new(),
                            path: "Процесс инсталлятора завершен. Нажмите 'Установить и следить' для анализа.".to_string(),
                        });
                    }
                    _ => {}
                }
            }
            ctx.request_repaint_after(std::time::Duration::from_millis(80));
        }

        // Keyboard shortcuts
        if ctx.input(|i| i.key_pressed(egui::Key::F5)) {
            self.refresh_data();
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.search_query.clear();
        }
        if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::F)) {
            self.focus_search_requested = true;
        }

        // True Dark Theme (neutral graphite/charcoal, ZERO BLUE) vs Windows Explorer Light
        if self.dark_mode {
            let mut visuals = egui::Visuals::dark();
            visuals.window_fill = Color32::from_rgb(18, 18, 20); // Neutral dark black/charcoal
            visuals.panel_fill = Color32::from_rgb(18, 18, 20);
            visuals.faint_bg_color = Color32::from_rgb(26, 26, 30);
            visuals.extreme_bg_color = Color32::from_rgb(12, 12, 14);
            visuals.override_text_color = Some(Color32::from_rgb(244, 244, 245));
            visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(18, 18, 20);
            visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(38, 38, 42));
            visuals.widgets.inactive.bg_fill = Color32::from_rgb(26, 26, 30);
            visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(42, 42, 48));
            visuals.widgets.hovered.bg_fill = Color32::from_rgb(38, 38, 45);
            visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(63, 63, 70));
            visuals.widgets.active.bg_fill = Color32::from_rgb(48, 52, 60);
            visuals.selection.bg_fill = Color32::from_rgb(45, 48, 56);
            visuals.selection.stroke = Stroke::new(1.0_f32, Color32::from_rgb(90, 95, 105));
            ctx.set_visuals(visuals);
        } else {
            let mut visuals = egui::Visuals::light();
            visuals.window_fill = Color32::from_rgb(240, 242, 245);
            visuals.panel_fill = Color32::from_rgb(240, 242, 245);
            visuals.faint_bg_color = Color32::from_rgb(248, 250, 252);
            visuals.extreme_bg_color = Color32::WHITE;
            visuals.override_text_color = Some(Color32::from_rgb(30, 41, 59));
            ctx.set_visuals(visuals);
        }

        // 1. Menu Bar (MFC / Win32 style)
        egui::TopBottomPanel::top("top_menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("Файл", |ui| {
                    if ui.button("📄 Экспортировать список в HTML...").clicked() {
                        self.export_html();
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Выход").clicked() {
                        std::process::exit(0);
                    }
                });

                ui.menu_button("Действие", |ui| {
                    let selected_count = self.batch_selected.len();
                    if selected_count > 1 {
                        if ui.button(format!("🗑 Пакетная деинсталляция ({})", selected_count)).clicked() {
                            self.start_batch_uninstall();
                            ui.close_menu();
                        }
                        if ui.button(format!("⚠ Пакетное принудительное удаление ({})", selected_count)).clicked() {
                            self.start_batch_force_removal();
                            ui.close_menu();
                        }
                        if ui.button("✕ Снять выбор со всех").clicked() {
                            self.batch_selected.clear();
                            ui.close_menu();
                        }
                    } else {
                        let selected_app_opt = self.selected_app_idx.and_then(|idx| self.apps.get(idx).cloned());
                        if let Some(app) = selected_app_opt {
                            if ui.button("🗑 Деинсталляция...").clicked() {
                                self.start_uninstall(app.clone());
                                ui.close_menu();
                            }
                            if ui.button("⚠ Принудительное удаление...").clicked() {
                                self.force_confirm_app = Some(app.clone());
                                self.force_confirm_open = true;
                                ui.close_menu();
                            }
                            if ui.button("❌ Удалить запись из списка...").clicked() {
                                self.remove_entry_confirm_app = Some(app.clone());
                                self.remove_entry_confirm_open = true;
                                ui.close_menu();
                            }
                            ui.separator();
                            if ui.button("📁 Папка программы").clicked() {
                                if !app.install_location.is_empty() {
                                    open_system_folder(&app.install_location, self.settings.use_total_commander);
                                }
                                ui.close_menu();
                            }
                            if !app.is_uwp && !app.registry_path.is_empty() {
                                if ui.button("🗄 Запись в реестре").clicked() {
                                    jump_to_regedit(&app.registry_path);
                                    ui.close_menu();
                                }
                            }
                            if ui.button("🌐 Искать в Google").clicked() {
                                let url = format!("https://www.google.com/search?q={}+{}", app.name, app.publisher);
                                let _ = open::that(url);
                                ui.close_menu();
                            }
                        } else {
                            ui.label("Программа не выбрана");
                        }
                    }
                });

                ui.menu_button("Вид", |ui| {
                    if ui.button("🔄 Обновить (F5)").clicked() {
                        self.refresh_data();
                        ui.close_menu();
                    }
                    ui.separator();
                    let theme_menu_label = if self.dark_mode { "Переключить на светлую тему" } else { "Переключить на темную тему" };
                    if ui.button(theme_menu_label).clicked() {
                        self.dark_mode = !self.dark_mode;
                        ui.close_menu();
                    }
                });

                ui.menu_button("Инструменты", |ui| {
                    if ui.button("🎮 Очистка кэша шейдеров GPU...").clicked() {
                        self.shader_locations = scan_shader_caches();
                        self.shader_clean_msg = None;
                        self.shader_modal_open = true;
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("🗄 Санитайзер системного PATH...").clicked() {
                        self.path_entries = scan_path_entries();
                        self.path_msg = None;
                        self.path_modal_open = true;
                        ui.close_menu();
                    }
                    if ui.button("📦 Сканер Portable-программ...").clicked() {
                        self.portable_apps = scan_portable_apps(&[], &self.apps);
                        self.portable_msg = None;
                        self.portable_modal_open = true;
                        ui.close_menu();
                    }
                    if ui.button("🧬 Анализатор дубликатов VC++ и Java...").clicked() {
                        self.runtime_groups = detect_runtime_duplicates(&self.apps);
                        self.runtime_msg = None;
                        self.runtime_modal_open = true;
                        ui.close_menu();
                    }
                    if ui.button("🚀 Менеджер пакетов Winget...").clicked() {
                        self.winget_packages = scan_winget_packages();
                        self.winget_msg = None;
                        self.winget_modal_open = true;
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("⚙ Настройки...").clicked() {
                        self.temp_settings = self.settings.clone();
                        self.settings_tab = SettingsTab::General;
                        self.settings_open = true;
                        ui.close_menu();
                    }
                });

                ui.menu_button("Помощь", |ui| {
                    ui.label("PurgeCraft — Нативный деинсталлятор для Windows (Pure Rust)");
                    ui.label("100% чистая реализация по Win32 API");
                    ui.label("Поддержка Win32, UWP/Store и пакетного удаления");
                    ui.separator();
                    if ui.button("🔄 Проверить обновления...").clicked() {
                        autoupdater::check_for_updates_async(
                            self.settings.github_repo.clone(),
                            env!("CARGO_PKG_VERSION").to_string(),
                            self.update_tx.clone(),
                        );
                        self.update_modal_open = true;
                        ui.close_menu();
                    }
                    if ui.button("🌐 Репозиторий на GitHub...").clicked() {
                        let _ = open::that(format!("https://github.com/{}", self.settings.github_repo));
                        ui.close_menu();
                    }
                });

                if ui.button("🔄 Обновить").on_hover_text("Обновить список программ (F5)").clicked() {
                    self.refresh_data();
                }

                if ui.button("⚙ Настройки").on_hover_text("Параметры и настройки программы").clicked() {
                    self.temp_settings = self.settings.clone();
                    self.settings_tab = SettingsTab::General;
                    self.settings_open = true;
                }

                match &self.update_status {
                    autoupdater::UpdateStatus::Available(info) => {
                        if ui.button(RichText::new(format!("✨ Доступно обновление v{}", info.version)).color(Color32::from_rgb(59, 130, 246)).strong())
                            .on_hover_text("Нажмите для просмотра и установки обновления")
                            .clicked()
                        {
                            self.update_modal_open = true;
                        }
                    }
                    autoupdater::UpdateStatus::ReadyToRestart => {
                        if ui.button(RichText::new("🚀 Перезапустить для обновления").color(Color32::from_rgb(34, 197, 94)).strong())
                            .clicked()
                        {
                            self.update_modal_open = true;
                        }
                    }
                    _ => {}
                }
            });
        });

        // 2. Main Toolbar (Large action buttons + Search)
        egui::TopBottomPanel::top("main_toolbar").show(ctx, |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                let logo_color = if self.dark_mode {
                    Color32::from_rgb(244, 244, 245)
                } else {
                    Color32::from_rgb(37, 99, 235)
                };
                if let Some(ref logo) = self.brand_logo_tex {
                    ui.image((logo.id(), egui::vec2(22.0, 22.0)));
                    ui.add_space(4.0);
                }
                ui.heading(RichText::new("PurgeCraft").color(logo_color).strong());
                ui.add_space(14.0);

                let uninst_active = self.active_tab == ActiveTab::Uninstaller;
                let (uninst_fill, uninst_stroke, uninst_text) = if uninst_active {
                    if self.dark_mode {
                        (Color32::from_rgb(45, 48, 56), Color32::from_rgb(82, 82, 91), Color32::WHITE)
                    } else {
                        (Color32::from_rgb(224, 242, 254), Color32::from_rgb(186, 230, 253), Color32::from_rgb(2, 132, 199))
                    }
                } else {
                    if self.dark_mode {
                        (Color32::from_rgb(24, 24, 28), Color32::from_rgb(39, 39, 44), Color32::from_rgb(161, 161, 170))
                    } else {
                        (Color32::from_rgb(241, 245, 249), Color32::from_rgb(226, 232, 240), Color32::from_rgb(71, 85, 105))
                    }
                };

                let uninst_title = if self.settings.show_toolbar_labels { "📦 Деинсталлятор" } else { "📦" };
                let uninst_btn = egui::Button::new(
                    RichText::new(uninst_title)
                        .strong()
                        .color(uninst_text)
                )
                .fill(uninst_fill)
                .stroke(Stroke::new(1.0_f32, uninst_stroke));

                if ui.add(uninst_btn).clicked() {
                    self.active_tab = ActiveTab::Uninstaller;
                }

                let startup_active = self.active_tab == ActiveTab::Startup;
                let (startup_fill, startup_stroke, startup_text) = if startup_active {
                    if self.dark_mode {
                        (Color32::from_rgb(45, 48, 56), Color32::from_rgb(82, 82, 91), Color32::WHITE)
                    } else {
                        (Color32::from_rgb(224, 242, 254), Color32::from_rgb(186, 230, 253), Color32::from_rgb(2, 132, 199))
                    }
                } else {
                    if self.dark_mode {
                        (Color32::from_rgb(24, 24, 28), Color32::from_rgb(39, 39, 44), Color32::from_rgb(161, 161, 170))
                    } else {
                        (Color32::from_rgb(241, 245, 249), Color32::from_rgb(226, 232, 240), Color32::from_rgb(71, 85, 105))
                    }
                };

                let startup_title = if self.settings.show_toolbar_labels { "⚡ Автозапуск" } else { "⚡" };
                let startup_btn = egui::Button::new(
                    RichText::new(startup_title)
                        .strong()
                        .color(startup_text)
                )
                .fill(startup_fill)
                .stroke(Stroke::new(1.0_f32, startup_stroke));

                if ui.add(startup_btn).clicked() {
                    self.active_tab = ActiveTab::Startup;
                }

                ui.add_space(8.0);
                let export_label = if self.settings.show_toolbar_labels { "📄 Экспорт HTML" } else { "📄" };
                if ui.button(export_label).on_hover_text("Экспортировать отчет в HTML").clicked() {
                    self.export_html();
                }

                // Install Tracker Button
                let tracker_title = if self.settings.show_toolbar_labels { "⚡ Установить и следить" } else { "⚡ Следить" };
                let tracker_btn = egui::Button::new(
                    RichText::new(tracker_title)
                        .strong()
                        .color(if self.dark_mode { Color32::from_rgb(253, 224, 71) } else { Color32::from_rgb(180, 83, 9) })
                )
                .fill(if self.dark_mode { Color32::from_rgb(66, 32, 6) } else { Color32::from_rgb(254, 243, 199) })
                .stroke(Stroke::new(1.0_f32, if self.dark_mode { Color32::from_rgb(202, 138, 4) } else { Color32::from_rgb(245, 158, 11) }));

                if ui.add(tracker_btn).on_hover_text("Монитор установки (Install Tracker): фиксирует все создаваемые файлы и ключи реестра для 100% чистого удаления").clicked() {
                    self.tracker_installer_path.clear();
                    self.tracker_app_name.clear();
                    self.tracker_traces.clear();
                    self.tracker_snapshot_before = None;
                    self.tracker_step = TrackerStep::SelectInstaller;
                    self.tracker_modal_open = true;
                }

                // GPU Shader Cache Cleaner Button
                let shader_btn = egui::Button::new(
                    RichText::new("🎮 Шейдеры GPU")
                        .strong()
                        .color(if self.dark_mode { Color32::from_rgb(192, 132, 252) } else { Color32::from_rgb(126, 34, 206) })
                )
                .fill(if self.dark_mode { Color32::from_rgb(46, 16, 77) } else { Color32::from_rgb(243, 232, 255) })
                .stroke(Stroke::new(1.0_f32, if self.dark_mode { Color32::from_rgb(168, 85, 247) } else { Color32::from_rgb(192, 132, 252) }));

                if ui.add(shader_btn).on_hover_text("Очистить гигабайты кэшей шейдеров DirectX, NVIDIA, AMD и Vulkan").clicked() {
                    self.shader_locations = scan_shader_caches();
                    self.shader_clean_msg = None;
                    self.shader_modal_open = true;
                }

                if ui.button("🗄 PATH").on_hover_text("Санитайзер переменных среды PATH: поиск и удаление мертвых путей").clicked() {
                    self.path_entries = scan_path_entries();
                    self.path_msg = None;
                    self.path_modal_open = true;
                }

                if ui.button("🧬 Дубликаты").on_hover_text("Детектор дублирующихся и устаревших версий Visual C++, Java и .NET").clicked() {
                    self.runtime_groups = detect_runtime_duplicates(&self.apps);
                    self.runtime_msg = None;
                    self.runtime_modal_open = true;
                }

                if ui.button("🚀 Winget").on_hover_text("Менеджер пакетов Winget: проверка обновлений и тихое удаление").clicked() {
                    self.winget_packages = scan_winget_packages();
                    self.winget_msg = None;
                    self.winget_modal_open = true;
                }

                // Theme Toggle Button
                let theme_btn_label = if self.dark_mode { "☀ Светлая" } else { "🌙 Темная" };
                if ui.button(theme_btn_label).on_hover_text("Переключить тему оформления (Светлая / Темная)").clicked() {
                    self.dark_mode = !self.dark_mode;
                }

                // Search box on right
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if !self.search_query.is_empty() {
                        if ui.button("✕").on_hover_text("Очистить поиск (Esc)").clicked() {
                            self.search_query.clear();
                        }
                    }
                    let search_box = ui.add(
                        egui::TextEdit::singleline(&mut self.search_query)
                            .hint_text("Поиск программ... (Ctrl+F, Esc)")
                            .desired_width(220.0)
                    );
                    if self.focus_search_requested {
                        search_box.request_focus();
                        self.focus_search_requested = false;
                    }
                    if search_box.changed() {
                        self.selected_app_idx = None;
                    }
                    ui.label("🔍");
                });
            });
            ui.add_space(4.0);
        });

        // 3. Secondary Toolbar (Category Tabs with Store support)
        if self.active_tab == ActiveTab::Uninstaller {
            egui::TopBottomPanel::top("category_bar").show(ctx, |ui| {
                ui.add_space(3.0);
                ui.horizontal(|ui| {
                    let total_all = self.apps.len();
                    let total_user = self.apps.iter().filter(|a| !a.is_system_component && !a.is_uwp && a.source_badge != "[Portable]").count();
                    let total_sys = self.apps.iter().filter(|a| a.is_system_component).count();
                    let total_store = self.apps.iter().filter(|a| a.is_uwp).count();
                    let total_portable = self.apps.iter().filter(|a| a.source_badge == "[Portable]").count();
                    let total_64 = self.apps.iter().filter(|a| a.is_64bit).count();
                    let total_32 = self.apps.iter().filter(|a| !a.is_64bit).count();

                    ui.selectable_value(&mut self.category, AppCategory::All, format!("Все ({})", total_all));
                    ui.selectable_value(&mut self.category, AppCategory::UserApps, format!("Программы ({})", total_user));
                    ui.selectable_value(&mut self.category, AppCategory::System, format!("Системные ({})", total_sys));
                    ui.selectable_value(&mut self.category, AppCategory::StoreApps, format!("Магазин Windows ({})", total_store));
                    ui.selectable_value(&mut self.category, AppCategory::Portable, format!("Portable ({})", total_portable));
                    ui.selectable_value(&mut self.category, AppCategory::Arch64, format!("64-бит ({})", total_64));
                    ui.selectable_value(&mut self.category, AppCategory::Arch32, format!("32-бит ({})", total_32));

                    // Drive filter combobox [ 💾 Диск: Все ▾ ]
                    let mut drives_count: std::collections::BTreeMap<char, usize> = std::collections::BTreeMap::new();
                    for a in &self.apps {
                        let d = scanner::get_app_drive_letter(a);
                        *drives_count.entry(d).or_insert(0) += 1;
                    }

                    ui.separator();
                    let current_drive_title = match self.drive_filter {
                        Some(d) => format!("💾 Диск {}: ({})", d, drives_count.get(&d).copied().unwrap_or(0)),
                        None => "💾 Диск: Все".to_string(),
                    };

                    egui::ComboBox::from_id_salt("drive_filter_combobox")
                        .width(135.0)
                        .selected_text(RichText::new(current_drive_title).small().strong())
                        .show_ui(ui, |ui| {
                            if ui.selectable_value(&mut self.drive_filter, None, format!("💾 Все диски ({})", self.apps.len())).clicked() {
                                self.selected_app_idx = None;
                            }
                            for (&drive, &count) in &drives_count {
                                if ui.selectable_value(&mut self.drive_filter, Some(drive), format!("💽 Диск {}: ({} прогр.)", drive, count)).clicked() {
                                    self.selected_app_idx = None;
                                }
                            }
                        });

                    let actual_new_count = self.apps.iter().filter(|a| a.is_new).count();
                    ui.separator();
                    let (clear_btn_text, clear_btn_color) = if actual_new_count > 0 {
                        (format!("✨ Убрать «Новое» ({})", actual_new_count), Color32::from_rgb(234, 179, 8))
                    } else {
                        ("✨ Сбросить «Новое»".to_string(), Color32::from_rgb(148, 163, 184))
                    };
                    if ui.button(RichText::new(clear_btn_text).small().color(clear_btn_color))
                        .on_hover_text("Снять отметку «Новое» со всех программ и убрать подсветку, чтобы не мешало")
                        .clicked()
                    {
                        save_known_apps(&self.apps);
                        for a in &mut self.apps {
                            a.is_new = false;
                        }
                        self.new_apps_count = 0;
                        self.status_msg = "Все отметки «Новое» успешно сняты.".to_string();
                    }

                    if !self.batch_selected.is_empty() {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(RichText::new(format!("Выбрано для пакета: {}", self.batch_selected.len())).strong().color(Color32::from_rgb(56, 189, 248)));
                        });
                    }
                });
                ui.add_space(3.0);
            });
        } else if self.active_tab == ActiveTab::Startup {
            egui::TopBottomPanel::top("startup_category_bar").show(ctx, |ui| {
                ui.add_space(3.0);
                ui.horizontal(|ui| {
                    let total_all = self.startup_items.len();
                    let total_reg = self.startup_items.iter().filter(|i| i.source_type.contains("run")).count();
                    let total_sm = self.startup_items.iter().filter(|i| i.source_type == "startmenu").count();
                    let total_ts = self.startup_items.iter().filter(|i| i.source_type == "taskscheduler").count();

                    ui.selectable_value(&mut self.startup_filter, StartupFilter::All, format!("Все ({})", total_all));
                    ui.selectable_value(&mut self.startup_filter, StartupFilter::Registry, format!("🗄 Реестр ({})", total_reg));
                    ui.selectable_value(&mut self.startup_filter, StartupFilter::StartMenu, format!("📁 Меню Пуск ({})", total_sm));
                    ui.selectable_value(&mut self.startup_filter, StartupFilter::TaskScheduler, format!("⏰ Планировщик ({})", total_ts));

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let add_btn = egui::Button::new(RichText::new("➕ Добавить программу...").strong().color(Color32::WHITE))
                            .fill(if self.dark_mode { Color32::from_rgb(16, 185, 129) } else { Color32::from_rgb(5, 150, 105) });
                        if ui.add(add_btn).on_hover_text("Добавить новое приложение в автозапуск Windows").clicked() {
                            self.new_startup_name.clear();
                            self.new_startup_path.clear();
                            self.new_startup_args.clear();
                            self.new_startup_hklm = false;
                            self.new_startup_error = None;
                            self.add_startup_modal_open = true;
                        }
                    });
                });
                ui.add_space(3.0);
            });
        }

        // 4. Status Bar (Bottom)
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                let status_color = if self.dark_mode { Color32::from_rgb(161, 161, 170) } else { Color32::from_rgb(71, 85, 105) };
                ui.label(RichText::new(&self.status_msg).small().color(status_color));
                let actual_new_count = self.apps.iter().filter(|a| a.is_new).count();
                if actual_new_count > 0 {
                    ui.separator();
                    ui.label(RichText::new(format!("✨ Недавно установлено: {}", actual_new_count)).color(Color32::from_rgb(234, 179, 8)).strong().small());
                    if ui.button(RichText::new("✨ Убрать «Новое»").small()).on_hover_text("Снять отметку «Новое» со всех программ, чтобы не мешало").clicked() {
                        save_known_apps(&self.apps);
                        for a in &mut self.apps {
                            a.is_new = false;
                        }
                        self.new_apps_count = 0;
                        self.status_msg = "Все отметки «Новое» успешно сняты.".to_string();
                    }
                }
            });
            ui.add_space(4.0);
        });

        // 5. Central Work Area
        egui::CentralPanel::default().show(ctx, |ui| {
            match self.active_tab {
                ActiveTab::Uninstaller => self.render_uninstaller(ui),
                ActiveTab::Startup => self.render_startup(ui),
            }
        });

        // 6. Modals (Wizard, Force Removal, Batch Queue, Batch Force)
        self.render_modals(ctx);
    }
}

impl PurgeCraftApp {
    fn refresh_data(&mut self) {
        let mut apps = scan_win32_apps();
        apps.retain(|a| !self.recently_uninstalled.contains(&a.id.to_lowercase()) && !self.recently_uninstalled.contains(&a.name.to_lowercase()));
        self.new_apps_count = apply_known_apps(&mut apps);
        self.apps = apps;
        self.traced_apps = load_traced_apps();
        spawn_size_calculator(&self.apps, self.size_tx.clone());
        let total_size: u64 = self.apps.iter().map(|a| a.size_bytes).sum();
        self.status_msg = format!(
            "Программ: {} | Общий объем: {} | Обновление данных...",
            self.apps.len(),
            format_size(total_size)
        );
        self.is_refreshing = true;

        let u_tx = self.uwp_tx.clone();
        let s_tx = self.size_tx.clone();
        std::thread::spawn(move || {
            let uwp = scan_uwp_apps();
            spawn_size_calculator(&uwp, s_tx);
            let _ = u_tx.send(uwp);
        });

        let (st_tx, st_rx) = std::sync::mpsc::channel();
        self.startup_rx = Some(st_rx);
        std::thread::spawn(move || {
            let startup = scan_startup_items();
            let _ = st_tx.send(startup);
        });

        let (port_tx, port_rx) = std::sync::mpsc::channel();
        self.portable_rx = Some(port_rx);
        let apps_snap = self.apps.clone();
        std::thread::spawn(move || {
            let portables = scan_portable_apps(&[], &apps_snap);
            let _ = port_tx.send(portables);
        });

        self.batch_selected.clear();
        self.icon_textures.clear();
        if !self.apps.is_empty() && self.selected_app_idx.is_none() {
            self.selected_app_idx = Some(0);
        }
    }

    fn remove_registry_entry(&mut self, app: &InstalledApp) {
        if app.is_uwp {
            let mut cmd = Command::new("powershell");
            #[cfg(windows)]
            cmd.creation_flags(scanner::CREATE_NO_WINDOW);
            let _ = cmd.args(["-NoProfile", "-NonInteractive", "-Command", &app.uninstall_string]).output();
        } else if !app.registry_path.is_empty() {
            let _ = scanner::delete_registry_key_by_path(&app.registry_path);
        }
        let id_low = app.id.to_lowercase();
        let name_low = app.name.to_lowercase();
        self.recently_uninstalled.insert(id_low.clone());
        self.recently_uninstalled.insert(name_low.clone());
        self.apps.retain(|a| a.id.to_lowercase() != id_low && a.name.to_lowercase() != name_low);
        self.status_msg = format!("Запись «{}» успешно удалена из списка Windows.", app.name);
        self.refresh_data();
    }

    fn render_uninstaller(&mut self, ui: &mut egui::Ui) {
        let q = self.search_query.to_lowercase();
        let cat = self.category;

        let mut filtered_indices: Vec<usize> = self.apps
            .iter()
            .enumerate()
            .filter(|(_, a)| {
                // Category filter
                let cat_match = match cat {
                    AppCategory::All => true,
                    AppCategory::UserApps => !a.is_system_component && !a.is_uwp && a.source_badge != "[Portable]",
                    AppCategory::System => a.is_system_component,
                    AppCategory::StoreApps => a.is_uwp,
                    AppCategory::Portable => a.source_badge == "[Portable]",
                    AppCategory::Arch64 => a.is_64bit,
                    AppCategory::Arch32 => !a.is_64bit,
                };
                if !cat_match {
                    return false;
                }

                // Drive filter
                if let Some(target_drive) = self.drive_filter {
                    if scanner::get_app_drive_letter(a) != target_drive {
                        return false;
                    }
                }

                if self.settings.tree_updates_mode == "hidden" {
                    let is_update = a.name.starts_with("KB")
                        || a.name.to_lowercase().contains("hotfix")
                        || a.name.to_lowercase().contains("update for")
                        || a.name.to_lowercase().contains("обновление для");
                    if is_update {
                        return false;
                    }
                }

                // Search query filter
                if q.is_empty() {
                    true
                } else {
                    a.name.to_lowercase().contains(&q) 
                        || a.publisher.to_lowercase().contains(&q)
                        || a.source_badge.to_lowercase().contains(&q)
                }
            })
            .map(|(i, _)| i)
            .collect();

        // Sort according to current sort column and direction
        let sort_col = self.sort_column;
        let sort_ord = self.sort_order;
        let highlight_recent = self.settings.highlight_recent;
        filtered_indices.sort_by(|&a_idx, &b_idx| {
            let a = &self.apps[a_idx];
            let b = &self.apps[b_idx];

            if highlight_recent && a.is_new != b.is_new {
                return if a.is_new { std::cmp::Ordering::Less } else { std::cmp::Ordering::Greater };
            }

            let cmp = match sort_col {
                SortColumn::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
                SortColumn::Size => a.size_bytes.cmp(&b.size_bytes),
                SortColumn::Date => a.install_date.cmp(&b.install_date),
                SortColumn::Publisher => a.publisher.to_lowercase().cmp(&b.publisher.to_lowercase()),
                SortColumn::AppType => {
                    let a_type = if a.is_uwp { 2 } else if a.is_64bit { 1 } else { 0 };
                    let b_type = if b.is_uwp { 2 } else if b.is_64bit { 1 } else { 0 };
                    a_type.cmp(&b_type)
                }
            };
            if sort_ord == SortOrder::Descending {
                cmp.reverse()
            } else {
                cmp
            }
        });

        let selected_app = self.selected_app_idx.and_then(|idx| self.apps.get(idx).cloned());

        // Dynamic True Dark vs Clean Light palette (ZERO BLUE IN DARK)
        let pane_fill = if self.dark_mode {
            Color32::from_rgb(24, 24, 27) // True neutral dark (Zinc 900)
        } else {
            Color32::from_rgb(235, 242, 249) // Soft sky-blue
        };
        let pane_stroke = if self.dark_mode {
            Color32::from_rgb(39, 39, 42) // Neutral dark border
        } else {
            Color32::from_rgb(203, 219, 235)
        };
        let heading_color = if self.dark_mode {
            Color32::WHITE
        } else {
            Color32::from_rgb(15, 23, 42)
        };
        let meta_label_color = if self.dark_mode {
            Color32::from_rgb(161, 161, 170) // Neutral light gray
        } else {
            Color32::from_rgb(100, 116, 139)
        };
        let (danger_fill, danger_stroke, danger_text) = if self.dark_mode {
            (Color32::from_rgb(42, 18, 21), Color32::from_rgb(220, 38, 38), Color32::from_rgb(254, 202, 202))
        } else {
            (Color32::from_rgb(254, 242, 242), Color32::from_rgb(252, 165, 165), Color32::from_rgb(220, 38, 38))
        };
        let table_container_fill = if self.dark_mode {
            Color32::from_rgb(18, 18, 20) // Deep neutral black
        } else {
            Color32::WHITE
        };

        // Left ExplorerBar (TaskPane)
        egui::SidePanel::left("explorer_task_pane")
            .exact_width(280.0)
            .resizable(false)
            .frame(
                egui::Frame::none()
                    .fill(pane_fill)
                    .stroke(Stroke::new(1.0_f32, pane_stroke))
                    .inner_margin(12.0)
            )
            .show_inside(ui, |ui| {
                if self.batch_selected.len() > 1 {
                    // BATCH OPERATIONS PANEL
                    let batch_count = self.batch_selected.len();
                    let selected_apps_vec: Vec<InstalledApp> = self.apps.iter()
                        .filter(|a| self.batch_selected.contains(&a.id))
                        .cloned()
                        .collect();
                    let batch_size: u64 = selected_apps_vec.iter().map(|a| a.size_bytes).sum();

                    let batch_heading_color = if self.dark_mode { heading_color } else { Color32::from_rgb(2, 132, 199) };
                    ui.heading(RichText::new("📦 ПАКЕТНЫЙ РЕЖИМ").size(15.0).strong().color(batch_heading_color));
                    ui.add_space(8.0);

                    ui.label(RichText::new(format!("Выбрано программ: {}", batch_count)).strong().color(heading_color));
                    ui.label(RichText::new(format!("Общий объем: {}", format_size(batch_size))).color(meta_label_color));

                    ui.add_space(14.0);
                    ui.separator();
                    ui.add_space(10.0);

                    // Batch Uninstall Button
                    let (batch_btn_fill, batch_btn_stroke) = if self.dark_mode {
                        (Color32::from_rgb(45, 48, 56), Stroke::new(1.0_f32, Color32::from_rgb(82, 82, 91)))
                    } else {
                        (Color32::from_rgb(37, 99, 235), Stroke::NONE)
                    };
                    let batch_uninst_btn = egui::Button::new(
                        RichText::new(format!("🗑 Пакетное удаление ({})", batch_count)).size(14.0).strong().color(Color32::WHITE)
                    )
                    .fill(batch_btn_fill)
                    .stroke(batch_btn_stroke)
                    .min_size(Vec2::new(ui.available_width(), 36.0));

                    if ui.add(batch_uninst_btn).clicked() {
                        self.start_batch_uninstall();
                    }

                    ui.add_space(6.0);

                    // Batch Force Removal Button
                    let batch_force_btn = egui::Button::new(
                        RichText::new(format!("⚠ Пакетная зачистка ({})", batch_count))
                            .size(13.0)
                            .strong()
                            .color(danger_text)
                    )
                    .fill(danger_fill)
                    .stroke(Stroke::new(1.0_f32, danger_stroke))
                    .min_size(Vec2::new(ui.available_width(), 32.0));

                    if ui.add(batch_force_btn).clicked() {
                        self.start_batch_force_removal();
                    }

                    ui.add_space(10.0);
                    if ui.button("✕ Снять выбор со всех").clicked() {
                        self.batch_selected.clear();
                    }

                    ui.add_space(14.0);
                    ui.separator();
                    ui.add_space(8.0);
                    ui.label(RichText::new("Список выбранных:").small().strong().color(meta_label_color));

                    egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                        for a in &selected_apps_vec {
                            ui.label(format!("• {}", truncate_chars(&a.name, 28)));
                        }
                    });
                } else if let Some(app) = &selected_app {
                    // SINGLE APP DETAILS PANEL with 36x36 Icon
                    ui.horizontal(|ui| {
                        if !app.display_icon.is_empty() {
                            if let Some(opt_tex) = self.icon_textures.get(&app.id) {
                                if let Some(tex) = opt_tex {
                                    ui.image((tex.id(), egui::vec2(36.0, 36.0)));
                                } else {
                                    let icon = if app.is_uwp { "🌐" } else if app.is_system_component { "⚙" } else { "📦" };
                                    ui.label(RichText::new(icon).size(26.0));
                                }
                            } else {
                                let opt_img = icons::extract_app_icon(&app.display_icon);
                                let tex_opt = opt_img.map(|img| ui.ctx().load_texture(&app.id, img, Default::default()));
                                if let Some(ref tex) = tex_opt {
                                    ui.image((tex.id(), egui::vec2(36.0, 36.0)));
                                } else {
                                    let icon = if app.is_uwp { "🌐" } else if app.is_system_component { "⚙" } else { "📦" };
                                    ui.label(RichText::new(icon).size(26.0));
                                }
                                self.icon_textures.insert(app.id.clone(), tex_opt);
                            }
                        } else {
                            let icon = if app.is_uwp { "🌐" } else if app.is_system_component { "⚙" } else { "📦" };
                            ui.label(RichText::new(icon).size(26.0));
                        }

                        ui.vertical(|ui| {
                            ui.heading(RichText::new(&app.name).size(15.0).strong().color(heading_color));
                            if !app.publisher.is_empty() {
                                ui.label(RichText::new(truncate_chars(&app.publisher, 26)).small().color(meta_label_color));
                            }
                        });
                    });
                    ui.add_space(8.0);

                    let loc = if !app.install_location.is_empty() {
                        app.install_location.clone()
                    } else if !app.display_icon.is_empty() {
                        let f = app.display_icon.split(',').next().unwrap_or(&app.display_icon).trim().trim_matches('"');
                        std::path::Path::new(f).parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_default()
                    } else if !app.uninstall_string.is_empty() {
                        let clean = app.uninstall_string.trim().trim_matches('"');
                        let exe_candidate = if let Some(pos) = clean.to_lowercase().find(".exe") {
                            &clean[..pos + 4]
                        } else {
                            clean
                        };
                        std::path::Path::new(exe_candidate).parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_default()
                    } else {
                        String::new()
                    };

                    // Metadata details
                    egui::Grid::new("app_meta_details").spacing([8.0, 4.0]).show(ui, |ui| {
                        ui.label(RichText::new("Тип:").color(meta_label_color));
                        let type_str = if app.source_badge == "[Portable]" {
                            "Portable (Переносная)"
                        } else if app.is_uwp {
                            "Магазин Windows"
                        } else if app.is_64bit {
                            "64-бит Win32"
                        } else {
                            "32-бит Win32"
                        };
                        ui.label(RichText::new(type_str).strong().color(heading_color));
                        ui.end_row();

                        ui.label(RichText::new("Версия:").color(meta_label_color));
                        ui.label(RichText::new(if app.version.is_empty() { "1.0" } else { &app.version }).strong());
                        ui.end_row();

                        ui.label(RichText::new("Размер:").color(meta_label_color));
                        ui.label(RichText::new(format_size(app.size_bytes)).strong());
                        ui.end_row();

                        ui.label(RichText::new("Установка:").color(meta_label_color));
                        ui.label(RichText::new(if app.install_date.is_empty() { "—" } else { &app.install_date }).strong());
                        ui.end_row();

                        ui.label(RichText::new("Издатель:").color(meta_label_color));
                        ui.label(truncate_chars(if app.publisher.is_empty() { "Не указан" } else { &app.publisher }, 20));
                        ui.end_row();

                        ui.label(RichText::new("Папка:").color(meta_label_color));
                        if !loc.is_empty() {
                            let display_path = truncate_chars(&loc, 22);
                            let link_color = if self.dark_mode { Color32::from_rgb(96, 165, 250) } else { Color32::from_rgb(2, 132, 199) };
                            let link_resp = ui.link(RichText::new(display_path).color(link_color).underline())
                                .on_hover_text(format!("Нажмите, чтобы открыть папку в Проводнике:\n{}", loc));
                            if link_resp.clicked() {
                                open_system_folder(&loc, self.settings.use_total_commander);
                            }
                        } else {
                            ui.label(RichText::new("—").color(meta_label_color));
                        }
                        ui.end_row();

                        let is_traced = self.traced_apps.iter().any(|t| t.name.eq_ignore_ascii_case(&app.name) || t.id == app.id);
                        if is_traced {
                            ui.label(RichText::new("Монитор:").color(meta_label_color));
                            ui.label(RichText::new("⚡ Отслежено").color(Color32::from_rgb(234, 179, 8)).strong())
                                .on_hover_text("Все компоненты программы зафиксированы при установке (100% чистый снос)");
                            ui.end_row();
                        }
                    });

                    ui.add_space(14.0);
                    ui.separator();
                    ui.add_space(10.0);

                    // Primary Uninstall Button
                    let (uninst_btn_fill, uninst_btn_stroke) = if self.dark_mode {
                        (Color32::from_rgb(45, 48, 56), Stroke::new(1.0_f32, Color32::from_rgb(82, 82, 91)))
                    } else {
                        (Color32::from_rgb(37, 99, 235), Stroke::NONE)
                    };
                    let uninst_title = if app.source_badge == "[Portable]" {
                        "🗑 Удалить Portable-папку"
                    } else {
                        "🗑 Деинсталляция"
                    };
                    let uninst_btn = egui::Button::new(
                        RichText::new(uninst_title).size(14.0).strong().color(Color32::WHITE)
                    )
                    .fill(uninst_btn_fill)
                    .stroke(uninst_btn_stroke)
                    .min_size(Vec2::new(ui.available_width(), 36.0));

                    if ui.add(uninst_btn).clicked() {
                        self.start_uninstall(app.clone());
                    }

                    ui.add_space(6.0);

                    // Danger Force Removal Button
                    let force_btn = egui::Button::new(
                        RichText::new("⚠ Принудительное удаление")
                            .size(13.0)
                            .strong()
                            .color(danger_text)
                    )
                    .fill(danger_fill)
                    .stroke(Stroke::new(1.0_f32, danger_stroke))
                    .min_size(Vec2::new(ui.available_width(), 32.0));

                    if ui.add(force_btn).clicked() {
                        self.force_confirm_app = Some(app.clone());
                        self.force_confirm_open = true;
                    }

                    ui.add_space(14.0);
                    ui.separator();
                    ui.add_space(8.0);

                    ui.label(RichText::new("Действия").small().strong().color(meta_label_color));
                    ui.add_space(4.0);

                    if ui.link("❌ Удалить запись из списка").on_hover_text("Удалить поврежденную запись из реестра Windows без запуска деинсталлятора").clicked() {
                        self.remove_entry_confirm_app = Some(app.clone());
                        self.remove_entry_confirm_open = true;
                    }
                    if ui.link("📁 Открыть папку программы").clicked() {
                        if !loc.is_empty() {
                            let p = std::path::Path::new(&loc);
                            let target = if p.is_file() {
                                p.parent().unwrap_or(p)
                            } else {
                                p
                            };
                            let _ = open::that(target);
                        }
                    }
                    if !app.is_uwp && !app.registry_path.is_empty() {
                        if ui.link("🗄 Ключ в реестре Windows").clicked() {
                            jump_to_regedit(&app.registry_path);
                        }
                    }
                    if ui.link("🌐 Найти информацию в Google").clicked() {
                        let url = format!("https://www.google.com/search?q={}+{}", app.name, app.publisher);
                        let _ = open::that(url);
                    }
                } else {
                    ui.label("Выберите программу из списка для просмотра сведений и действий.");
                }
            });

        // Right Main Table Area
        egui::Frame::none()
            .fill(table_container_fill)
            .inner_margin(4.0)
            .show(ui, |ui| {
                let mut app_to_select = None;
                let mut app_to_uninstall = None;
                let mut app_to_force = None;
                let mut app_to_remove_entry = None;
                let mut app_to_dismiss_single: Option<String> = None;
                let mut app_to_dismiss_all = false;

                if filtered_indices.is_empty() {
                    ui.add_space(30.0);
                    ui.vertical_centered(|ui| {
                        ui.label(RichText::new("Ничего не найдено").size(16.0).color(Color32::from_rgb(100, 116, 139)));
                        ui.label("Попробуйте сбросить поисковый фильтр (Esc) или переключить категорию.");
                    });
                } else {
                    let all_filtered_selected = !filtered_indices.is_empty()
                        && filtered_indices.iter().all(|&i| self.batch_selected.contains(&self.apps[i].id));

                    TableBuilder::new(ui)
                        .striped(true)
                        .resizable(true)
                        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                        .column(Column::exact(34.0))                    // Batch Checkbox Column
                        .column(Column::initial(340.0).at_least(220.0)) // Program Name (Resizable!)
                        .column(Column::exact(95.0))                    // Size
                        .column(Column::exact(110.0))                   // Date
                        .column(Column::initial(170.0).at_least(110.0)) // Publisher (Resizable!)
                        .column(Column::remainder().at_least(130.0))    // Type / Arch (LAST COLUMN gets remainder!)
                        .header(30.0, |mut header| {
                            // Column 0: Checkbox Header
                            header.col(|ui| {
                                let mut master_chk = all_filtered_selected;
                                if ui.checkbox(&mut master_chk, "").on_hover_text("Выбрать все / Снять выделение").changed() {
                                    if master_chk {
                                        for &i in &filtered_indices {
                                            self.batch_selected.insert(self.apps[i].id.clone());
                                        }
                                    } else {
                                        for &i in &filtered_indices {
                                            self.batch_selected.remove(&self.apps[i].id);
                                        }
                                    }
                                }
                            });

                            // Column 1: Name Sort Button
                            header.col(|ui| {
                                if render_sort_header(ui, "Имя программы", self.sort_column == SortColumn::Name, self.sort_order, self.dark_mode).clicked() {
                                    if self.sort_column == SortColumn::Name {
                                        self.sort_order = match self.sort_order {
                                            SortOrder::Ascending => SortOrder::Descending,
                                            SortOrder::Descending => SortOrder::Ascending,
                                        };
                                    } else {
                                        self.sort_column = SortColumn::Name;
                                        self.sort_order = SortOrder::Ascending;
                                    }
                                }
                            });

                            // Column 2: Size Sort Button
                            header.col(|ui| {
                                if render_sort_header(ui, "Размер", self.sort_column == SortColumn::Size, self.sort_order, self.dark_mode).clicked() {
                                    if self.sort_column == SortColumn::Size {
                                        self.sort_order = match self.sort_order {
                                            SortOrder::Ascending => SortOrder::Descending,
                                            SortOrder::Descending => SortOrder::Ascending,
                                        };
                                    } else {
                                        self.sort_column = SortColumn::Size;
                                        self.sort_order = SortOrder::Descending;
                                    }
                                }
                            });

                            // Column 3: Date Sort Button
                            header.col(|ui| {
                                if render_sort_header(ui, "Установлено", self.sort_column == SortColumn::Date, self.sort_order, self.dark_mode).clicked() {
                                    if self.sort_column == SortColumn::Date {
                                        self.sort_order = match self.sort_order {
                                            SortOrder::Ascending => SortOrder::Descending,
                                            SortOrder::Descending => SortOrder::Ascending,
                                        };
                                    } else {
                                        self.sort_column = SortColumn::Date;
                                        self.sort_order = SortOrder::Descending;
                                    }
                                }
                            });

                            // Column 4: Publisher Sort Button
                            header.col(|ui| {
                                if render_sort_header(ui, "Издатель", self.sort_column == SortColumn::Publisher, self.sort_order, self.dark_mode).clicked() {
                                    if self.sort_column == SortColumn::Publisher {
                                        self.sort_order = match self.sort_order {
                                            SortOrder::Ascending => SortOrder::Descending,
                                            SortOrder::Descending => SortOrder::Ascending,
                                        };
                                    } else {
                                        self.sort_column = SortColumn::Publisher;
                                        self.sort_order = SortOrder::Ascending;
                                    }
                                }
                            });

                            // Column 5: Type Sort Button
                            header.col(|ui| {
                                if render_sort_header(ui, "Тип / Разрядность", self.sort_column == SortColumn::AppType, self.sort_order, self.dark_mode).clicked() {
                                    if self.sort_column == SortColumn::AppType {
                                        self.sort_order = match self.sort_order {
                                            SortOrder::Ascending => SortOrder::Descending,
                                            SortOrder::Descending => SortOrder::Ascending,
                                        };
                                    } else {
                                        self.sort_column = SortColumn::AppType;
                                        self.sort_order = SortOrder::Ascending;
                                    }
                                }
                            });
                        })
                        .body(|body| {
                            let row_height = if self.settings.compact_list { 26.0 } else { 34.0 };
                            body.rows(row_height, filtered_indices.len(), |mut row| {
                                let row_idx = row.index();
                                let idx = filtered_indices[row_idx];
                                let app = &self.apps[idx];
                                let is_selected = self.selected_app_idx == Some(idx);
                                row.set_selected(is_selected);

                                // Column 0: Checkbox
                                row.col(|ui| {
                                    let mut is_checked = self.batch_selected.contains(&app.id);
                                    if ui.checkbox(&mut is_checked, "").changed() {
                                        if is_checked {
                                            self.batch_selected.insert(app.id.clone());
                                        } else {
                                            self.batch_selected.remove(&app.id);
                                        }
                                    }
                                });

                                let show_row_context_menu = |ui: &mut egui::Ui,
                                                             target_app: &InstalledApp,
                                                             uninst: &mut Option<InstalledApp>,
                                                             force: &mut Option<InstalledApp>,
                                                             remove_entry: &mut Option<InstalledApp>,
                                                             dismiss_single: &mut Option<String>,
                                                             dismiss_all: &mut bool| {
                                    if ui.button("🗑 Деинсталляция...").clicked() {
                                        *uninst = Some(target_app.clone());
                                        ui.close_menu();
                                    }
                                    if ui.button("⚠ Принудительное удаление...").clicked() {
                                        *force = Some(target_app.clone());
                                        ui.close_menu();
                                    }
                                    if ui.button("❌ Удалить запись из списка...").clicked() {
                                        *remove_entry = Some(target_app.clone());
                                        ui.close_menu();
                                    }
                                    ui.separator();
                                    if !target_app.install_location.is_empty() && std::path::Path::new(&target_app.install_location).exists() {
                                        if ui.button("📁 Открыть папку программы").clicked() {
                                            open_system_folder(&target_app.install_location, self.settings.use_total_commander);
                                            ui.close_menu();
                                        }
                                    }
                                    if !target_app.is_uwp && !target_app.registry_path.is_empty() {
                                        if ui.button("🗄 Открыть ключ в реестре").clicked() {
                                            jump_to_regedit(&target_app.registry_path);
                                            ui.close_menu();
                                        }
                                    }
                                    if ui.button("🌐 Искать в Google").clicked() {
                                        let url = format!("https://www.google.com/search?q={}+{}", target_app.name, target_app.publisher);
                                        let _ = open::that(url);
                                        ui.close_menu();
                                    }
                                    if target_app.is_new {
                                        ui.separator();
                                        if ui.button("✔ Снять отметку «Новое» с этой программы").clicked() {
                                            *dismiss_single = Some(target_app.id.clone());
                                            ui.close_menu();
                                        }
                                    }
                                    if self.new_apps_count > 0 {
                                        if ui.button("✨ Сбросить все отметки «Новое»").clicked() {
                                            *dismiss_all = true;
                                            ui.close_menu();
                                        }
                                    }
                                };

                                // Column 1: Name with REAL 24x24 ICON & CONTEXT MENU
                                let _ = row.col(|ui| {
                                    let cell_resp = ui.interact(ui.max_rect(), ui.id().with(1), egui::Sense::click());
                                    if cell_resp.clicked() {
                                        app_to_select = Some(idx);
                                    }
                                    if cell_resp.double_clicked() {
                                        app_to_uninstall = Some(app.clone());
                                    }
                                    cell_resp.context_menu(|ui| {
                                        show_row_context_menu(ui, app, &mut app_to_uninstall, &mut app_to_force, &mut app_to_remove_entry, &mut app_to_dismiss_single, &mut app_to_dismiss_all);
                                    });

                                    ui.horizontal(|ui| {
                                        // Render real icon if available, or load on demand
                                        if !app.display_icon.is_empty() {
                                            if let Some(opt_tex) = self.icon_textures.get(&app.id) {
                                                if let Some(tex) = opt_tex {
                                                    ui.image((tex.id(), egui::vec2(22.0, 22.0)));
                                                } else {
                                                    let icon = if app.is_uwp { "🌐" } else if app.is_system_component { "⚙" } else { "📦" };
                                                    ui.label(RichText::new(icon).size(16.0));
                                                }
                                            } else {
                                                let opt_img = icons::extract_app_icon(&app.display_icon);
                                                let tex_opt = opt_img.map(|img| ui.ctx().load_texture(&app.id, img, Default::default()));
                                                if let Some(ref tex) = tex_opt {
                                                    ui.image((tex.id(), egui::vec2(22.0, 22.0)));
                                                } else {
                                                    let icon = if app.is_uwp { "🌐" } else if app.is_system_component { "⚙" } else { "📦" };
                                                    ui.label(RichText::new(icon).size(16.0));
                                                }
                                                self.icon_textures.insert(app.id.clone(), tex_opt);
                                            }
                                        } else {
                                            let icon = if app.is_uwp { "🌐" } else if app.is_system_component { "⚙" } else { "📦" };
                                            ui.label(RichText::new(icon).size(16.0));
                                        }

                                        if self.settings.highlight_recent && app.is_new {
                                            ui.colored_label(Color32::from_rgb(234, 179, 8), "✨ НОВОЕ")
                                                .on_hover_text("Недавно установленная программа");
                                        }

                                        let is_traced = self.traced_apps.iter().any(|t| t.name.eq_ignore_ascii_case(&app.name) || t.id == app.id);
                                        if is_traced {
                                            ui.label(RichText::new("⚡").color(Color32::from_rgb(234, 179, 8)).strong())
                                                .on_hover_text("Установка отслежена с помощью PurgeCraft Install Tracker");
                                        }

                                        if !app.source_badge.is_empty() {
                                            let (badge_text_color, badge_bg) = match app.source_badge.as_str() {
                                                "[Portable]" => (Color32::from_rgb(251, 146, 60), if self.dark_mode { Color32::from_rgb(67, 20, 7) } else { Color32::from_rgb(255, 237, 213) }),
                                                "[Steam]" => (Color32::from_rgb(56, 189, 248), if self.dark_mode { Color32::from_rgb(15, 23, 42) } else { Color32::from_rgb(224, 242, 254) }),
                                                "[Epic]" => (Color32::from_rgb(226, 232, 240), if self.dark_mode { Color32::from_rgb(30, 41, 59) } else { Color32::from_rgb(241, 245, 249) }),
                                                "[Store]" => (Color32::from_rgb(45, 212, 191), if self.dark_mode { Color32::from_rgb(4, 47, 46) } else { Color32::from_rgb(204, 251, 241) }),
                                                "[Winget]" => (Color32::from_rgb(192, 132, 252), if self.dark_mode { Color32::from_rgb(46, 16, 77) } else { Color32::from_rgb(243, 232, 255) }),
                                                _ => (Color32::from_rgb(161, 161, 170), if self.dark_mode { Color32::from_rgb(39, 39, 42) } else { Color32::from_rgb(244, 244, 245) }),
                                            };
                                            egui::Frame::none()
                                                .fill(badge_bg)
                                                .rounding(3.0)
                                                .inner_margin(egui::Margin::symmetric(5.0, 1.5))
                                                .show(ui, |ui| {
                                                    ui.label(RichText::new(&app.source_badge).size(11.0).strong().color(badge_text_color));
                                                });
                                        }

                                        let name_color = if self.settings.highlight_recent && app.is_new {
                                            if is_selected {
                                                Color32::WHITE
                                            } else if self.dark_mode {
                                                Color32::from_rgb(250, 204, 21)
                                            } else {
                                                Color32::from_rgb(180, 83, 9)
                                            }
                                        } else if is_selected {
                                            Color32::WHITE
                                        } else if self.dark_mode {
                                            Color32::from_rgb(244, 244, 245)
                                        } else {
                                            Color32::from_rgb(15, 23, 42)
                                        };

                                        let label_widget = egui::Label::new(RichText::new(&app.name).strong().color(name_color)).truncate();
                                        ui.add(label_widget);
                                    });
                                });

                                // Column 2: Size - CLICKABLE ANYWHERE + CONTEXT MENU
                                let _ = row.col(|ui| {
                                    let cell_resp = ui.interact(ui.max_rect(), ui.id().with(2), egui::Sense::click());
                                    if cell_resp.clicked() {
                                        app_to_select = Some(idx);
                                    }
                                    if cell_resp.double_clicked() {
                                        app_to_uninstall = Some(app.clone());
                                    }
                                    cell_resp.context_menu(|ui| {
                                        show_row_context_menu(ui, app, &mut app_to_uninstall, &mut app_to_force, &mut app_to_remove_entry, &mut app_to_dismiss_single, &mut app_to_dismiss_all);
                                    });
                                    let sz_color = if is_selected { Color32::WHITE } else if self.dark_mode { Color32::from_rgb(212, 212, 216) } else { Color32::from_rgb(51, 65, 85) };
                                    ui.label(RichText::new(format_size(app.size_bytes)).color(sz_color));
                                });

                                // Column 3: Date - CLICKABLE ANYWHERE + CONTEXT MENU
                                let _ = row.col(|ui| {
                                    let cell_resp = ui.interact(ui.max_rect(), ui.id().with(3), egui::Sense::click());
                                    if cell_resp.clicked() {
                                        app_to_select = Some(idx);
                                    }
                                    if cell_resp.double_clicked() {
                                        app_to_uninstall = Some(app.clone());
                                    }
                                    cell_resp.context_menu(|ui| {
                                        show_row_context_menu(ui, app, &mut app_to_uninstall, &mut app_to_force, &mut app_to_remove_entry, &mut app_to_dismiss_single, &mut app_to_dismiss_all);
                                    });
                                    let dt_color = if is_selected { Color32::WHITE } else if self.dark_mode { Color32::from_rgb(161, 161, 170) } else { Color32::from_rgb(100, 116, 139) };
                                    let date_str = if app.install_date.is_empty() { "—" } else { &app.install_date };
                                    ui.label(RichText::new(date_str).color(dt_color));
                                });

                                // Column 4: Publisher - CLICKABLE ANYWHERE + CONTEXT MENU
                                let _ = row.col(|ui| {
                                    let cell_resp = ui.interact(ui.max_rect(), ui.id().with(4), egui::Sense::click());
                                    if cell_resp.clicked() {
                                        app_to_select = Some(idx);
                                    }
                                    if cell_resp.double_clicked() {
                                        app_to_uninstall = Some(app.clone());
                                    }
                                    cell_resp.context_menu(|ui| {
                                        show_row_context_menu(ui, app, &mut app_to_uninstall, &mut app_to_force, &mut app_to_remove_entry, &mut app_to_dismiss_single, &mut app_to_dismiss_all);
                                    });
                                    let pub_color = if is_selected { Color32::WHITE } else if self.dark_mode { Color32::from_rgb(161, 161, 170) } else { Color32::from_rgb(100, 116, 139) };
                                    let pub_str = truncate_chars(if app.publisher.is_empty() { "—" } else { &app.publisher }, 26);
                                    ui.label(RichText::new(pub_str).color(pub_color));
                                });

                                // Column 5: Type - CLICKABLE ANYWHERE + CONTEXT MENU
                                let _ = row.col(|ui| {
                                    let cell_resp = ui.interact(ui.max_rect(), ui.id().with(5), egui::Sense::click());
                                    if cell_resp.clicked() {
                                        app_to_select = Some(idx);
                                    }
                                    if cell_resp.double_clicked() {
                                        app_to_uninstall = Some(app.clone());
                                    }
                                    cell_resp.context_menu(|ui| {
                                        show_row_context_menu(ui, app, &mut app_to_uninstall, &mut app_to_force, &mut app_to_remove_entry, &mut app_to_dismiss_single, &mut app_to_dismiss_all);
                                    });
                                    let is_traced = self.traced_apps.iter().any(|t| t.name.eq_ignore_ascii_case(&app.name) || t.id == app.id);
                                    let tag = if is_traced {
                                        "⚡ Отслежено"
                                    } else if app.is_uwp {
                                        "Store"
                                    } else if app.is_64bit {
                                        "64-бит"
                                    } else if self.settings.mark_32bit {
                                        "32-бит"
                                    } else {
                                        "—"
                                    };
                                    let tag_color = if is_selected { Color32::WHITE } else if self.dark_mode { Color32::from_rgb(161, 161, 170) } else { Color32::from_rgb(100, 116, 139) };
                                    ui.label(RichText::new(tag).color(tag_color));
                                });
                            });
                        });
                }

                if let Some(id_to_clear) = app_to_dismiss_single {
                    if let Some(app) = self.apps.iter_mut().find(|a| a.id == id_to_clear) {
                        app.is_new = false;
                    }
                    save_known_apps(&self.apps);
                    self.new_apps_count = self.apps.iter().filter(|a| a.is_new).count();
                    self.status_msg = "Отметка «Новое» снята.".to_string();
                }
                if app_to_dismiss_all {
                    for a in &mut self.apps {
                        a.is_new = false;
                    }
                    save_known_apps(&self.apps);
                    self.new_apps_count = 0;
                    self.status_msg = "Все отметки «Новое» сброшены.".to_string();
                }

                if let Some(sel) = app_to_select {
                    self.selected_app_idx = Some(sel);
                }
                if let Some(app) = app_to_uninstall {
                    self.start_uninstall(app);
                }
                if let Some(app) = app_to_force {
                    self.force_confirm_app = Some(app);
                    self.force_confirm_open = true;
                }
                if let Some(app) = app_to_remove_entry {
                    self.remove_entry_confirm_app = Some(app);
                    self.remove_entry_confirm_open = true;
                }
            });
    }

    fn render_startup(&mut self, ui: &mut egui::Ui) {
        let table_bg = if self.dark_mode {
            Color32::from_rgb(18, 18, 20)
        } else {
            Color32::WHITE
        };

        let q = self.search_query.to_lowercase();
        let filter = self.startup_filter;
        let filtered_indices: Vec<usize> = self.startup_items
            .iter()
            .enumerate()
            .filter(|(_, item)| {
                let matches_filter = match filter {
                    StartupFilter::All => true,
                    StartupFilter::Registry => item.source_type == "hklm_run" || item.source_type == "hkcu_run" || item.source_type == "hklm_runonce" || item.source_type == "hkcu_runonce",
                    StartupFilter::StartMenu => item.source_type == "startmenu",
                    StartupFilter::TaskScheduler => item.source_type == "taskscheduler",
                };
                if !matches_filter {
                    return false;
                }
                if q.is_empty() {
                    true
                } else {
                    item.name.to_lowercase().contains(&q) || item.command.to_lowercase().contains(&q) || item.source.to_lowercase().contains(&q)
                }
            })
            .map(|(i, _)| i)
            .collect();

        egui::Frame::none()
            .fill(table_bg)
            .inner_margin(4.0)
            .show(ui, |ui| {
                if filtered_indices.is_empty() {
                    ui.vertical_centered(|ui| {
                        ui.add_space(60.0);
                        ui.label(RichText::new("Записей автозагрузки не найдено").size(16.0).color(Color32::from_rgb(100, 116, 139)));
                        ui.label("Попробуйте сбросить поисковый фильтр (Esc) или переключить категорию.");
                    });
                    return;
                }

                let mut to_toggle = None;
                let mut to_delete = None;
                let count = filtered_indices.len();

                TableBuilder::new(ui)
                    .striped(true)
                    .resizable(true)
                    .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                    .column(Column::exact(86.0))                   // State Toggle (✔ Вкл / ○ Откл)
                    .column(Column::initial(220.0).at_least(160.0)) // Name
                    .column(Column::exact(130.0))                  // Startup Impact
                    .column(Column::exact(170.0))                  // Source
                    .column(Column::remainder().at_least(220.0))   // Command / Path
                    .column(Column::exact(90.0))                   // Action
                    .header(28.0, |mut header| {
                        header.col(|ui| { ui.strong("Состояние"); });
                        header.col(|ui| { ui.strong("Имя"); });
                        header.col(|ui| { ui.strong("Влияние"); });
                        header.col(|ui| { ui.strong("Источник"); });
                        header.col(|ui| { ui.strong("Команда / Расположение"); });
                        header.col(|ui| { ui.strong("Действие"); });
                    })
                    .body(|body| {
                        body.rows(28.0, count, |mut row| {
                            let row_idx = row.index();
                            let orig_idx = filtered_indices[row_idx];
                            let item = &self.startup_items[orig_idx];

                            // Column 0: Toggle Switch (✔ Вкл / ○ Откл)
                            row.col(|ui| {
                                let (label_text, btn_fill, btn_color) = if item.is_enabled {
                                    ("✔ Вкл", if self.dark_mode { Color32::from_rgb(20, 83, 45) } else { Color32::from_rgb(220, 252, 231) }, Color32::from_rgb(34, 197, 94))
                                } else {
                                    ("○ Откл", if self.dark_mode { Color32::from_rgb(39, 39, 42) } else { Color32::from_rgb(244, 244, 245) }, Color32::from_rgb(148, 163, 184))
                                };
                                let toggle_btn = egui::Button::new(RichText::new(label_text).color(btn_color).strong().size(12.0))
                                    .fill(btn_fill)
                                    .min_size(Vec2::new(72.0, 22.0));
                                if ui.add(toggle_btn).on_hover_text("Нажмите для включения/отключения автозапуска").clicked() {
                                    to_toggle = Some(orig_idx);
                                }
                            });

                            // Column 1: Name with Context Menu
                            row.col(|ui| {
                                let resp = ui.label(RichText::new(&item.name).strong());
                                resp.context_menu(|ui| {
                                    if ui.button(if item.is_enabled { "○ Отключить автозапуск" } else { "✔ Включить автозапуск" }).clicked() {
                                        to_toggle = Some(orig_idx);
                                        ui.close_menu();
                                    }
                                    ui.separator();
                                    if ui.button("📁 Открыть расположение файла").clicked() {
                                        let cmd_clean = item.command.trim().trim_matches('"');
                                        let exe_part = if let Some(pos) = cmd_clean.to_lowercase().find(".exe") {
                                            &cmd_clean[..pos + 4]
                                        } else {
                                            cmd_clean
                                        };
                                        if let Some(parent) = std::path::Path::new(exe_part).parent() {
                                            let _ = open::that(parent);
                                        }
                                        ui.close_menu();
                                    }
                                    if item.source_type.contains("run") {
                                        if ui.button("🗄 Открыть ключ в реестре").clicked() {
                                            let sub_hive = if item.source_type.starts_with("hklm") { "HKLM" } else { "HKCU" };
                                            let key_name = if item.source_type.contains("runonce") { "RunOnce" } else { "Run" };
                                            let reg_path = format!("{}\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\{}", sub_hive, key_name);
                                            jump_to_regedit(&reg_path);
                                            ui.close_menu();
                                        }
                                    }
                                    ui.separator();
                                    if ui.button("❌ Удалить из автозагрузки").clicked() {
                                        to_delete = Some(orig_idx);
                                        ui.close_menu();
                                    }
                                });
                            });

                            // Column 2: Startup Impact
                            row.col(|ui| {
                                let (label_text, badge_color, badge_bg) = match item.impact {
                                    StartupImpact::High => {
                                        ("⚡ Высокое", Color32::from_rgb(239, 68, 68), if self.dark_mode { Color32::from_rgb(69, 10, 10) } else { Color32::from_rgb(254, 226, 226) })
                                    }
                                    StartupImpact::Medium => {
                                        ("🟡 Среднее", Color32::from_rgb(234, 179, 8), if self.dark_mode { Color32::from_rgb(66, 32, 6) } else { Color32::from_rgb(254, 243, 199) })
                                    }
                                    StartupImpact::Low => {
                                        ("🟢 Низкое", Color32::from_rgb(34, 197, 94), if self.dark_mode { Color32::from_rgb(20, 83, 45) } else { Color32::from_rgb(220, 252, 231) })
                                    }
                                    StartupImpact::None => {
                                        ("⚪ Не измерено", Color32::from_rgb(148, 163, 184), if self.dark_mode { Color32::from_rgb(39, 39, 42) } else { Color32::from_rgb(241, 245, 249) })
                                    }
                                };
                                egui::Frame::none()
                                    .fill(badge_bg)
                                    .rounding(4.0)
                                    .inner_margin(egui::Margin::symmetric(6.0, 2.0))
                                    .show(ui, |ui| {
                                        ui.label(RichText::new(label_text).size(12.0).strong().color(badge_color));
                                    });
                            });

                            // Column 2: Source
                            row.col(|ui| {
                                ui.label(&item.source);
                            });

                            // Column 3: Command
                            row.col(|ui| {
                                ui.label(truncate_chars(&item.command, 55)).on_hover_text(&item.command);
                            });

                            // Column 4: Delete button
                            row.col(|ui| {
                                if ui.button("Удалить").on_hover_text("Полностью удалить запись из автозапуска").clicked() {
                                    to_delete = Some(orig_idx);
                                }
                            });
                        });
                    });

                if let Some(toggle_idx) = to_toggle {
                    let item = &mut self.startup_items[toggle_idx];
                    let old_name = item.name.clone();
                    if toggle_startup_item(item) {
                        self.status_msg = format!("Автозапуск для «{}» теперь {}.", old_name, if item.is_enabled { "включен" } else { "отключен" });
                    } else {
                        self.status_msg = format!("Не удалось изменить статус автозапуска для «{}». Возможно, требуются права администратора.", old_name);
                    }
                }

                if let Some(del_idx) = to_delete {
                    let item = &self.startup_items[del_idx];
                    let name = item.name.clone();
                    if delete_startup_item(item) {
                        self.startup_items.remove(del_idx);
                        self.status_msg = format!("Запись автозагрузки «{}» успешно удалена.", name);
                    } else {
                        self.status_msg = format!("Не удалось удалить запись «{}».", name);
                    }
                }
            });
    }

    fn request_uninstall(&mut self, app: InstalledApp) {
        self.uninstall_target_app = Some(app);
        self.uninstall_confirm_open = true;
    }

    fn execute_uninstall(&mut self, app: InstalledApp) {
        if !app.uninstall_string.is_empty() {
            let child = if app.is_uwp {
                let mut cmd = Command::new("powershell");
                #[cfg(windows)]
                cmd.creation_flags(scanner::CREATE_NO_WINDOW);
                cmd.args(["-NoProfile", "-NonInteractive", "-Command", &app.uninstall_string]).spawn().ok()
            } else {
                let mut cmd = Command::new("cmd");
                cmd.args(["/c", &app.uninstall_string]).spawn().ok()
            };

            if let Some(ch) = child {
                self.uninstall_child = Some(ch);
                self.uninstall_waiting_app = Some(app);
                self.uninstall_waiting_open = true;
                return;
            }
        }

        self.start_wizard_scan(app);
    }

    fn start_wizard_scan(&mut self, app: InstalledApp) {
        let id_low = app.id.to_lowercase();
        let name_low = app.name.to_lowercase();
        self.recently_uninstalled.insert(id_low.clone());
        self.recently_uninstalled.insert(name_low.clone());
        self.apps.retain(|a| a.id.to_lowercase() != id_low && a.name.to_lowercase() != name_low);
        if let Some(idx) = self.selected_app_idx {
            if idx >= self.apps.len() {
                self.selected_app_idx = if !self.apps.is_empty() { Some(0) } else { None };
            }
        }
        self.wizard_app_title = app.name.clone();
        self.wizard_target_app = Some(app.clone());
        self.wizard_phase = WizardPhase::Scanning;
        self.wizard_scan_progress = 0.0;
        self.wizard_scan_current_path = "Инициализация поиска остатков...".to_string();
        self.wizard_traces.clear();
        self.wizard_report = None;
        self.wizard_xml_status = None;
        self.wizard_scan_rx = Some(spawn_scan_leftovers_stream(app, self.traced_apps.clone()));
        self.wizard_open = true;
    }

    fn start_uninstall(&mut self, app: InstalledApp) {
        self.request_uninstall(app);
    }

    fn start_batch_uninstall(&mut self) {
        let targets: Vec<InstalledApp> = self.apps.iter()
            .filter(|a| self.batch_selected.contains(&a.id))
            .cloned()
            .collect();

        if targets.is_empty() {
            return;
        }

        let n = targets.len();
        self.batch_queue_apps = targets;
        self.batch_queue_idx = 0;
        self.batch_queue_statuses = vec!["Ожидает".to_string(); n];
        if !self.batch_queue_statuses.is_empty() {
            self.batch_queue_statuses[0] = "Выполняется...".to_string();
        }
        self.batch_queue_open = true;

        // Launch first app in queue
        self.launch_current_batch_app();
    }

    fn launch_current_batch_app(&mut self) {
        if self.batch_queue_idx < self.batch_queue_apps.len() {
            let app = &self.batch_queue_apps[self.batch_queue_idx];
            if !app.uninstall_string.is_empty() {
                if app.is_uwp {
                    let mut cmd = Command::new("powershell");
                    #[cfg(windows)]
                    cmd.creation_flags(scanner::CREATE_NO_WINDOW);
                    let _ = cmd.args(["-NoProfile", "-NonInteractive", "-Command", &app.uninstall_string]).spawn();
                } else {
                    let mut cmd = Command::new("cmd");
                    #[cfg(windows)]
                    cmd.creation_flags(scanner::CREATE_NO_WINDOW);
                    let _ = cmd.args(["/c", &app.uninstall_string]).spawn();
                }
            }
            self.batch_queue_statuses[self.batch_queue_idx] = "Выполняется...".to_string();
        }
    }

    fn start_batch_force_removal(&mut self) {
        let targets: Vec<InstalledApp> = self.apps.iter()
            .filter(|a| self.batch_selected.contains(&a.id))
            .cloned()
            .collect();

        if targets.is_empty() {
            return;
        }

        self.batch_force_apps = targets;
        self.batch_force_open = true;
    }

    fn render_modals(&mut self, ctx: &egui::Context) {
        // 0. Single Uninstall Confirmation Modal
        if self.uninstall_confirm_open {
            if let Some(app) = self.uninstall_target_app.clone() {
                egui::Window::new("Подтверждение деинсталляции")
                    .collapsible(false)
                    .resizable(false)
                    .pivot(egui::Align2::CENTER_CENTER)
                    .default_pos(ctx.screen_rect().center())
                    .show(ctx, |ui| {
                        ui.set_width(450.0);
                        ui.heading("Деинсталляция программы");
                        ui.add_space(8.0);
                        ui.label(format!("Вы действительно хотите удалить программу «{}»?", app.name));
                        ui.label("После выполнения удаления PurgeCraft проверит систему на оставшиеся файлы, папки и записи реестра.");
                        ui.add_space(16.0);

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.add(egui::Button::new("Отмена").min_size(Vec2::new(85.0, 26.0))).clicked() {
                                self.uninstall_confirm_open = false;
                                self.uninstall_target_app = None;
                            }
                            let uninst_btn = egui::Button::new(
                                RichText::new("Деинсталлировать").strong().color(Color32::WHITE),
                            )
                            .fill(Color32::from_rgb(37, 99, 235))
                            .min_size(Vec2::new(145.0, 26.0));

                            if ui.add(uninst_btn).clicked() {
                                self.uninstall_confirm_open = false;
                                self.uninstall_target_app = None;
                                let running = scanner::find_running_processes_for_app(&app);
                                if !running.is_empty() {
                                    self.running_processes = running;
                                    self.running_processes_target_app = Some(app);
                                    self.running_processes_modal_open = true;
                                } else {
                                    self.execute_uninstall(app);
                                }
                            }
                        });
                    });
            }
        }

        // 0a. Running Processes Termination Modal (S09 RunningProcessesDialog 1:1 with CrystalIdea)
        if self.running_processes_modal_open {
            if let Some(target_app) = self.running_processes_target_app.clone() {
                egui::Window::new("Принудительно завершить запущенные процессы?")
                    .collapsible(false)
                    .resizable(false)
                    .pivot(egui::Align2::CENTER_CENTER)
                    .default_pos(ctx.screen_rect().center())
                    .show(ctx, |ui| {
                        ui.set_width(520.0);

                        ui.horizontal(|ui| {
                            ui.label(RichText::new("⛔").size(24.0));
                            ui.vertical(|ui| {
                                ui.heading(RichText::new("Принудительно завершить запущенные процессы?").strong());
                                ui.label(RichText::new("Следующие процессы были обнаружены:").color(Color32::from_rgb(156, 163, 175)));
                            });
                        });

                        ui.add_space(8.0);

                        egui::Frame::none()
                            .fill(if self.dark_mode { Color32::from_rgb(24, 24, 27) } else { Color32::from_rgb(243, 244, 246) })
                            .rounding(4.0)
                            .stroke(Stroke::new(1.0_f32, if self.dark_mode { Color32::from_rgb(63, 63, 70) } else { Color32::from_rgb(229, 231, 235) }))
                            .inner_margin(8.0)
                            .show(ui, |ui| {
                                egui::ScrollArea::vertical().max_height(140.0).show(ui, |ui| {
                                    for proc in &self.running_processes {
                                        ui.horizontal(|ui| {
                                            ui.label(RichText::new(format!("[PID {}]", proc.pid)).monospace().color(Color32::from_rgb(245, 158, 11)));
                                            ui.label(RichText::new(&proc.name).strong());
                                            ui.label(RichText::new(format!("({})", truncate_chars(&proc.exe_path, 40))).color(Color32::GRAY));
                                        });
                                    }
                                });
                            });

                        ui.add_space(10.0);
                        ui.label(RichText::new("Рекомендуется завершать процессы вручную (если это возможно) перед удалением программы, однако Мастер Удаления это может сделать принудительно.")
                            .small()
                            .color(if self.dark_mode { Color32::from_rgb(209, 213, 219) } else { Color32::from_rgb(55, 65, 81) }));

                        ui.add_space(14.0);

                        ui.horizontal(|ui| {
                            if ui.button(RichText::new("⛔ Завершить процессы").strong().color(Color32::WHITE)).clicked() {
                                for proc in &self.running_processes {
                                    let _ = scanner::terminate_process_by_pid(proc.pid);
                                }
                                self.running_processes_modal_open = false;
                                self.running_processes.clear();
                                self.running_processes_target_app = None;
                                self.execute_uninstall(target_app.clone());
                            }

                            if ui.button("⏩ Пропустить").clicked() {
                                self.running_processes_modal_open = false;
                                self.running_processes.clear();
                                self.running_processes_target_app = None;
                                self.execute_uninstall(target_app);
                            }

                            if ui.button("✖ Отмена").clicked() {
                                self.running_processes_modal_open = false;
                                self.running_processes.clear();
                                self.running_processes_target_app = None;
                            }
                        });
                    });
            }
        }

        // 0d. Delayed Files Removal On Reboot Modal (S10 DelayedFilesRemovalDialog 1:1 with CrystalIdea)
        if self.delayed_removal_modal_open {
            egui::Window::new("Некоторые файлы не удалены")
                .collapsible(false)
                .resizable(false)
                .pivot(egui::Align2::CENTER_CENTER)
                .default_pos(ctx.screen_rect().center())
                .show(ctx, |ui| {
                    ui.set_width(540.0);

                    ui.horizontal(|ui| {
                        ui.label(RichText::new("🔒").size(24.0));
                        ui.vertical(|ui| {
                            ui.heading(RichText::new("Некоторые файлы не удалены").strong());
                            ui.label(RichText::new("Следующие файлы на данный момент используются системой и не могут быть удалены:")
                                .color(Color32::from_rgb(156, 163, 175)));
                        });
                    });

                    ui.add_space(8.0);

                    egui::Frame::none()
                        .fill(if self.dark_mode { Color32::from_rgb(24, 24, 27) } else { Color32::from_rgb(243, 244, 246) })
                        .rounding(4.0)
                        .stroke(Stroke::new(1.0_f32, if self.dark_mode { Color32::from_rgb(63, 63, 70) } else { Color32::from_rgb(229, 231, 235) }))
                        .inner_margin(8.0)
                        .show(ui, |ui| {
                            egui::ScrollArea::vertical().max_height(140.0).show(ui, |ui| {
                                for file in &self.delayed_removal_files {
                                    ui.horizontal(|ui| {
                                        ui.label(RichText::new("⏳").color(Color32::from_rgb(245, 158, 11)));
                                        ui.label(RichText::new(file).monospace().color(if self.dark_mode { Color32::from_rgb(229, 231, 235) } else { Color32::from_rgb(31, 41, 55) }));
                                    });
                                }
                            });
                        });

                    ui.add_space(10.0);
                    ui.label(RichText::new("Эти файлы будут удалены автоматически при следующем запуске компьютера.")
                        .small()
                        .strong()
                        .color(Color32::from_rgb(34, 197, 94)));

                    ui.add_space(14.0);

                    ui.horizontal(|ui| {
                        if ui.button(RichText::new("🔄 Перезагрузить").strong().color(Color32::WHITE)).clicked() {
                            self.delayed_removal_modal_open = false;
                            self.delayed_removal_files.clear();
                            let mut cmd = Command::new("shutdown");
                            #[cfg(windows)]
                            cmd.creation_flags(scanner::CREATE_NO_WINDOW);
                            let _ = cmd.args(["/r", "/t", "5"]).spawn();
                        }

                        if ui.button("Позже").clicked() {
                            self.delayed_removal_modal_open = false;
                            self.delayed_removal_files.clear();
                        }
                    });
                });
        }

        // 0b. Waiting for Native Uninstaller Completion Modal
        if self.uninstall_waiting_open {
            if let Some(app) = self.uninstall_waiting_app.clone() {
                egui::Window::new(format!("Деинсталляция — {}", app.name))
                    .id(egui::Id::new("single_uninstall_progress_dialog"))
                    .collapsible(false)
                    .resizable(false)
                    .pivot(egui::Align2::CENTER_CENTER)
                    .default_pos(ctx.screen_rect().center())
                    .show(ctx, |ui| {
                        ui.set_width(480.0);
                        ui.heading(RichText::new("Выполняется штатная деинсталляция...").color(Color32::from_rgb(96, 165, 250)));
                        ui.add_space(8.0);
                        ui.label("Запущен штатный деинсталлятор программы.");
                        ui.label("Пожалуйста, завершите процесс удаления в окне программы.");
                        ui.label("Как только процесс завершится, PurgeCraft автоматически запустит поиск оставшихся следов.");
                        ui.add_space(14.0);
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label("Ожидание закрытия деинсталлятора...");
                        });
                        ui.add_space(12.0);
                        if ui.button(RichText::new("⚡ Перейти к поиску остатков").strong()).clicked() {
                            self.uninstall_waiting_open = false;
                            self.uninstall_child = None;
                            self.start_wizard_scan(app);
                        }
                    });
            }
        }

        // 1. Single Force Removal Confirmation
        if self.force_confirm_open {
            if let Some(app) = self.force_confirm_app.clone() {
                egui::Window::new("Принудительное удаление программы")
                    .collapsible(false)
                    .resizable(false)
                    .pivot(egui::Align2::CENTER_CENTER)
                    .default_pos(ctx.screen_rect().center())
                    .show(ctx, |ui| {
                        ui.set_width(450.0);
                        ui.label(RichText::new("Внимание!").color(Color32::from_rgb(239, 68, 68)).heading());
                        ui.add_space(8.0);
                        ui.label(format!("Вы действительно хотите принудительно удалить «{}»?", app.name));
                        ui.label("Штатный деинсталлятор запускаться не будет. Система напрямую выполнит поиск и зачистку всех связанных записей реестра и папок установки.");
                        ui.add_space(16.0);

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.add(egui::Button::new("Отмена").min_size(Vec2::new(85.0, 26.0))).clicked() {
                                self.force_confirm_open = false;
                                self.force_confirm_app = None;
                            }
                            let btn = egui::Button::new(RichText::new("Начать удаление").strong().color(Color32::WHITE))
                                .fill(Color32::from_rgb(220, 38, 38))
                                .min_size(Vec2::new(135.0, 26.0));
                            if ui.add(btn).clicked() {
                                self.force_confirm_open = false;
                                self.force_confirm_app = None;
                                self.start_wizard_scan(app);
                            }
                        });
                    });
            }
        }

        // 1b. Remove Registry Entry Confirmation Modal
        if self.remove_entry_confirm_open {
            if let Some(app) = self.remove_entry_confirm_app.clone() {
                egui::Window::new("Удаление записи из реестра")
                    .collapsible(false)
                    .resizable(false)
                    .pivot(egui::Align2::CENTER_CENTER)
                    .default_pos(ctx.screen_rect().center())
                    .show(ctx, |ui| {
                        ui.set_width(450.0);
                        ui.label(RichText::new("Удалить запись из списка?").heading());
                        ui.add_space(8.0);
                        ui.label(format!("Вы действительно хотите удалить запись «{}»?", app.name));
                        ui.label("Штатный деинсталлятор запускаться не будет, а файлы программы не будут затронуты. Это действие удаляет только запись из реестра Windows (полезно для уже удаленных вручную программ).");
                        ui.add_space(16.0);

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.add(egui::Button::new("Отмена").min_size(Vec2::new(85.0, 26.0))).clicked() {
                                self.remove_entry_confirm_open = false;
                                self.remove_entry_confirm_app = None;
                            }
                            let btn = egui::Button::new(RichText::new("Удалить запись").strong().color(Color32::WHITE))
                                .fill(Color32::from_rgb(220, 38, 38))
                                .min_size(Vec2::new(135.0, 26.0));
                            if ui.add(btn).clicked() {
                                self.remove_entry_confirm_open = false;
                                self.remove_entry_confirm_app = None;
                                self.remove_registry_entry(&app);
                            }
                        });
                    });
            }
        }

        // 2. Batch Uninstall Queue Modal
        if self.batch_queue_open {
            let total = self.batch_queue_apps.len();
            let curr = self.batch_queue_idx;

            egui::Window::new(format!("Очередь пакетной деинсталляции ({}/{})", (curr + 1).min(total), total))
                .id(egui::Id::new("batch_uninstall_queue_dialog"))
                .collapsible(false)
                .resizable(true)
                .pivot(egui::Align2::CENTER_CENTER)
                .default_pos(ctx.screen_rect().center())
                .show(ctx, |ui| {
                    ui.set_width(540.0);
                    ui.set_height(380.0);

                    ui.heading("Процесс пакетного удаления программ");
                    ui.add_space(4.0);
                    ui.label("Программы удаляются по очереди для исключения конфликтов деинсталляторов.");
                    ui.separator();

                    egui::ScrollArea::vertical().max_height(220.0).show(ui, |ui| {
                        for (i, app) in self.batch_queue_apps.iter().enumerate() {
                            ui.horizontal(|ui| {
                                let st = &self.batch_queue_statuses[i];
                                let color = match st.as_str() {
                                    "Выполняется..." => Color32::from_rgb(56, 189, 248),
                                    "Завершено" => Color32::from_rgb(34, 197, 94),
                                    "Пропущено" => Color32::from_rgb(148, 163, 184),
                                    _ => Color32::from_rgb(100, 116, 139),
                                };
                                ui.label(RichText::new(format!("[{}]", st)).color(color).strong());
                                ui.label(RichText::new(&app.name).strong());
                                ui.label(format!("({})", format_size(app.size_bytes)));
                            });
                        }
                    });

                    ui.separator();
                    ui.add_space(8.0);

                    if curr < total {
                        let app_name = self.batch_queue_apps[curr].name.clone();
                        ui.label(format!("Текущая: «{}»", app_name));
                        ui.horizontal(|ui| {
                            if ui.button("⏩ Следующая программа").clicked() {
                                self.batch_queue_statuses[curr] = "Завершено".to_string();
                                self.batch_queue_idx += 1;
                                self.launch_current_batch_app();
                            }
                            if ui.button("Пропустить").clicked() {
                                self.batch_queue_statuses[curr] = "Пропущено".to_string();
                                self.batch_queue_idx += 1;
                                self.launch_current_batch_app();
                            }
                            if ui.button("Прервать очередь").clicked() {
                                self.batch_queue_open = false;
                            }
                        });
                    } else {
                        ui.label(RichText::new("Все программы из очереди обработаны!").color(Color32::from_rgb(34, 197, 94)).strong());
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            if ui.button("🧹 Запустить поиск остатков для всех").clicked() {
                                let mut combined_traces = Vec::new();
                                for a in &self.batch_queue_apps {
                                    combined_traces.extend(scan_leftovers(a));
                                }
                                self.wizard_traces = combined_traces;
                                self.wizard_app_title = format!("Пакет из {} программ", self.batch_queue_apps.len());
                                self.wizard_report = None;
                                self.batch_queue_open = false;
                                self.wizard_open = true;
                            }
                            if ui.button("Закрыть").clicked() {
                                self.batch_queue_open = false;
                                self.batch_selected.clear();
                                self.refresh_data();
                            }
                        });
                    }
                });
        }

        // 3. Batch Force Removal Confirmation Modal
        if self.batch_force_open {
            let count = self.batch_force_apps.len();
            egui::Window::new(format!("Пакетное принудительное удаление ({} программ)", count))
                .id(egui::Id::new("batch_force_modal"))
                .collapsible(false)
                .resizable(false)
                .pivot(egui::Align2::CENTER_CENTER)
                .default_pos(ctx.screen_rect().center())
                .show(ctx, |ui| {
                    ui.set_width(480.0);
                    ui.label(RichText::new("Внимание! Пакетная зачистка следов").color(Color32::from_rgb(239, 68, 68)).heading());
                    ui.add_space(8.0);
                    ui.label(format!("Вы выбрали {} программ для одновременного принудительного удаления.", count));
                    ui.label("Система соберет все следы (реестр, папки, пакеты) воедино и полностью удалит их без запуска штатных деинсталляторов.");
                    ui.add_space(16.0);

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add(egui::Button::new("Отмена").min_size(Vec2::new(85.0, 26.0))).clicked() {
                            self.batch_force_open = false;
                        }
                        let btn = egui::Button::new(RichText::new("Собрать остатки и удалить").strong().color(Color32::WHITE))
                            .fill(Color32::from_rgb(220, 38, 38))
                            .min_size(Vec2::new(195.0, 26.0));
                        if ui.add(btn).clicked() {
                            let mut combined_traces = Vec::new();
                            for a in &self.batch_force_apps {
                                combined_traces.extend(scan_leftovers(a));
                            }
                            self.wizard_traces = combined_traces;
                            self.wizard_app_title = format!("Пакет из {} программ", count);
                            self.wizard_report = None;
                            self.batch_force_open = false;
                            self.wizard_open = true;
                        }
                    });
                });
        }

        // 4. Consolidated Leftovers Wizard Modal (4-Step Parity)
        if self.wizard_open {
            let mut close_wizard = false;

            egui::Window::new(format!("Удаление остатков {} — Мастер Удаления ™", self.wizard_app_title))
                .id(egui::Id::new("wizard_traces_modal"))
                .collapsible(false)
                .resizable(true)
                .pivot(egui::Align2::CENTER_CENTER)
                .default_pos(ctx.screen_rect().center())
                .show(ctx, |ui| {
                    ui.set_width(620.0);
                    ui.set_height(420.0);

                    match self.wizard_phase {
                        WizardPhase::Scanning => {
                            ui.heading(RichText::new("Сканирование оставшихся файлов/папок, элементов реестра...").size(15.0).strong());
                            ui.add_space(8.0);
                            ui.label(RichText::new(truncate_chars(&self.wizard_scan_current_path, 75)).monospace().small());
                            ui.add_space(12.0);

                            let pb = egui::ProgressBar::new(self.wizard_scan_progress)
                                .show_percentage()
                                .fill(Color32::from_rgb(34, 197, 94));
                            ui.add(pb);

                            ui.add_space(16.0);
                            ui.horizontal(|ui| {
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if ui.button("Отмена").clicked() {
                                        close_wizard = true;
                                    }
                                });
                            });
                        }
                        WizardPhase::Review => {
                            ui.label(RichText::new(format!("Принудительное удаление '{}'", self.wizard_app_title)).heading().strong());
                            ui.add_space(4.0);
                            ui.label("Uninstall Tool обнаружила, что следующие части данной программы все еще остались на вашем компьютере. Просмотрите внимательно выбранные элементы прежде, чем вы их решите удалить, нажав кнопку 'Удалить'.");
                            ui.add_space(10.0);

                            // Action buttons top row
                            ui.horizontal(|ui| {
                                if ui.button("📄 Экспортировать в XML").on_hover_text("Экспортировать список остатков в XML-файл").clicked() {
                                    match export_traces_to_xml(&self.wizard_app_title, &self.wizard_traces) {
                                        Ok(path) => {
                                            let _ = Command::new("notepad.exe").arg(&path).spawn();
                                            self.wizard_xml_status = Some("Экспорт завершен (открыт в Блокноте)".to_string());
                                        }
                                        Err(e) => {
                                            self.wizard_xml_status = Some(format!("Ошибка экспорта: {}", e));
                                        }
                                    }
                                }
                                if let Some(st) = &self.wizard_xml_status {
                                    ui.label(RichText::new(st).small().color(Color32::from_rgb(34, 197, 94)));
                                }

                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if ui.button("Отмена").clicked() {
                                        close_wizard = true;
                                    }
                                    let selected_count = self.wizard_traces.iter().filter(|t| t.selected).count();
                                    let del_btn = egui::Button::new(RichText::new("Удалить").strong().color(Color32::WHITE))
                                        .fill(Color32::from_rgb(220, 38, 38));
                                    if ui.add_enabled(selected_count > 0, del_btn).clicked() {
                                        let uninst_reg = self.wizard_target_app.as_ref().map(|a| a.registry_path.clone());
                                        let selected_traces: Vec<_> = self.wizard_traces.iter().filter(|t| t.selected).cloned().collect();
                                        self.wizard_phase = WizardPhase::Deleting;
                                        self.wizard_delete_progress = 0.0;
                                        self.wizard_delete_rx = Some(spawn_delete_leftovers_stream(self.wizard_app_title.clone(), selected_traces, uninst_reg));
                                    }
                                });
                            });

                            ui.separator();

                            // Grouped tree list
                            let fs_traces: Vec<_> = self.wizard_traces.iter().filter(|t| t.trace_type != "registry_key").cloned().collect();
                            let reg_traces: Vec<_> = self.wizard_traces.iter().filter(|t| t.trace_type == "registry_key").cloned().collect();

                            egui::ScrollArea::vertical().max_height(240.0).show(ui, |ui| {
                                if self.wizard_traces.is_empty() {
                                    ui.label("Оставшихся следов программы не обнаружено (чистая система).");
                                } else {
                                    if !fs_traces.is_empty() {
                                        let fs_size: u64 = fs_traces.iter().map(|t| t.size_bytes).sum();
                                        ui.strong(format!("🗀 Файловая система ({} элементов, {})", fs_traces.len(), format_size(fs_size)));
                                        for t in &mut self.wizard_traces {
                                            if t.trace_type != "registry_key" {
                                                ui.horizontal(|ui| {
                                                    ui.add_space(14.0);
                                                    ui.checkbox(&mut t.selected, "");
                                                    let icon = if t.trace_type == "folder" { "📁" } else { "📄" };
                                                    ui.label(format!("{} {}", icon, t.path));
                                                });
                                            }
                                        }
                                    }

                                    if !reg_traces.is_empty() {
                                        ui.add_space(6.0);
                                        ui.strong(format!("🗁 Реестр ({} элементов)", reg_traces.len()));
                                        for t in &mut self.wizard_traces {
                                            if t.trace_type == "registry_key" {
                                                ui.horizontal(|ui| {
                                                    ui.add_space(14.0);
                                                    ui.checkbox(&mut t.selected, "");
                                                    ui.label(format!("🗄 {}", t.path));
                                                });
                                            }
                                        }
                                    }
                                }
                            });

                            ui.separator();

                            // Bottom selection & summary row
                            ui.horizontal(|ui| {
                                if ui.button("Выбрать все").clicked() {
                                    for t in &mut self.wizard_traces {
                                        t.selected = true;
                                    }
                                }
                                if ui.button("Снять выбор").clicked() {
                                    for t in &mut self.wizard_traces {
                                        t.selected = false;
                                    }
                                }

                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    let fs_count = self.wizard_traces.iter().filter(|t| t.trace_type != "registry_key").count();
                                    let reg_count = self.wizard_traces.iter().filter(|t| t.trace_type == "registry_key").count();
                                    let fs_size: u64 = self.wizard_traces.iter().filter(|t| t.trace_type != "registry_key").map(|t| t.size_bytes).sum();
                                    ui.label(RichText::new(format!("{} файлов/папок ({}) и {} элементов реестра", fs_count, format_size(fs_size), reg_count)).small());
                                });
                            });
                        }
                        WizardPhase::Deleting => {
                            ui.heading(RichText::new("Удаление элементов...").size(15.0).strong());
                            ui.add_space(10.0);
                            ui.label("Идет очистка выбранных файлов, папок и веток реестра Windows...");
                            ui.add_space(12.0);

                            let pb = egui::ProgressBar::new(self.wizard_delete_progress)
                                .show_percentage()
                                .fill(Color32::from_rgb(34, 197, 94));
                            ui.add(pb);
                            ui.add_space(8.0);
                            let sub_color = if self.dark_mode { Color32::from_rgb(156, 163, 175) } else { Color32::from_rgb(107, 114, 128) };
                            ui.label(RichText::new(truncate_chars(&self.wizard_scan_current_path, 75)).size(11.0).monospace().color(sub_color));
                        }
                        WizardPhase::Finished => {
                            ui.heading(RichText::new("✔ Все остатки программы удалены").size(16.0).strong().color(Color32::from_rgb(34, 197, 94)));
                            ui.add_space(8.0);
                            ui.label(format!("Программа «{}» успешно удалена с вашего компьютера.", self.wizard_app_title));
                            if let Some(rep) = &self.wizard_report {
                                ui.add_space(4.0);
                                ui.label(RichText::new(rep).color(Color32::from_rgb(34, 197, 94)));
                            }
                            ui.add_space(10.0);

                            // Scrollable list of deleted items in bright green with checkmarks
                            egui::ScrollArea::vertical().max_height(140.0).show(ui, |ui| {
                                for trace in self.wizard_traces.iter().filter(|t| t.selected) {
                                    ui.horizontal(|ui| {
                                        ui.label(RichText::new("✔").color(Color32::from_rgb(34, 197, 94)).strong());
                                        let icon = match trace.trace_type.as_str() {
                                            "registry_key" => "🗄",
                                            "folder" => "📁",
                                            _ => "📄",
                                        };
                                        ui.label(RichText::new(format!("{} {}", icon, trace.path)).color(Color32::from_rgb(34, 197, 94)).size(11.0));
                                    });
                                }
                            });

                            ui.add_space(14.0);
                            ui.horizontal(|ui| {
                                if let Some(log_p) = &self.wizard_last_log_path {
                                    let link_color = if self.dark_mode { Color32::from_rgb(96, 165, 250) } else { Color32::from_rgb(37, 99, 235) };
                                    if ui.link(RichText::new("Лог удаления").underline().color(link_color)).clicked() {
                                        let _ = Command::new("notepad.exe").arg(log_p).spawn();
                                    }
                                }

                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if ui.button(RichText::new("Закрыть").strong()).clicked() {
                                        close_wizard = true;
                                    }
                                });
                            });
                        }
                    }
                });

            if close_wizard {
                self.wizard_open = false;
                if let Some(target) = self.wizard_target_app.take() {
                    let target_id = target.id.to_lowercase();
                    let target_name = target.name.to_lowercase();
                    self.recently_uninstalled.insert(target_id.clone());
                    self.recently_uninstalled.insert(target_name.clone());
                    self.apps.retain(|a| a.id.to_lowercase() != target_id && a.name.to_lowercase() != target_name);
                }
                self.wizard_app_title.clear();
                self.wizard_traces.clear();
                self.wizard_report = None;
                self.wizard_scan_rx = None;
                self.wizard_delete_rx = None;
                self.wizard_xml_status = None;
                let total_size: u64 = self.apps.iter().map(|a| a.size_bytes).sum();
                self.status_msg = format!("Программ: {} | Общий объем: {}", self.apps.len(), format_size(total_size));
                if let Some(idx) = self.selected_app_idx {
                    if idx >= self.apps.len() {
                        self.selected_app_idx = if !self.apps.is_empty() { Some(0) } else { None };
                    }
                }
            }
        }

        // 5. Add Startup Item Modal
        if self.add_startup_modal_open {
            egui::Window::new("➕ Добавить программу в автозапуск")
                .collapsible(false)
                .resizable(false)
                .pivot(egui::Align2::CENTER_CENTER)
                .default_pos(ctx.screen_rect().center())
                .show(ctx, |ui| {
                    ui.set_width(460.0);
                    ui.heading("Новая запись автозагрузки");
                    ui.add_space(8.0);

                    ui.label("Имя программы:");
                    ui.text_edit_singleline(&mut self.new_startup_name);

                    ui.add_space(6.0);
                    ui.label("Путь к исполняемому файлу (.exe):");
                    ui.horizontal(|ui| {
                        ui.add(egui::TextEdit::singleline(&mut self.new_startup_path).desired_width(360.0));
                        if ui.button("📁 Обзор...").clicked() {
                            if let Some(p) = pick_file_dialog("Исполняемые файлы", "*.exe") {
                                if self.new_startup_name.is_empty() {
                                    if let Some(stem) = std::path::Path::new(&p).file_stem() {
                                        self.new_startup_name = stem.to_string_lossy().to_string();
                                    }
                                }
                                self.new_startup_path = p;
                            }
                        }
                    });

                    ui.add_space(6.0);
                    ui.label("Аргументы запуска (необязательно):");
                    ui.text_edit_singleline(&mut self.new_startup_args);

                    ui.add_space(8.0);
                    ui.checkbox(&mut self.new_startup_hklm, "Для всех пользователей (HKLM - требует прав администратора)");

                    if let Some(err) = &self.new_startup_error {
                        ui.add_space(4.0);
                        ui.label(RichText::new(err).color(Color32::from_rgb(239, 68, 68)));
                    }

                    ui.add_space(14.0);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add(egui::Button::new("Отмена").min_size(Vec2::new(85.0, 26.0))).clicked() {
                            self.add_startup_modal_open = false;
                        }
                        let add_btn = egui::Button::new(RichText::new("Добавить").strong().color(Color32::WHITE))
                            .fill(Color32::from_rgb(37, 99, 235))
                            .min_size(Vec2::new(95.0, 26.0));
                        if ui.add(add_btn).clicked() {
                            if self.new_startup_name.trim().is_empty() {
                                self.new_startup_error = Some("Укажите имя программы".to_string());
                            } else if self.new_startup_path.trim().is_empty() {
                                self.new_startup_error = Some("Укажите путь к исполняемому файлу".to_string());
                            } else {
                                if add_startup_item(&self.new_startup_name, &self.new_startup_path, &self.new_startup_args, self.new_startup_hklm) {
                                    self.startup_items = scan_startup_items();
                                    self.status_msg = format!("Программа «{}» добавлена в автозапуск.", self.new_startup_name);
                                    self.add_startup_modal_open = false;
                                } else {
                                    self.new_startup_error = Some("Ошибка добавления в реестр. Возможно, требуются права администратора.".to_string());
                                }
                            }
                        }
                    });
                });
        }

        // 6. Install Tracker Modal
        if self.tracker_modal_open {
            let mut is_open = true;
            egui::Window::new("Установить и следить за программой")
                .open(&mut is_open)
                .collapsible(false)
                .resizable(true)
                .pivot(egui::Align2::CENTER_CENTER)
                .default_pos(ctx.screen_rect().center())
                .show(ctx, |ui| {
                    ui.set_width(580.0);
                    ui.set_height(420.0);

                    match self.tracker_step {
                        TrackerStep::SelectInstaller => {
                            ui.heading("Установка программы с отслеживанием");
                            ui.add_space(6.0);
                            ui.label("PurgeCraft зафиксирует снимок реестра и файловой системы до установки, запустит инсталлятор, а затем определит все добавленные ключи, ветки и папки. Это гарантирует 100% чистое удаление в будущем.");
                            ui.add_space(12.0);

                            ui.label("Путь к инсталлятору (.exe, .msi):");
                            ui.horizontal(|ui| {
                                ui.add(egui::TextEdit::singleline(&mut self.tracker_installer_path).desired_width(420.0));
                                if ui.button("📁 Обзор...").clicked() {
                                    if let Some(p) = pick_file_dialog("Инсталляторы", "*.exe;*.msi") {
                                        if self.tracker_app_name.is_empty() {
                                            if let Some(stem) = std::path::Path::new(&p).file_stem() {
                                                self.tracker_app_name = stem.to_string_lossy().to_string();
                                            }
                                        }
                                        self.tracker_installer_path = p;
                                    }
                                }
                            });

                            ui.add_space(8.0);
                            ui.label("Название программы:");
                            ui.add(egui::TextEdit::singleline(&mut self.tracker_app_name).desired_width(420.0));

                            ui.add_space(16.0);
                            ui.horizontal(|ui| {
                                if ui.button("Отмена").clicked() {
                                    self.tracker_modal_open = false;
                                }
                                let can_start = !self.tracker_installer_path.trim().is_empty() && std::path::Path::new(&self.tracker_installer_path).exists();
                                if ui.add_enabled(can_start, egui::Button::new(RichText::new("▶ Начать снимок и запустить установку").strong().color(Color32::WHITE))).clicked() {
                                    let snap = take_system_snapshot();
                                    self.tracker_snapshot_before = Some(snap);
                                    self.tracker_live_log.clear();
                                    self.tracker_process_finished = false;
                                    match scanner::spawn_install_tracker(self.tracker_installer_path.clone()) {
                                        Ok((rx, stop_tx, child)) => {
                                            self.tracker_rx = Some(rx);
                                            self.tracker_stop_tx = Some(stop_tx);
                                            self.tracker_child = Some(child);
                                            self.tracker_step = TrackerStep::Installing;
                                        }
                                        Err(e) => {
                                            self.status_msg = format!("Ошибка запуска: {}", e);
                                        }
                                    }
                                }
                            });
                        }
                        TrackerStep::Installing => {
                            let mut finish_tracking = false;
                            let mut abort_tracking = false;

                            ui.horizontal(|ui| {
                                ui.label(RichText::new("📦").size(22.0));
                                ui.heading(RichText::new("Установить и следить за программой").size(15.0).strong());
                            });
                            ui.add_space(6.0);
                            ui.label(RichText::new("Прогресс установки:").strong().size(12.5));
                            ui.add_space(4.0);

                            // Dark Terminal Log Box (exact 1:1 match with Screenshot 2)
                            let log_bg = if self.dark_mode { Color32::from_rgb(15, 17, 23) } else { Color32::from_rgb(15, 23, 42) };
                            let border_color = if self.dark_mode { Color32::from_rgb(60, 64, 75) } else { Color32::from_rgb(51, 65, 85) };

                            egui::Frame::none()
                                .fill(log_bg)
                                .stroke(Stroke::new(1.0_f32, border_color))
                                .inner_margin(8.0)
                                .show(ui, |ui| {
                                    ui.set_min_width(540.0);
                                    ui.set_height(250.0);

                                    egui::ScrollArea::vertical()
                                        .stick_to_bottom(true)
                                        .max_height(245.0)
                                        .show(ui, |ui| {
                                            for entry in &self.tracker_live_log {
                                                ui.horizontal(|ui| {
                                                    // Col 1: Time (width 70)
                                                    ui.allocate_ui_with_layout(Vec2::new(70.0, 16.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                                                        ui.label(RichText::new(&entry.time).monospace().size(11.5).color(Color32::from_rgb(203, 213, 225)));
                                                    });

                                                    // Col 2: Action (width 110)
                                                    ui.allocate_ui_with_layout(Vec2::new(110.0, 16.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                                                        if !entry.action.is_empty() {
                                                            let act_color = match entry.action.as_str() {
                                                                "Создана папка" => Color32::from_rgb(234, 179, 8),
                                                                "Создан ключ реестра" | "Запись реестра" => Color32::from_rgb(168, 85, 247),
                                                                _ => Color32::from_rgb(56, 189, 248),
                                                            };
                                                            ui.label(RichText::new(&entry.action).monospace().size(11.5).color(act_color));
                                                        }
                                                    });

                                                    // Col 3: Path / Message
                                                    let path_color = if entry.action.is_empty() {
                                                        Color32::from_rgb(241, 245, 249)
                                                    } else {
                                                        Color32::from_rgb(226, 232, 240)
                                                    };
                                                    ui.label(RichText::new(&entry.path).monospace().size(11.5).color(path_color));
                                                });
                                            }
                                        });
                                });

                            ui.add_space(10.0);
                            ui.horizontal(|ui| {
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    let stop_btn = egui::Button::new(RichText::new("Стоп").size(12.5))
                                        .min_size(Vec2::new(75.0, 26.0));
                                    if ui.add(stop_btn).on_hover_text("Остановить монитор установки").clicked() {
                                        abort_tracking = true;
                                    }

                                    let finish_btn = egui::Button::new(
                                        RichText::new("Установить и следить").size(12.5).strong().color(Color32::WHITE)
                                    )
                                    .fill(if self.dark_mode { Color32::from_rgb(45, 52, 65) } else { Color32::from_rgb(30, 41, 59) })
                                    .stroke(Stroke::new(1.0_f32, Color32::from_rgb(75, 85, 99)))
                                    .min_size(Vec2::new(175.0, 26.0));

                                    if ui.add(finish_btn).on_hover_text("Завершить отслеживание и зафиксировать созданные следы").clicked() {
                                        finish_tracking = true;
                                    }
                                });
                            });

                            if abort_tracking {
                                if let Some(stop) = self.tracker_stop_tx.take() {
                                    let _ = stop.send(());
                                }
                                self.tracker_rx = None;
                                self.tracker_child = None;
                                self.tracker_modal_open = false;
                                self.tracker_step = TrackerStep::SelectInstaller;
                            }

                            if finish_tracking {
                                if let Some(stop) = self.tracker_stop_tx.take() {
                                    let _ = stop.send(());
                                }
                                self.tracker_rx = None;
                                self.tracker_child = None;
                                if let Some(before) = &self.tracker_snapshot_before {
                                    let traces = diff_system_snapshot(before, &self.tracker_app_name);
                                    self.tracker_traces = traces;
                                    self.tracker_step = TrackerStep::ReviewTraces;
                                }
                            }
                        }
                        TrackerStep::ReviewTraces => {
                            ui.heading(format!("Следы установки «{}»", self.tracker_app_name));
                            ui.add_space(4.0);
                            ui.label(format!("Зафиксировано новых элементов в системе: {}", self.tracker_traces.len()));
                            ui.separator();

                            egui::ScrollArea::vertical().max_height(240.0).show(ui, |ui| {
                                if self.tracker_traces.is_empty() {
                                    ui.label("Новых файлов или ключей реестра не зафиксировано.");
                                } else {
                                    for t in &self.tracker_traces {
                                        let icon = if t.trace_type == "registry_key" { "🗄" } else { "📁" };
                                        ui.label(format!("{} {}", icon, t.path));
                                    }
                                }
                            });

                            ui.separator();
                            ui.add_space(8.0);
                            ui.horizontal(|ui| {
                                if ui.button("Отмена").clicked() {
                                    self.tracker_modal_open = false;
                                }
                                if ui.button(RichText::new("💾 Сохранить в базу отслеженных программ").strong().color(Color32::WHITE)).clicked() {
                                    let now = chrono::Local::now().format("%Y-%m-%d").to_string();
                                    let traced = TracedApp {
                                        id: format!("traced_{}_{}", self.tracker_app_name, self.tracker_traces.len()),
                                        name: self.tracker_app_name.clone(),
                                        install_date: now,
                                        installer_path: self.tracker_installer_path.clone(),
                                        traces: self.tracker_traces.clone(),
                                    };
                                    self.traced_apps.push(traced);
                                    save_traced_apps(&self.traced_apps);
                                    self.status_msg = format!("Программа «{}» успешно зарегистрирована как отслеженная ({} следов).", self.tracker_app_name, self.tracker_traces.len());
                                    self.tracker_modal_open = false;
                                    self.refresh_data();
                                }
                            });
                        }
                    }
                });
            if !is_open {
                if let Some(stop) = self.tracker_stop_tx.take() {
                    let _ = stop.send(());
                }
                self.tracker_rx = None;
                self.tracker_child = None;
                self.tracker_modal_open = false;
                self.tracker_step = TrackerStep::SelectInstaller;
            }
        }

        // 7. Settings Modal
        if self.settings_open {
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
                                    if self.dark_mode { Color32::from_rgb(45, 48, 56) } else { Color32::from_rgb(224, 242, 254) },
                                    Stroke::new(1.0_f32, if self.dark_mode { Color32::from_rgb(96, 165, 250) } else { Color32::from_rgb(2, 132, 199) }),
                                    if self.dark_mode { Color32::WHITE } else { Color32::from_rgb(2, 132, 199) },
                                )
                            } else {
                                (
                                    if self.dark_mode { Color32::from_rgb(24, 24, 28) } else { Color32::from_rgb(241, 245, 249) },
                                    Stroke::NONE,
                                    if self.dark_mode { Color32::from_rgb(161, 161, 170) } else { Color32::from_rgb(71, 85, 105) },
                                )
                            };
                            let btn = egui::Button::new(
                                RichText::new(format!("{} {}", icon, label)).strong().color(text_color)
                            )
                            .fill(fill)
                            .stroke(stroke)
                            .min_size(Vec2::new(145.0, 32.0));
                            ui.add(btn)
                        };

                        if tab_btn(ui, self.settings_tab == SettingsTab::General, "📁", "Основные").clicked() {
                            self.settings_tab = SettingsTab::General;
                        }
                        if tab_btn(ui, self.settings_tab == SettingsTab::Interface, "🖥", "Интерфейс").clicked() {
                            self.settings_tab = SettingsTab::Interface;
                        }
                        if tab_btn(ui, self.settings_tab == SettingsTab::Uninstaller, "📦", "Деинсталлятор").clicked() {
                            self.settings_tab = SettingsTab::Uninstaller;
                        }
                    });

                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(8.0);

                    // 2. Tab Contents
                    egui::ScrollArea::vertical().max_height(280.0).show(ui, |ui| {
                        match self.settings_tab {
                            SettingsTab::General => {
                                ui.checkbox(
                                    &mut self.temp_settings.skip_uac_task,
                                    "Упростить запуск программы используя планировщик задач (избежать запроса Контроля учетных записей на разрешение вносить изменения)"
                                ).on_hover_text("Позволяет запускать PurgeCraft без всплывающего окна UAC Windows");

                                ui.add_space(6.0);
                                ui.checkbox(
                                    &mut self.temp_settings.explorer_context_menu,
                                    "Добавлять пункт 'Установить и следить' для установочных файлов (exe/msi) в контекстное меню Проводника Windows"
                                ).on_hover_text("Интегрирует пункт «Установить и следить (PurgeCraft)» в меню правой кнопки мыши");

                                ui.add_space(6.0);
                                ui.checkbox(
                                    &mut self.temp_settings.check_updates,
                                    "Проверять наличие новой версии при старте программы"
                                );

                                ui.horizontal(|ui| {
                                    ui.add_space(20.0);
                                    ui.add_enabled(
                                        self.temp_settings.check_updates,
                                        egui::Checkbox::new(&mut self.temp_settings.check_beta, "Проверять также наличие бета-версий")
                                    );
                                });

                                ui.horizontal(|ui| {
                                    ui.add_space(20.0);
                                    ui.label("Репозиторий GitHub:");
                                    ui.add(egui::TextEdit::singleline(&mut self.temp_settings.github_repo).desired_width(160.0));
                                    if ui.button("🔍 Проверить сейчас").clicked() {
                                        autoupdater::check_for_updates_async(
                                            self.temp_settings.github_repo.clone(),
                                            env!("CARGO_PKG_VERSION").to_string(),
                                            self.update_tx.clone(),
                                        );
                                        self.update_modal_open = true;
                                    }
                                });

                                ui.add_space(6.0);
                                ui.checkbox(
                                    &mut self.temp_settings.show_in_control_panel,
                                    "Отображать значок PurgeCraft в Панели управления"
                                );

                                ui.add_space(6.0);
                                ui.checkbox(
                                    &mut self.temp_settings.use_total_commander,
                                    "Переходить в папку программы используя Total Commander"
                                );

                                ui.add_space(10.0);
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new("Язык интерфейса:").strong());
                                    egui::ComboBox::from_id_salt("settings_lang")
                                        .selected_text(&self.temp_settings.language)
                                        .width(180.0)
                                        .show_ui(ui, |ui| {
                                            ui.selectable_value(&mut self.temp_settings.language, "Russian (Русский)".to_string(), "Russian (Русский)");
                                            ui.selectable_value(&mut self.temp_settings.language, "English".to_string(), "English");
                                        });
                                });
                            }
                            SettingsTab::Interface => {
                                ui.checkbox(
                                    &mut self.temp_settings.show_toolbar_labels,
                                    "Показывать подписи к кнопкам в тулбаре"
                                );

                                ui.add_space(10.0);
                                ui.label(RichText::new("Список программ").heading().size(14.0));
                                ui.checkbox(&mut self.temp_settings.compact_list, "Компактный список программ");
                                ui.checkbox(&mut self.temp_settings.mark_32bit, "Помечать 32-битные приложения");

                                ui.add_space(12.0);
                                ui.label(RichText::new("Цветовая схема").heading().size(14.0));
                                let theme_label = match self.temp_settings.theme_mode.as_str() {
                                    "system" => "Соответствовать цветовой схеме Windows",
                                    "light" => "Светлая",
                                    _ => "Темная",
                                };
                                egui::ComboBox::from_id_salt("settings_theme")
                                    .selected_text(theme_label)
                                    .width(260.0)
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(&mut self.temp_settings.theme_mode, "system".to_string(), "Соответствовать цветовой схеме Windows");
                                        ui.selectable_value(&mut self.temp_settings.theme_mode, "dark".to_string(), "Темная");
                                        ui.selectable_value(&mut self.temp_settings.theme_mode, "light".to_string(), "Светлая");
                                    });

                                ui.add_space(12.0);
                                ui.label(RichText::new("Масштаб и размер текста").heading().size(14.0));
                                ui.horizontal(|ui| {
                                    ui.label("Размер текста интерфейса:");
                                    let scale_pct = (self.temp_settings.text_scale * 100.0).round() as i32;
                                    egui::ComboBox::from_id_salt("settings_text_scale")
                                        .selected_text(format!("{}%", scale_pct))
                                        .width(180.0)
                                        .show_ui(ui, |ui| {
                                            if ui.selectable_value(&mut self.temp_settings.text_scale, 1.0, "100% (Стандартный)").clicked() {
                                                ctx.set_zoom_factor(1.0);
                                            }
                                            if ui.selectable_value(&mut self.temp_settings.text_scale, 1.15, "115% (Рекомендуемый)").clicked() {
                                                ctx.set_zoom_factor(1.15);
                                            }
                                            if ui.selectable_value(&mut self.temp_settings.text_scale, 1.25, "125% (Увеличенный)").clicked() {
                                                ctx.set_zoom_factor(1.25);
                                            }
                                            if ui.selectable_value(&mut self.temp_settings.text_scale, 1.35, "135% (Крупный)").clicked() {
                                                ctx.set_zoom_factor(1.35);
                                            }
                                            if ui.selectable_value(&mut self.temp_settings.text_scale, 1.50, "150% (Очень крупный)").clicked() {
                                                ctx.set_zoom_factor(1.50);
                                            }
                                        });
                                });

                                let mut scale_val = self.temp_settings.text_scale;
                                if ui.add(egui::Slider::new(&mut scale_val, 0.85..=1.75).step_by(0.05).text("Плавная настройка")).changed() {
                                    self.temp_settings.text_scale = scale_val;
                                    ctx.set_zoom_factor(scale_val);
                                }
                            }
                            SettingsTab::Uninstaller => {
                                ui.label(RichText::new("Древовидное отображение обновлений программ (дочерние элементы):").strong());
                                ui.horizontal(|ui| {
                                    ui.label("Отображать обновления программ");
                                    let upd_label = match self.temp_settings.tree_updates_mode.as_str() {
                                        "expanded" => "Развернутыми",
                                        "hidden" => "Не отображать",
                                        _ => "Изначально свернутыми",
                                    };
                                    egui::ComboBox::from_id_salt("settings_updates")
                                        .selected_text(upd_label)
                                        .width(180.0)
                                        .show_ui(ui, |ui| {
                                            ui.selectable_value(&mut self.temp_settings.tree_updates_mode, "collapsed".to_string(), "Изначально свернутыми");
                                            ui.selectable_value(&mut self.temp_settings.tree_updates_mode, "expanded".to_string(), "Развернутыми");
                                            ui.selectable_value(&mut self.temp_settings.tree_updates_mode, "hidden".to_string(), "Не отображать");
                                        });
                                    ui.label("🛈").on_hover_text("Определяет группировку компонентов софта в основном списке");
                                });

                                ui.add_space(10.0);
                                ui.label(RichText::new("Недавно установленные программы:").strong());
                                ui.checkbox(&mut self.temp_settings.highlight_recent, "Подсвечивать недавно установленные программы");
                                ui.checkbox(&mut self.temp_settings.popup_recent, "Показывать всплывающее окно с недавно установленными программами");
                                ui.checkbox(&mut self.temp_settings.show_recent_in_toolbar, "Отображать в дополнительной панели инструментов");

                                ui.add_space(10.0);
                                ui.label(RichText::new("Мастер удаления").strong());
                                ui.label("Запуск Мастера удаления после выполнения штатной деинсталляции:");
                                let wiz_label = match self.temp_settings.wizard_auto_start.as_str() {
                                    "ask" => "Спрашивать перед запуском",
                                    "never" => "Никогда",
                                    _ => "Автоматически",
                                };
                                egui::ComboBox::from_id_salt("settings_wiz_mode")
                                    .selected_text(wiz_label)
                                    .width(220.0)
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(&mut self.temp_settings.wizard_auto_start, "auto".to_string(), "Автоматически");
                                        ui.selectable_value(&mut self.temp_settings.wizard_auto_start, "ask".to_string(), "Спрашивать перед запуском");
                                        ui.selectable_value(&mut self.temp_settings.wizard_auto_start, "never".to_string(), "Никогда");
                                    });
                            }
                        }
                    });

                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(6.0);

                    // 3. Bottom Button Row
                    ui.horizontal(|ui| {
                        if ui.button("❓ Помощь").on_hover_text("Открыть справку по настройкам").clicked() {
                            let _ = open::that("https://github.com/dimal/PurgeCraft");
                        }

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("Отмена").clicked() {
                                do_cancel = true;
                            }

                            let ok_btn = egui::Button::new(RichText::new("ОК").strong().color(Color32::WHITE))
                                .fill(if self.dark_mode { Color32::from_rgb(37, 99, 235) } else { Color32::from_rgb(2, 132, 199) })
                                .min_size(Vec2::new(75.0, 24.0));
                            if ui.add(ok_btn).clicked() {
                                do_save = true;
                            }
                        });
                    });
                });

            if !is_open || do_cancel {
                ctx.set_zoom_factor(self.settings.text_scale);
                self.temp_settings = self.settings.clone();
                self.settings_open = false;
            }

            if do_save {
                self.settings = self.temp_settings.clone();
                save_settings(&self.settings);
                ctx.set_zoom_factor(self.settings.text_scale);
                self.dark_mode = match self.settings.theme_mode.as_str() {
                    "light" => false,
                    "system" => is_windows_dark_mode(),
                    _ => true,
                };
                self.status_msg = "Настройки успешно сохранены.".to_string();
                self.settings_open = false;
            }
        }

        // 8. GPU & DirectX Shader Cache Cleaner Modal
        if self.shader_modal_open {
            let mut is_open = true;
            let mut do_clean = false;
            let mut do_rescan = false;
            let mut do_close = false;

            let total_selected_bytes: u64 = self.shader_locations
                .iter()
                .filter(|l| l.selected)
                .map(|l| l.size_bytes)
                .sum();
            let total_selected_files: usize = self.shader_locations
                .iter()
                .filter(|l| l.selected)
                .map(|l| l.file_count)
                .sum();

            egui::Window::new("🎮 Очиститель кэша шейдеров GPU и DirectX")
                .open(&mut is_open)
                .collapsible(false)
                .resizable(true)
                .pivot(egui::Align2::CENTER_CENTER)
                .default_pos(ctx.screen_rect().center())
                .show(ctx, |ui| {
                    ui.set_width(640.0);
                    ui.set_height(460.0);

                    ui.horizontal(|ui| {
                        ui.heading(RichText::new("🎮 Глубокая очистка кэшей GPU").strong());
                    });
                    ui.add_space(4.0);
                    ui.label("Игры и графические приложения компилируют шейдеры в системные папки драйверов (DirectX, NVIDIA, AMD). Со временем эти кэши разрастаются до десятков гигабайтов и не удаляются автоматически.");
                    ui.add_space(8.0);

                    if let Some(msg) = &self.shader_clean_msg {
                        egui::Frame::none()
                            .fill(if self.dark_mode { Color32::from_rgb(20, 83, 45) } else { Color32::from_rgb(220, 252, 231) })
                            .rounding(4.0)
                            .inner_margin(8.0)
                            .show(ui, |ui| {
                                ui.label(RichText::new(msg).strong().color(Color32::from_rgb(22, 163, 74)));
                            });
                        ui.add_space(8.0);
                    }

                    // Selection buttons
                    ui.horizontal(|ui| {
                        if ui.button("☑ Выбрать все").clicked() {
                            for loc in &mut self.shader_locations {
                                loc.selected = true;
                            }
                        }
                        if ui.button("☐ Снять выбор").clicked() {
                            for loc in &mut self.shader_locations {
                                loc.selected = false;
                            }
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("🔄 Пересканировать").clicked() {
                                do_rescan = true;
                            }
                        });
                    });
                    ui.add_space(6.0);

                    // List of locations
                    let list_bg = if self.dark_mode { Color32::from_rgb(18, 18, 20) } else { Color32::WHITE };
                    egui::Frame::none()
                        .fill(list_bg)
                        .stroke(Stroke::new(1.0_f32, if self.dark_mode { Color32::from_rgb(39, 39, 42) } else { Color32::from_rgb(226, 232, 240) }))
                        .rounding(4.0)
                        .inner_margin(8.0)
                        .show(ui, |ui| {
                            egui::ScrollArea::vertical().max_height(240.0).show(ui, |ui| {
                                if self.shader_locations.is_empty() {
                                    ui.vertical_centered(|ui| {
                                        ui.add_space(30.0);
                                        ui.label(RichText::new("Кэшей шейдеров не обнаружено или они уже пусты.").color(Color32::from_rgb(148, 163, 184)));
                                    });
                                } else {
                                    for loc in &mut self.shader_locations {
                                        ui.horizontal(|ui| {
                                            ui.checkbox(&mut loc.selected, "");
                                            let vendor_color = match loc.vendor.as_str() {
                                                "NVIDIA" => Color32::from_rgb(34, 197, 94),
                                                "AMD" => Color32::from_rgb(239, 68, 68),
                                                _ => Color32::from_rgb(59, 130, 246),
                                            };
                                            ui.label(RichText::new(format!("[{}]", loc.vendor)).strong().color(vendor_color));
                                            ui.label(RichText::new(&loc.name).strong());
                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                ui.label(RichText::new(format_size(loc.size_bytes)).strong());
                                                ui.label(format!("({} файлов)", loc.file_count));
                                            });
                                        });
                                        ui.label(RichText::new(&loc.path).small().color(Color32::from_rgb(113, 113, 122)));
                                        ui.separator();
                                    }
                                }
                            });
                        });

                    ui.add_space(10.0);
                    ui.separator();
                    ui.add_space(8.0);

                    // Bottom action panel
                    ui.horizontal(|ui| {
                        let info_text = format!("Выбрано для очистки: {} ({} файлов)", format_size(total_selected_bytes), total_selected_files);
                        ui.label(RichText::new(info_text).strong().size(13.0));

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("Закрыть").clicked() {
                                do_close = true;
                            }
                            let can_clean = total_selected_bytes > 0 && !self.shader_cleaning;
                            let clean_btn = egui::Button::new(
                                RichText::new(format!("🧹 Очистить ({})", format_size(total_selected_bytes)))
                                    .strong()
                                    .color(Color32::WHITE)
                            )
                            .fill(if can_clean { Color32::from_rgb(147, 51, 234) } else { Color32::from_rgb(100, 100, 100) })
                            .min_size(Vec2::new(160.0, 28.0));

                            if ui.add_enabled(can_clean, clean_btn).clicked() {
                                do_clean = true;
                            }
                        });
                    });
                });

            if do_close || !is_open {
                self.shader_modal_open = false;
            }

            if do_rescan {
                self.shader_locations = scan_shader_caches();
                self.shader_clean_msg = None;
            }

            if do_clean {
                let (deleted_files, deleted_bytes) = clear_shader_cache_targets(&self.shader_locations);
                self.shader_clean_msg = Some(format!("Очистка завершена! Освобождено {} (удалено {} файлов).", format_size(deleted_bytes), deleted_files));
                self.status_msg = format!("Кэш шейдеров очищен. Освобождено {}.", format_size(deleted_bytes));
                self.shader_locations = scan_shader_caches();
            }
        }

        // ====================================================================
        // 9. Environment Variable PATH Sanitizer Modal
        // ====================================================================
        if self.path_modal_open {
            let mut is_open = true;
            let mut do_clean = false;
            let mut do_rescan = false;
            let mut do_close = false;

            let total_selected = self.path_entries.iter().filter(|e| e.selected).count();
            let total_broken = self.path_entries.iter().filter(|e| !e.exists).count();
            let total_duplicates = self.path_entries.iter().filter(|e| e.is_duplicate).count();

            egui::Window::new("🗄 Санитайзер системного и пользовательского PATH")
                .open(&mut is_open)
                .collapsible(false)
                .resizable(true)
                .pivot(egui::Align2::CENTER_CENTER)
                .default_pos(ctx.screen_rect().center())
                .show(ctx, |ui| {
                    ui.set_width(740.0);
                    ui.set_height(480.0);

                    ui.heading(RichText::new("🗄 Санитайзер переменных среды PATH").strong());
                    ui.add_space(4.0);
                    ui.label("Удаленные приложения часто оставляют «мертвые» ссылки на несуществующие каталоги в переменной PATH. Это замедляет работу командной строки и может вызывать конфликты библиотек.");
                    ui.add_space(6.0);

                    if let Some(msg) = &self.path_msg {
                        egui::Frame::none()
                            .fill(if self.dark_mode { Color32::from_rgb(20, 83, 45) } else { Color32::from_rgb(220, 252, 231) })
                            .rounding(4.0)
                            .inner_margin(8.0)
                            .show(ui, |ui| {
                                ui.label(RichText::new(msg).strong().color(Color32::from_rgb(22, 163, 74)));
                            });
                        ui.add_space(6.0);
                    }

                    // Action buttons
                    ui.horizontal(|ui| {
                        if ui.button("☑ Выбрать битые и дубликаты").clicked() {
                            for e in &mut self.path_entries {
                                e.selected = !e.exists || e.is_duplicate;
                            }
                        }
                        if ui.button("☑ Выбрать все").clicked() {
                            for e in &mut self.path_entries {
                                e.selected = true;
                            }
                        }
                        if ui.button("☐ Снять выбор").clicked() {
                            for e in &mut self.path_entries {
                                e.selected = false;
                            }
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("🔄 Пересканировать").clicked() {
                                do_rescan = true;
                            }
                            ui.label(format!("Битых: {} | Дубликатов: {}", total_broken, total_duplicates));
                        });
                    });
                    ui.add_space(6.0);

                    let list_bg = if self.dark_mode { Color32::from_rgb(18, 18, 20) } else { Color32::WHITE };
                    egui::Frame::none()
                        .fill(list_bg)
                        .stroke(Stroke::new(1.0_f32, if self.dark_mode { Color32::from_rgb(39, 39, 42) } else { Color32::from_rgb(226, 232, 240) }))
                        .rounding(4.0)
                        .inner_margin(8.0)
                        .show(ui, |ui| {
                            egui::ScrollArea::vertical().max_height(270.0).show(ui, |ui| {
                                if self.path_entries.is_empty() {
                                    ui.vertical_centered(|ui| {
                                        ui.add_space(30.0);
                                        ui.label(RichText::new("Записей в PATH не обнаружено.").color(Color32::from_rgb(148, 163, 184)));
                                    });
                                } else {
                                    for entry in &mut self.path_entries {
                                        ui.horizontal(|ui| {
                                            ui.checkbox(&mut entry.selected, "");

                                            // Hive badge
                                            let hive_label = if entry.is_system { "[Система]" } else { "[Пользователь]" };
                                            let hive_color = if entry.is_system { Color32::from_rgb(168, 85, 247) } else { Color32::from_rgb(59, 130, 246) };
                                            ui.label(RichText::new(hive_label).small().strong().color(hive_color));

                                            // Status badge
                                            if !entry.exists {
                                                egui::Frame::none()
                                                    .fill(if self.dark_mode { Color32::from_rgb(69, 10, 10) } else { Color32::from_rgb(254, 226, 226) })
                                                    .rounding(3.0)
                                                    .inner_margin(egui::Margin::symmetric(4.0, 1.0))
                                                    .show(ui, |ui| {
                                                        ui.label(RichText::new("🔴 МЕРТВЫЙ ПУТЬ").small().strong().color(Color32::from_rgb(239, 68, 68)));
                                                    });
                                            } else if entry.is_duplicate {
                                                egui::Frame::none()
                                                    .fill(if self.dark_mode { Color32::from_rgb(66, 32, 6) } else { Color32::from_rgb(254, 243, 199) })
                                                    .rounding(3.0)
                                                    .inner_margin(egui::Margin::symmetric(4.0, 1.0))
                                                    .show(ui, |ui| {
                                                        ui.label(RichText::new("🟡 ДУБЛИКАТ").small().strong().color(Color32::from_rgb(217, 119, 6)));
                                                    });
                                            } else {
                                                ui.label(RichText::new("🟢 Существует").small().color(Color32::from_rgb(34, 197, 94)));
                                            }

                                            ui.label(RichText::new(&entry.path).monospace().size(12.0));
                                        });
                                        ui.separator();
                                    }
                                }
                            });
                        });

                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(6.0);

                    ui.horizontal(|ui| {
                        ui.label(RichText::new(format!("Выбрано для удаления: {} записей", total_selected)).strong());

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("Закрыть").clicked() {
                                do_close = true;
                            }
                            let can_clean = total_selected > 0;
                            let clean_btn = egui::Button::new(
                                RichText::new(format!("🧹 Очистить выбранные ({})", total_selected))
                                    .strong()
                                    .color(Color32::WHITE)
                            )
                            .fill(if can_clean { Color32::from_rgb(220, 38, 38) } else { Color32::from_rgb(100, 100, 100) })
                            .min_size(Vec2::new(180.0, 28.0));

                            if ui.add_enabled(can_clean, clean_btn).clicked() {
                                do_clean = true;
                            }
                        });
                    });
                });

            if do_close || !is_open {
                self.path_modal_open = false;
            }

            if do_rescan {
                self.path_entries = scan_path_entries();
                self.path_msg = None;
            }

            if do_clean {
                match clean_path_entries(&self.path_entries) {
                    Ok((user_del, sys_del)) => {
                        self.path_msg = Some(format!("Успешно очищено {} записей (Пользовательских: {}, Системных: {}). Переменные обновлены!", user_del + sys_del, user_del, sys_del));
                        self.status_msg = format!("PATH очищен: удалено {} битых путей.", user_del + sys_del);
                    }
                    Err(err) => {
                        self.path_msg = Some(err);
                    }
                }
                self.path_entries = scan_path_entries();
            }
        }

        // ====================================================================
        // 10. Portable Applications Scanner Modal
        // ====================================================================
        if self.portable_modal_open {
            let mut is_open = true;
            let mut do_rescan = false;
            let mut do_close = false;
            let mut do_pick_folder = false;
            let mut app_to_delete: Option<PortableApp> = None;
            let mut app_to_launch: Option<String> = None;
            let mut app_to_open_folder: Option<String> = None;

            let total_portable_bytes: u64 = self.portable_apps.iter().map(|a| a.size_bytes).sum();

            egui::Window::new("📦 Сканер Portable-программ")
                .open(&mut is_open)
                .collapsible(false)
                .resizable(true)
                .pivot(egui::Align2::CENTER_CENTER)
                .default_pos(ctx.screen_rect().center())
                .show(ctx, |ui| {
                    ui.set_width(740.0);
                    ui.set_height(480.0);

                    ui.heading(RichText::new("📦 Сканер Portable-софта").strong());
                    ui.add_space(4.0);
                    ui.label("Обнаруживает автономные программы и утилиты, которые запускаются без установки (на Рабочем столе, в Загрузках или в пользовательских папках).");
                    ui.add_space(6.0);

                    if let Some(msg) = &self.portable_msg {
                        egui::Frame::none()
                            .fill(if self.dark_mode { Color32::from_rgb(20, 83, 45) } else { Color32::from_rgb(220, 252, 231) })
                            .rounding(4.0)
                            .inner_margin(8.0)
                            .show(ui, |ui| {
                                ui.label(RichText::new(msg).strong().color(Color32::from_rgb(22, 163, 74)));
                            });
                        ui.add_space(6.0);
                    }

                    // Action buttons
                    ui.horizontal(|ui| {
                        if ui.button("📂 Добавить папку для сканирования...").clicked() {
                            do_pick_folder = true;
                        }
                        if ui.button("🔄 Пересканировать").clicked() {
                            do_rescan = true;
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(format!("Найдено: {} программ ({})", self.portable_apps.len(), format_size(total_portable_bytes)));
                        });
                    });
                    ui.add_space(6.0);

                    let list_bg = if self.dark_mode { Color32::from_rgb(18, 18, 20) } else { Color32::WHITE };
                    egui::Frame::none()
                        .fill(list_bg)
                        .stroke(Stroke::new(1.0_f32, if self.dark_mode { Color32::from_rgb(39, 39, 42) } else { Color32::from_rgb(226, 232, 240) }))
                        .rounding(4.0)
                        .inner_margin(8.0)
                        .show(ui, |ui| {
                            egui::ScrollArea::vertical().max_height(270.0).show(ui, |ui| {
                                if self.portable_apps.is_empty() {
                                    ui.vertical_centered(|ui| {
                                        ui.add_space(30.0);
                                        ui.label(RichText::new("Portable-программ в стандартных каталогах не найдено.").color(Color32::from_rgb(148, 163, 184)));
                                    });
                                } else {
                                    for app in &self.portable_apps {
                                        ui.horizontal(|ui| {
                                            ui.label(RichText::new("📦").size(18.0));
                                            ui.vertical(|ui| {
                                                ui.horizontal(|ui| {
                                                    ui.label(RichText::new(&app.name).strong());
                                                    ui.label(RichText::new(format!("({})", format_size(app.size_bytes))).color(Color32::from_rgb(34, 197, 94)));
                                                    ui.label(RichText::new(format!("Файлов: {} | Изменен: {}", app.file_count, app.modified_date)).small().color(Color32::from_rgb(148, 163, 184)));
                                                });
                                                ui.label(RichText::new(&app.exe_path).small().color(Color32::from_rgb(113, 113, 122)));
                                            });

                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                if ui.button("🗑 Удалить").clicked() {
                                                    app_to_delete = Some(app.clone());
                                                }
                                                if ui.button("📁 Папка").clicked() {
                                                    app_to_open_folder = Some(app.folder_path.clone());
                                                }
                                                if ui.button("▶ Запустить").clicked() {
                                                    app_to_launch = Some(app.exe_path.clone());
                                                }
                                            });
                                        });
                                        ui.separator();
                                    }
                                }
                            });
                        });

                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(6.0);

                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Portable-софт удаляется без следов в системных реестрах.").small());
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("Закрыть").clicked() {
                                do_close = true;
                            }
                        });
                    });
                });

            if do_close || !is_open {
                self.portable_modal_open = false;
            }

            if do_pick_folder {
                if let Some(folder) = pick_folder_dialog() {
                    let scanned = scan_portable_apps(&[folder], &self.apps);
                    self.portable_apps = scanned;
                }
            }

            if do_rescan {
                self.portable_apps = scan_portable_apps(&[], &self.apps);
                self.portable_msg = None;
            }

            if let Some(exe) = app_to_launch {
                let _ = open::that(&exe);
            }

            if let Some(folder) = app_to_open_folder {
                open_system_folder(&folder, self.settings.use_total_commander);
            }

            if let Some(app) = app_to_delete {
                match delete_portable_app(&app) {
                    Ok(bytes) => {
                        let id = format!("portable_{}", app.exe_path.to_lowercase());
                        self.recently_uninstalled.insert(id);
                        self.recently_uninstalled.insert(app.name.to_lowercase());
                        self.apps.retain(|a| !self.recently_uninstalled.contains(&a.id.to_lowercase()) && !self.recently_uninstalled.contains(&a.name.to_lowercase()));
                        self.portable_msg = Some(format!("Программа '{}' успешно удалена! Освобождено {}.", app.name, format_size(bytes)));
                        self.status_msg = format!("Удалена portable-программа {}.", app.name);
                        self.portable_apps = scan_portable_apps(&[], &self.apps);
                    }
                    Err(e) => {
                        self.portable_msg = Some(format!("Ошибка удаления: {}", e));
                    }
                }
            }
        }

        // ====================================================================
        // 11. Duplicate Runtime Detector Modal
        // ====================================================================
        if self.runtime_modal_open {
            let mut is_open = true;
            let mut do_close = false;
            let mut runtime_to_uninstall: Option<String> = None;

            egui::Window::new("🧬 Анализатор дубликатов версий (Visual C++, Java, .NET)")
                .open(&mut is_open)
                .collapsible(false)
                .resizable(true)
                .pivot(egui::Align2::CENTER_CENTER)
                .default_pos(ctx.screen_rect().center())
                .show(ctx, |ui| {
                    ui.set_width(760.0);
                    ui.set_height(480.0);

                    ui.heading(RichText::new("🧬 Анализатор дубликатов системных сред выполнения").strong());
                    ui.add_space(4.0);
                    ui.label("Игры и приложения часто устанавливают избыточные и устаревшие сборки Microsoft Visual C++, Java и .NET. Здесь вы можете найти дубликаты и безопасно удалить их.");
                    ui.add_space(6.0);

                    if let Some(msg) = &self.runtime_msg {
                        egui::Frame::none()
                            .fill(if self.dark_mode { Color32::from_rgb(20, 83, 45) } else { Color32::from_rgb(220, 252, 231) })
                            .rounding(4.0)
                            .inner_margin(8.0)
                            .show(ui, |ui| {
                                ui.label(RichText::new(msg).strong().color(Color32::from_rgb(22, 163, 74)));
                            });
                        ui.add_space(6.0);
                    }

                    let list_bg = if self.dark_mode { Color32::from_rgb(18, 18, 20) } else { Color32::WHITE };
                    egui::Frame::none()
                        .fill(list_bg)
                        .stroke(Stroke::new(1.0_f32, if self.dark_mode { Color32::from_rgb(39, 39, 42) } else { Color32::from_rgb(226, 232, 240) }))
                        .rounding(4.0)
                        .inner_margin(8.0)
                        .show(ui, |ui| {
                            egui::ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
                                if self.runtime_groups.is_empty() {
                                    ui.vertical_centered(|ui| {
                                        ui.add_space(30.0);
                                        ui.label(RichText::new("Установленных сред выполнения не обнаружено.").color(Color32::from_rgb(148, 163, 184)));
                                    });
                                } else {
                                    for group in &self.runtime_groups {
                                        ui.horizontal(|ui| {
                                            ui.heading(RichText::new(&group.family_name).size(15.0).strong());
                                            if group.duplicate_count > 0 {
                                                ui.label(RichText::new(format!("⚠ Дубликатов: {}", group.duplicate_count)).color(Color32::from_rgb(234, 179, 8)).strong());
                                            }
                                            if group.has_deprecated {
                                                ui.label(RichText::new("🛑 Есть устаревшие (EOL)").color(Color32::from_rgb(239, 68, 68)).strong());
                                            }
                                        });
                                        ui.add_space(2.0);

                                        for item in &group.items {
                                            ui.horizontal(|ui| {
                                                ui.label(RichText::new(format!("[{}]", item.arch)).strong().color(Color32::from_rgb(59, 130, 246)));
                                                ui.label(RichText::new(&item.display_name).strong());

                                                if item.is_deprecated {
                                                    egui::Frame::none()
                                                        .fill(if self.dark_mode { Color32::from_rgb(69, 10, 10) } else { Color32::from_rgb(254, 226, 226) })
                                                        .rounding(3.0)
                                                        .inner_margin(egui::Margin::symmetric(4.0, 1.0))
                                                        .show(ui, |ui| {
                                                            ui.label(RichText::new("УСТАРЕЛО (EOL)").small().strong().color(Color32::from_rgb(239, 68, 68)));
                                                        });
                                                }

                                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                    if !item.uninstall_string.is_empty() {
                                                        if ui.button("🗑 Удалить").clicked() {
                                                            runtime_to_uninstall = Some(item.uninstall_string.clone());
                                                        }
                                                    }
                                                    ui.label(RichText::new(format_size(item.size_bytes)).small().color(Color32::from_rgb(148, 163, 184)));
                                                });
                                            });
                                            ui.separator();
                                        }
                                        ui.add_space(8.0);
                                    }
                                }
                            });
                        });

                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(6.0);

                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Совет: Visual C++ 2015–2022 является единой универсальной библиотекой.").small());
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("Закрыть").clicked() {
                                do_close = true;
                            }
                        });
                    });
                });

            if do_close || !is_open {
                self.runtime_modal_open = false;
            }

            if let Some(uninst_str) = runtime_to_uninstall {
                let _ = Command::new("cmd").args(["/C", &uninst_str]).spawn();
                self.runtime_msg = Some("Запущен процесс деинсталляции выбранной среды выполнения.".to_string());
            }
        }

        // ====================================================================
        // 12. Winget Package Manager Modal
        // ====================================================================
        if self.winget_modal_open {
            let mut is_open = true;
            let mut do_rescan = false;
            let mut do_close = false;
            let mut pkg_to_upgrade: Option<String> = None;
            let mut pkg_to_uninstall: Option<String> = None;

            egui::Window::new("🚀 Менеджер пакетов и обновлений Winget")
                .open(&mut is_open)
                .collapsible(false)
                .resizable(true)
                .pivot(egui::Align2::CENTER_CENTER)
                .default_pos(ctx.screen_rect().center())
                .show(ctx, |ui| {
                    ui.set_width(760.0);
                    ui.set_height(480.0);

                    ui.heading(RichText::new("🚀 Диспетчер пакетов Windows (Winget CLI)").strong());
                    ui.add_space(4.0);
                    ui.label("Интеграция с репозиторием Winget. Позволяет проверять наличие официальных обновлений установленных программ и производить их тихое удаление без открытия сторонних мастеров.");
                    ui.add_space(6.0);

                    if let Some(msg) = &self.winget_msg {
                        egui::Frame::none()
                            .fill(if self.dark_mode { Color32::from_rgb(20, 83, 45) } else { Color32::from_rgb(220, 252, 231) })
                            .rounding(4.0)
                            .inner_margin(8.0)
                            .show(ui, |ui| {
                                ui.label(RichText::new(msg).strong().color(Color32::from_rgb(22, 163, 74)));
                            });
                        ui.add_space(6.0);
                    }

                    // Top Action Bar
                    ui.horizontal(|ui| {
                        let refresh_btn = egui::Button::new("🔄 Обновить список пакетов Winget");
                        if ui.add_enabled(!self.winget_is_busy, refresh_btn).clicked() {
                            do_rescan = true;
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let updatable_count = self.winget_packages.iter().filter(|p| p.available_version.is_some()).count();
                            ui.label(RichText::new(format!("Всего: {} | Доступно обновлений: {}", self.winget_packages.len(), updatable_count)).strong());
                        });
                    });
                    ui.add_space(6.0);

                    if self.winget_is_busy {
                        egui::Frame::none()
                            .fill(if self.dark_mode { Color32::from_rgb(30, 27, 75) } else { Color32::from_rgb(238, 242, 255) })
                            .rounding(4.0)
                            .stroke(Stroke::new(1.0_f32, if self.dark_mode { Color32::from_rgb(99, 102, 241) } else { Color32::from_rgb(165, 180, 252) }))
                            .inner_margin(8.0)
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.spinner();
                                    ui.label(RichText::new(&self.winget_busy_msg).strong().color(Color32::from_rgb(99, 102, 241)));
                                });
                            });
                        ui.add_space(6.0);
                        ctx.request_repaint_after(std::time::Duration::from_millis(100));
                    }

                    let list_bg = if self.dark_mode { Color32::from_rgb(18, 18, 20) } else { Color32::WHITE };
                    egui::Frame::none()
                        .fill(list_bg)
                        .stroke(Stroke::new(1.0_f32, if self.dark_mode { Color32::from_rgb(39, 39, 42) } else { Color32::from_rgb(226, 232, 240) }))
                        .rounding(4.0)
                        .inner_margin(8.0)
                        .show(ui, |ui| {
                            egui::ScrollArea::vertical().max_height(280.0).show(ui, |ui| {
                                if self.winget_packages.is_empty() {
                                    ui.vertical_centered(|ui| {
                                        ui.add_space(30.0);
                                        ui.label(RichText::new("Пакеты Winget загружаются или Winget недоступен на этом ПК.").color(Color32::from_rgb(148, 163, 184)));
                                    });
                                } else {
                                    for pkg in &self.winget_packages {
                                        ui.horizontal(|ui| {
                                            ui.vertical(|ui| {
                                                ui.horizontal(|ui| {
                                                    ui.label(RichText::new(&pkg.name).strong());
                                                    ui.label(RichText::new(format!("v{}", pkg.installed_version)).small().color(Color32::from_rgb(148, 163, 184)));

                                                    if let Some(avail) = &pkg.available_version {
                                                        egui::Frame::none()
                                                            .fill(if self.dark_mode { Color32::from_rgb(20, 83, 45) } else { Color32::from_rgb(220, 252, 231) })
                                                            .rounding(3.0)
                                                            .inner_margin(egui::Margin::symmetric(4.0, 1.0))
                                                            .show(ui, |ui| {
                                                                ui.label(RichText::new(format!("⬆ Доступна v{}", avail)).small().strong().color(Color32::from_rgb(22, 163, 74)));
                                                            });
                                                    }
                                                });
                                                ui.label(RichText::new(&pkg.id).small().color(Color32::from_rgb(113, 113, 122)));
                                            });

                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                let uninst_btn = egui::Button::new("🗑 Удалить (Winget)");
                                                if ui.add_enabled(!self.winget_is_busy, uninst_btn).clicked() {
                                                    pkg_to_uninstall = Some(pkg.id.clone());
                                                }
                                                if pkg.available_version.is_some() {
                                                    let up_btn = egui::Button::new(RichText::new("⬆ Обновить").color(Color32::WHITE).strong())
                                                        .fill(Color32::from_rgb(22, 163, 74));
                                                    if ui.add_enabled(!self.winget_is_busy, up_btn).clicked() {
                                                        pkg_to_upgrade = Some(pkg.id.clone());
                                                    }
                                                }
                                            });
                                        });
                                        ui.separator();
                                    }
                                }
                            });
                        });

                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(6.0);

                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Winget выполняет установку и удаление из проверенных манифестов.").small());
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("Закрыть").clicked() {
                                do_close = true;
                            }
                        });
                    });
                });

            if do_close || !is_open {
                self.winget_modal_open = false;
            }

            if do_rescan && !self.winget_is_busy {
                self.winget_is_busy = true;
                self.winget_busy_msg = "Получение списка пакетов Winget...".to_string();
                self.winget_msg = None;
                let (tx, rx) = std::sync::mpsc::channel();
                self.winget_task_rx = Some(rx);
                std::thread::spawn(move || {
                    let pkgs = scan_winget_packages();
                    let _ = tx.send((Ok("Список пакетов Winget успешно обновлен.".to_string()), Some(pkgs)));
                });
            }

            if let Some(id) = pkg_to_upgrade {
                if !self.winget_is_busy {
                    self.winget_is_busy = true;
                    self.winget_busy_msg = format!("Обновление пакета {} через Winget (загрузка и установка)...", id);
                    self.winget_msg = Some(format!("Запущен процесс обновления пакета {}...", id));
                    let (tx, rx) = std::sync::mpsc::channel();
                    self.winget_task_rx = Some(rx);
                    let id_clone = id.clone();
                    std::thread::spawn(move || {
                        let res = upgrade_winget_package(&id_clone);
                        let pkgs = scan_winget_packages();
                        let _ = tx.send((res, Some(pkgs)));
                    });
                }
            }

            if let Some(id) = pkg_to_uninstall {
                if !self.winget_is_busy {
                    self.winget_is_busy = true;
                    self.winget_busy_msg = format!("Удаление пакета {} через Winget...", id);
                    self.winget_msg = Some(format!("Запущен процесс удаления пакета {}...", id));
                    let (tx, rx) = std::sync::mpsc::channel();
                    self.winget_task_rx = Some(rx);
                    let id_clone = id.clone();
                    std::thread::spawn(move || {
                        let res = uninstall_winget_package(&id_clone);
                        let pkgs = scan_winget_packages();
                        let _ = tx.send((res, Some(pkgs)));
                    });
                }
            }
        }

        // Render GitHub Auto-update Modal
        self.render_update_modal(ctx);
    }

    fn render_update_modal(&mut self, ctx: &egui::Context) {
        if !self.update_modal_open {
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

                match &self.update_status {
                    autoupdater::UpdateStatus::Idle => {
                        ui.label("Проверка обновлений не запущена.");
                        ui.add_space(8.0);
                        if ui.button("🔍 Проверить сейчас").clicked() {
                            autoupdater::check_for_updates_async(
                                self.settings.github_repo.clone(),
                                env!("CARGO_PKG_VERSION").to_string(),
                                self.update_tx.clone(),
                            );
                        }
                    }
                    autoupdater::UpdateStatus::Checking => {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label(RichText::new("Проверка наличия новой версии на GitHub...").strong());
                        });
                        ui.add_space(6.0);
                        ui.label(format!("Репозиторий: github.com/{}", self.settings.github_repo));
                    }
                    autoupdater::UpdateStatus::UpToDate => {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("✅ Установлена последняя версия").strong().color(Color32::from_rgb(34, 197, 94)));
                        });
                        ui.add_space(6.0);
                        ui.label(format!("Текущая версия PurgeCraft v{} является актуальной.", env!("CARGO_PKG_VERSION")));
                        ui.add_space(10.0);
                        if ui.button("Закрыть").clicked() {
                            self.update_modal_open = false;
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
                                autoupdater::download_and_apply_update_async(info.download_url.clone(), self.update_tx.clone());
                            }

                            if ui.button("🌐 Открыть на GitHub").clicked() {
                                let _ = open::that(&info.html_url);
                            }

                            if ui.button("Отмена").clicked() {
                                self.update_modal_open = false;
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
                                    self.update_status = autoupdater::UpdateStatus::Error(e);
                                }
                            }
                            if ui.button("Позже").clicked() {
                                self.update_modal_open = false;
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
                                    self.settings.github_repo.clone(),
                                    env!("CARGO_PKG_VERSION").to_string(),
                                    self.update_tx.clone(),
                                );
                            }
                            if ui.button("Закрыть").clicked() {
                                self.update_modal_open = false;
                            }
                        });
                    }
                }
            });

        if !is_open {
            self.update_modal_open = false;
        }
    }

    fn export_html(&self) {
        let path = "Installed_Software_Audit.html";
        let mut html = String::from(
            "<!DOCTYPE html><html><head><meta charset='utf-8'><title>PurgeCraft Audit</title>"
        );
        html.push_str("<style>body{font-family:Segoe UI, sans-serif;padding:24px;background:#f9fafb;}table{width:100%;border-collapse:collapse;background:#fff;}th,td{padding:8px 12px;border:1px solid #e5e7eb;text-align:left;}th{background:#f3f4f6;}</style></head><body>");
        html.push_str(&format!("<h1>Аудит установленного ПО ({} программ)</h1><table><tr><th>Название</th><th>Тип</th><th>Размер</th><th>Дата</th><th>Издатель</th></tr>", self.apps.len()));
        for a in &self.apps {
            let type_str = if a.is_uwp { "Windows Store" } else if a.is_64bit { "64-бит" } else { "32-бит" };
            html.push_str(&format!("<tr><td><b>{}</b></td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>", a.name, type_str, format_size(a.size_bytes), a.install_date, a.publisher));
        }
        html.push_str("</table></body></html>");

        if std::fs::write(path, html).is_ok() {
            let _ = open::that(path);
        }
    }
}

fn main() -> eframe::Result<()> {
    // Silently remove any old binary left from a previous update
    autoupdater::cleanup_previous_update();

    let args: Vec<String> = std::env::args().collect();
    let track_arg = if let Some(pos) = args.iter().position(|a| a == "--track") {
        args.get(pos + 1).cloned()
    } else {
        None
    };

    let app_icon = egui::IconData {
        rgba: include_bytes!("icon_64.rgba").to_vec(),
        width: 64,
        height: 64,
    };

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1220.0, 780.0])
            .with_min_inner_size([850.0, 520.0])
            .with_title("PurgeCraft")
            .with_icon(app_icon),
        ..Default::default()
    };

    eframe::run_native(
        "PurgeCraft",
        options,
        Box::new(move |cc| {
            let initial_settings = settings::load_settings();
            cc.egui_ctx.set_zoom_factor(initial_settings.text_scale);

            let mut style = (*cc.egui_ctx.style()).clone();
            use egui::{FontFamily, FontId, TextStyle};
            style.text_styles = [
                (TextStyle::Heading, FontId::new(19.0, FontFamily::Proportional)),
                (TextStyle::Body, FontId::new(14.5, FontFamily::Proportional)),
                (TextStyle::Monospace, FontId::new(13.5, FontFamily::Monospace)),
                (TextStyle::Button, FontId::new(14.0, FontFamily::Proportional)),
                (TextStyle::Small, FontId::new(11.5, FontFamily::Proportional)),
            ].into();
            cc.egui_ctx.set_style(style);

            let mut app = PurgeCraftApp::default();
            if let Some(installer) = track_arg {
                if std::path::Path::new(&installer).exists() {
                    let stem = std::path::Path::new(&installer)
                        .file_stem()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_default();
                    app.tracker_installer_path = installer;
                    app.tracker_app_name = stem;
                    app.tracker_step = TrackerStep::SelectInstaller;
                    app.tracker_modal_open = true;
                }
            }
            Ok(Box::new(app))
        }),
    )
}

