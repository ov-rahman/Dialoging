//! Словарь токенов, сериализация в разметку и разбор пользовательских значений.
//!
//! Порт логики из веб-версии (`index.html`: `T`, `COLORS`, `VOICES`, `serialize`,
//! `normVal`). Формат разметки менять нельзя — им уже пользуются.

use std::fmt;

// ---------------------------------------------------------------- виды

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum Kind {
    Pause,
    Clock,
    Speed,
    Instant,
    Newline,
    Advance,
    Close,
    Color,
    Reset,
    Voice,
    Face,
    Shake,
    Jitter,
    Wave,
    Wobble,
    Glitch,
    Sound,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Group {
    Timing,
    Flow,
    Style,
    Character,
    Motion,
    Sound,
}

impl Group {
    pub fn title(self) -> &'static str {
        match self {
            Group::Timing => "Тайминг",
            Group::Flow => "Поток",
            Group::Style => "Стиль",
            Group::Character => "Персонаж",
            Group::Motion => "Движение",
            Group::Sound => "Звук",
        }
    }
}

pub const GROUPS: [Group; 6] = [
    Group::Timing,
    Group::Flow,
    Group::Style,
    Group::Character,
    Group::Motion,
    Group::Sound,
];

/// Порядок здесь задаёт порядок кнопок в панели токенов.
pub const ALL: [Kind; 17] = [
    Kind::Pause,
    Kind::Clock,
    Kind::Speed,
    Kind::Instant,
    Kind::Newline,
    Kind::Advance,
    Kind::Close,
    Kind::Color,
    Kind::Reset,
    Kind::Voice,
    Kind::Face,
    Kind::Shake,
    Kind::Jitter,
    Kind::Wave,
    Kind::Wobble,
    Kind::Glitch,
    Kind::Sound,
];

// ---------------------------------------------------------------- свободный ввод

/// Описание поля «своё значение» под быстрыми пресетами.
#[derive(Copy, Clone, Debug)]
pub struct Free {
    /// Числовое поле: запятая приводится к точке, лишние пробелы срезаются.
    pub numeric: bool,
    pub hint: &'static str,
    /// Текст предупреждения, если значение спорное. Оно не блокирует ввод:
    /// повторное подтверждение ставит значение как есть.
    pub warn: &'static str,
}

// ---------------------------------------------------------------- описание токена

#[derive(Clone, Debug)]
pub struct Spec {
    pub kind: Kind,
    pub group: Group,
    pub name: &'static str,
    /// Значение по умолчанию при вставке одним кликом.
    pub default: Option<&'static str>,
    pub presets: &'static [&'static str],
    pub free: Option<Free>,
    /// Парный токен: оборачивает выделение и требует закрывающего конца.
    pub wrap: bool,
    /// Выбор значения открывается сразу при вставке (цвет, голос, лицо, звук).
    pub pick_on_insert: bool,
}

const AMP: &[&str] = &["1", "2", "3"];

const FREE_AMP: Free = Free {
    numeric: true,
    hint: "сила эффекта",
    warn: "Сила не может быть отрицательной.",
};

