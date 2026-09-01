//! Редактор реплики: текст, внутри которого живут атомарные чипы-токены.
//!
//! В вебе это давал `contenteditable`. Здесь всё своё: раскладка по строкам,
//! каретка, выделение, попадание мышью. Документ короткий (одна реплика),
//! поэтому раскладка считается каждый кадр — кэш не нужен и не усложняет.

use crate::doc::{Caret, Doc, Node, Role};
use crate::icons;
use crate::theme::{self, font};
use crate::tokens::{self, Kind};
use egui::{Align2, Color32, Galley, Rect, Response, Sense, Stroke, StrokeKind, Ui, Vec2};
use std::sync::Arc;

/// Что редактор просит сделать снаружи.
#[derive(Clone, PartialEq, Debug)]
pub enum Action {
    None,
    /// Кликнули по чипу — надо открыть выбор значения.
    EditChip(usize),
}

#[derive(Default)]
pub struct State {
    /// Вторая точка выделения; равна каретке, когда выделения нет.
    pub anchor: Option<Caret>,
}

impl State {
    pub fn clear_selection(&mut self) {
        self.anchor = None;
    }
    fn selection(&self, caret: Caret) -> Option<(Caret, Caret)> {
        let a = self.anchor?;
        if a == caret {
            None
        } else {
            Some(Doc::ordered(a, caret))
        }
    }
}

// ---------------------------------------------------------------- размеры

const LINE_H: f32 = 38.0;
const CHIP_H: f32 = 24.0;
const CHIP_PAD: f32 = 8.0;
const CHIP_ICON: f32 = 14.0;
const CHIP_GAP: f32 = 5.0;

fn chip_value_label(kind: Kind, value: &str) -> Option<String> {
    match kind {
        Kind::Color => None, // цвет показывает кружком, а не текстом
        _ if !value.is_empty() => Some(value.to_owned()),
        _ => None,
    }
}

fn chip_size(ui: &Ui, kind: Kind, value: &str, role: Role) -> Vec2 {
    let mut w = CHIP_PAD + CHIP_ICON + CHIP_PAD;
    if role == Role::Open {
        if kind == Kind::Color {
            w += CHIP_GAP + 11.0;
        }
        if let Some(lbl) = chip_value_label(kind, value) {
            let g = ui.painter().layout_no_wrap(lbl, font::chip(), theme::INK_2);
            w += CHIP_GAP + g.size().x;
        }
    }
    Vec2::new(w, CHIP_H)
}

fn draw_chip(ui: &Ui, rect: Rect, kind: Kind, value: &str, role: Role, hovered: bool) {
    let p = ui.painter();
    if role == Role::Close {
        // Закрывающий конец — пунктирный контур без заливки: он не несёт
        // значения, и в тексте не должен спорить с открывающим.
        p.rect(
            rect,
            theme::R_CHIP,
            Color32::TRANSPARENT,
            Stroke::new(1.0, theme::INK_4),
            StrokeKind::Inside,
        );
    } else {
        let sh = if hovered {
            theme::shadow_card()
        } else {
            theme::shadow_control()
        };
        p.add(sh.as_shape(rect, theme::R_CHIP));
        p.rect(
            rect,
            theme::R_CHIP,
            theme::WHITE,
            Stroke::new(1.0, theme::LINE_2),
            StrokeKind::Inside,
        );
    }

    let color = if role == Role::Close {
        theme::INK_2
    } else {
        theme::INK
    };
    let icon_rect = Rect::from_center_size(
        egui::pos2(rect.left() + CHIP_PAD + CHIP_ICON / 2.0, rect.center().y),
        Vec2::splat(CHIP_ICON),
    );
    icons::draw_token(p, icon_rect, kind, color, 1.7);

    if role == Role::Close {
        return;
    }
    let mut x = rect.left() + CHIP_PAD + CHIP_ICON;
    if kind == Kind::Color {
        x += CHIP_GAP;
        let c = tokens::color_rgb(value).unwrap_or([0, 0, 0]);
        p.circle_filled(
            egui::pos2(x + 5.5, rect.center().y),
            5.5,
            Color32::from_rgb(c[0], c[1], c[2]),
        );
        p.circle_stroke(
            egui::pos2(x + 5.5, rect.center().y),
            5.5,
            Stroke::new(1.0, Color32::from_black_alpha(30)),
        );
        x += 11.0;
    }
    if let Some(lbl) = chip_value_label(kind, value) {
        x += CHIP_GAP;
        p.text(
            egui::pos2(x, rect.center().y),
            Align2::LEFT_CENTER,
            lbl,
            font::chip(),
            theme::INK_2,
        );
    }
}

