//! Оболочка приложения.
//!
//! Раскладка панельная, а не страничная: у окна фиксированная высота, поэтому
//! шапка и лента токенов прижаты к краям, а редактор занимает всё остальное.
//! Большой заголовок с веб-страницы сюда не переносится — в инструменте он
//! съедал бы высоту, которая нужна для работы.

use crate::audio::Audio;
use crate::doc::{Doc, Node, Role};
use crate::editor;
use crate::palette::{self, Custom};
use crate::player::{self, Player};
use crate::theme::{self, font};
use crate::tokens::{self, Kind};
use crate::ui;
use egui::{Align, Frame, Layout, Margin, RichText};

/// Открытый выбор значения токена.
struct Picker {
    kind: Kind,
    /// Узел, который правим. `None` — токен ещё не вставлен.
    target: Option<usize>,
    input: String,
    warn: Option<&'static str>,
    /// Спорное значение подтверждается вторым нажатием, а не запрещается.
    warned: bool,
    pos: egui::Pos2,
    hsv: egui::ecolor::Hsva,
    hex: String,
    /// Первый кадр после открытия: тот же клик, что открыл поповер, иначе
    /// был бы засчитан как «клик мимо» и закрыл бы его немедленно.
    fresh: bool,
}

pub struct App {
    doc: Doc,
    ed: editor::State,
    picker: Option<Picker>,
    custom: Custom,
    player: Player,
    audio: Audio,
    sound_on: bool,
    /// Автозапуск проигрывания — только для снятия скриншотов при разработке.
    dev_autoplay: bool,
}

