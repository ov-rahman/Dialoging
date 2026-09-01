//! Иконки токенов, перенесённые с контурного набора веб-версии.
//!
//! Не парсер SVG: геометрия из `ICON` в `index.html` переписана на примитивы
//! egui в той же сетке 24×24, поэтому пропорции и штрих совпадают с макетом.

use crate::tokens::Kind;
use egui::{Color32, Painter, Pos2, Rect, Shape, Stroke};

/// Перо, работающее в координатах 24×24 и переводящее их в экранные.
struct Pen<'a> {
    p: &'a Painter,
    rect: Rect,
    stroke: Stroke,
}

impl<'a> Pen<'a> {
    fn new(p: &'a Painter, rect: Rect, stroke: Stroke) -> Self {
        Self { p, rect, stroke }
    }

    fn k(&self) -> f32 {
        self.rect.width().min(self.rect.height()) / 24.0
    }

    fn pt(&self, x: f32, y: f32) -> Pos2 {
        let k = self.k();
        let w = 24.0 * k;
        let o = self.rect.center() - egui::vec2(w, w) / 2.0;
        egui::pos2(o.x + x * k, o.y + y * k)
    }

    fn poly(&self, pts: &[(f32, f32)]) {
        let v: Vec<Pos2> = pts.iter().map(|(x, y)| self.pt(*x, *y)).collect();
        self.p.add(Shape::line(v, self.stroke));
    }

    fn fill_poly(&self, pts: &[(f32, f32)]) {
        let v: Vec<Pos2> = pts.iter().map(|(x, y)| self.pt(*x, *y)).collect();
        self.p
            .add(Shape::convex_polygon(v, self.stroke.color, Stroke::NONE));
    }

    fn closed(&self, pts: &[(f32, f32)]) {
        let mut v: Vec<Pos2> = pts.iter().map(|(x, y)| self.pt(*x, *y)).collect();
        v.push(v[0]);
        self.p.add(Shape::line(v, self.stroke));
    }

    fn circle(&self, cx: f32, cy: f32, r: f32) {
        self.p
            .circle_stroke(self.pt(cx, cy), r * self.k(), self.stroke);
    }

    fn dot(&self, cx: f32, cy: f32, r: f32) {
        self.p
            .circle_filled(self.pt(cx, cy), r * self.k(), self.stroke.color);
    }

    /// Дуга по градусам: 0° — вправо, 90° — вниз (экранная система координат).
    fn arc_pts(&self, cx: f32, cy: f32, r: f32, a0: f32, a1: f32) -> Vec<(f32, f32)> {
        let steps = (((a1 - a0).abs() / 12.0).ceil() as usize).max(3);
        (0..=steps)
            .map(|i| {
                let a = (a0 + (a1 - a0) * i as f32 / steps as f32).to_radians();
                (cx + r * a.cos(), cy + r * a.sin())
            })
            .collect()
    }

    fn arc(&self, cx: f32, cy: f32, r: f32, a0: f32, a1: f32) {
        let pts = self.arc_pts(cx, cy, r, a0, a1);
        self.poly(&pts);
    }

    /// Прямоугольник со скруглением — из отрезков и дуг, чтобы он мог
    /// участвовать в повороте (у egui скруглённый прямоугольник не вращается).
    fn rrect(&self, x: f32, y: f32, w: f32, h: f32, r: f32, rot_deg: f32) {
        let r = r.min(w / 2.0).min(h / 2.0);
        let (x1, y1) = (x + w, y + h);
        let mut pts: Vec<(f32, f32)> = Vec::new();
        pts.extend(self.arc_pts(x + r, y + r, r, 180.0, 270.0));
        pts.extend(self.arc_pts(x1 - r, y + r, r, 270.0, 360.0));
        pts.extend(self.arc_pts(x1 - r, y1 - r, r, 0.0, 90.0));
        pts.extend(self.arc_pts(x + r, y1 - r, r, 90.0, 180.0));

        if rot_deg != 0.0 {
            let (cx, cy) = (x + w / 2.0, y + h / 2.0);
            let a = rot_deg.to_radians();
            let (s, c) = (a.sin(), a.cos());
            for p in pts.iter_mut() {
                let (dx, dy) = (p.0 - cx, p.1 - cy);
                *p = (cx + dx * c - dy * s, cy + dx * s + dy * c);
            }
        }
        self.closed(&pts);
    }
}

// ---------------------------------------------------------------- токены