// ---------------------------------------------------------------- раскладка

enum Atom {
    /// Кусок текста: узел, байтовое смещение начала, готовая раскладка.
    Word {
        node: usize,
        start: usize,
        text: String,
        galley: Arc<Galley>,
    },
    Chip {
        node: usize,
        kind: Kind,
        value: String,
        role: Role,
        size: Vec2,
    },
    /// Токен переноса строки: рисуется чипом и переносит строку.
    Break {
        node: usize,
        size: Vec2,
    },
}

impl Atom {
    fn size(&self) -> Vec2 {
        match self {
            Atom::Word { galley, .. } => galley.size(),
            Atom::Chip { size, .. } | Atom::Break { size, .. } => *size,
        }
    }
    fn node(&self) -> usize {
        match self {
            Atom::Word { node, .. } | Atom::Chip { node, .. } | Atom::Break { node, .. } => *node,
        }
    }
}

/// Режет текст на куски «слово + следующие за ним пробелы»: так перенос
/// происходит по словам, а не по буквам.
fn split_words(t: &str) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    let mut chars = t.char_indices().peekable();
    while let Some(&(i, _)) = chars.peek() {
        // непробельная часть
        while let Some(&(_, c)) = chars.peek() {
            if c.is_whitespace() {
                break;
            }
            chars.next();
        }
        // прилипающие пробелы
        while let Some(&(_, c)) = chars.peek() {
            if !c.is_whitespace() {
                break;
            }
            chars.next();
        }
        let end = chars.peek().map(|(j, _)| *j).unwrap_or(t.len());
        if end > i {
            out.push((i, t[i..end].to_owned()));
        }
        if chars.peek().is_none() {
            break;
        }
    }
    out
}

fn build_atoms(ui: &Ui, doc: &Doc) -> Vec<Atom> {
    let mut atoms = Vec::new();
    for (ni, n) in doc.nodes.iter().enumerate() {
        match n {
            Node::Text(t) => {
                for (off, w) in split_words(t) {
                    let galley =
                        ui.painter()
                            .layout_no_wrap(w.clone(), font::editor(), theme::INK);
                    atoms.push(Atom::Word {
                        node: ni,
                        start: off,
                        text: w,
                        galley,
                    });
                }
            }
            Node::Token { kind, value, role } if *kind == Kind::Newline => {
                atoms.push(Atom::Break {
                    node: ni,
                    size: chip_size(ui, *kind, value, *role),
                });
            }
            Node::Token { kind, value, role } => atoms.push(Atom::Chip {
                node: ni,
                kind: *kind,
                value: value.clone(),
                role: *role,
                size: chip_size(ui, *kind, value, *role),
            }),
        }
    }
    atoms
}

struct Placed {
    idx: usize,
    rect: Rect,
    line: usize,
}

fn place(atoms: &[Atom], origin: egui::Pos2, max_w: f32) -> (Vec<Placed>, f32) {
    let mut out = Vec::with_capacity(atoms.len());
    let (mut x, mut line) = (origin.x, 0usize);
    for (i, a) in atoms.iter().enumerate() {
        let s = a.size();
        if x > origin.x && x + s.x > origin.x + max_w {
            line += 1;
            x = origin.x;
        }
        let y = origin.y + line as f32 * LINE_H;
        let rect = Rect::from_min_size(
            egui::pos2(x, y + (LINE_H - s.y) / 2.0),
            s,
        );
        out.push(Placed { idx: i, rect, line });
        x += s.x;
        if matches!(a, Atom::Break { .. }) {
            line += 1;
            x = origin.x;
        }
    }
    (out, (line + 1) as f32 * LINE_H)
}

// ---------------------------------------------------------------- позиции

/// Байтовое смещение внутри слова по экранной координате X.
fn offset_in_word(ui: &Ui, text: &str, local_x: f32) -> usize {
    let mut best = (0usize, f32::MAX);
    for (i, _) in text.char_indices().chain(std::iter::once((text.len(), ' '))) {
        let g = ui
            .painter()
            .layout_no_wrap(text[..i].to_owned(), font::editor(), theme::INK);
        let d = (g.size().x - local_x).abs();
        if d < best.1 {
            best = (i, d);
        }
    }
    best.0
}

/// Экранная X каретки внутри слова.
fn x_of_offset(ui: &Ui, text: &str, off: usize) -> f32 {
    let off = off.min(text.len());
    ui.painter()
        .layout_no_wrap(text[..off].to_owned(), font::editor(), theme::INK)
        .size()
        .x
}

