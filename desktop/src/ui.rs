//! Мелкие переиспользуемые куски интерфейса: лейблы, карточки, кнопки.
//! Всё рисуется вручную, потому что стандартные виджеты egui выглядят
//! как отладочная панель, а не как утверждённый макет.

use crate::theme::{self, font};
use egui::{Align, Layout, Response, RichText, Sense, Ui, Vec2};

/// Микро-лейбл капсом с трекингом: «ВВОД», «ТАЙМИНГ», «01».
pub fn eyebrow(ui: &mut Ui, text: &str, color: egui::Color32) -> Response {
    ui.label(
        RichText::new(theme::eyebrow_text(text))
            .font(font::eyebrow())
            .color(color),
    )
}

/// Заголовок панели: слева название и подпись, справа — короткая метка.
pub fn panel_head(ui: &mut Ui, title: &str, hint: Option<&str>, right: Option<&str>) {
    ui.horizontal(|ui| {
        ui.add_space(18.0);
        eyebrow(ui, title, theme::INK_2);
        if let Some(h) = hint {
            ui.add_space(2.0);
            eyebrow(ui, h, theme::INK_3);
        }
        if let Some(r) = right {
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add_space(18.0);
                eyebrow(ui, r, theme::INK_3);
            });
        }
    });
}

/// Панель-карточка с заголовком и произвольным содержимым.
pub fn panel<R>(
    ui: &mut Ui,
    title: &str,
    hint: Option<&str>,
    right: Option<&str>,
    body: impl FnOnce(&mut Ui) -> R,
) -> R {
    theme::panel_frame()
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.add_space(14.0);
            panel_head(ui, title, hint, right);
            ui.add_space(12.0);

            // Разделитель во всю ширину карточки.
            let r = ui.available_rect_before_wrap();
            ui.painter()
                .hline(r.x_range(), r.top(), egui::Stroke::new(1.0, theme::LINE_2));
            ui.add_space(1.0);

            egui::Frame::new()
                .inner_margin(egui::Margin::same(18))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    body(ui)
                })
                .inner
        })
        .inner
}

/// Кнопка-«призрак» в шапке: без фона, подсвечивается при наведении.
pub fn ghost(ui: &mut Ui, text: &str, active: bool) -> Response {
    let galley = ui.painter().layout_no_wrap(
        text.to_owned(),
        font::body(),
        if active { theme::WHITE } else { theme::INK_2 },
    );
    let size = Vec2::new(galley.size().x + 22.0, 30.0);
    let (rect, resp) = ui.allocate_at_least(size, Sense::click());

    let hovered = resp.hovered();
    if active || hovered {
        ui.painter().rect_filled(
            rect,
            theme::R_CTRL,
            if active { theme::INK } else { theme::WHITE },
        );
    }
    if hovered && !active {
        ui.painter().rect_stroke(
            rect,
            theme::R_CTRL,
            egui::Stroke::new(1.0, theme::LINE_2),
            egui::StrokeKind::Inside,
        );
    }
    let pos = rect.center() - galley.size() / 2.0;
    ui.painter().galley(pos, galley, theme::INK);
    resp
}

/// Кнопка токена в панели: иконка, подпись, мягкая тень, подъём при наведении.
pub fn token_button(ui: &mut Ui, kind: crate::tokens::Kind) -> Response {
    use crate::icons;
    let name = crate::tokens::spec(kind).name;
    let galley = ui
        .painter()
        .layout_no_wrap(name.to_owned(), font::body(), theme::INK);

    let icon = 17.0;
    let size = Vec2::new(12.0 + icon + 7.0 + galley.size().x + 12.0, 38.0);
    let (rect, resp) = ui.allocate_at_least(size, Sense::click());

    let lift = if resp.is_pointer_button_down_on() {
        0.0
    } else if resp.hovered() {
        -1.5
    } else {
        0.0
    };
    let r = rect.translate(egui::vec2(0.0, lift));

    let shadow = if resp.hovered() {
        theme::shadow_card()
    } else {
        theme::shadow_control()
    };
    ui.painter().add(shadow.as_shape(r, theme::R_CTRL));
    ui.painter().rect(
        r,
        theme::R_CTRL,
        theme::WHITE,
        egui::Stroke::new(1.0, theme::LINE_2),
        egui::StrokeKind::Inside,
    );

    let icon_rect = egui::Rect::from_center_size(
        egui::pos2(r.left() + 12.0 + icon / 2.0, r.center().y),
        Vec2::splat(icon),
    );
    icons::draw_token(ui.painter(), icon_rect, kind, theme::INK, 1.6);

    let tp = egui::pos2(
        r.left() + 12.0 + icon + 7.0,
        r.center().y - galley.size().y / 2.0,
    );
    ui.painter().galley(tp, galley, theme::INK);

    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    resp
}