pub fn spec(kind: Kind) -> Spec {
    use Group::*;
    use Kind::*;
    let (group, name, default, presets, free, wrap, pick) = match kind {
        Pause => (
            Timing,
            "Пауза",
            Some("2"),
            &["1", "2", "3", "4", "5", "6", "7", "8", "9"][..],
            Some(Free {
                numeric: true,
                hint: "кадров",
                warn: "Undertale читает после ^ ровно одну цифру 0–9. \
                       Для длинных пауз бери «Пауза мс».",
            }),
            false,
            false,
        ),
        Clock => (
            Timing,
            "Пауза мс",
            Some("250"),
            &["100", "250", "500", "1000", "2000"][..],
            Some(Free {
                numeric: true,
                hint: "миллисекунд",
                warn: "Пауза не может быть отрицательной.",
            }),
            false,
            false,
        ),
        Speed => (
            Timing,
            "Скорость",
            Some("2"),
            &["0.25", "0.5", "1", "2", "3", "5"][..],
            Some(Free {
                numeric: true,
                hint: "× от обычной",
                warn: "Скорость должна быть больше нуля.",
            }),
            false,
            false,
        ),
        Instant => (Timing, "Мгновенно", None, &[][..], None, false, false),

        Newline => (Flow, "Строка", None, &[][..], None, false, false),
        Advance => (Flow, "Ждать", None, &[][..], None, false, false),
        Close => (Flow, "Закрыть", None, &[][..], None, false, false),

        Color => (
            Style,
            "Цвет",
            Some("R"),
            &["R", "G", "B", "Y", "P", "O", "W"][..],
            Some(Free {
                numeric: false,
                hint: "#RRGGBB или буква",
                warn: "Нужен hex вида #E4483C или буква из палитры.",
            }),
            false,
            true,
        ),
        Reset => (Style, "Сброс", None, &[][..], None, false, false),

        Voice => (
            Character,
            "Голос",
            Some("S"),
            &["S", "L", "H", "R", "W"][..],
            Some(Free {
                numeric: false,
                hint: "код персонажа",
                warn: "Код голоса — от одного до четырёх символов без пробелов.",
            }),
            false,
            true,
        ),
        Face => (
            Character,
            "Лицо",
            Some("1"),
            &["1", "2", "3", "4", "5", "6"][..],
            Some(Free {
                numeric: false,
                hint: "номер портрета",
                warn: "Номер портрета — от одного до четырёх символов без пробелов.",
            }),
            false,
            true,
        ),

        Shake => (Motion, "Тряска", Some("2"), AMP, Some(FREE_AMP), true, false),
        Jitter => (
            Motion,
            "Дрожь букв",
            Some("2"),
            AMP,
            Some(FREE_AMP),
            true,
            false,
        ),
        Wave => (Motion, "Волна", Some("2"), AMP, Some(FREE_AMP), true, false),
        Wobble => (
            Motion,
            "Качание",
            Some("2"),
            AMP,
            Some(FREE_AMP),
            true,
            false,
        ),
        Glitch => (Motion, "Глитч", Some("2"), AMP, Some(FREE_AMP), true, false),

        Kind::Sound => (
            Group::Sound,
            "Звук",
            Some("hit"),
            &["hit", "ping", "thud", "coin"][..],
            Some(Free {
                numeric: false,
                hint: "имя звука",
                warn: "Имя звука — буквы, цифры, точка и дефис.",
            }),
            false,
            true,
        ),
    };
    Spec {
        kind,
        group,
        name,
        default,
        presets,
        free,
        wrap,
        pick_on_insert: pick,
    }
}

// ---------------------------------------------------------------- цвет

pub fn is_hex(v: &str) -> bool {
    let b = v.as_bytes();
    b.len() == 7 && b[0] == b'#' && b[1..].iter().all(|c| c.is_ascii_hexdigit())
}

/// Разрешает значение цвета в конкретный RGB: буква палитры либо свой hex.
pub fn color_rgb(v: &str) -> Option<[u8; 3]> {
    if is_hex(v) {
        let n = u32::from_str_radix(&v[1..], 16).ok()?;
        return Some([(n >> 16) as u8, (n >> 8) as u8, n as u8]);
    }
    let ch = v.chars().next()?;
    crate::theme::PALETTE
        .iter()
        .find(|(c, _, _)| *c == ch)
        .map(|(_, _, col)| [col.r(), col.g(), col.b()])
}

// ---------------------------------------------------------------- сериализация

