pub mod dialogs;
pub mod wizard;
pub mod tracker;
pub mod settings;
pub mod tools;
pub mod cleaners;
pub mod update;

use eframe::egui;
use crate::app::PurgeCraftApp;

pub fn render_all_modals(app: &mut PurgeCraftApp, ctx: &egui::Context) {
    dialogs::render_dialogs(app, ctx);
    wizard::render_wizard_modal(app, ctx);
    tracker::render_tracker_modal(app, ctx);
    settings::render_settings_modal(app, ctx);
    tools::render_shader_modal(app, ctx);
    tools::render_path_modal(app, ctx);
    tools::render_portable_modal(app, ctx);
    tools::render_runtime_modal(app, ctx);
    tools::render_winget_modal(app, ctx);
    cleaners::render_system_clean_modal(app, ctx);
    cleaners::render_dev_clean_modal(app, ctx);
    cleaners::render_services_modal(app, ctx);
    update::render_update_modal(app, ctx);
}