impl Default for App {
    fn default() -> Self {
        Self {
            doc: demo_doc(),
            ed: editor::State::default(),
            picker: None,
            custom: Custom::default(),
            player: Player::default(),
            audio: Audio::new(),
            sound_on: true,
            dev_autoplay: false,
        }
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
        let mut app = Self {
            dev_autoplay: std::env::var("DIALOGING_DEV_PLAY").is_ok(),
            ..Self::default()
        };
        // Только для снятия скриншотов при разработке.
        if let Ok(k) = std::env::var("DIALOGING_DEV_PICKER") {
            let kind = match k.as_str() {
                "color" => Kind::Color,
                "speed" => Kind::Speed,
                _ => Kind::Pause,
            };
            app.open_picker(kind, None, egui::pos2(60.0, 120.0));
        }
        app
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
                let has_device = self.audio.available();
                let label = if !has_device {
                    "Нет устройства"
                } else if self.sound_on {
                    "Звук"
                } else {
                    "Тихо"
                };
                let r = ui::ghost(ui, label, self.sound_on && has_device);
                if r.clicked() && has_device {
                    self.sound_on = !self.sound_on;
                    self.audio.enabled = self.sound_on;
                }
                if !has_device {
                    r.on_hover_text("Звуковое устройство не найдено");
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
        // Один переносящийся поток вместо строки на группу: шесть отдельных
        // рядов съедали высоту, которая нужна редактору и превью.
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(7.0, 7.0);
            for (gi, group) in tokens::GROUPS.iter().enumerate() {
                if gi > 0 {
                    let (r, _) =
                        ui.allocate_exact_size(egui::vec2(1.0, 26.0), egui::Sense::hover());
                    ui.painter().vline(
                        r.center().x,
                        r.y_range(),
                        egui::Stroke::new(1.0, theme::LINE),
                    );
                }
                let g = ui.painter().layout_no_wrap(
                    theme::eyebrow_text(group.title()),
                    font::eyebrow(),
                    theme::INK_3,
                );
                let (lr, _) = ui
                    .allocate_exact_size(egui::vec2(g.size().x + 2.0, 38.0), egui::Sense::hover());
                let gy = lr.center().y - g.size().y / 2.0;
                ui.painter()
                    .galley(egui::pos2(lr.left(), gy), g, theme::INK_3);

                for kind in tokens::ALL {
                    if tokens::spec(kind).group != *group {
                        continue;
                    }
                    let btn = ui::token_button(ui, kind);
                    if btn.clicked() {
                        let sp = tokens::spec(kind);
                        if sp.pick_on_insert {
                            let p = btn.rect.left_top() + egui::vec2(0.0, -8.0);
                            self.open_picker(kind, None, p);
                        } else {
                            let v = sp.default.unwrap_or("");
                            if sp.wrap {
                                let sel = self.ed.selection_for(&self.doc);
                                self.doc.wrap_with(kind, v, sel);
                            } else {
                                self.doc.insert_token(kind, v);
                            }
                        }
                    }
                }
            }
        });
    }

    // ------------------------------------------------------------ выбор значения

    fn open_picker(&mut self, kind: Kind, target: Option<usize>, pos: egui::Pos2) {
        let cur = match target.and_then(|i| self.doc.nodes.get(i)) {
            Some(Node::Token { value, .. }) => value.clone(),
            _ => tokens::spec(kind).default.unwrap_or("").to_owned(),
        };
        let rgb = tokens::color_rgb(&cur).unwrap_or([0xE4, 0x48, 0x3C]);
        self.picker = Some(Picker {
            kind,
            target,
            input: cur.clone(),
            warn: None,
            warned: false,
            pos,
            hsv: egui::ecolor::Hsva::from(egui::Color32::from_rgb(rgb[0], rgb[1], rgb[2])),
            hex: palette::hex(rgb),
            fresh: true,
        });
    }

    /// Ставит значение: правит существующий чип или вставляет новый токен.
    fn apply_value(&mut self, kind: Kind, target: Option<usize>, value: &str) {
        match target {
            Some(i) => self.doc.set_value(i, value),
            None => {
                if tokens::spec(kind).wrap {
                    let sel = self.ed.selection_for(&self.doc);
                    self.doc.wrap_with(kind, value, sel);
                } else {
                    self.doc.insert_token(kind, value);
                }
            }
        }
        self.picker = None;
    }

    fn picker_ui(&mut self, ctx: &egui::Context) {
        let Some(p) = self.picker.as_ref() else {
            return;
        };
        let (kind, target, pos) = (p.kind, p.target, p.pos);
        let sp = tokens::spec(kind);
        let is_color = kind == Kind::Color;
        let width = if is_color { 250.0 } else { 236.0 };

        let mut close = false;
        let mut apply: Option<String> = None;
        let mut submit = false;
        let mut delete = false;

        let area = egui::Area::new(egui::Id::new("token-picker"))
            .order(egui::Order::Foreground)
            .fixed_pos(pos)
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(theme::WHITE)
                    .stroke(egui::Stroke::new(1.0, theme::LINE_2))
                    .corner_radius(theme::R_PANEL)
                    .shadow(theme::shadow_pop())
                    .inner_margin(Margin::same(14))
                    .show(ui, |ui| {
                        ui.set_width(width);
                        ui::eyebrow(ui, sp.name, theme::INK_3);
                        ui.add_space(9.0);

                        if is_color {
                            let cur = self
                                .picker
                                .as_ref()
                                .map(|p| p.input.clone())
                                .unwrap_or_default();
                            let mut hsv = self.picker.as_ref().unwrap().hsv;
                            let mut hexf = self.picker.as_ref().unwrap().hex.clone();
                            if let Some(v) = palette::picker(
                                ui,
                                &mut hsv,
                                &mut hexf,
                                &mut self.custom,
                                &cur,
                                width,
                            ) {
                                apply = Some(v);
                            }
                            if let Some(p) = self.picker.as_mut() {
                                p.hsv = hsv;
                                p.hex = hexf;
                            }
                        } else {
                            if !sp.presets.is_empty() {
                                let cur = self.picker.as_ref().unwrap().input.clone();
                                ui.horizontal_wrapped(|ui| {
                                    ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                                    for v in sp.presets {
                                        let on = cur == *v;
                                        if ui::chip_option(ui, v, on).clicked() {
                                            apply = Some((*v).to_owned());
                                        }
                                    }
                                });
                                ui.add_space(9.0);
                            }
                            if let Some(free) = sp.free {
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 6.0;
                                    let input = &mut self.picker.as_mut().unwrap().input;
                                    let te = ui.add(
                                        egui::TextEdit::singleline(input)
                                            .desired_width(width - 96.0)
                                            .hint_text("своё значение")
                                            .font(font::body()),
                                    );
                                    let enter = te.lost_focus()
                                        && ui.input(|i| i.key_pressed(egui::Key::Enter));
                                    if ui::solid_button(ui, "OK").clicked() || enter {
                                        submit = true;
                                    }
                                });
                                ui.add_space(6.0);
                                ui.label(
                                    RichText::new(theme::eyebrow_text(free.hint))
                                        .font(font::eyebrow())
                                        .color(theme::INK_3),
                                );
                            }
                        }

                        if let Some(w) = self.picker.as_ref().and_then(|p| p.warn) {
                            ui.add_space(7.0);
                            ui.label(
                                RichText::new(format!(
                                    "{w} Нажми OK ещё раз, чтобы поставить как есть."
                                ))
                                .font(font::chip())
                                .color(theme::WARN),
                            );
                        }

                        if target.is_some() {
                            ui.add_space(10.0);
                            let r = ui.available_rect_before_wrap();
                            ui.painter().hline(
                                r.x_range(),
                                r.top(),
                                egui::Stroke::new(1.0, theme::LINE_2),
                            );
                            ui.add_space(8.0);
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if ui::danger_link(ui, "Удалить").clicked() {
                                    delete = true;
                                }
                            });
                        }
                    });
            });

        // Клик, которым поповер открыли, не должен его же и закрыть.
        if let Some(p) = self.picker.as_mut() {
            if p.fresh {
                p.fresh = false;
                return;
            }
        }

        // клик мимо поповера закрывает его
        let clicked_outside = ctx.input(|i| i.pointer.any_click())
            && !area
                .response
                .rect
                .contains(ctx.pointer_interact_pos().unwrap_or(egui::Pos2::ZERO));
        if clicked_outside || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            close = true;
        }

        if delete {
            if let Some(i) = target {
                self.doc.remove_pair(i);
            }
            self.picker = None;
            return;
        }
        if submit {
            self.submit_free(kind, target);
            return;
        }
        if let Some(v) = apply {
            self.apply_value(kind, target, &v);
            return;
        }
        if close {
            self.picker = None;
        }
    }

    /// Своё значение: нормализуем, проверяем, при спорном предупреждаем,
    /// но по второму нажатию ставим как есть.
    fn submit_free(&mut self, kind: Kind, target: Option<usize>) {
        let Some(p) = self.picker.as_mut() else {
            return;
        };
        let raw = p.input.clone();
        let Some(v) = tokens::normalize(kind, &raw) else {
            p.warn = Some("Введи значение.");
            p.warned = false;
            return;
        };
        if let Err(w) = tokens::validate(kind, &v) {
            if !p.warned {
                p.warn = Some(w);
                p.warned = true;
                return;
            }
        }
        self.apply_value(kind, target, &v);
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
                egui::TextFormat {
                    font_id: font::code(),
                    color: col,
                    ..Default::default()
                },
            );
        }
        job.wrap.max_width = ui.available_width();
        ui.label(job);
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        ui.painter().rect_filled(ui.max_rect(), 0.0, theme::BG);

        let ctx = ui.ctx().clone();
        let now = ctx.input(|i| i.time);
        if self.dev_autoplay && now > 0.4 {
            self.dev_autoplay = false;
            self.player.start(&self.doc, now);
        }
        let spoken = self.player.step(now);
        if spoken > 0 {
            self.audio.tick(&self.player.last_voice, spoken);
        }
        for name in std::mem::take(&mut self.player.pending_sfx) {
            self.audio.event(&name);
        }
        if self.player.animating() {
            ctx.request_repaint();
        }

        let bg = |t: i8, b: i8| {
            Frame::new().fill(theme::BG).inner_margin(Margin {
                left: 22,
                right: 22,
                top: t,
                bottom: b,
            })
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
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui::panel(ui, "Вывод", None, Some("02"), |ui| {
                            ui.set_min_height(52.0);
                            self.output(ui);
                        });
                        ui.add_space(16.0);
                        let status = self.player.status();
                        ui::panel(ui, "Превью", Some(status), Some("03"), |ui| {
                            // Высота стенда — остаток панели за вычетом кнопок и полей;
                            // ниже минимума включается прокрутка, а не обрезка.
                            let h = (ui.available_height() - 74.0).max(120.0);
                            if player::stage(ui, &self.player, h) {
                                self.player.advance(now);
                            }
                            ui.add_space(14.0);
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 8.0;
                                let running = self.player.is_running();
                                let label = if running { "Стоп" } else { "Играть" };
                                if ui::icon_button(ui, label, running).clicked() {
                                    if running {
                                        self.player.reset();
                                    } else {
                                        self.player.start(&self.doc, now);
                                    }
                                }
                                if ui::light_button(ui, "Сброс").clicked() {
                                    self.player.reset();
                                }
                            });
                        });
                    });
            });

        egui::CentralPanel::default()
            .frame(bg(2, 0))
            .show(ui, |ui| {
                ui::panel(ui, "Ввод", Some("реплика"), Some("01"), |ui| {
                    let (r, act) = editor::show(ui, &mut self.doc, &mut self.ed);
                    if let editor::Action::EditChip(n) = act {
                        if let Some(Node::Token { kind, .. }) = self.doc.nodes.get(n) {
                            let k = *kind;
                            if tokens::spec(k).free.is_some() || !tokens::spec(k).presets.is_empty()
                            {
                                let pos = r.interact_pointer_pos().unwrap_or(r.rect.left_top());
                                self.open_picker(k, Some(n), pos + egui::vec2(-8.0, 16.0));
                            }
                        }
                    }
                });
            });

        self.picker_ui(&ctx);
    }
}
