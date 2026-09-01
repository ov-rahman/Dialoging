//! Модель реплики: плоский список узлов, каретка и правки.
//!
//! Порт того, что в вебе делал `contenteditable` (`readOps`, `serialize`,
//! `balancePairs` в `index.html`). Здесь это явная структура данных, поэтому
//! её можно проверять тестами без экрана.

use crate::tokens::{self, Kind};

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Role {
    Open,
    Close,
}

#[derive(Clone, PartialEq, Debug)]
pub enum Node {
    Text(String),
    Token {
        kind: Kind,
        value: String,
        role: Role,
    },
}

impl Node {
    pub fn token(kind: Kind, value: &str, role: Role) -> Self {
        Node::Token {
            kind,
            value: value.to_owned(),
            role,
        }
    }
    pub fn text(s: &str) -> Self {
        Node::Text(s.to_owned())
    }
    pub fn is_text(&self) -> bool {
        matches!(self, Node::Text(_))
    }
}

/// Позиция каретки: индекс узла и смещение в байтах внутри текста.
/// На токене смещение всегда 0 — каретка стоит перед ним; позиция «после»
/// выражается как начало следующего узла.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub struct Caret {
    pub node: usize,
    pub offset: usize,
}

#[derive(Clone, Default, Debug)]
pub struct Doc {
    pub nodes: Vec<Node>,
    pub caret: Caret,
}

impl Doc {
    pub fn new() -> Self {
        Self::default()
    }

