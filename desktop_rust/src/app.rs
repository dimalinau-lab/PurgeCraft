use std::process::Command;
use std::collections::{HashSet, HashMap};
#[cfg(windows)]
use std::os::windows::process::CommandExt;

use eframe::egui::{self, Color32, RichText, Stroke};
use crate::scanner::{
    self, scan_win32_apps, scan_uwp_apps, scan_startup_items,
    apply_known_apps, save_known_apps, load_traced_apps,
    spawn_scan_leftovers_stream,
    ScanProgressMsg, DeleteProgressMsg,
    InstalledApp, StartupItem, LeftoverTrace, TracedApp, SystemSnapshot,
    ShaderCacheLocation, scan_shader_caches,
    PathEntry, scan_path_entries,
    PortableApp, scan_portable_apps,
    RuntimeFamilyGroup, detect_runtime_duplicates,
    WingetAppItem, scan_winget_packages,
};
use crate::settings::{AppSettings, load_settings, is_windows_dark_mode};
use crate::autoupdater;
use crate::types::{
    WizardPhase, ActiveTab, SettingsTab, AppCategory, StartupFilter, TrackerStep,
    SortColumn, SortOrder, format_size, jump_to_regedit, open_system_folder, spawn_size_calculator,
    UninstallerViewMode,
};
use crate::tabs;
use crate::modals;

pub struct PurgeCraftApp {
    pub active_tab: ActiveTab,
    pub uninstaller_view: UninstallerViewMode,
    pub category: AppCategory,
    pub drive_filter: Option<char>,
    pub startup_filter: StartupFilter,
    pub apps: Vec<InstalledApp>,
    pub startup_items: Vec<StartupItem>,
    pub selected_app_idx: Option<usize>,
    pub search_query: String,
    pub status_msg: String,
    pub dark_mode: bool,

    // Background size calculation stream
    pub size_rx: std::sync::mpsc::Receiver<(String, u64)>,
    pub size_tx: std::sync::mpsc::Sender<(String, u64)>,

    // Sorting
    pub sort_column: SortColumn,
    pub sort_order: SortOrder,

    // Real App Icon texture cache
    pub icon_textures: HashMap<String, Option<egui::TextureHandle>>,
    pub brand_logo_tex: Option<egui::TextureHandle>,

    // Batch operations
    pub batch_selected: HashSet<String>, // set of app.id
    pub batch_queue_open: bool,
    pub batch_queue_apps: Vec<InstalledApp>,
    pub batch_queue_idx: usize,
    pub batch_queue_statuses: Vec<String>,
    pub batch_force_open: bool,
    pub batch_force_apps: Vec<InstalledApp>,

    // Leftovers wizard (1:1 with Uninstall Tool 4-step wizard)
    pub wizard_open: bool,
    pub wizard_phase: WizardPhase,
    pub wizard_app_title: String,
    pub wizard_target_app: Option<InstalledApp>,
    pub wizard_scan_current_path: String,
    pub wizard_scan_progress: f32,
    pub wizard_scan_rx: Option<std::sync::mpsc::Receiver<ScanProgressMsg>>,
    pub wizard_traces: Vec<LeftoverTrace>,
    pub wizard_delete_progress: f32,
    pub wizard_delete_rx: Option<std::sync::mpsc::Receiver<DeleteProgressMsg>>,
    pub wizard_last_log_path: Option<std::path::PathBuf>,
    pub wizard_last_log_content: String,
    pub wizard_xml_status: Option<String>,
    pub wizard_report: Option<String>,

    // Uninstaller Execution & Waiting
    pub uninstall_confirm_open: bool,
    pub uninstall_target_app: Option<InstalledApp>,
    pub uninstall_waiting_open: bool,
    pub uninstall_waiting_app: Option<InstalledApp>,
    pub uninstall_child: Option<std::process::Child>,

    // Force removal confirm
    pub force_confirm_open: bool,
    pub force_confirm_app: Option<InstalledApp>,

