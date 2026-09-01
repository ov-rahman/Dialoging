//! Оболочка приложения: шапка, заголовок, две колонки с панелями.

use crate::theme::{self, font};
use crate::ui;
use egui::{Align, Layout, RichText};

pub struct App {
    sound_on: bool,
    token_count: usize,
}

impl Default for App {
    fn default() -> Self {
        Self {
            sound_on: true,
            token_count: 0,
        }
    }
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        theme::install(&cc.egui_ctx);
        Self::default()
    }

    fn header(&mut self, ui: &mut egui::Ui) {
        ui.add_space(18.0);
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("Dialoging")
                    .family(egui::FontFamily::Name("semibold".into()))
                    .size(15.0)
                    .color(theme::INK),
            );
            ui.add_space(2.0);
            ui.label(
                RichText::new(theme::eyebrow_text("Редактор"))
                    .font(font::eyebrow())
                    .color(theme::INK_3),
            );

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui::ghost(ui, "Очистить", false).clicked() {
                    self.token_count = 0;
                }
                let label = if self.sound_on { "Звук" } else { "Тихо" };
                if ui::ghost(ui, label, self.sound_on).clicked() {
                    self.sound_on = !self.sound_on;
                }
            });
        });
        ui.add_space(12.0);
        let r = ui.available_rect_before_wrap();
        ui.painter()
            .hline(r.x_range(), r.top(), egui::Stroke::new(1.0, theme::LINE));
    }

    fn masthead(&mut self, ui: &mut egui::Ui) {
        ui.add_space(24.0);
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.label(
                    RichText::new("Пиши как текст,")
                        .font(font::display())
                        .color(theme::INK),
                );
                ui.add_space(-6.0);
                ui.label(
                    RichText::new("получай разметку")
                        .font(font::display())
                        .color(theme::INK_3),
                );
            });
            ui.with_layout(Layout::right_to_left(Align::TOP), |ui| {
                ui.vertical(|ui| {
                    ui.with_layout(Layout::top_down(Align::Max), |ui| {
                        ui.label(
                            RichText::new(self.token_count.to_string())
                                .font(font::counter())
                                .color(theme::INK),
                        );
                        ui.label(
                            RichText::new(theme::eyebrow_text("токенов"))
                                .font(font::eyebrow())
                                .color(theme::INK_3),
                        );
                    });
                });
            });
        });
        ui.add_space(20.0);
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::Frame::new()
            .fill(theme::BG)
            .inner_margin(egui::Margin::symmetric(22, 0))
            .show(ui, |ui| {
                ui.set_min_size(ui.available_size());
                self.header(ui);
                self.masthead(ui);

                let full = ui.available_width();
                let gap = 20.0;
                let left_w = ((full - gap) * 0.52).max(320.0);

                ui.horizontal_top(|ui| {
                    ui.allocate_ui_with_layout(
                        egui::vec2(left_w, ui.available_height()),
                        Layout::top_down(Align::Min),
                        |ui| {
                            ui::panel(ui, "Ввод", Some("жми иконки снизу"), Some("01"), |ui| {
                                ui.set_min_height(120.0);
                                ui.label(
                                    RichText::new("Здесь будет редактор с чипами")
                                        .font(font::editor())
                                        .color(theme::INK_4),
                                );
                            });
                            ui.add_space(16.0);
                            ui::panel(ui, "Токены", None, None, |ui| {
                                ui.set_min_height(80.0);
                                ui.label(
                                    RichText::new("Панель токенов")
                                        .font(font::body())
                                        .color(theme::INK_4),
                                );
                            });
                        },
                    );

                    ui.add_space(gap);

                    ui.allocate_ui_with_layout(
                        egui::vec2(ui.available_width(), ui.available_height()),
                        Layout::top_down(Align::Min),
                        |ui| {
                            ui::panel(ui, "Вывод", None, Some("02"), |ui| {
                                ui.set_min_height(60.0);
                                ui.label(
                                    RichText::new("\\TLПривет^3 как дела?")
                                        .font(font::code())
                                        .color(theme::ACCENT_INK),
                                );
                            });
                            ui.add_space(16.0);
                            ui::panel(ui, "Превью", Some("готово"), Some("03"), |ui| {
                                let (rect, _) = ui.allocate_exact_size(
                                    egui::vec2(ui.available_width(), 150.0),
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
                        },
                    );
                });
            });
    }
}
