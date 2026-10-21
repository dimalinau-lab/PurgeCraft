#![windows_subsystem = "windows"]

pub mod scanner;
pub mod icons;
pub mod settings;
pub mod autoupdater;
pub mod types;
pub mod ui;
pub mod tabs;
pub mod modals;
pub mod cleaner;
pub mod app;

use eframe::egui;
use app::PurgeCraftApp;
use types::TrackerStep;

fn main() -> eframe::Result<()> {
    // Silently remove any old binary left from a previous update
    autoupdater::cleanup_previous_update();

    let args: Vec<String> = std::env::args().collect();
    let track_arg = if let Some(pos) = args.iter().position(|a| a == "--track") {
        args.get(pos + 1).cloned()
    } else {
        None
    };
    let clean_path_arg = if let Some(pos) = args.iter().position(|a| a == "--clean-path") {
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

            // Configure Segoe UI Emoji fallback for full Windows emoji rendering
            let mut fonts = egui::FontDefinitions::default();
            if let Ok(emoji_data) = std::fs::read(r"C:\Windows\Fonts\seguiemj.ttf") {
                fonts.font_data.insert(
                    "seguiemj".to_owned(),
                    egui::FontData::from_owned(emoji_data).into(),
                );
                fonts.families
                    .entry(egui::FontFamily::Proportional)
                    .or_default()
                    .push("seguiemj".to_owned());
                fonts.families
                    .entry(egui::FontFamily::Monospace)
                    .or_default()
                    .push("seguiemj".to_owned());
            }
            cc.egui_ctx.set_fonts(fonts);

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
            } else if let Some(clean_path) = clean_path_arg {
                let p = std::path::Path::new(&clean_path);
                let stem = p.file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| clean_path.clone());
                app.search_query = stem.clone();
                if let Some(pos) = app.apps.iter().position(|a| {
                    (!a.install_location.is_empty() && clean_path.to_lowercase().contains(&a.install_location.to_lowercase()))
                        || a.name.to_lowercase().contains(&stem.to_lowercase())
                }) {
                    app.selected_app_idx = Some(pos);
                }
            }
            Ok(Box::new(app))
        }),
    )
}
