//! Проигрывание реплики в игровом текстбоксе.
//!
//! Порт цикла `play` из веб-версии. Пять эффектов движения сделаны на одной
//! механике с разными стратегиями фазы — как и в CSS-варианте: тряска двигает
//! весь блок синхронно, дрожь даёт каждой букве свою фазу, волна сдвигает фазу
//! по индексу, качание чередует через букву, глитч срабатывает редкими рывками.

use crate::doc::{Doc, Node, Role};
use crate::theme::{self, font};
use crate::tokens::{self, Kind};
use egui::{Color32, Rect, Stroke, StrokeKind, Ui, Vec2};

#[derive(Clone, Debug)]
enum Op {
    Ch(char),
    Cmd {
        kind: Kind,
        value: String,
        close: bool,
    },
}

#[derive(Clone, Copy)]
struct Fx {
    kind: Kind,
    amp: f32,
}

#[derive(Clone)]
struct G {
    ch: char,
    color: Color32,
    fx: Option<Fx>,
    /// Индекс внутри текущего эффекта — по нему считается фаза волны.
    fx_i: usize,
    seed: f32,
}

struct Run {
    color: Color32,
    speed: f32,
    fx: Option<Fx>,
    fx_i: usize,
    instant: bool,
    voice: String,
    portrait: Option<String>,
}

impl Default for Run {
    fn default() -> Self {
        Self {
            color: Color32::WHITE,
            speed: 1.0,
            fx: None,
            fx_i: 0,
            instant: false,
            voice: "S".into(),
            portrait: None,
        }
    }
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum Phase {
    Idle,
    Playing,
    Waiting,
    Done,
}

pub struct Player {
    ops: Vec<Op>,
    i: usize,
    next_at: f64,
    glyphs: Vec<G>,
    run: Run,
    pub phase: Phase,
    /// Сколько символов уже озвучено — чтобы звук шёл ровно по одному разу.
    pub ticks: usize,
    pub last_voice: String,
}

impl Default for Player {
    fn default() -> Self {
        Self {
            ops: Vec::new(),
            i: 0,
            next_at: 0.0,
            glyphs: Vec::new(),
            run: Run::default(),
            phase: Phase::Idle,
            ticks: 0,
            last_voice: "S".into(),
        }
    }
}

impl Player {
    pub fn status(&self) -> &'static str {
        match self.phase {
            Phase::Idle => "готово",
            Phase::Playing => "играет",
            Phase::Waiting => "ждёт нажатия",
            Phase::Done => "конец",
        }
    }

    pub fn is_running(&self) -> bool {
        matches!(self.phase, Phase::Playing | Phase::Waiting)
    }