/// Код открывающего (или одиночного) токена.
pub fn code(kind: Kind, value: &str) -> String {
    use Kind::*;
    match kind {
        Pause => format!("^{value}"),
        Clock => format!("{{p:{value}}}"),
        Speed => format!("{{s:{value}}}"),
        Instant => "{instant}".into(),
        Newline => "&".into(),
        Advance => "/".into(),
        Close => "%".into(),
        // Своя буква палитры пишется коротко, произвольный цвет — полным hex,
        // чтобы не занимать буквы под случайные значения.
        Color => {
            if is_hex(value) {
                format!("{{c:{value}}}")
            } else {
                format!("\\{value}")
            }
        }
        Reset => "\\X".into(),
        Voice => format!("\\T{value}"),
        Face => format!("\\E{value}"),
        Shake => format!("{{shake:{value}}}"),
        Jitter => format!("{{jitter:{value}}}"),
        Wave => format!("{{wave:{value}}}"),
        Wobble => format!("{{wobble:{value}}}"),
        Glitch => format!("{{glitch:{value}}}"),
        Sound => format!("{{snd:{value}}}"),
    }
}

/// Код закрывающего конца парного токена.
pub fn end_code(kind: Kind) -> String {
    use Kind::*;
    match kind {
        Shake => "{/shake}".into(),
        Jitter => "{/jitter}".into(),
        Wave => "{/wave}".into(),
        Wobble => "{/wobble}".into(),
        Glitch => "{/glitch}".into(),
        _ => String::new(),
    }
}

// ---------------------------------------------------------------- разбор значений

/// Приводит введённое к каноническому виду: запятая → точка, пробелы прочь,
/// `0.250` → `0.25`. `None` — если ввод пустой или не число там, где нужно число.
pub fn normalize(kind: Kind, raw: &str) -> Option<String> {
    let sp = spec(kind);
    let v = raw.trim();
    match sp.free {
        Some(f) if f.numeric => {
            let cleaned: String = v
                .replace(',', ".")
                .chars()
                .filter(|c| !c.is_whitespace())
                .collect();
            if cleaned.is_empty() {
                return None;
            }
            let n: f64 = cleaned.parse().ok()?;
            if !n.is_finite() {
                return None;
            }
            Some(fmt_num(n))
        }
        _ => {
            if v.is_empty() {
                None
            } else {
                Some(v.to_owned())
            }
        }
    }
}

/// Короткая запись числа без хвостовых нулей: 2.0 → «2», 0.250 → «0.25».
fn fmt_num(n: f64) -> String {
    let s = format!("{n}");
    s
}

/// Значение допустимо? `Err` — это предупреждение, а не запрет: интерфейс
/// показывает его и по второму подтверждению ставит значение как есть.
pub fn validate(kind: Kind, value: &str) -> Result<(), &'static str> {
    use Kind::*;
    let sp = spec(kind);
    let Some(free) = sp.free else { return Ok(()) };
    let ok = match kind {
        Pause => value.len() == 1 && value.as_bytes()[0].is_ascii_digit(),
        Clock => value.parse::<f64>().map(|n| n >= 0.0).unwrap_or(false),
        Speed => value.parse::<f64>().map(|n| n > 0.0).unwrap_or(false),
        Color => is_hex(value) || (value.len() == 1 && color_rgb(value).is_some()),
        Voice | Face => {
            let n = value.chars().count();
            (1..=4).contains(&n) && value.chars().all(|c| c.is_alphanumeric() || c == '_')
        }
        Sound => {
            let n = value.chars().count();
            (1..=24).contains(&n)
                && value
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == '_' || c == '.' || c == '-')
        }
        Shake | Jitter | Wave | Wobble | Glitch => {
            value.parse::<f64>().map(|n| n >= 0.0).unwrap_or(false)
        }
        _ => true,
    };
    if ok {
        Ok(())
    } else {
        Err(free.warn)
    }
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(spec(*self).name)
    }
}

