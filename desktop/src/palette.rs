//! Работа с цветом: свой пикер и пользовательская палитра.
//!
//! Главное отличие от веб-версии, где цвет выбирался из семи фиксированных
//! значений. Готовый `egui::color_picker` не используется намеренно: он
//! выглядит как отладочный виджет и ломает оформление.

use crate::theme;
use egui::{
    ecolor::Hsva, Color32, Mesh, Rect, Response, Sense, Shape, Stroke, StrokeKind, Ui, Vec2,
};

/// Цвет, добавленный пользователем в свою палитру.
#[derive(Clone, PartialEq, Debug)]
pub struct Swatch {
    pub name: String,
    pub rgb: [u8; 3],
}

#[derive(Clone, Default, Debug)]
pub struct Custom {
    pub swatches: Vec<Swatch>,
}

impl Custom {
    pub fn add(&mut self, rgb: [u8; 3]) {
        if self.swatches.iter().any(|s| s.rgb == rgb) {
            return;
        }
        self.swatches.push(Swatch {
            name: hex(rgb),
            rgb,
        });
    }
    pub fn remove(&mut self, i: usize) {
        if i < self.swatches.len() {
            self.swatches.remove(i);
        }
    }
}

pub fn hex(rgb: [u8; 3]) -> String {
    format!("#{:02X}{:02X}{:02X}", rgb[0], rgb[1], rgb[2])
}

pub fn parse_hex(s: &str) -> Option<[u8; 3]> {
    let s = s.trim().trim_start_matches('#');
    if s.len() != 6 || !s.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let n = u32::from_str_radix(s, 16).ok()?;
    Some([(n >> 16) as u8, (n >> 8) as u8, n as u8])
}

fn col(rgb: [u8; 3]) -> Color32 {
    Color32::from_rgb(rgb[0], rgb[1], rgb[2])
}

// ---------------------------------------------------------------- пикер

/// Квадрат «насыщенность × яркость». Рисуется сеткой четырёхугольников:
/// у градиента две оси, одним прямоугольником его не задать.
fn sv_square(ui: &mut Ui, hsv: &mut Hsva, size: f32) -> Response {
    let (rect, resp) = ui.allocate_at_least(Vec2::splat(size), Sense::click_and_drag());
    let n = 24;
    let mut mesh = Mesh::default();
    for iy in 0..=n {
        for ix in 0..=n {
            let (s, v) = (ix as f32 / n as f32, 1.0 - iy as f32 / n as f32);
            let c = Color32::from(Hsva::new(hsv.h, s, v, 1.0));
            mesh.colored_vertex(
                egui::pos2(
                    rect.left() + rect.width() * ix as f32 / n as f32,
                    rect.top() + rect.height() * iy as f32 / n as f32,
                ),
                c,
            );
        }
    }
    let w = n + 1;
    for iy in 0..n {
        for ix in 0..n {
            let i = (iy * w + ix) as u32;
            mesh.add_triangle(i, i + 1, i + w as u32);
            mesh.add_triangle(i + 1, i + w as u32 + 1, i + w as u32);
        }
    }
    ui.painter().add(Shape::mesh(mesh));
    ui.painter().rect_stroke(
        rect,
        theme::R_CTRL,
        Stroke::new(1.0, Color32::from_black_alpha(28)),
        StrokeKind::Inside,
    );

    if resp.is_pointer_button_down_on() {
        if let Some(p) = resp.interact_pointer_pos() {
            hsv.s = ((p.x - rect.left()) / rect.width()).clamp(0.0, 1.0);
            hsv.v = (1.0 - (p.y - rect.top()) / rect.height()).clamp(0.0, 1.0);
        }
    }

    let cp = egui::pos2(
        rect.left() + hsv.s * rect.width(),
        rect.top() + (1.0 - hsv.v) * rect.height(),
    );
    ui.painter()
        .circle_stroke(cp, 7.0, Stroke::new(2.0, Color32::WHITE));
    ui.painter()
        .circle_stroke(cp, 7.0, Stroke::new(1.0, Color32::from_black_alpha(90)));
    resp
}