/// Тёмная кнопка-действие.
pub fn solid_button(ui: &mut Ui, text: &str) -> Response {
    button_impl(ui, text, theme::INK, theme::WHITE, None)
}

/// Светлая кнопка с рамкой.
pub fn light_button(ui: &mut Ui, text: &str) -> Response {
    button_impl(ui, text, theme::WHITE, theme::INK, Some(theme::LINE_2))
}

fn button_impl(
    ui: &mut Ui,
    text: &str,
    fill: egui::Color32,
    ink: egui::Color32,
    border: Option<egui::Color32>,
) -> Response {
    let galley = ui
        .painter()
        .layout_no_wrap(text.to_owned(), font::body(), ink);
    let size = Vec2::new(galley.size().x + 26.0, 34.0);
    let (rect, resp) = ui.allocate_at_least(size, Sense::click());
    let r = if resp.hovered() && !resp.is_pointer_button_down_on() {
        rect.translate(egui::vec2(0.0, -1.0))
    } else {
        rect
    };
    ui.painter()
        .add(theme::shadow_control().as_shape(r, theme::R_CTRL));
    ui.painter().rect(
        r,
        theme::R_CTRL,
        fill,
        egui::Stroke::new(1.0, border.unwrap_or(fill)),
        egui::StrokeKind::Inside,
    );
    ui.painter()
        .galley(r.center() - galley.size() / 2.0, galley, ink);
    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    resp
}

/// Кнопка-вариант в поповере значения.
pub fn chip_option(ui: &mut Ui, text: &str, on: bool) -> Response {
    let ink = if on { theme::WHITE } else { theme::INK };
    let galley = ui
        .painter()
        .layout_no_wrap(text.to_owned(), font::body(), ink);
    let size = Vec2::new(galley.size().x.max(16.0) + 20.0, 32.0);
    let (rect, resp) = ui.allocate_at_least(size, Sense::click());
    let fill = if on {
        theme::INK
    } else if resp.hovered() {
        theme::WHITE
    } else {
        theme::CARD
    };
    ui.painter().rect(
        rect,
        theme::R_CHIP,
        fill,
        egui::Stroke::new(1.0, if on { theme::INK } else { theme::LINE_2 }),
        egui::StrokeKind::Inside,
    );
    ui.painter()
        .galley(rect.center() - galley.size() / 2.0, galley, ink);
    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    resp
}

/// Текстовая кнопка удаления.
pub fn danger_link(ui: &mut Ui, text: &str) -> Response {
    let red = egui::Color32::from_rgb(0xC0, 0x39, 0x2B);
    let galley = ui
        .painter()
        .layout_no_wrap(text.to_owned(), font::body(), red);
    let size = Vec2::new(galley.size().x + 18.0, 30.0);
    let (rect, resp) = ui.allocate_at_least(size, Sense::click());
    if resp.hovered() {
        ui.painter().rect_filled(
            rect,
            theme::R_CHIP,
            egui::Color32::from_rgba_unmultiplied(192, 57, 43, 22),
        );
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    ui.painter()
        .galley(rect.center() - galley.size() / 2.0, galley, red);
    resp
}

/// Тёмная кнопка с иконкой воспроизведения или остановки.
pub fn icon_button(ui: &mut Ui, text: &str, stop: bool) -> Response {
    use crate::icons;
    let galley = ui
        .painter()
        .layout_no_wrap(text.to_owned(), font::body(), theme::WHITE);
    let size = Vec2::new(galley.size().x + 15.0 + 7.0 + 26.0, 36.0);
    let (rect, resp) = ui.allocate_at_least(size, Sense::click());
    let r = if resp.hovered() && !resp.is_pointer_button_down_on() {
        rect.translate(egui::vec2(0.0, -1.0))
    } else {
        rect
    };
    ui.painter()
        .add(theme::shadow_control().as_shape(r, theme::R_CTRL));
    ui.painter().rect_filled(r, theme::R_CTRL, theme::INK);
    let ir = egui::Rect::from_center_size(
        egui::pos2(r.left() + 13.0 + 7.5, r.center().y),
        Vec2::splat(15.0),
    );
    icons::draw_ui(
        ui.painter(),
        ir,
        if stop {
            icons::Ui::Stop
        } else {
            icons::Ui::Play
        },
        theme::WHITE,
        1.7,
    );
    ui.painter().galley(
        egui::pos2(
            r.left() + 13.0 + 15.0 + 7.0,
            r.center().y - galley.size().y / 2.0,
        ),
        galley,
        theme::WHITE,
    );
    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    resp
}