    /// Документ из готового списка узлов, каретка в конец.
    pub fn from_nodes(nodes: Vec<Node>) -> Self {
        let mut d = Self {
            nodes,
            caret: Caret::default(),
        };
        d.caret = d.end_caret();
        d.normalize();
        d
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Сколько открывающих токенов — это число в счётчике на экране.
    pub fn token_count(&self) -> usize {
        self.nodes
            .iter()
            .filter(|n| matches!(n, Node::Token { role: Role::Open, .. }))
            .count()
    }

    pub fn end_caret(&self) -> Caret {
        match self.nodes.last() {
            Some(Node::Text(t)) => Caret {
                node: self.nodes.len() - 1,
                offset: t.len(),
            },
            Some(_) => Caret {
                node: self.nodes.len(),
                offset: 0,
            },
            None => Caret::default(),
        }
    }

    // ------------------------------------------------------------ вывод

    pub fn serialize(&self) -> String {
        let mut out = String::new();
        for n in &self.nodes {
            match n {
                Node::Text(t) => out.push_str(t),
                Node::Token { kind, value, role } => match role {
                    Role::Open => out.push_str(&tokens::code(*kind, value)),
                    Role::Close => out.push_str(&tokens::end_code(*kind)),
                },
            }
        }
        out
    }

    // ------------------------------------------------------------ чистка

    /// Склеивает соседние текстовые узлы и выбрасывает пустые. Без этого
    /// список узлов после правок распухает и каретка ведёт себя непредсказуемо.
    fn merge_text(&mut self) {
        let mut out: Vec<Node> = Vec::with_capacity(self.nodes.len());
        for n in self.nodes.drain(..) {
            match (out.last_mut(), n) {
                (_, Node::Text(t)) if t.is_empty() => {}
                (Some(Node::Text(prev)), Node::Text(t)) => prev.push_str(&t),
                (_, n) => out.push(n),
            }
        }
        self.nodes = out;
    }

    /// Снимает осиротевшие концы парных токенов: если удалить одну половину,
    /// вторая сделала бы разметку невалидной.
    pub fn balance_pairs(&mut self) {
        let mut doomed = vec![false; self.nodes.len()];
        for kind in tokens::ALL.iter().filter(|k| tokens::spec(**k).wrap) {
            let mut open_stack: Vec<usize> = Vec::new();
            for (i, n) in self.nodes.iter().enumerate() {
                if let Node::Token { kind: k, role, .. } = n {
                    if k != kind {
                        continue;
                    }
                    match role {
                        Role::Open => open_stack.push(i),
                        Role::Close => {
                            if open_stack.pop().is_none() {
                                doomed[i] = true; // конец без начала
                            }
                        }
                    }
                }
            }
            for i in open_stack {
                doomed[i] = true; // начало без конца
            }
        }
        if doomed.iter().any(|d| *d) {
            let mut i = 0;
            self.nodes.retain(|_| {
                let keep = !doomed[i];
                i += 1;
                keep
            });
        }
    }

    pub fn normalize(&mut self) {
        self.balance_pairs();
        self.merge_text();
        self.clamp_caret();
    }

    fn clamp_caret(&mut self) {
        if self.caret.node > self.nodes.len() {
            self.caret = self.end_caret();
            return;
        }
        // «Перед узлом i» и «в конце текста i-1» — одна и та же точка на экране.
        // Оставляем одну форму, иначе вставка ведёт себя по-разному в одном месте.
        if self.caret.offset == 0 && self.caret.node > 0 {
            if let Some(Node::Text(prev)) = self.nodes.get(self.caret.node - 1) {
                self.caret = Caret {
                    node: self.caret.node - 1,
                    offset: prev.len(),
                };
            }
        }
        match self.nodes.get(self.caret.node) {
            Some(Node::Text(t)) => {
                if self.caret.offset > t.len() {
                    self.caret.offset = t.len();
                }
                while self.caret.offset < t.len() && !t.is_char_boundary(self.caret.offset) {
                    self.caret.offset += 1;
                }
            }
            _ => self.caret.offset = 0,
        }
    }

    // ------------------------------------------------------------ правки

    /// Разрезает документ в позиции каретки и возвращает индекс, перед которым
    /// нужно вставлять. Текстовый узел при необходимости делится надвое.
    fn split_at_caret(&mut self) -> usize {
        let Caret { node, offset } = self.caret;
        match self.nodes.get(node) {
            Some(Node::Text(t)) => {
                if offset == 0 {
                    node
                } else if offset >= t.len() {
                    node + 1
                } else {
                    let (a, b) = t.split_at(offset);
                    let (a, b) = (a.to_owned(), b.to_owned());
                    self.nodes[node] = Node::Text(a);
                    self.nodes.insert(node + 1, Node::Text(b));
                    node + 1
                }
            }
            _ => node,
        }
    }

    pub fn insert_text(&mut self, s: &str) {
        if s.is_empty() {
            return;
        }
        // Пишем прямо в существующий текстовый узел: если вставлять новый,
        // склейка в normalize сдвинет индексы и каретка уедет в чужой узел.
        if let Some(Node::Text(t)) = self.nodes.get_mut(self.caret.node) {
            let off = self.caret.offset.min(t.len());
            t.insert_str(off, s);
            self.caret.offset = off + s.len();
        } else {
            let at = self.caret.node;
            self.nodes.insert(at, Node::Text(s.to_owned()));
            self.caret = Caret {
                node: at,
                offset: s.len(),
            };
        }
        self.normalize();
    }

    /// Вставляет одиночный токен в позицию каретки.
    pub fn insert_token(&mut self, kind: Kind, value: &str) {
        let at = self.split_at_caret();
        self.nodes.insert(at, Node::token(kind, value, Role::Open));
        self.caret = Caret {
            node: at + 1,
            offset: 0,
        };
        self.normalize();
    }

    /// Вставляет парный токен. Если задан диапазон — оборачивает его,
    /// иначе ставит пустую пару и оставляет каретку между концами.
    pub fn wrap_with(&mut self, kind: Kind, value: &str, sel: Option<(Caret, Caret)>) {
        match sel {
            Some((a, b)) if a != b => {
                let (a, b) = if (a.node, a.offset) <= (b.node, b.offset) {
                    (a, b)
                } else {
                    (b, a)
                };
                // Сначала правый край: левый разрез не сдвинет его индекс.
                self.caret = b;
                let end = self.split_at_caret();
                self.caret = a;
                let start = self.split_at_caret();
                self.nodes
                    .insert(end, Node::token(kind, value, Role::Close));
                self.nodes
                    .insert(start, Node::token(kind, value, Role::Open));
                self.caret = Caret {
                    node: end + 2,
                    offset: 0,
                };
            }
            _ => {
                let at = self.split_at_caret();
                self.nodes
                    .insert(at, Node::token(kind, value, Role::Close));
                self.nodes.insert(at, Node::token(kind, value, Role::Open));
                self.caret = Caret {
                    node: at + 1,
                    offset: 0,
                };
            }
        }
        self.normalize();
    }

    /// Удаляет пару целиком по индексу любой из половин.
    pub fn remove_pair(&mut self, idx: usize) {
        let Some(Node::Token { kind, role, .. }) = self.nodes.get(idx).cloned() else {
            return;
        };
        if !tokens::spec(kind).wrap {
            self.nodes.remove(idx);
            self.normalize();
            return;
        }
        let mut mate = None;
        match role {
            Role::Open => {
                let mut depth = 0;
                for (i, n) in self.nodes.iter().enumerate().skip(idx + 1) {
                    if let Node::Token { kind: k, role: r, .. } = n {
                        if *k != kind {
                            continue;
                        }
                        match r {
                            Role::Open => depth += 1,
                            Role::Close if depth == 0 => {
                                mate = Some(i);
                                break;
                            }
                            Role::Close => depth -= 1,
                        }
                    }
                }
            }
            Role::Close => {
                let mut depth = 0;
                for i in (0..idx).rev() {
                    if let Node::Token { kind: k, role: r, .. } = &self.nodes[i] {
                        if *k != kind {
                            continue;
                        }
                        match r {
                            Role::Close => depth += 1,
                            Role::Open if depth == 0 => {
                                mate = Some(i);
                                break;
                            }
                            Role::Open => depth -= 1,
                        }
                    }
                }
            }
        }
        let mut victims = vec![idx];
        if let Some(m) = mate {
            victims.push(m);
        }
        victims.sort_unstable_by(|a, b| b.cmp(a));
        for v in victims {
            self.nodes.remove(v);
        }
        self.normalize();
    }

    /// Меняет значение токена и его пары.
    pub fn set_value(&mut self, idx: usize, value: &str) {
        let Some(Node::Token { kind, .. }) = self.nodes.get(idx).cloned() else {
            return;
        };
        let wrap = tokens::spec(kind).wrap;
        if let Some(Node::Token { value: v, .. }) = self.nodes.get_mut(idx) {
            *v = value.to_owned();
        }
        if wrap {
            for n in self.nodes.iter_mut() {
                if let Node::Token {
                    kind: k, value: v, ..
                } = n
                {
                    if *k == kind {
                        *v = value.to_owned();
                    }
                }
            }
        }
    }


    // ------------------------------------------------------------ движение каретки

    /// Приводит позицию к канонической форме (см. `clamp_caret`).
    pub fn canon(&self, c: Caret) -> Caret {
        let mut c = c;
        if c.node > self.nodes.len() {
            return self.end_caret();
        }
        if let Some(Node::Text(t)) = self.nodes.get(c.node) {
            if c.offset > t.len() {
                c.offset = t.len();
            }
            while c.offset < t.len() && !t.is_char_boundary(c.offset) {
                c.offset += 1;
            }
        } else {
            c.offset = 0;
        }
        if c.offset == 0 && c.node > 0 {
            if let Some(Node::Text(prev)) = self.nodes.get(c.node - 1) {
                return Caret { node: c.node - 1, offset: prev.len() };
            }
        }
        c
    }

    pub fn caret_left(&self, c: Caret) -> Caret {
        let c = self.canon(c);
        if let Some(Node::Text(t)) = self.nodes.get(c.node) {
            if c.offset > 0 {
                let prev = t[..c.offset]
                    .char_indices()
                    .next_back()
                    .map(|(i, _)| i)
                    .unwrap_or(0);
                return self.canon(Caret { node: c.node, offset: prev });
            }
        }
        if c.node == 0 {
            return Caret::default();
        }
        let target = c.node - 1;
        if let Some(Node::Text(t)) = self.nodes.get(target) {
            let prev = t.char_indices().next_back().map(|(i, _)| i).unwrap_or(0);
            return self.canon(Caret { node: target, offset: prev });
        }
        self.canon(Caret { node: target, offset: 0 })
    }

    pub fn caret_right(&self, c: Caret) -> Caret {
        let c = self.canon(c);
        if let Some(Node::Text(t)) = self.nodes.get(c.node) {
            if c.offset < t.len() {
                let next = t[c.offset..]
                    .char_indices()
                    .nth(1)
                    .map(|(i, _)| c.offset + i)
                    .unwrap_or(t.len());
                return self.canon(Caret { node: c.node, offset: next });
            }
        }
        self.canon(Caret {
            node: (c.node + 1).min(self.nodes.len()),
            offset: 0,
        })
    }

    pub fn start_caret(&self) -> Caret {
        self.canon(Caret { node: 0, offset: 0 })
    }

    /// Линейный порядок двух позиций — нужен, чтобы нормализовать выделение.
    fn key(c: Caret) -> (usize, usize) {
        (c.node, c.offset)
    }

    pub fn ordered(a: Caret, b: Caret) -> (Caret, Caret) {
        if Self::key(a) <= Self::key(b) {
            (a, b)
        } else {
            (b, a)
        }
    }

    // ------------------------------------------------------------ выделение

    /// Удаляет всё между двумя позициями. Если внутри оказалась половина
    /// парного токена, вторая снимется в `normalize`.
    pub fn delete_range(&mut self, a: Caret, b: Caret) {
        let (a, b) = Self::ordered(self.canon(a), self.canon(b));
        if a == b {
            return;
        }
        if a.node == b.node {
            if let Some(Node::Text(t)) = self.nodes.get_mut(a.node) {
                t.replace_range(a.offset..b.offset, "");
                self.caret = a;
                self.normalize();
                return;
            }
        }
        // хвост последнего узла
        if let Some(Node::Text(t)) = self.nodes.get_mut(b.node) {
            t.replace_range(..b.offset.min(t.len()), "");
        }
        // узлы строго между
        let from = a.node + 1;
        let to = b.node.min(self.nodes.len());
        if from < to {
            self.nodes.drain(from..to);
        }
        // начало первого узла
        if let Some(Node::Text(t)) = self.nodes.get_mut(a.node) {
            t.truncate(a.offset.min(t.len()));
        } else if !matches!(self.nodes.get(a.node), None) && a.offset == 0 {
            // каретка стояла перед токеном — сам токен попадает в удаление
            self.nodes.remove(a.node);
        }
        self.caret = a;
        self.normalize();
    }

    /// Delete: удаляет символ или токен справа.
    pub fn delete_forward(&mut self) {
        let right = self.caret_right(self.caret);
        if right == self.caret {
            return;
        }
        let here = self.caret;
        self.delete_range(here, right);
    }

    /// Backspace: удаляет символ слева, а если слева токен — удаляет его
    /// целиком (парный — вместе со вторым концом).
    pub fn backspace(&mut self) {
        let Caret { node, offset } = self.caret;
        if offset > 0 {
            if let Some(Node::Text(t)) = self.nodes.get_mut(node) {
                let prev = t[..offset]
                    .char_indices()
                    .next_back()
                    .map(|(i, _)| i)
                    .unwrap_or(0);
                t.replace_range(prev..offset, "");
                self.caret.offset = prev;
                self.normalize();
                return;
            }
        }
        // каретка в начале узла — цель слева
        if node == 0 {
            return;
        }
        let target = node - 1;
        match self.nodes.get(target).cloned() {
            Some(Node::Text(t)) => {
                let prev = t.char_indices().next_back().map(|(i, _)| i).unwrap_or(0);
                if let Some(Node::Text(tt)) = self.nodes.get_mut(target) {
                    tt.truncate(prev);
                }
                self.caret = Caret {
                    node: target,
                    offset: prev,
                };
            }
            Some(Node::Token { .. }) => {
                self.remove_pair(target);
                self.caret = Caret {
                    node: target.min(self.nodes.len()),
                    offset: 0,
                };
            }
            None => {}
        }
        self.normalize();
    }
}

// ---------------------------------------------------------------- тесты

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokens::Kind;

