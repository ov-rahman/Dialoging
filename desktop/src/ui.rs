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
