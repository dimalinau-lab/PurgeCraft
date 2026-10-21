use eframe::egui::{self, Color32, RichText, Vec2, Stroke};
use egui_extras::{TableBuilder, Column};
use crate::app::PurgeCraftApp;
use crate::icons;
use crate::scanner::{self, InstalledApp, save_known_apps};
use crate::types::{AppCategory, SortColumn, SortOrder, format_size, truncate_chars, jump_to_regedit, open_system_folder, UninstallerViewMode};
use crate::ui::render_sort_header;

pub fn render_uninstaller_tab(app: &mut PurgeCraftApp, ui: &mut egui::Ui) {
    let q = app.search_query.to_lowercase();
    let cat = app.category;

    let mut filtered_indices: Vec<usize> = app.apps
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
            if let Some(target_drive) = app.drive_filter {
                if scanner::get_app_drive_letter(a) != target_drive {
                    return false;
                }
            }

            if app.settings.tree_updates_mode == "hidden" {
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
    let sort_col = app.sort_column;
    let sort_ord = app.sort_order;
    let highlight_recent = app.settings.highlight_recent;
    filtered_indices.sort_by(|&a_idx, &b_idx| {
        let a = &app.apps[a_idx];
        let b = &app.apps[b_idx];

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

    let selected_app = app.selected_app_idx.and_then(|idx| app.apps.get(idx).cloned());

    // Dynamic True Dark vs Clean Light palette (ZERO BLUE IN DARK)
    let pane_fill = if app.dark_mode {
        Color32::from_rgb(24, 24, 27) // True neutral dark (Zinc 900)
    } else {
        Color32::from_rgb(235, 242, 249) // Soft sky-blue
    };
    let pane_stroke = if app.dark_mode {
        Color32::from_rgb(39, 39, 42) // Neutral dark border
    } else {
        Color32::from_rgb(203, 219, 235)
    };
    let heading_color = if app.dark_mode {
        Color32::WHITE
    } else {
        Color32::from_rgb(15, 23, 42)
    };
    let meta_label_color = if app.dark_mode {
        Color32::from_rgb(161, 161, 170) // Neutral light gray
    } else {
        Color32::from_rgb(100, 116, 139)
    };
    let (danger_fill, danger_stroke, danger_text) = if app.dark_mode {
        (Color32::from_rgb(42, 18, 21), Color32::from_rgb(220, 38, 38), Color32::from_rgb(254, 202, 202))
    } else {
        (Color32::from_rgb(254, 242, 242), Color32::from_rgb(252, 165, 165), Color32::from_rgb(220, 38, 38))
    };
    let table_container_fill = if app.dark_mode {
        Color32::from_rgb(18, 18, 20) // Deep neutral black
    } else {
        Color32::WHITE
    };

    // Left ExplorerBar (TaskPane) - Resizable & Collapsible for Windowed Mode
    if app.sidebar_open {
        egui::SidePanel::left("explorer_task_pane")
            .default_width(240.0)
            .min_width(190.0)
            .max_width(340.0)
            .resizable(true)
            .frame(
                egui::Frame::none()
                    .fill(pane_fill)
                    .stroke(Stroke::new(1.0_f32, pane_stroke))
                    .inner_margin(8.0)
            )
            .show_inside(ui, |ui| {
            if app.batch_selected.len() > 1 {
                // BATCH OPERATIONS PANEL
                let batch_count = app.batch_selected.len();
                let selected_apps_vec: Vec<InstalledApp> = app.apps.iter()
                    .filter(|a| app.batch_selected.contains(&a.id))
                    .cloned()
                    .collect();
                let batch_size: u64 = selected_apps_vec.iter().map(|a| a.size_bytes).sum();

                let batch_heading_color = if app.dark_mode { heading_color } else { Color32::from_rgb(2, 132, 199) };
                ui.heading(RichText::new("📦 ПАКЕТНЫЙ РЕЖИМ").size(15.0).strong().color(batch_heading_color));
                ui.add_space(8.0);

                ui.label(RichText::new(format!("Выбрано программ: {}", batch_count)).strong().color(heading_color));
                ui.label(RichText::new(format!("Общий объем: {}", format_size(batch_size))).color(meta_label_color));

                ui.add_space(14.0);
                ui.separator();
                ui.add_space(10.0);

                // Batch Uninstall Button
                let (batch_btn_fill, batch_btn_stroke) = if app.dark_mode {
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
                    app.start_batch_uninstall();
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
                    app.start_batch_force_removal();
                }

                ui.add_space(10.0);
                if ui.button("✕ Снять выбор со всех").clicked() {
                    app.batch_selected.clear();
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
            } else if let Some(a_item) = &selected_app {
                // SINGLE APP DETAILS PANEL with 36x36 Icon
                ui.horizontal(|ui| {
                    if !a_item.display_icon.is_empty() {
                        if let Some(opt_tex) = app.icon_textures.get(&a_item.id) {
                            if let Some(tex) = opt_tex {
                                ui.image((tex.id(), egui::vec2(36.0, 36.0)));
                            } else {
                                let icon = if a_item.is_uwp { "🌐" } else if a_item.is_system_component { "⚙" } else { "📦" };
                                ui.label(RichText::new(icon).size(26.0));
                            }
                        } else {
                            let opt_img = icons::extract_app_icon(&a_item.display_icon);
                            let tex_opt = opt_img.map(|img| ui.ctx().load_texture(&a_item.id, img, Default::default()));
                            if let Some(ref tex) = tex_opt {
                                ui.image((tex.id(), egui::vec2(36.0, 36.0)));
                            } else {
                                let icon = if a_item.is_uwp { "🌐" } else if a_item.is_system_component { "⚙" } else { "📦" };
                                ui.label(RichText::new(icon).size(26.0));
                            }
                            app.icon_textures.insert(a_item.id.clone(), tex_opt);
                        }
                    } else {
                        let icon = if a_item.is_uwp { "🌐" } else if a_item.is_system_component { "⚙" } else { "📦" };
                        ui.label(RichText::new(icon).size(26.0));
                    }

                    ui.vertical(|ui| {
                        ui.heading(RichText::new(&a_item.name).size(15.0).strong().color(heading_color));
                        if !a_item.publisher.is_empty() {
                            ui.label(RichText::new(truncate_chars(&a_item.publisher, 26)).small().color(meta_label_color));
                        }
                    });
                });
                ui.add_space(8.0);

                let loc = if !a_item.install_location.is_empty() {
                    a_item.install_location.clone()
                } else if !a_item.display_icon.is_empty() {
                    let f = a_item.display_icon.split(',').next().unwrap_or(&a_item.display_icon).trim().trim_matches('"');
                    std::path::Path::new(f).parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_default()
                } else if !a_item.uninstall_string.is_empty() {
                    let clean = a_item.uninstall_string.trim().trim_matches('"');
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
                    let type_str = if a_item.source_badge == "[Portable]" {
                        "Portable (Переносная)"
                    } else if a_item.is_uwp {
                        "Магазин Windows"
                    } else if a_item.is_64bit {
                        "64-бит Win32"
                    } else {
                        "32-бит Win32"
                    };
                    ui.label(RichText::new(type_str).strong().color(heading_color));
                    ui.end_row();

                    ui.label(RichText::new("Версия:").color(meta_label_color));
                    ui.label(RichText::new(if a_item.version.is_empty() { "1.0" } else { &a_item.version }).strong());
                    ui.end_row();

                    ui.label(RichText::new("Размер:").color(meta_label_color));
                    ui.label(RichText::new(format_size(a_item.size_bytes)).strong());
                    ui.end_row();

                    ui.label(RichText::new("Установка:").color(meta_label_color));
                    ui.label(RichText::new(if a_item.install_date.is_empty() { "—" } else { &a_item.install_date }).strong());
                    ui.end_row();

                    ui.label(RichText::new("Издатель:").color(meta_label_color));
                    ui.label(truncate_chars(if a_item.publisher.is_empty() { "Не указан" } else { &a_item.publisher }, 20));
                    ui.end_row();

                    ui.label(RichText::new("Папка:").color(meta_label_color));
                    if !loc.is_empty() {
                        let display_path = truncate_chars(&loc, 22);
                        let link_color = if app.dark_mode { Color32::from_rgb(96, 165, 250) } else { Color32::from_rgb(2, 132, 199) };
                        let link_resp = ui.link(RichText::new(display_path).color(link_color).underline())
                            .on_hover_text(format!("Нажмите, чтобы открыть папку в Проводнике:\n{}", loc));
                        if link_resp.clicked() {
                            open_system_folder(&loc, app.settings.use_total_commander);
                        }
                    } else {
                        ui.label(RichText::new("—").color(meta_label_color));
                    }
                    ui.end_row();

                    let is_traced = app.traced_apps.iter().any(|t| t.name.eq_ignore_ascii_case(&a_item.name) || t.id == a_item.id);
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
                let (uninst_btn_fill, uninst_btn_stroke) = if app.dark_mode {
                    (Color32::from_rgb(45, 48, 56), Stroke::new(1.0_f32, Color32::from_rgb(82, 82, 91)))
                } else {
                    (Color32::from_rgb(37, 99, 235), Stroke::NONE)
                };
                let uninst_title = if a_item.source_badge == "[Portable]" {
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
                    app.start_uninstall(a_item.clone());
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
                    app.force_confirm_app = Some(a_item.clone());
                    app.force_confirm_open = true;
                }

                ui.add_space(14.0);
                ui.separator();
                ui.add_space(8.0);

                ui.label(RichText::new("Действия").small().strong().color(meta_label_color));
                ui.add_space(4.0);

                if ui.link("❌ Удалить запись из списка").on_hover_text("Удалить поврежденную запись из реестра Windows без запуска деинсталлятора").clicked() {
                    app.remove_entry_confirm_app = Some(a_item.clone());
                    app.remove_entry_confirm_open = true;
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
                if !a_item.is_uwp && !a_item.registry_path.is_empty() {
                    if ui.link("🗄 Ключ в реестре Windows").clicked() {
                        jump_to_regedit(&a_item.registry_path);
                    }
                }
                if ui.link("🌐 Найти информацию в Google").clicked() {
                    let url = format!("https://www.google.com/search?q={}+{}", a_item.name, a_item.publisher);
                    let _ = open::that(url);
                }
            } else {
                ui.label("Выберите программу из списка для просмотра сведений и действий.");
            }
        });
    }

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

            if app.uninstaller_view == UninstallerViewMode::DiskTreemap {
                crate::tabs::render_disk_treemap(app, ui, &filtered_indices);
                return;
            }

            if filtered_indices.is_empty() {
                ui.add_space(30.0);
                ui.vertical_centered(|ui| {
                    ui.label(RichText::new("Ничего не найдено").size(16.0).color(Color32::from_rgb(100, 116, 139)));
                    ui.label("Попробуйте сбросить поисковый фильтр (Esc) или переключить категорию.");
                });
            } else {
                let all_filtered_selected = !filtered_indices.is_empty()
                    && filtered_indices.iter().all(|&i| app.batch_selected.contains(&app.apps[i].id));

                TableBuilder::new(ui)
                    .striped(true)
                    .resizable(true)
                    .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                    .column(Column::exact(32.0))                    // Batch Checkbox Column
                    .column(Column::initial(260.0).at_least(140.0)) // Program Name (Resizable!)
                    .column(Column::exact(85.0))                    // Size
                    .column(Column::exact(95.0))                    // Date
                    .column(Column::initial(150.0).at_least(80.0))  // Publisher (Resizable!)
                    .column(Column::remainder().at_least(85.0))     // Type / Arch
                    .header(30.0, |mut header| {
                        // Column 0: Checkbox Header
                        header.col(|ui| {
                            let mut master_chk = all_filtered_selected;
                            if ui.checkbox(&mut master_chk, "").on_hover_text("Выбрать все / Снять выделение").changed() {
                                if master_chk {
                                    for &i in &filtered_indices {
                                        app.batch_selected.insert(app.apps[i].id.clone());
                                    }
                                } else {
                                    for &i in &filtered_indices {
                                        app.batch_selected.remove(&app.apps[i].id);
                                    }
                                }
                            }
                        });

                        // Column 1: Name Sort Button
                        header.col(|ui| {
                            if render_sort_header(ui, "Имя программы", app.sort_column == SortColumn::Name, app.sort_order, app.dark_mode).clicked() {
                                if app.sort_column == SortColumn::Name {
                                    app.sort_order = match app.sort_order {
                                        SortOrder::Ascending => SortOrder::Descending,
                                        SortOrder::Descending => SortOrder::Ascending,
                                    };
                                } else {
                                    app.sort_column = SortColumn::Name;
                                    app.sort_order = SortOrder::Ascending;
                                }
                            }
                        });

                        // Column 2: Size Sort Button
                        header.col(|ui| {
                            if render_sort_header(ui, "Размер", app.sort_column == SortColumn::Size, app.sort_order, app.dark_mode).clicked() {
                                if app.sort_column == SortColumn::Size {
                                    app.sort_order = match app.sort_order {
                                        SortOrder::Ascending => SortOrder::Descending,
                                        SortOrder::Descending => SortOrder::Ascending,
                                    };
                                } else {
                                    app.sort_column = SortColumn::Size;
                                    app.sort_order = SortOrder::Descending;
                                }
                            }
                        });

                        // Column 3: Date Sort Button
                        header.col(|ui| {
                            if render_sort_header(ui, "Установлено", app.sort_column == SortColumn::Date, app.sort_order, app.dark_mode).clicked() {
                                if app.sort_column == SortColumn::Date {
                                    app.sort_order = match app.sort_order {
                                        SortOrder::Ascending => SortOrder::Descending,
                                        SortOrder::Descending => SortOrder::Ascending,
                                    };
                                } else {
                                    app.sort_column = SortColumn::Date;
                                    app.sort_order = SortOrder::Descending;
                                }
                            }
                        });

                        // Column 4: Publisher Sort Button
                        header.col(|ui| {
                            if render_sort_header(ui, "Издатель", app.sort_column == SortColumn::Publisher, app.sort_order, app.dark_mode).clicked() {
                                if app.sort_column == SortColumn::Publisher {
                                    app.sort_order = match app.sort_order {
                                        SortOrder::Ascending => SortOrder::Descending,
                                        SortOrder::Descending => SortOrder::Ascending,
                                    };
                                } else {
                                    app.sort_column = SortColumn::Publisher;
                                    app.sort_order = SortOrder::Ascending;
                                }
                            }
                        });

                        // Column 5: Type Sort Button
                        header.col(|ui| {
                            if render_sort_header(ui, "Тип / Разрядность", app.sort_column == SortColumn::AppType, app.sort_order, app.dark_mode).clicked() {
                                if app.sort_column == SortColumn::AppType {
                                    app.sort_order = match app.sort_order {
                                        SortOrder::Ascending => SortOrder::Descending,
                                        SortOrder::Descending => SortOrder::Ascending,
                                    };
                                } else {
                                    app.sort_column = SortColumn::AppType;
                                    app.sort_order = SortOrder::Ascending;
                                }
                            }
                        });
                    })
                    .body(|body| {
                        let row_height = if app.settings.compact_list { 26.0 } else { 34.0 };
                        body.rows(row_height, filtered_indices.len(), |mut row| {
                            let row_idx = row.index();
                            let idx = filtered_indices[row_idx];
                            let item_app = &app.apps[idx];
                            let is_selected = app.selected_app_idx == Some(idx);
                            row.set_selected(is_selected);

                            // Column 0: Checkbox
                            row.col(|ui| {
                                let mut is_checked = app.batch_selected.contains(&item_app.id);
                                if ui.checkbox(&mut is_checked, "").changed() {
                                    if is_checked {
                                        app.batch_selected.insert(item_app.id.clone());
                                    } else {
                                        app.batch_selected.remove(&item_app.id);
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
                                        open_system_folder(&target_app.install_location, app.settings.use_total_commander);
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
                                if app.new_apps_count > 0 {
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
                                    app_to_uninstall = Some(item_app.clone());
                                }
                                cell_resp.context_menu(|ui| {
                                    show_row_context_menu(ui, item_app, &mut app_to_uninstall, &mut app_to_force, &mut app_to_remove_entry, &mut app_to_dismiss_single, &mut app_to_dismiss_all);
                                });

                                ui.horizontal(|ui| {
                                    // Render real icon if available, or load on demand
                                    if !item_app.display_icon.is_empty() {
                                        if let Some(opt_tex) = app.icon_textures.get(&item_app.id) {
                                            if let Some(tex) = opt_tex {
                                                ui.image((tex.id(), egui::vec2(22.0, 22.0)));
                                            } else {
                                                let icon = if item_app.is_uwp { "🌐" } else if item_app.is_system_component { "⚙" } else { "📦" };
                                                ui.label(RichText::new(icon).size(16.0));
                                            }
                                        } else {
                                            let opt_img = icons::extract_app_icon(&item_app.display_icon);
                                            let tex_opt = opt_img.map(|img| ui.ctx().load_texture(&item_app.id, img, Default::default()));
                                            if let Some(ref tex) = tex_opt {
                                                ui.image((tex.id(), egui::vec2(22.0, 22.0)));
                                            } else {
                                                let icon = if item_app.is_uwp { "🌐" } else if item_app.is_system_component { "⚙" } else { "📦" };
                                                ui.label(RichText::new(icon).size(16.0));
                                            }
                                            app.icon_textures.insert(item_app.id.clone(), tex_opt);
                                        }
                                    } else {
                                        let icon = if item_app.is_uwp { "🌐" } else if item_app.is_system_component { "⚙" } else { "📦" };
                                        ui.label(RichText::new(icon).size(16.0));
                                    }

                                    if app.settings.highlight_recent && item_app.is_new {
                                        ui.colored_label(Color32::from_rgb(234, 179, 8), "✨ НОВОЕ")
                                            .on_hover_text("Недавно установленная программа");
                                    }

                                    let is_traced = app.traced_apps.iter().any(|t| t.name.eq_ignore_ascii_case(&item_app.name) || t.id == item_app.id);
                                    if is_traced {
                                        ui.label(RichText::new("⚡").color(Color32::from_rgb(234, 179, 8)).strong())
                                            .on_hover_text("Установка отслежена с помощью PurgeCraft Install Tracker");
                                    }

                                    if !item_app.source_badge.is_empty() {
                                        let (badge_text_color, badge_bg) = match item_app.source_badge.as_str() {
                                            "[Portable]" => (Color32::from_rgb(251, 146, 60), if app.dark_mode { Color32::from_rgb(67, 20, 7) } else { Color32::from_rgb(255, 237, 213) }),
                                            "[Steam]" => (Color32::from_rgb(56, 189, 248), if app.dark_mode { Color32::from_rgb(15, 23, 42) } else { Color32::from_rgb(224, 242, 254) }),
                                            "[Epic]" => (Color32::from_rgb(226, 232, 240), if app.dark_mode { Color32::from_rgb(30, 41, 59) } else { Color32::from_rgb(241, 245, 249) }),
                                            "[Store]" => (Color32::from_rgb(45, 212, 191), if app.dark_mode { Color32::from_rgb(4, 47, 46) } else { Color32::from_rgb(204, 251, 241) }),
                                            "[Winget]" => (Color32::from_rgb(192, 132, 252), if app.dark_mode { Color32::from_rgb(46, 16, 77) } else { Color32::from_rgb(243, 232, 255) }),
                                            _ => (Color32::from_rgb(161, 161, 170), if app.dark_mode { Color32::from_rgb(39, 39, 42) } else { Color32::from_rgb(244, 244, 245) }),
                                        };
                                        egui::Frame::none()
                                            .fill(badge_bg)
                                            .rounding(3.0)
                                            .inner_margin(egui::Margin::symmetric(5.0, 1.5))
                                            .show(ui, |ui| {
                                                ui.label(RichText::new(&item_app.source_badge).size(11.0).strong().color(badge_text_color));
                                            });
                                    }

                                    let name_color = if app.settings.highlight_recent && item_app.is_new {
                                        if is_selected {
                                            Color32::WHITE
                                        } else if app.dark_mode {
                                            Color32::from_rgb(250, 204, 21)
                                        } else {
                                            Color32::from_rgb(180, 83, 9)
                                        }
                                    } else if is_selected {
                                        Color32::WHITE
                                    } else if app.dark_mode {
                                        Color32::from_rgb(244, 244, 245)
                                    } else {
                                        Color32::from_rgb(15, 23, 42)
                                    };

                                    let label_widget = egui::Label::new(RichText::new(&item_app.name).strong().color(name_color)).truncate();
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
                                    app_to_uninstall = Some(item_app.clone());
                                }
                                cell_resp.context_menu(|ui| {
                                    show_row_context_menu(ui, item_app, &mut app_to_uninstall, &mut app_to_force, &mut app_to_remove_entry, &mut app_to_dismiss_single, &mut app_to_dismiss_all);
                                });
                                let sz_color = if is_selected { Color32::WHITE } else if app.dark_mode { Color32::from_rgb(212, 212, 216) } else { Color32::from_rgb(51, 65, 85) };
                                ui.label(RichText::new(format_size(item_app.size_bytes)).color(sz_color));
                            });

                            // Column 3: Date - CLICKABLE ANYWHERE + CONTEXT MENU
                            let _ = row.col(|ui| {
                                let cell_resp = ui.interact(ui.max_rect(), ui.id().with(3), egui::Sense::click());
                                if cell_resp.clicked() {
                                    app_to_select = Some(idx);
                                }
                                if cell_resp.double_clicked() {
                                    app_to_uninstall = Some(item_app.clone());
                                }
                                cell_resp.context_menu(|ui| {
                                    show_row_context_menu(ui, item_app, &mut app_to_uninstall, &mut app_to_force, &mut app_to_remove_entry, &mut app_to_dismiss_single, &mut app_to_dismiss_all);
                                });
                                let dt_color = if is_selected { Color32::WHITE } else if app.dark_mode { Color32::from_rgb(161, 161, 170) } else { Color32::from_rgb(100, 116, 139) };
                                let date_str = if item_app.install_date.is_empty() { "—" } else { &item_app.install_date };
                                ui.label(RichText::new(date_str).color(dt_color));
                            });

                            // Column 4: Publisher - CLICKABLE ANYWHERE + CONTEXT MENU
                            let _ = row.col(|ui| {
                                let cell_resp = ui.interact(ui.max_rect(), ui.id().with(4), egui::Sense::click());
                                if cell_resp.clicked() {
                                    app_to_select = Some(idx);
                                }
                                if cell_resp.double_clicked() {
                                    app_to_uninstall = Some(item_app.clone());
                                }
                                cell_resp.context_menu(|ui| {
                                    show_row_context_menu(ui, item_app, &mut app_to_uninstall, &mut app_to_force, &mut app_to_remove_entry, &mut app_to_dismiss_single, &mut app_to_dismiss_all);
                                });
                                let pub_color = if is_selected { Color32::WHITE } else if app.dark_mode { Color32::from_rgb(161, 161, 170) } else { Color32::from_rgb(100, 116, 139) };
                                let pub_str = truncate_chars(if item_app.publisher.is_empty() { "—" } else { &item_app.publisher }, 26);
                                ui.label(RichText::new(pub_str).color(pub_color));
                            });

                            // Column 5: Type - CLICKABLE ANYWHERE + CONTEXT MENU
                            let _ = row.col(|ui| {
                                let cell_resp = ui.interact(ui.max_rect(), ui.id().with(5), egui::Sense::click());
                                if cell_resp.clicked() {
                                    app_to_select = Some(idx);
                                }
                                if cell_resp.double_clicked() {
                                    app_to_uninstall = Some(item_app.clone());
                                }
                                cell_resp.context_menu(|ui| {
                                    show_row_context_menu(ui, item_app, &mut app_to_uninstall, &mut app_to_force, &mut app_to_remove_entry, &mut app_to_dismiss_single, &mut app_to_dismiss_all);
                                });
                                let is_traced = app.traced_apps.iter().any(|t| t.name.eq_ignore_ascii_case(&item_app.name) || t.id == item_app.id);
                                let tag = if is_traced {
                                    "⚡ Отслежено"
                                } else if item_app.is_uwp {
                                    "Store"
                                } else if item_app.is_64bit {
                                    "64-бит"
                                } else if app.settings.mark_32bit {
                                    "32-бит"
                                } else {
                                    "—"
                                };
                                let tag_color = if is_selected { Color32::WHITE } else if app.dark_mode { Color32::from_rgb(161, 161, 170) } else { Color32::from_rgb(100, 116, 139) };
                                ui.label(RichText::new(tag).color(tag_color));
                            });
                        });
                    });
            }

            if let Some(id_to_clear) = app_to_dismiss_single {
                if let Some(a_mut) = app.apps.iter_mut().find(|a| a.id == id_to_clear) {
                    a_mut.is_new = false;
                }
                save_known_apps(&app.apps);
                app.new_apps_count = app.apps.iter().filter(|a| a.is_new).count();
                app.status_msg = "Отметка «Новое» снята.".to_string();
            }
            if app_to_dismiss_all {
                for a in &mut app.apps {
                    a.is_new = false;
                }
                save_known_apps(&app.apps);
                app.new_apps_count = 0;
                app.status_msg = "Все отметки «Новое» сброшены.".to_string();
            }

            if let Some(sel) = app_to_select {
                app.selected_app_idx = Some(sel);
            }
            if let Some(app_to_uninst) = app_to_uninstall {
                app.start_uninstall(app_to_uninst);
            }
            if let Some(app_to_f) = app_to_force {
                app.force_confirm_app = Some(app_to_f);
                app.force_confirm_open = true;
            }
            if let Some(app_to_rem) = app_to_remove_entry {
                app.remove_entry_confirm_app = Some(app_to_rem);
                app.remove_entry_confirm_open = true;
            }
        });
}