    fn demo() -> Doc {
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

    #[test]
    fn сериализация_совпадает_с_веб_версией() {
        assert_eq!(
            demo().serialize(),
            "\\TLПривет^3 как дела?&Я \\Rочень\\X рад {shake:2}тебя видеть{/shake}/"
        );
    }

    #[test]
    fn счётчик_считает_только_открывающие() {
        assert_eq!(demo().token_count(), 7);
    }

    #[test]
    fn осиротевший_конец_пары_снимается() {
        let mut d = demo();
        // удаляем открывающую половину напрямую, как это делает Backspace по тексту
        let idx = d
            .nodes
            .iter()
            .position(|n| matches!(n, Node::Token { kind: Kind::Shake, role: Role::Open, .. }))
            .unwrap();
        d.nodes.remove(idx);
        d.normalize();
        assert!(
            !d.serialize().contains("{/shake}"),
            "остался висячий конец: {}",
            d.serialize()
        );
        assert_eq!(
            d.serialize(),
            "\\TLПривет^3 как дела?&Я \\Rочень\\X рад тебя видеть/"
        );
    }

    #[test]
    fn удаление_любой_половины_убирает_обе() {
        for role in [Role::Open, Role::Close] {
            let mut d = demo();
            let idx = d
                .nodes
                .iter()
                .position(|n| matches!(n, Node::Token { kind: Kind::Shake, role: r, .. } if *r == role))
                .unwrap();
            d.remove_pair(idx);
            let s = d.serialize();
            assert!(!s.contains("shake"), "{role:?}: {s}");
        }
    }

    #[test]
    fn парный_токен_оборачивает_выделение() {
        let mut d = Doc::from_nodes(vec![Node::text("тебя видеть")]);
        let a = Caret { node: 0, offset: 0 };
        let b = Caret {
            node: 0,
            offset: "тебя видеть".len(),
        };
        d.wrap_with(Kind::Wave, "3", Some((a, b)));
        assert_eq!(d.serialize(), "{wave:3}тебя видеть{/wave}");
    }

    #[test]
    fn без_выделения_ставится_пустая_пара_с_кареткой_внутри() {
        let mut d = Doc::from_nodes(vec![Node::text("аб")]);
        d.caret = d.end_caret(); // «аб» — 4 байта, не 2: кириллица двухбайтовая
        d.wrap_with(Kind::Glitch, "1", None);
        assert_eq!(d.serialize(), "аб{glitch:1}{/glitch}");
        // каретка между концами: следующий ввод попадает внутрь
        d.insert_text("вг");
        assert_eq!(d.serialize(), "аб{glitch:1}вг{/glitch}");
    }

    #[test]
    fn backspace_по_тексту_режет_по_символам_а_не_по_байтам() {
        let mut d = Doc::from_nodes(vec![Node::text("Привет")]);
        d.backspace();
        assert_eq!(d.serialize(), "Приве");
        d.backspace();
        assert_eq!(d.serialize(), "Прив");
    }

    #[test]
    fn backspace_перед_токеном_сносит_токен_целиком() {
        let mut d = Doc::from_nodes(vec![
            Node::text("а"),
            Node::token(Kind::Pause, "2", Role::Open),
        ]);
        d.caret = d.end_caret();
        d.backspace();
        assert_eq!(d.serialize(), "а");
    }

    #[test]
    fn backspace_перед_парой_сносит_оба_конца() {
        let mut d = Doc::from_nodes(vec![
            Node::text("а"),
            Node::token(Kind::Wave, "2", Role::Open),
            Node::token(Kind::Wave, "2", Role::Close),
        ]);
        d.caret = d.end_caret();
        d.backspace();
        assert_eq!(d.serialize(), "а");
    }

    #[test]
    fn смена_значения_меняет_оба_конца() {
        let mut d = demo();
        let idx = d
            .nodes
            .iter()
            .position(|n| matches!(n, Node::Token { kind: Kind::Shake, .. }))
            .unwrap();
        d.set_value(idx, "3");
        assert!(d.serialize().contains("{shake:3}"));
    }

    #[test]
    fn соседние_текстовые_узлы_склеиваются() {
        let mut d = Doc::from_nodes(vec![
            Node::text("аб"),
            Node::text("вг"),
            Node::text(""),
            Node::text("де"),
        ]);
        d.normalize();
        assert_eq!(d.nodes.len(), 1);
        assert_eq!(d.serialize(), "абвгде");
    }

    #[test]
    fn вставка_в_середину_текста_делит_узел() {
        let mut d = Doc::from_nodes(vec![Node::text("абвг")]);
        d.caret = Caret { node: 0, offset: 4 }; // после «аб» (2 символа по 2 байта)
        d.insert_token(Kind::Pause, "2");
        assert_eq!(d.serialize(), "аб^2вг");
    }

    #[test]
    fn каретка_ходит_по_символам_и_перешагивает_токены() {
        let d = demo();
        let mut c = d.start_caret();
        // первый узел — токен голоса, шаг вправо ставит перед «П»
        let mut seen = Vec::new();
        for _ in 0..8 {
            c = d.caret_right(c);
            seen.push(c);
        }
        // возврат влево той же дорогой
        for _ in 0..8 {
            c = d.caret_left(c);
        }
        assert_eq!(c, d.start_caret(), "путь влево не вернул в начало");
    }

    #[test]
    fn каретка_не_застревает_на_границах() {
        let d = demo();
        let mut c = d.start_caret();
        for _ in 0..500 {
            c = d.caret_left(c);
        }
        assert_eq!(c, d.start_caret());
        let mut c = d.end_caret();
        for _ in 0..500 {
            c = d.caret_right(c);
        }
        assert_eq!(c, d.canon(d.end_caret()));
    }

    #[test]
    fn каретка_не_встаёт_между_байтами_кириллицы() {
        let d = Doc::from_nodes(vec![Node::text("Привет")]);
        let mut c = d.start_caret();
        let mut offs = vec![c.offset];
        for _ in 0..6 {
            c = d.caret_right(c);
            offs.push(c.offset);
        }
        assert_eq!(offs, vec![0, 2, 4, 6, 8, 10, 12], "шаг должен быть по 2 байта");
    }

    #[test]
    fn удаление_выделения_внутри_одного_узла() {
        let mut d = Doc::from_nodes(vec![Node::text("Привет мир")]);
        let a = Caret { node: 0, offset: 0 };
        let b = Caret { node: 0, offset: "Привет ".len() };
        d.delete_range(a, b);
        assert_eq!(d.serialize(), "мир");
    }

    #[test]
    fn удаление_выделения_через_токены_чистит_и_пары() {
        let mut d = demo();
        let a = d.canon(Caret { node: 5, offset: 0 });
        let b = d.end_caret();
        d.delete_range(a, b);
        let s = d.serialize();
        assert!(!s.contains("shake"), "остатки парного токена: {s}");
        assert!(!s.contains("{/"), "остался висячий конец: {s}");
    }

    #[test]
    fn delete_справа_сносит_токен_целиком() {
        let mut d = Doc::from_nodes(vec![
            Node::token(Kind::Pause, "2", Role::Open),
            Node::text("а"),
        ]);
        d.caret = d.start_caret();
        d.delete_forward();
        assert_eq!(d.serialize(), "а");
    }
}