pub fn draw_token(p: &Painter, rect: Rect, kind: Kind, color: Color32, width: f32) {
    let pen = Pen::new(p, rect, Stroke::new(width, color));
    use Kind::*;
    match kind {
        Pause => {
            pen.poly(&[(9.2, 4.9), (9.2, 19.1)]);
            pen.poly(&[(14.8, 4.9), (14.8, 19.1)]);
        }
        Clock => {
            pen.circle(12.0, 12.0, 8.2);
            pen.poly(&[(12.0, 7.1), (12.0, 12.3), (15.3, 14.3)]);
        }
        Speed => {
            pen.poly(&[(4.6, 6.6), (11.0, 12.0), (4.6, 17.4)]);
            pen.poly(&[(12.8, 6.6), (19.2, 12.0), (12.8, 17.4)]);
        }
        Instant => pen.closed(&[
            (13.3, 3.2),
            (6.4, 13.6),
            (11.0, 13.6),
            (10.3, 20.8),
            (17.6, 9.9),
            (12.8, 9.9),
        ]),
        Newline => {
            let mut pts = vec![(20.0, 5.2), (20.0, 10.3)];
            pts.extend(pen.arc_pts(16.6, 10.3, 3.4, 0.0, 90.0));
            pts.push((4.9, 13.7));
            pen.poly(&pts);
            pen.poly(&[(8.7, 9.6), (4.4, 13.7), (8.7, 17.8)]);
        }
        Advance => {
            pen.rrect(3.2, 4.4, 17.6, 15.2, 2.6, 0.0);
            pen.fill_poly(&[(8.8, 10.4), (15.2, 10.4), (12.0, 15.4)]);
        }
        Close => {
            pen.rrect(3.2, 4.4, 17.6, 15.2, 2.6, 0.0);
            pen.poly(&[(9.4, 9.6), (14.6, 14.4)]);
            pen.poly(&[(14.6, 9.6), (9.4, 14.4)]);
        }
        Color => {
            pen.circle(12.0, 12.0, 8.2);
            let mut half = pen.arc_pts(12.0, 12.0, 8.2, -90.0, 90.0);
            half.push((12.0, 3.8));
            pen.fill_poly(&half);
        }
        Reset => {
            // Разомкнутое кольцо со стрелкой на конце: разрыв сверху слева,
            // как в контурном наборе веб-версии.
            pen.arc(12.0, 12.0, 7.8, 200.0, 480.0);
            let tip = (12.0 + 7.8 * 200f32.to_radians().cos(),
                       12.0 + 7.8 * 200f32.to_radians().sin());
            pen.fill_poly(&[
                (tip.0 - 2.6, tip.1 - 1.2),
                (tip.0 + 1.4, tip.1 - 2.6),
                (tip.0 + 0.6, tip.1 + 2.2),
            ]);
        }
        Voice => {
            pen.dot(7.4, 12.0, 2.3);
            pen.arc(12.4, 12.0, 3.6, -60.0, 60.0);
            pen.arc(15.9, 12.0, 6.6, -60.0, 60.0);
        }
        Face => {
            pen.circle(12.0, 12.0, 8.2);
            pen.dot(9.4, 10.3, 1.05);
            pen.dot(14.6, 10.3, 1.05);
            pen.arc(12.0, 11.6, 4.0, 40.0, 140.0);
        }
        Shake => {
            pen.rrect(7.4, 5.8, 9.2, 12.4, 2.2, 0.0);
            pen.poly(&[(4.3, 9.4), (4.3, 14.6)]);
            pen.poly(&[(1.9, 10.9), (1.9, 13.1)]);
            pen.poly(&[(19.7, 9.4), (19.7, 14.6)]);
            pen.poly(&[(22.1, 10.9), (22.1, 13.1)]);
        }
        Jitter => {
            pen.rrect(2.8, 12.4, 5.6, 5.6, 1.4, 0.0);
            pen.rrect(9.2, 6.4, 5.6, 5.6, 1.4, 0.0);
            pen.rrect(15.6, 11.2, 5.6, 5.6, 1.4, 0.0);
            pen.poly(&[(4.6, 10.4), (5.6, 9.2)]);
            pen.poly(&[(11.4, 14.4), (12.4, 15.6)]);
            pen.poly(&[(17.6, 9.2), (18.6, 8.0)]);
        }
        Wave => {
            let pts: Vec<(f32, f32)> = (0..=48)
                .map(|i| {
                    let t = i as f32 / 48.0;
                    let x = 2.4 + t * 19.2;
                    let y = 12.6 - (t * std::f32::consts::TAU * 1.5).sin() * 3.0;
                    (x, y)
                })
                .collect();
            pen.poly(&pts);
        }
        Wobble => {
            pen.rrect(3.6, 8.0, 3.4, 8.0, 1.1, -21.0);
            pen.rrect(10.3, 8.0, 3.4, 8.0, 1.1, 21.0);
            pen.rrect(17.0, 8.0, 3.4, 8.0, 1.1, -21.0);
        }
        Glitch => {
            pen.rrect(5.4, 5.4, 14.0, 3.6, 1.0, 0.0);
            pen.rrect(8.2, 10.2, 13.0, 3.6, 1.0, 0.0);
            pen.rrect(3.0, 15.0, 13.0, 3.6, 1.0, 0.0);
        }
        Sound => {
            pen.closed(&[
                (3.6, 9.6),
                (7.1, 9.6),
                (11.6, 5.9),
                (11.6, 18.1),
                (7.1, 14.4),
                (3.6, 14.4),
            ]);
            pen.arc(15.1, 12.0, 3.9, -60.0, 60.0);
            pen.arc(18.2, 12.0, 8.2, -55.0, 55.0);
        }
    }
}

