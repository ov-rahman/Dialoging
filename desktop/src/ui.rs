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
            ui.painter().hline(
                r.x_range(),
                r.top(),
                egui::Stroke::new(1.0, theme::LINE_2),
            );
            ui.add_space(1.0);

            let out = egui::Frame::new()
                .inner_margin(egui::Margin::same(18))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    body(ui)
                })
                .inner;
            out
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
