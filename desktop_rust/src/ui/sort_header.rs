use eframe::egui::{self, Color32, Stroke, Vec2};
use crate::types::SortOrder;

pub fn render_sort_header(
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