    /// Нужна ли перерисовка: либо идёт проигрывание, либо на экране есть
    /// буквы с эффектом — они двигаются и после конца реплики.
    pub fn animating(&self) -> bool {
        self.is_running() || self.glyphs.iter().any(|g| g.fx.is_some())
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn start(&mut self, doc: &Doc, now: f64) {
        self.reset();
        self.ops = flatten(doc);
        if self.ops.is_empty() {
            return;
        }
        self.phase = Phase::Playing;
        self.next_at = now;
    }

    /// Нажатие в текстбоксе снимает ожидание.
    pub fn advance(&mut self, now: f64) {
        if self.phase == Phase::Waiting {
            self.phase = Phase::Playing;
            self.next_at = now;
        }
    }

    /// Двигает проигрывание до текущего момента. Возвращает, сколько символов
    /// нужно озвучить в этом кадре.
    pub fn step(&mut self, now: f64) -> usize {
        if self.phase != Phase::Playing {
            return 0;
        }
        let mut spoken = 0;
        // Ограничение на кадр, чтобы «мгновенно» не подвесило интерфейс.
        for _ in 0..4000 {
            if self.phase != Phase::Playing || now < self.next_at {
                break;
            }
            let Some(op) = self.ops.get(self.i).cloned() else {
                self.phase = Phase::Done;
                break;
            };
            self.i += 1;
            match op {
                Op::Ch(ch) => {
                    self.glyphs.push(G {
                        ch,
                        color: self.run.color,
                        fx: self.run.fx,
                        fx_i: self.run.fx_i,
                        seed: (self.glyphs.len() as f32 * 0.6180339).fract(),
                    });
                    if self.run.fx.is_some() {
                        self.run.fx_i += 1;
                    }
                    if !self.run.instant {
                        if !ch.is_whitespace() {
                            spoken += 1;
                        }
                        self.next_at = now + f64::from((34.0 / self.run.speed).max(6.0)) / 1000.0;
                    }
                }
                Op::Cmd { kind, value, close } => self.command(kind, &value, close, now),
            }
        }
        self.last_voice = self.run.voice.clone();
        spoken
    }

    fn command(&mut self, kind: Kind, value: &str, close: bool, now: f64) {
        if close {
            self.run.fx = None;
            return;
        }
        let num = |d: f32| value.parse::<f32>().unwrap_or(d);
        match kind {
            Kind::Pause => self.next_at = now + f64::from(num(0.0).max(0.0)) / 30.0,
            Kind::Clock => self.next_at = now + f64::from(num(0.0).max(0.0)) / 1000.0,
            Kind::Speed => {
                let n = num(1.0);
                if n > 0.0 {
                    self.run.speed = n;
                }
            }
            Kind::Instant => self.run.instant = true,
            Kind::Newline => self.glyphs.push(G {
                ch: '\n',
                color: self.run.color,
                fx: None,
                fx_i: 0,
                seed: 0.0,
            }),
            Kind::Advance => self.phase = Phase::Waiting,
            Kind::Close => {
                self.glyphs.clear();
                self.next_at = now + 0.16;
            }
            Kind::Color => {
                let c = tokens::color_rgb(value).unwrap_or([255, 255, 255]);
                self.run.color = Color32::from_rgb(c[0], c[1], c[2]);
            }
            Kind::Reset => {
                self.run.color = Color32::WHITE;
                self.run.speed = 1.0;
                self.run.instant = false;
                self.run.fx = None;
            }
            Kind::Voice => self.run.voice = value.to_owned(),
            Kind::Face => self.run.portrait = Some(value.to_owned()),
            Kind::Sound => {} // звук события — снаружи
            Kind::Shake | Kind::Jitter | Kind::Wave | Kind::Wobble | Kind::Glitch => {
                self.run.fx = Some(Fx {
                    kind,
                    amp: num(2.0).max(0.0),
                });
                self.run.fx_i = 0;
            }
        }
    }
}

fn flatten(doc: &Doc) -> Vec<Op> {
    let mut ops = Vec::new();
    for n in &doc.nodes {
        match n {
            Node::Text(t) => ops.extend(t.chars().map(Op::Ch)),
            Node::Token { kind, value, role } => ops.push(Op::Cmd {
                kind: *kind,
                value: value.clone(),
                close: *role == Role::Close,
            }),
        }
    }
    ops
}

// ---------------------------------------------------------------- эффекты

/// Смещение и поворот буквы. Одна механика, разные стратегии фазы.
fn fx_transform(fx: Fx, t: f64, i: usize, seed: f32) -> (Vec2, f32) {
    use std::f32::consts::TAU;
    let a = fx.amp;
    let t = t as f32;
    match fx.kind {
        // весь блок синхронно — фаза общая
        Kind::Shake => {
            let p = t / 0.42 * TAU;
            (Vec2::new(a * p.sin(), a * 0.6 * (p * 1.3).cos()), 0.0)
        }
        // каждая буква со своей фазой
        Kind::Jitter => {
            let p = (t / 0.42 + seed * 3.0) * TAU;
            (
                Vec2::new(a * p.sin(), a * 0.7 * (p * 1.7 + seed).cos()),
                0.0,
            )
        }
        // бегущая волна: фаза сдвинута по индексу
        Kind::Wave => {
            let p = (t / 1.1 - i as f32 * 0.082) * TAU;
            (Vec2::new(0.0, -1.6 * a * p.sin()), 0.0)
        }
        // через букву в противофазе
        Kind::Wobble => {
            let dir = if i % 2 == 0 { 1.0 } else { -1.0 };
            let p = t / 0.8 * TAU;
            (Vec2::ZERO, (3.0 * a * dir * p.sin()).to_radians())
        }
        // редкие рывки
        Kind::Glitch => {
            let x = (t * 0.45 + seed * 2.2).fract();
            let d = if (0.90..0.93).contains(&x) {
                1.8
            } else if (0.94..0.97).contains(&x) {
                -1.8
            } else {
                0.0
            };
            (Vec2::new(a * d, if d == 0.0 { 0.0 } else { -1.0 }), 0.0)
        }
        _ => (Vec2::ZERO, 0.0),
    }
}

// ---------------------------------------------------------------- отрисовка

/// Рисует игровой текстбокс. Возвращает true, если по нему кликнули.
pub fn stage(ui: &mut Ui, player: &Player, height: f32) -> bool {
    let (rect, resp) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), height),
        egui::Sense::click(),
    );
    let p = ui.painter();
    p.rect_filled(rect, 3.0, Color32::BLACK);
    p.rect_stroke(
        rect,
        3.0,
        Stroke::new(5.0, theme::WHITE),
        StrokeKind::Outside,
    );

    let pad = 18.0;
    let mut area = rect.shrink(pad);

    // портрет слева, если реплика его задала
    if let Some(face) = &player.run.portrait {
        let box_r = Rect::from_min_size(area.min, Vec2::splat(56.0));
        p.rect_stroke(
            box_r,
            2.0,
            Stroke::new(3.0, theme::WHITE),
            StrokeKind::Inside,
        );
        let n = face.parse::<u8>().unwrap_or(1);
        crate::icons::draw_face(p, box_r.shrink(12.0), n, theme::WHITE, 1.7);
        area = Rect::from_min_max(
            egui::pos2(box_r.right() + 14.0, area.top()),
            area.max,
        );
    }

    let t = ui.input(|i| i.time);
    let fid = font::stage();
    // Высоту строки берём из реальной раскладки образца: так она совпадает
    // с тем, чем рисуются сами буквы.
    let line_h = ui
        .painter()
        .layout_no_wrap("Ая".to_owned(), fid.clone(), Color32::WHITE)
        .size()
        .y
        * 1.5;
    let (mut x, mut y) = (area.left(), area.top());

    for (i, g) in player.glyphs.iter().enumerate() {
        if g.ch == '\n' {
            x = area.left();
            y += line_h;
            continue;
        }
        let galley = ui
            .painter()
            .layout_no_wrap(g.ch.to_string(), fid.clone(), g.color);
        let w = galley.size().x;
        if x + w > area.right() {
            x = area.left();
            y += line_h;
        }
        let (off, angle) = match g.fx {
            Some(fx) => fx_transform(fx, t, g.fx_i, g.seed),
            None => (Vec2::ZERO, 0.0),
        };
        let mut shape = egui::epaint::TextShape::new(
            egui::pos2(x + off.x, y + off.y),
            galley,
            g.color,
        );
        shape.angle = angle;
        ui.painter().add(shape);
        x += w;
        let _ = i;
    }

    // мигающая стрелка ожидания
    if player.phase == Phase::Waiting && (t * 1.4).fract() < 0.6 {
        let c = egui::pos2(rect.right() - 20.0, rect.bottom() - 16.0);
        ui.painter().add(egui::Shape::convex_polygon(
            vec![
                egui::pos2(c.x - 7.0, c.y - 4.0),
                egui::pos2(c.x + 7.0, c.y - 4.0),
                egui::pos2(c.x, c.y + 5.0),
            ],
            theme::WHITE,
            Stroke::NONE,
        ));
    }

    resp.clicked()
}