    // Running processes detection & termination modal
    pub running_processes_modal_open: bool,
    pub running_processes: Vec<scanner::RunningProcessInfo>,
    pub running_processes_target_app: Option<InstalledApp>,

    // Delayed reboot files removal modal
    pub delayed_removal_modal_open: bool,
    pub delayed_removal_files: Vec<String>,

    // New Apps Notification
    pub new_apps_count: usize,

    // Remove entry confirm
    pub remove_entry_confirm_open: bool,
    pub remove_entry_confirm_app: Option<InstalledApp>,
    pub focus_search_requested: bool,
    pub uwp_rx: std::sync::mpsc::Receiver<Vec<InstalledApp>>,
    pub uwp_tx: std::sync::mpsc::Sender<Vec<InstalledApp>>,

    // Add Startup Item Dialog
    pub add_startup_modal_open: bool,
    pub new_startup_name: String,
    pub new_startup_path: String,
    pub new_startup_args: String,
    pub new_startup_hklm: bool,
    pub new_startup_error: Option<String>,

    // Install Tracker
    pub tracker_modal_open: bool,
    pub tracker_step: TrackerStep,
    pub tracker_installer_path: String,
    pub tracker_app_name: String,
    pub tracker_snapshot_before: Option<SystemSnapshot>,
    pub tracker_traces: Vec<LeftoverTrace>,
    pub traced_apps: Vec<TracedApp>,
    pub tracker_live_log: Vec<scanner::TrackerLiveMsg>,
    pub tracker_rx: Option<std::sync::mpsc::Receiver<scanner::TrackerLiveMsg>>,
    pub tracker_stop_tx: Option<std::sync::mpsc::Sender<()>>,
    pub tracker_child: Option<std::process::Child>,
    pub tracker_process_finished: bool,

    // Settings Window
    pub settings_open: bool,
    pub settings_tab: SettingsTab,
    pub settings: AppSettings,
    pub temp_settings: AppSettings,

    // Shader Cache Cleaner
    pub shader_modal_open: bool,
    pub shader_locations: Vec<ShaderCacheLocation>,
    pub shader_clean_msg: Option<String>,
    pub shader_cleaning: bool,

    // PATH Sanitizer
    pub path_modal_open: bool,
    pub path_entries: Vec<PathEntry>,
    pub path_msg: Option<String>,

    // Portable Apps Scanner
    pub portable_modal_open: bool,
    pub portable_apps: Vec<PortableApp>,
    pub portable_msg: Option<String>,

    // Duplicate Runtime Detector
    pub runtime_modal_open: bool,
    pub runtime_groups: Vec<RuntimeFamilyGroup>,
    pub runtime_msg: Option<String>,

    // Winget Manager
    pub winget_modal_open: bool,
    pub winget_packages: Vec<WingetAppItem>,
    pub winget_msg: Option<String>,
    pub winget_task_rx: Option<std::sync::mpsc::Receiver<(Result<String, String>, Option<Vec<WingetAppItem>>)>>,
    pub winget_is_busy: bool,
    pub winget_busy_msg: String,

    // System Deep Clean
    pub system_clean_modal_open: bool,
    pub system_clean_items: Vec<crate::cleaner::SystemCleanItem>,
    pub system_clean_msg: Option<String>,
    pub system_clean_is_busy: bool,
    pub system_clean_busy_msg: String,
    pub system_clean_rx: Option<std::sync::mpsc::Receiver<crate::cleaner::SystemCleanTaskMsg>>,

    // Dev Environments Clean
    pub dev_clean_modal_open: bool,
    pub dev_clean_items: Vec<crate::cleaner::DevCleanItem>,
    pub dev_clean_msg: Option<String>,
    pub dev_clean_is_busy: bool,
    pub dev_clean_busy_msg: String,
    pub dev_clean_rx: Option<std::sync::mpsc::Receiver<crate::cleaner::DevCleanTaskMsg>>,