// ---------------------------------------------------------------- служебные

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Ui {
    Play,
    Stop,
    VolumeOn,
    VolumeOff,
    Check,
    Cross,
}

pub fn draw_ui(p: &Painter, rect: Rect, ico: Ui, color: Color32, width: f32) {
    let pen = Pen::new(p, rect, Stroke::new(width, color));
    match ico {
        Ui::Play => pen.fill_poly(&[(7.0, 4.6), (19.0, 12.0), (7.0, 19.4)]),
        Ui::Stop => pen.rrect(6.0, 6.0, 12.0, 12.0, 2.0, 0.0),
        Ui::VolumeOn => {
            pen.closed(&[
                (4.0, 9.4),
                (7.4, 9.4),
                (11.8, 5.8),
                (11.8, 18.2),
                (7.4, 14.6),
                (4.0, 14.6),
            ]);
            pen.arc(15.2, 12.0, 3.7, -60.0, 60.0);
        }
        Ui::VolumeOff => {
            pen.closed(&[
                (4.0, 9.4),
                (7.4, 9.4),
                (11.8, 5.8),
                (11.8, 18.2),
                (7.4, 14.6),
                (4.0, 14.6),
            ]);
            pen.poly(&[(15.4, 10.0), (19.6, 14.0)]);
            pen.poly(&[(19.6, 10.0), (15.4, 14.0)]);
        }
        Ui::Check => pen.poly(&[(5.0, 12.4), (10.0, 17.0), (19.0, 7.0)]),
        Ui::Cross => {
            pen.poly(&[(6.5, 6.5), (17.5, 17.5)]);
            pen.poly(&[(17.5, 6.5), (6.5, 17.5)]);
        }
    }
}

/// Портреты для токена «Лицо». Шесть простых выражений — в вебе они были
/// такими же, отдельных изображений проект не требует.
pub fn draw_face(p: &Painter, rect: Rect, n: u8, color: Color32, width: f32) {
    let pen = Pen::new(p, rect, Stroke::new(width, color));
    pen.circle(12.0, 12.0, 8.2);
    match n {
        4 => {
            pen.poly(&[(7.8, 9.4), (10.4, 10.8)]);
            pen.poly(&[(16.2, 9.4), (13.6, 10.8)]);
            pen.poly(&[(9.6, 15.2), (14.4, 15.2)]);
        }
        6 => {
            pen.poly(&[(8.2, 10.4), (10.6, 10.4)]);
            pen.poly(&[(13.4, 10.4), (15.8, 10.4)]);
            pen.arc(12.0, 14.2, 3.0, 20.0, 160.0);
        }
        _ => {
            let r = if n == 5 { 1.35 } else { 1.05 };
            pen.dot(9.4, 10.2, r);
            pen.dot(14.6, 10.2, r);
            match n {
                2 => pen.poly(&[(9.0, 15.2), (15.0, 15.2)]),
                3 => pen.arc(12.0, 17.6, 4.0, 200.0, 340.0),
                5 => pen.circle(12.0, 15.2, 2.2),
                _ => pen.arc(12.0, 11.6, 4.0, 40.0, 140.0),
            }
        }
    }
}