// ---------------------------------------------------------------- тесты

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn коды_совпадают_с_веб_версией() {
        assert_eq!(code(Kind::Pause, "3"), "^3");
        assert_eq!(code(Kind::Clock, "250"), "{p:250}");
        assert_eq!(code(Kind::Speed, "0.25"), "{s:0.25}");
        assert_eq!(code(Kind::Instant, ""), "{instant}");
        assert_eq!(code(Kind::Newline, ""), "&");
        assert_eq!(code(Kind::Advance, ""), "/");
        assert_eq!(code(Kind::Close, ""), "%");
        assert_eq!(code(Kind::Reset, ""), "\\X");
        assert_eq!(code(Kind::Voice, "L"), "\\TL");
        assert_eq!(code(Kind::Face, "1"), "\\E1");
        assert_eq!(code(Kind::Sound, "hit"), "{snd:hit}");
        assert_eq!(code(Kind::Shake, "2"), "{shake:2}");
        assert_eq!(end_code(Kind::Shake), "{/shake}");
    }

    #[test]
    fn буква_палитры_коротко_свой_цвет_полным_hex() {
        assert_eq!(code(Kind::Color, "R"), "\\R");
        assert_eq!(code(Kind::Color, "#8ED1FC"), "{c:#8ED1FC}");
    }

    #[test]
    fn нормализация_числовых_значений() {
        assert_eq!(normalize(Kind::Speed, "0.25").as_deref(), Some("0.25"));
        assert_eq!(normalize(Kind::Speed, "1,5").as_deref(), Some("1.5"));
        assert_eq!(normalize(Kind::Clock, "  3000 ").as_deref(), Some("3000"));
        assert_eq!(normalize(Kind::Speed, "2.0").as_deref(), Some("2"));
        assert_eq!(normalize(Kind::Speed, "0.250").as_deref(), Some("0.25"));
        assert_eq!(normalize(Kind::Speed, "abc"), None);
        assert_eq!(normalize(Kind::Speed, "   "), None);
    }

    #[test]
    fn нормализация_текстовых_значений() {
        assert_eq!(normalize(Kind::Voice, " SANS ").as_deref(), Some("SANS"));
        assert_eq!(normalize(Kind::Color, "#8ED1FC").as_deref(), Some("#8ED1FC"));
        assert_eq!(normalize(Kind::Sound, ""), None);
    }

    #[test]
    fn предупреждения_не_запрещают_а_предупреждают() {
        // допустимое
        assert!(validate(Kind::Pause, "3").is_ok());
        assert!(validate(Kind::Speed, "0.25").is_ok());
        assert!(validate(Kind::Color, "#8ED1FC").is_ok());
        assert!(validate(Kind::Color, "R").is_ok());
        assert!(validate(Kind::Voice, "SANS").is_ok());
        assert!(validate(Kind::Sound, "door_open").is_ok());
        assert!(validate(Kind::Shake, "0.5").is_ok());
        // спорное — возвращает текст предупреждения
        assert!(validate(Kind::Pause, "12").is_err());
        assert!(validate(Kind::Speed, "0").is_err());
        assert!(validate(Kind::Clock, "-5").is_err());
        assert!(validate(Kind::Color, "#XYZ").is_err());
        assert!(validate(Kind::Voice, "СЛИШКОМДЛИННЫЙ").is_err());
    }

    #[test]
    fn цвет_разрешается_и_из_буквы_и_из_hex() {
        assert_eq!(color_rgb("#8ED1FC"), Some([0x8E, 0xD1, 0xFC]));
        assert_eq!(color_rgb("R"), Some([0xE4, 0x48, 0x3C]));
        assert_eq!(color_rgb("Щ"), None);
    }

    #[test]
    fn у_каждого_вида_есть_описание_и_парные_имеют_конец() {
        for k in ALL {
            let s = spec(k);
            assert!(!s.name.is_empty(), "{k:?} без имени");
            if s.wrap {
                assert!(!end_code(k).is_empty(), "{k:?} без закрывающего кода");
            }
            // у токена с пресетами обязано быть значение по умолчанию
            if !s.presets.is_empty() {
                assert!(s.default.is_some(), "{k:?} без значения по умолчанию");
            }
        }
    }
}