fn caret_rect(
    ui: &Ui,
    atoms: &[Atom],
    placed: &[Placed],
    doc: &Doc,
    caret: Caret,
    origin: egui::Pos2,
) -> Rect {
    let c = doc.canon(caret);
    // ищем слово, которому принадлежит позиция
    for p in placed {
        if let Atom::Word {
            node, start, text, ..
        } = &atoms[p.idx]
        {
            if *node == c.node && c.offset >= *start && c.offset <= start + text.len() {
                let x = p.rect.left() + x_of_offset(ui, text, c.offset - start);
                return Rect::from_min_size(
                    egui::pos2(x, p.rect.center().y - 11.0),
                    Vec2::new(1.5, 22.0),
                );
            }
        }
    }
    // позиция перед узлом
    for p in placed {
        if atoms[p.idx].node() == c.node {
            return Rect::from_min_size(
                egui::pos2(p.rect.left(), p.rect.center().y - 11.0),
                Vec2::new(1.5, 22.0),
            );
        }
    }
    // конец документа
    match placed.last() {
        Some(p) => Rect::from_min_size(
            egui::pos2(p.rect.right(), p.rect.center().y - 11.0),
            Vec2::new(1.5, 22.0),
        ),
        None => Rect::from_min_size(
            egui::pos2(origin.x, origin.y + LINE_H / 2.0 - 11.0),
            Vec2::new(1.5, 22.0),
        ),
    }
}

fn caret_at_pos(
    ui: &Ui,
    atoms: &[Atom],
    placed: &[Placed],
    doc: &Doc,
    pos: egui::Pos2,
) -> Caret {
    // ближайшая строка
    let line = placed
        .iter()
        .min_by(|a, b| {
            let da = (a.rect.center().y - pos.y).abs();
            let db = (b.rect.center().y - pos.y).abs();
            da.partial_cmp(&db).unwrap()
        })
        .map(|p| p.line);
    let Some(line) = line else {
        return Caret::default();
    };

    let mut best: Option<(&Placed, f32)> = None;
    for p in placed.iter().filter(|p| p.line == line) {
        let d = if pos.x < p.rect.left() {
            p.rect.left() - pos.x
        } else if pos.x > p.rect.right() {
            pos.x - p.rect.right()
        } else {
            0.0
        };
        if best.is_none() || d < best.unwrap().1 {
            best = Some((p, d));
        }
    }
    let Some((p, _)) = best else {
        return Caret::default();
    };

    match &atoms[p.idx] {
        Atom::Word {
            node, start, text, ..
        } => {
            let off = offset_in_word(ui, text, pos.x - p.rect.left());
            doc.canon(Caret {
                node: *node,
                offset: start + off,
            })
        }
        a => {
            // на чипе: левее середины — перед ним, правее — после
            let n = a.node();
            if pos.x < p.rect.center().x {
                doc.canon(Caret { node: n, offset: 0 })
            } else {
                doc.canon(Caret {
                    node: n + 1,
                    offset: 0,
                })
            }
        }
    }
}

// ---------------------------------------------------------------- виджет