    // Windows Services & Zombie Hunter
    pub services_modal_open: bool,
    pub services_items: Vec<crate::cleaner::ServiceItem>,
    pub services_search: String,
    pub services_filter: crate::cleaner::ServiceFilter,
    pub services_msg: Option<String>,

    // Async Startup & Portable Scan
    pub startup_rx: Option<std::sync::mpsc::Receiver<Vec<StartupItem>>>,
    pub portable_rx: Option<std::sync::mpsc::Receiver<Vec<PortableApp>>>,
    pub is_refreshing: bool,

    // Blacklist of uninstalled apps during this session
    pub recently_uninstalled: HashSet<String>,

    // GitHub Auto-updater
    pub update_status: autoupdater::UpdateStatus,
    pub update_rx: std::sync::mpsc::Receiver<autoupdater::UpdateStatus>,
    pub update_tx: std::sync::mpsc::Sender<autoupdater::UpdateStatus>,
    pub update_modal_open: bool,

    // Responsive UI state
    pub sidebar_open: bool,
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
            uninstaller_view: UninstallerViewMode::List,
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
            system_clean_modal_open: false,
            system_clean_items: Vec::new(),
            system_clean_msg: None,
            system_clean_is_busy: false,
            system_clean_busy_msg: String::new(),
            system_clean_rx: None,
            dev_clean_modal_open: false,
            dev_clean_items: Vec::new(),
            dev_clean_msg: None,
            dev_clean_is_busy: false,
            dev_clean_busy_msg: String::new(),
            dev_clean_rx: None,
            services_modal_open: false,
            services_items: Vec::new(),
            services_search: String::new(),
            services_filter: crate::cleaner::ServiceFilter::All,
            services_msg: None,
            startup_rx: Some(startup_rx),
            portable_rx: Some(portable_rx),
            is_refreshing: false,
            recently_uninstalled: HashSet::new(),
            update_status: autoupdater::UpdateStatus::Idle,
            update_rx,
            update_tx,
            update_modal_open: false,
            sidebar_open: true,
        }
    }
}

impl PurgeCraftApp {
    pub fn start_system_clean_scan(&mut self) {
        self.system_clean_is_busy = true;
        self.system_clean_busy_msg = "Сканирование системы...".to_string();
        let (tx, rx) = std::sync::mpsc::channel();
        self.system_clean_rx = Some(rx);
        crate::cleaner::spawn_system_clean_scan(tx);
    }

    pub fn start_system_clean_delete(&mut self) {
        self.system_clean_is_busy = true;
        self.system_clean_busy_msg = "Очистка системного мусора...".to_string();
        let (tx, rx) = std::sync::mpsc::channel();
        self.system_clean_rx = Some(rx);
        crate::cleaner::spawn_system_clean_delete(self.system_clean_items.clone(), tx);
    }

    pub fn start_dev_clean_scan(&mut self) {
        self.dev_clean_is_busy = true;
        self.dev_clean_busy_msg = "Сканирование сред разработки...".to_string();
        let (tx, rx) = std::sync::mpsc::channel();
        self.dev_clean_rx = Some(rx);
        crate::cleaner::spawn_dev_clean_scan(tx);
    }

    pub fn start_dev_clean_delete(&mut self) {
        self.dev_clean_is_busy = true;
        self.dev_clean_busy_msg = "Очистка кэшей библиотек...".to_string();
        let (tx, rx) = std::sync::mpsc::channel();
        self.dev_clean_rx = Some(rx);
        crate::cleaner::spawn_dev_clean_delete(self.dev_clean_items.clone(), tx);
    }

    pub fn start_winget_scan(&mut self) {
        if self.winget_is_busy {
            return;
        }
        self.winget_is_busy = true;
        self.winget_busy_msg = "Получение списка пакетов Winget...".to_string();
        let (tx, rx) = std::sync::mpsc::channel();
        self.winget_task_rx = Some(rx);
        std::thread::spawn(move || {
            let pkgs = scan_winget_packages();
            let _ = tx.send((Ok("Список пакетов Winget успешно обновлен.".to_string()), Some(pkgs)));
        });
    }