/// Полоса тона.
fn hue_strip(ui: &mut Ui, hsv: &mut Hsva, width: f32) -> Response {
    let (rect, resp) = ui.allocate_at_least(Vec2::new(width, 16.0), Sense::click_and_drag());
    let n = 48;
    let mut mesh = Mesh::default();
    for i in 0..=n {
        let t = i as f32 / n as f32;
        let c = Color32::from(Hsva::new(t, 1.0, 1.0, 1.0));
        let x = rect.left() + rect.width() * t;
        mesh.colored_vertex(egui::pos2(x, rect.top()), c);
        mesh.colored_vertex(egui::pos2(x, rect.bottom()), c);
    }
    for i in 0..n {
        let a = (i * 2) as u32;
        mesh.add_triangle(a, a + 1, a + 2);
        mesh.add_triangle(a + 1, a + 3, a + 2);
    }
    ui.painter().add(Shape::mesh(mesh));
    ui.painter().rect_stroke(
        rect,
        egui::CornerRadius::same(4),
        Stroke::new(1.0, Color32::from_black_alpha(28)),
        StrokeKind::Inside,
    );

    if resp.is_pointer_button_down_on() {
        if let Some(p) = resp.interact_pointer_pos() {
            hsv.h = ((p.x - rect.left()) / rect.width()).clamp(0.0, 1.0);
        }
    }
    let x = rect.left() + hsv.h * rect.width();
    ui.painter().rect_filled(
        Rect::from_center_size(
            egui::pos2(x, rect.center().y),
            Vec2::new(4.0, rect.height() + 6.0),
        ),
        2.0,
        Color32::WHITE,
    );
    ui.painter().rect_stroke(
        Rect::from_center_size(
            egui::pos2(x, rect.center().y),
            Vec2::new(4.0, rect.height() + 6.0),
        ),
        egui::CornerRadius::same(2),
        Stroke::new(1.0, Color32::from_black_alpha(80)),
        StrokeKind::Outside,
    );
    resp
}

/// Кружок-образец. Возвращает клик.
pub fn swatch(ui: &mut Ui, rgb: [u8; 3], selected: bool, size: f32) -> Response {
    let (rect, resp) = ui.allocate_at_least(Vec2::splat(size), Sense::click());
    let c = rect.center();
    ui.painter().circle_filled(c, size / 2.0 - 1.0, col(rgb));
    ui.painter().circle_stroke(
        c,
        size / 2.0 - 1.0,
        Stroke::new(1.0, Color32::from_black_alpha(30)),
    );
    if selected {
        ui.painter()
            .circle_stroke(c, size / 2.0 + 2.0, Stroke::new(2.0, theme::ACCENT));
    }
    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    resp
}

/// Полный блок выбора цвета: пресеты, своя палитра, пикер и поле HEX.
/// Возвращает выбранное значение токена (буква палитры или `#RRGGBB`).
pub fn picker(
    ui: &mut Ui,
    hsv: &mut Hsva,
    hex_field: &mut String,
    custom: &mut Custom,
    current: &str,
    width: f32,
) -> Option<String> {
    let mut chosen: Option<String> = None;

    crate::ui::eyebrow(ui, "Палитра Undertale", theme::INK_3);
    ui.add_space(6.0);
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
        for (letter, name, c) in theme::PALETTE {
            let rgb = [c.r(), c.g(), c.b()];
            let sel = current.len() == 1 && current.starts_with(letter);
            if swatch(ui, rgb, sel, 22.0).on_hover_text(name).clicked() {
                chosen = Some(letter.to_string());
                *hsv = Hsva::from(col(rgb));
                *hex_field = hex(rgb);
            }
        }
    });

    ui.add_space(12.0);
    crate::ui::eyebrow(ui, "Своя палитра", theme::INK_3);
    ui.add_space(6.0);
    let mut remove: Option<usize> = None;
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
        for (i, s) in custom.swatches.iter().enumerate() {
            let sel = current.eq_ignore_ascii_case(&hex(s.rgb));
            let r = swatch(ui, s.rgb, sel, 22.0)
                .on_hover_text(format!("{}\nправый клик — убрать", s.name));
            if r.clicked() {
                chosen = Some(hex(s.rgb));
                *hsv = Hsva::from(col(s.rgb));
                *hex_field = hex(s.rgb);
            }
            if r.secondary_clicked() {
                remove = Some(i);
            }
        }
        if custom.swatches.is_empty() {
            ui.label(
                egui::RichText::new("пусто — добавь цвет ниже")
                    .font(theme::font::chip())
                    .color(theme::INK_4),
            );
        }
    });
    if let Some(i) = remove {
        custom.remove(i);
    }

    ui.add_space(12.0);
    sv_square(ui, hsv, width);
    ui.add_space(8.0);
    hue_strip(ui, hsv, width);
    ui.add_space(10.0);

    let rgb_now = {
        let c = Color32::from(*hsv);
        [c.r(), c.g(), c.b()]
    };

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        let te = ui.add(
            egui::TextEdit::singleline(hex_field)
                .desired_width(96.0)
                .font(theme::font::code()),
        );
        if te.changed() {
            if let Some(rgb) = parse_hex(hex_field) {
                *hsv = Hsva::from(col(rgb));
            }
        }
        if crate::ui::solid_button(ui, "Взять").clicked() {
            chosen = Some(hex(rgb_now));
            *hex_field = hex(rgb_now);
        }
        if crate::ui::light_button(ui, "В палитру").clicked() {
            custom.add(rgb_now);
        }
        swatch(ui, rgb_now, false, 26.0);
    });

    chosen
}
