//! Оболочка приложения.
//!
//! Раскладка панельная, а не страничная: у окна фиксированная высота, поэтому
//! шапка и лента токенов прижаты к краям, а редактор занимает всё остальное.
//! Большой заголовок с веб-страницы сюда не переносится — в инструменте он
//! съедал бы высоту, которая нужна для работы.

use crate::doc::{Doc, Node, Role};
use crate::editor;
use crate::theme::{self, font};
use crate::tokens::{self, Kind};
use crate::ui;
use egui::{Align, Frame, Layout, Margin, RichText};

pub struct App {
    doc: Doc,
    ed: editor::State,
    sound_on: bool,
}

impl Default for App {
    fn default() -> Self {
        Self { doc: demo_doc(), ed: editor::State::default(), sound_on: true }
    }
}

/// Стартовая реплика — показывает возможности сразу, без пустого экрана.
fn demo_doc() -> Doc {
    Doc::from_nodes(vec![
        Node::token(Kind::Voice, "L", Role::Open),
        Node::text("Привет"),
        Node::token(Kind::Pause, "3", Role::Open),
        Node::text(" как дела?"),
        Node::token(Kind::Newline, "", Role::Open),
        Node::text("Я "),
        Node::token(Kind::Color, "R", Role::Open),
        Node::text("очень"),
        Node::token(Kind::Reset, "", Role::Open),
        Node::text(" рад "),
        Node::token(Kind::Shake, "2", Role::Open),
        Node::text("тебя видеть"),
        Node::token(Kind::Shake, "2", Role::Close),
        Node::token(Kind::Advance, "", Role::Open),
    ])
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        theme::install(&cc.egui_ctx);
        Self::default()
    }

    // ------------------------------------------------------------ шапка

    fn header(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("Dialoging")
                    .family(egui::FontFamily::Name("semibold".into()))
                    .size(15.0)
                    .color(theme::INK),
            );
            ui.add_space(2.0);
            ui.label(
                RichText::new(theme::eyebrow_text("Редактор диалогов"))
                    .font(font::eyebrow())
                    .color(theme::INK_3),
            );

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui::ghost(ui, "Очистить", false).clicked() {
                    self.doc = Doc::default();
                }
                let label = if self.sound_on { "Звук" } else { "Тихо" };
                if ui::ghost(ui, label, self.sound_on).clicked() {
                    self.sound_on = !self.sound_on;
                }
                ui.add_space(12.0);
                ui.label(
                    RichText::new(theme::eyebrow_text("токенов"))
                        .font(font::eyebrow())
                        .color(theme::INK_3),
                );
                ui.add_space(5.0);
                ui.label(
                    RichText::new(self.doc.token_count().to_string())
                        .family(egui::FontFamily::Name("semibold".into()))
                        .size(15.0)
                        .color(theme::INK),
                );
            });
        });
    }

    // ------------------------------------------------------------ лента токенов

    fn token_bar(&mut self, ui: &mut egui::Ui) {
        for (gi, group) in tokens::GROUPS.iter().enumerate() {
            if gi > 0 {
                ui.add_space(7.0);
            }
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(7.0, 7.0);
                let (lr, _) = ui.allocate_exact_size(egui::vec2(72.0, 38.0), egui::Sense::hover());
                let g = ui.painter().layout_no_wrap(
                    theme::eyebrow_text(group.title()),
                    font::eyebrow(),
                    theme::INK_3,
                );
                let gy = lr.center().y - g.size().y / 2.0;
                ui.painter().galley(egui::pos2(lr.left(), gy), g, theme::INK_3);

                for kind in tokens::ALL {
                    if tokens::spec(kind).group != *group {
                        continue;
                    }
                    if ui::token_button(ui, kind).clicked() {
                        let sp = tokens::spec(kind);
                        let v = sp.default.unwrap_or("");
                        if sp.wrap {
                            self.doc.wrap_with(kind, v, None);
                        } else {
                            self.doc.insert_token(kind, v);
                        }
                    }
                }
            });
        }
    }

    // ------------------------------------------------------------ вывод

    fn output(&mut self, ui: &mut egui::Ui) {
        if self.doc.serialize().is_empty() {
            ui.label(RichText::new("—").font(font::code()).color(theme::INK_4));
            return;
        }
        // Команды подсвечены, обычный текст чёрный — как в веб-версии.
        let mut job = egui::text::LayoutJob::default();
        for n in &self.doc.nodes {
            let (s, col) = match n {
                Node::Text(t) => (t.clone(), theme::INK),
                Node::Token { kind, value, role } => (
                    match role {
                        Role::Open => tokens::code(*kind, value),
                        Role::Close => tokens::end_code(*kind),
                    },
                    theme::ACCENT_INK,
                ),
            };
            job.append(
                &s,
                0.0,
                egui::TextFormat { font_id: font::code(), color: col, ..Default::default() },
            );
        }
        job.wrap.max_width = ui.available_width();
        ui.label(job);
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        ui.painter().rect_filled(ui.max_rect(), 0.0, theme::BG);

        let bg = |t: i8, b: i8| {
            Frame::new()
                .fill(theme::BG)
                .inner_margin(Margin { left: 22, right: 22, top: t, bottom: b })
        };

        egui::Panel::top("head")
            .frame(bg(16, 13))
            .show_separator_line(false)
            .show(ui, |ui| self.header(ui));

        egui::Panel::bottom("tokbar")
            .frame(bg(14, 16))
            .show_separator_line(false)
            .resizable(false)
            .show(ui, |ui| {
                theme::panel_frame()
                    .inner_margin(Margin::symmetric(18, 15))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        self.token_bar(ui);
                    });
            });

        egui::Panel::right("side")
            .frame(bg(2, 0))
            .default_size(450.0)
            .min_size(320.0)
            .show_separator_line(false)
            .show(ui, |ui| {
                ui::panel(ui, "Вывод", None, Some("02"), |ui| {
                    ui.set_min_height(52.0);
                    self.output(ui);
                });
                ui.add_space(16.0);
                ui::panel(ui, "Превью", Some("готово"), Some("03"), |ui| {
                    let h = (ui.available_height() - 24.0).max(150.0);
                    let (rect, _) = ui.allocate_exact_size(
                        egui::vec2(ui.available_width(), h),
                        egui::Sense::hover(),
                    );
                    ui.painter().rect_filled(rect, 3.0, egui::Color32::BLACK);
                    ui.painter().rect_stroke(
                        rect,
                        3.0,
                        egui::Stroke::new(5.0, theme::WHITE),
                        egui::StrokeKind::Outside,
                    );
                });
            });

        egui::CentralPanel::default()
            .frame(bg(2, 0))
            .show(ui, |ui| {
                ui::panel(ui, "Ввод", Some("реплика"), Some("01"), |ui| {
                    let (_, act) = editor::show(ui, &mut self.doc, &mut self.ed);
                    if let editor::Action::EditChip(_n) = act {
                        // выбор значения появится следующим шагом
                    }
                });
            });
    }
}