    pub fn refresh_data(&mut self) {
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

    pub fn remove_registry_entry(&mut self, app: &InstalledApp) {
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

    pub fn render_uninstaller(&mut self, ui: &mut egui::Ui) {
        tabs::render_uninstaller_tab(self, ui);
    }

    pub fn render_startup(&mut self, ui: &mut egui::Ui) {
        tabs::render_startup_tab(self, ui);
    }

    pub fn request_uninstall(&mut self, app: InstalledApp) {
        self.uninstall_target_app = Some(app);
        self.uninstall_confirm_open = true;
    }

    pub fn execute_uninstall(&mut self, app: InstalledApp) {
        if !app.uninstall_string.is_empty() {
            let child = if app.is_uwp {
                let mut cmd = Command::new("powershell");
                #[cfg(windows)]
                cmd.creation_flags(scanner::CREATE_NO_WINDOW);
                cmd.args(["-NoProfile", "-NonInteractive", "-Command", &app.uninstall_string]).spawn().ok()
            } else {
                let mut cmd = Command::new("cmd");
                #[cfg(windows)]
                cmd.creation_flags(scanner::CREATE_NO_WINDOW);
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

    pub fn start_wizard_scan(&mut self, app: InstalledApp) {
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

    pub fn start_uninstall(&mut self, app: InstalledApp) {
        self.request_uninstall(app);
    }

    pub fn start_batch_uninstall(&mut self) {
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

    pub fn launch_current_batch_app(&mut self) {
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

    pub fn start_batch_force_removal(&mut self) {
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

    pub fn render_modals(&mut self, ctx: &egui::Context) {
        modals::render_all_modals(self, ctx);
    }

    pub fn export_html(&self) {
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

        // Drain asynchronous System Clean Task
        if let Some(rx) = &self.system_clean_rx {
            if let Ok(msg) = rx.try_recv() {
                self.system_clean_is_busy = false;
                self.system_clean_rx = None;
                match msg {
                    crate::cleaner::SystemCleanTaskMsg::ScanFinished(items) => {
                        self.system_clean_items = items;
                        self.system_clean_msg = Some("Сканирование системы завершено".to_string());
                    }
                    crate::cleaner::SystemCleanTaskMsg::CleanFinished { files, bytes, refreshed_items } => {
                        self.system_clean_items = refreshed_items;
                        self.system_clean_msg = Some(format!("Успешно очищено: {} файлов ({})", files, format_size(bytes)));
                    }
                }
                ctx.request_repaint();
            }
        }
        if self.system_clean_is_busy {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }

        // Drain asynchronous Dev Clean Task
        if let Some(rx) = &self.dev_clean_rx {
            if let Ok(msg) = rx.try_recv() {
                self.dev_clean_is_busy = false;
                self.dev_clean_rx = None;
                match msg {
                    crate::cleaner::DevCleanTaskMsg::ScanFinished(items) => {
                        self.dev_clean_items = items;
                        self.dev_clean_msg = Some("Сканирование сред разработки завершено".to_string());
                    }
                    crate::cleaner::DevCleanTaskMsg::CleanFinished { files, bytes, refreshed_items } => {
                        self.dev_clean_items = refreshed_items;
                        self.dev_clean_msg = Some(format!("Успешно освобождено: {} файлов ({})", files, format_size(bytes)));
                    }
                }
                ctx.request_repaint();
            }
        }
        if self.dev_clean_is_busy {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
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

        // 2. Main Toolbar (Responsive & Clean for Windowed Mode)
        egui::TopBottomPanel::top("main_toolbar").show(ctx, |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                // Sidebar collapse toggle button
                let side_tip = if self.sidebar_open { "Скрыть боковую панель" } else { "Показать боковую панель" };
                let side_icon = if self.sidebar_open { "◀" } else { "▶" };
                if ui.button(RichText::new(side_icon).strong()).on_hover_text(side_tip).clicked() {
                    self.sidebar_open = !self.sidebar_open;
                }
                ui.separator();

                let logo_color = if self.dark_mode {
                    Color32::from_rgb(244, 244, 245)
                } else {
                    Color32::from_rgb(37, 99, 235)
                };
                if let Some(ref logo) = self.brand_logo_tex {
                    ui.image((logo.id(), egui::vec2(20.0, 20.0)));
                    ui.add_space(2.0);
                }
                ui.heading(RichText::new("PurgeCraft").color(logo_color).strong().size(16.0));
                ui.add_space(6.0);

                // Core Mode: Деинсталлятор
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
                    RichText::new(uninst_title).strong().color(uninst_text)
                )
                .fill(uninst_fill)
                .stroke(Stroke::new(1.0_f32, uninst_stroke));

                if ui.add(uninst_btn).clicked() {
                    self.active_tab = ActiveTab::Uninstaller;
                }

                // Core Mode: Автозапуск
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
                    RichText::new(startup_title).strong().color(startup_text)
                )
                .fill(startup_fill)
                .stroke(Stroke::new(1.0_f32, startup_stroke));

                if ui.add(startup_btn).clicked() {
                    self.active_tab = ActiveTab::Startup;
                }

                // Install Tracker
                let tracker_title = if self.settings.show_toolbar_labels { "⚡ Следить" } else { "⚡" };
                let tracker_btn = egui::Button::new(
                    RichText::new(tracker_title)
                        .strong()
                        .color(if self.dark_mode { Color32::from_rgb(253, 224, 71) } else { Color32::from_rgb(180, 83, 9) })
                )
                .fill(if self.dark_mode { Color32::from_rgb(66, 32, 6) } else { Color32::from_rgb(254, 243, 199) })
                .stroke(Stroke::new(1.0_f32, if self.dark_mode { Color32::from_rgb(202, 138, 4) } else { Color32::from_rgb(245, 158, 11) }));

                if ui.add(tracker_btn).on_hover_text("Монитор установки (Install Tracker): отслеживать установку для 100% чистого удаления").clicked() {
                    self.tracker_installer_path.clear();
                    self.tracker_app_name.clear();
                    self.tracker_traces.clear();
                    self.tracker_snapshot_before = None;
                    self.tracker_step = TrackerStep::SelectInstaller;
                    self.tracker_modal_open = true;
                }

                // Always keep tools cleanly grouped into dropdown menu to avoid toolbar clutter
                ui.menu_button("🛠 Утилиты ▾", |ui| {
                    if ui.button("🎮 Шейдеры GPU...").clicked() {
                        self.shader_locations = scan_shader_caches();
                        self.shader_clean_msg = None;
                        self.shader_modal_open = true;
                        ui.close_menu();
                    }
                    if ui.button("🗄 Санитайзер PATH...").clicked() {
                        self.path_entries = scan_path_entries();
                        self.path_msg = None;
                        self.path_modal_open = true;
                        ui.close_menu();
                    }
                    if ui.button("📦 Сканер Portable...").clicked() {
                        self.portable_apps = scan_portable_apps(&[], &self.apps);
                        self.portable_msg = None;
                        self.portable_modal_open = true;
                        ui.close_menu();
                    }
                    if ui.button("⚖ Анализатор дубликатов VC++/Java...").clicked() {
                        self.runtime_groups = detect_runtime_duplicates(&self.apps);
                        self.runtime_msg = None;
                        self.runtime_modal_open = true;
                        ui.close_menu();
                    }
                    if ui.button("🚀 Менеджер Winget...").clicked() {
                        self.winget_msg = None;
                        self.winget_modal_open = true;
                        if self.winget_packages.is_empty() && !self.winget_is_busy {
                            self.start_winget_scan();
                        }
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("♻ Очистка системы Windows...").clicked() {
                        self.system_clean_msg = None;
                        self.system_clean_modal_open = true;
                        if self.system_clean_items.is_empty() && !self.system_clean_is_busy {
                            self.start_system_clean_scan();
                        }
                        ui.close_menu();
                    }
                    if ui.button("🛠 Кэши сред разработки (npm, cargo...)...").clicked() {
                        self.dev_clean_msg = None;
                        self.dev_clean_modal_open = true;
                        if self.dev_clean_items.is_empty() && !self.dev_clean_is_busy {
                            self.start_dev_clean_scan();
                        }
                        ui.close_menu();
                    }
                    if ui.button("⚙ Менеджер служб и зомби...").clicked() {
                        self.services_items = crate::cleaner::scan_windows_services();
                        self.services_search.clear();
                        self.services_filter = crate::cleaner::ServiceFilter::All;
                        self.services_msg = None;
                        self.services_modal_open = true;
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("📄 Экспорт списка в HTML...").clicked() {
                        self.export_html();
                        ui.close_menu();
                    }
                });

                // Right aligned search and compact theme toggle
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let theme_icon = if self.dark_mode { "☀" } else { "🌙" };
                    let theme_tip = if self.dark_mode { "Переключить на светлую тему" } else { "Переключить на темную тему" };
                    if ui.button(theme_icon).on_hover_text(theme_tip).clicked() {
                        self.dark_mode = !self.dark_mode;
                    }

                    if !self.search_query.is_empty() {
                        if ui.button("✕").on_hover_text("Очистить поиск (Esc)").clicked() {
                            self.search_query.clear();
                        }
                    }

                    let search_w = (ui.available_width() - 24.0).clamp(100.0, 180.0);
                    let search_box = ui.add(
                        egui::TextEdit::singleline(&mut self.search_query)
                            .hint_text("Поиск... (Ctrl+F)")
                            .desired_width(search_w)
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

        // 3. Secondary Toolbar (Compact Category Tabs & Filter Menu)
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

                    // Primary Category Tabs
                    ui.selectable_value(&mut self.category, AppCategory::All, format!("Все ({})", total_all));
                    ui.selectable_value(&mut self.category, AppCategory::UserApps, format!("Программы ({})", total_user));
                    ui.selectable_value(&mut self.category, AppCategory::System, format!("Системные ({})", total_sys));

                    ui.separator();

                    // Secondary Filters Dropdown Menu [ 🔍 Фильтры ▾ ]
                    let is_sub_filter_active = match self.category {
                        AppCategory::StoreApps | AppCategory::Portable | AppCategory::Arch64 | AppCategory::Arch32 => true,
                        _ => self.drive_filter.is_some(),
                    };

                    let mut drives_count: std::collections::BTreeMap<char, usize> = std::collections::BTreeMap::new();
                    for a in &self.apps {
                        let d = scanner::get_app_drive_letter(a);
                        *drives_count.entry(d).or_insert(0) += 1;
                    }

                    let filter_title = match self.category {
                        AppCategory::StoreApps => format!("Магазин ({}) ▾", total_store),
                        AppCategory::Portable => format!("Portable ({}) ▾", total_portable),
                        AppCategory::Arch64 => format!("64-бит ({}) ▾", total_64),
                        AppCategory::Arch32 => format!("32-бит ({}) ▾", total_32),
                        _ => if let Some(d) = self.drive_filter {
                            format!("Диск {}: ▾", d)
                        } else {
                            "Фильтры ▾".to_string()
                        },
                    };

                    let filter_label = if is_sub_filter_active {
                        RichText::new(format!("🔍 {}", filter_title)).strong().color(Color32::from_rgb(56, 189, 248))
                    } else {
                        RichText::new(filter_title)
                    };

                    ui.menu_button(filter_label, |ui| {
                        if is_sub_filter_active {
                            if ui.button("✕ Сбросить фильтры").clicked() {
                                self.category = AppCategory::All;
                                self.drive_filter = None;
                                self.selected_app_idx = None;
                                ui.close_menu();
                            }
                            ui.separator();
                        }

                        ui.label(RichText::new("Тип приложения:").size(11.0).color(Color32::GRAY));
                        if ui.selectable_value(&mut self.category, AppCategory::Portable, format!("📦 Portable ({})", total_portable)).clicked() {
                            self.selected_app_idx = None;
                            ui.close_menu();
                        }
                        if ui.selectable_value(&mut self.category, AppCategory::StoreApps, format!("🏪 Магазин Windows ({})", total_store)).clicked() {
                            self.selected_app_idx = None;
                            ui.close_menu();
                        }

                        ui.separator();
                        ui.label(RichText::new("Архитектура:").size(11.0).color(Color32::GRAY));
                        if ui.selectable_value(&mut self.category, AppCategory::Arch64, format!("⚙ 64-бит ({})", total_64)).clicked() {
                            self.selected_app_idx = None;
                            ui.close_menu();
                        }
                        if ui.selectable_value(&mut self.category, AppCategory::Arch32, format!("⚙ 32-бит ({})", total_32)).clicked() {
                            self.selected_app_idx = None;
                            ui.close_menu();
                        }

                        ui.separator();
                        ui.label(RichText::new("Диск установки:").size(11.0).color(Color32::GRAY));
                        if ui.selectable_value(&mut self.drive_filter, None, format!("💾 Все диски ({})", self.apps.len())).clicked() {
                            self.selected_app_idx = None;
                            ui.close_menu();
                        }
                        for (&drive, &count) in &drives_count {
                            if ui.selectable_value(&mut self.drive_filter, Some(drive), format!("💽 Диск {}: ({} прогр.)", drive, count)).clicked() {
                                self.selected_app_idx = None;
                                ui.close_menu();
                            }
                        }
                    });

                    // New apps pill button (shown only if new apps actually exist)
                    let actual_new_count = self.apps.iter().filter(|a| a.is_new).count();
                    if actual_new_count > 0 {
                        ui.separator();
                        if ui.button(RichText::new(format!("✨ Сбросить «Новое» ({})", actual_new_count)).small().color(Color32::from_rgb(234, 179, 8)))
                            .on_hover_text("Снять отметку «Новое» со всех недавно установленных программ")
                            .clicked()
                        {
                            save_known_apps(&self.apps);
                            for a in &mut self.apps {
                                a.is_new = false;
                            }
                            self.new_apps_count = 0;
                            self.status_msg = "Все отметки «Новое» успешно сняты.".to_string();
                        }
                    }

                    // Right side: Batch counter & Compact View Switcher
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if !self.batch_selected.is_empty() {
                            ui.label(RichText::new(format!("Выбрано: {}", self.batch_selected.len())).strong().color(Color32::from_rgb(56, 189, 248)));
                            ui.separator();
                        }

                        // Compact Segmented View Switcher
                        let treemap_active = self.uninstaller_view == UninstallerViewMode::DiskTreemap;
                        let list_active = self.uninstaller_view == UninstallerViewMode::List;

                        let treemap_btn = ui.selectable_value(
                            &mut self.uninstaller_view,
                            UninstallerViewMode::DiskTreemap,
                            RichText::new("📊 Treemap").color(if treemap_active { Color32::WHITE } else { Color32::GRAY }),
                        );
                        treemap_btn.on_hover_text("Визуальная карта дискового пространства и крупнейших программ");

                        let list_btn = ui.selectable_value(
                            &mut self.uninstaller_view,
                            UninstallerViewMode::List,
                            RichText::new("📋 Список").color(if list_active { Color32::WHITE } else { Color32::GRAY }),
                        );
                        list_btn.on_hover_text("Табличный список установленных программ");
                    });
                });
                ui.add_space(3.0);
            });
        } else if self.active_tab == ActiveTab::Startup {
            egui::TopBottomPanel::top("startup_category_bar").show(ctx, |ui| {
                ui.add_space(3.0);
                ui.horizontal_wrapped(|ui| {
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

        // 6. Modals (Wizard, Force Removal, Batch Queue, Batch Force, Tools, Settings, Updates)
        self.render_modals(ctx);
    }
}