pub fn show(ui: &mut Ui, doc: &mut Doc, st: &mut State) -> (Response, Action) {
    let mut action = Action::None;

    let max_w = ui.available_width();
    let atoms = build_atoms(ui, doc);
    let probe_origin = ui.cursor().min;
    let (_, height) = place(&atoms, probe_origin, max_w);
    let height = height.max(LINE_H * 3.0).max(ui.available_height());

    let (rect, resp) = ui.allocate_at_least(Vec2::new(max_w, height), Sense::click_and_drag());
    let origin = rect.min;
    let (placed, _) = place(&atoms, origin, max_w);

    let id = resp.id;
    let focused = ui.memory(|m| m.has_focus(id));

    // -------------------------------------------------- мышь
    if resp.clicked() || resp.drag_started() {
        ui.memory_mut(|m| m.request_focus(id));
        if let Some(pos) = resp.interact_pointer_pos() {
            // клик по чипу открывает выбор значения
            let hit_chip = placed.iter().find(|p| {
                p.rect.contains(pos) && !matches!(atoms[p.idx], Atom::Word { .. })
            });
            if let (Some(p), true) = (hit_chip, resp.clicked()) {
                action = Action::EditChip(atoms[p.idx].node());
            }
            let c = caret_at_pos(ui, &atoms, &placed, doc, pos);
            doc.caret = c;
            st.anchor = Some(c);
        }
    }
    if resp.dragged() {
        if let Some(pos) = resp.interact_pointer_pos() {
            doc.caret = caret_at_pos(ui, &atoms, &placed, doc, pos);
        }
    }
    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Text);
    }

    // -------------------------------------------------- клавиатура
    if focused {
        let events = ui.input(|i| i.events.clone());
        for ev in events {
            match ev {
                egui::Event::Text(t) if !t.is_empty() => {
                    if let Some((a, b)) = st.selection(doc.caret) {
                        doc.delete_range(a, b);
                    }
                    st.clear_selection();
                    doc.insert_text(&t);
                }
                egui::Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } => {
                    let sel = st.selection(doc.caret);
                    match key {
                        egui::Key::Backspace => {
                            if let Some((a, b)) = sel {
                                doc.delete_range(a, b);
                            } else {
                                doc.backspace();
                            }
                            st.clear_selection();
                        }
                        egui::Key::Delete => {
                            if let Some((a, b)) = sel {
                                doc.delete_range(a, b);
                            } else {
                                doc.delete_forward();
                            }
                            st.clear_selection();
                        }
                        egui::Key::ArrowLeft => {
                            let c = doc.caret_left(doc.caret);
                            doc.caret = c;
                            if !modifiers.shift {
                                st.anchor = Some(c);
                            }
                        }
                        egui::Key::ArrowRight => {
                            let c = doc.caret_right(doc.caret);
                            doc.caret = c;
                            if !modifiers.shift {
                                st.anchor = Some(c);
                            }
                        }
                        egui::Key::Home => {
                            let c = doc.start_caret();
                            doc.caret = c;
                            if !modifiers.shift {
                                st.anchor = Some(c);
                            }
                        }
                        egui::Key::End => {
                            let c = doc.canon(doc.end_caret());
                            doc.caret = c;
                            if !modifiers.shift {
                                st.anchor = Some(c);
                            }
                        }
                        egui::Key::A if modifiers.command => {
                            st.anchor = Some(doc.start_caret());
                            doc.caret = doc.canon(doc.end_caret());
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
    }

    // -------------------------------------------------- отрисовка
    let sel = st.selection(doc.caret);
    if let Some((a, b)) = sel {
        for p in &placed {
            if let Atom::Word {
                node, start, text, ..
            } = &atoms[p.idx]
            {
                let (s, e) = (*start, start + text.len());
                let from = if a.node < *node { s } else { a.offset.max(s) };
                let to = if b.node > *node { e } else { b.offset.min(e) };
                let inside = (*node, e) > (a.node, a.offset) && (*node, s) < (b.node, b.offset);
                if inside && from < to {
                    let x0 = p.rect.left() + x_of_offset(ui, text, from - s);
                    let x1 = p.rect.left() + x_of_offset(ui, text, to - s);
                    ui.painter().rect_filled(
                        Rect::from_min_max(
                            egui::pos2(x0, p.rect.top() - 3.0),
                            egui::pos2(x1, p.rect.bottom() + 3.0),
                        ),
                        3.0,
                        theme::ACCENT_SOFT,
                    );
                }
            } else {
                let n = atoms[p.idx].node();
                let inside =
                    (n, 0) >= (a.node, a.offset) && (n + 1, 0) <= (b.node, b.offset + 1);
                if inside {
                    ui.painter()
                        .rect_filled(p.rect.expand(2.0), 5.0, theme::ACCENT_SOFT);
                }
            }
        }
    }

    let hover_pos = ui.ctx().pointer_hover_pos();
    for p in &placed {
        match &atoms[p.idx] {
            Atom::Word { galley, .. } => {
                ui.painter()
                    .galley(p.rect.min, galley.clone(), theme::INK);
            }
            Atom::Chip {
                kind, value, role, ..
            } => {
                let hov = hover_pos.map(|h| p.rect.contains(h)).unwrap_or(false);
                draw_chip(ui, p.rect, *kind, value, *role, hov);
            }
            Atom::Break { .. } => {
                let hov = hover_pos.map(|h| p.rect.contains(h)).unwrap_or(false);
                draw_chip(ui, p.rect, Kind::Newline, "", Role::Open, hov);
            }
        }
    }

    if doc.nodes.is_empty() {
        ui.painter().text(
            egui::pos2(origin.x, origin.y + LINE_H / 2.0),
            Align2::LEFT_CENTER,
            "Начни печатать…",
            font::editor(),
            theme::INK_4,
        );
    }

    if focused {
        let t = ui.input(|i| i.time);
        if (t * 1.6).fract() < 0.6 {
            let cr = caret_rect(ui, &atoms, &placed, doc, doc.caret, origin);
            ui.painter().rect_filled(cr, 1.0, theme::INK);
        }
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(120));
    }

    (resp, action)
}
