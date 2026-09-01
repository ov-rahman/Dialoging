//! Палитра, типографика и общий стиль.
//!
//! Значения перенесены один в один из веб-версии (`index.html`, блок `:root`),
//! чтобы десктоп и веб выглядели как одно приложение.

use egui::{Color32, CornerRadius, FontData, FontDefinitions, FontFamily, Margin, Shadow};

// ---------------------------------------------------------------- цвета

pub const BG: Color32 = Color32::from_rgb(0xEF, 0xED, 0xEA);
pub const CARD: Color32 = Color32::from_rgb(0xF7, 0xF6, 0xF4);
pub const WHITE: Color32 = Color32::from_rgb(0xFF, 0xFF, 0xFF);

pub const INK: Color32 = Color32::from_rgb(0x0A, 0x0A, 0x0A);
pub const INK_2: Color32 = Color32::from_rgb(0x6B, 0x68, 0x64);
pub const INK_3: Color32 = Color32::from_rgb(0x9C, 0x98, 0x94);
pub const INK_4: Color32 = Color32::from_rgb(0xC3, 0xBF, 0xBA);

pub const LINE: Color32 = Color32::from_rgb(0xDE, 0xDA, 0xD5);
pub const LINE_2: Color32 = Color32::from_rgb(0xE8, 0xE4, 0xDF);

pub const ACCENT: Color32 = Color32::from_rgb(0x16, 0xA3, 0x4A);
pub const ACCENT_INK: Color32 = Color32::from_rgb(0x0E, 0x7A, 0x37);
pub const ACCENT_SOFT: Color32 = Color32::from_rgba_premultiplied(0x16, 0xA3, 0x4A, 0x1F);

pub const WARN: Color32 = Color32::from_rgb(0xB4, 0x55, 0x2F);

/// Семь цветов разметки с короткими кодами Undertale.
/// Всё остальное пользователь набирает пикером и получает `{c:#RRGGBB}`.
pub const PALETTE: [(char, &str, Color32); 7] = [
    ('R', "Красный", Color32::from_rgb(0xE4, 0x48, 0x3C)),
    ('G', "Зелёный", Color32::from_rgb(0x22, 0xB0, 0x4D)),
    ('B', "Синий", Color32::from_rgb(0x2E, 0x7B, 0xF6)),
    ('Y', "Жёлтый", Color32::from_rgb(0xF2, 0xC2, 0x30)),
    ('P', "Фиолетовый", Color32::from_rgb(0x9B, 0x6B, 0xF0)),
    ('O', "Оранжевый", Color32::from_rgb(0xF2, 0x80, 0x2B)),
    ('W', "Белый", Color32::from_rgb(0xFF, 0xFF, 0xFF)),
];

// ---------------------------------------------------------------- шрифты

/// Именованные роли шрифтов вместо разрозненных чисел по коду.
pub mod font {
    use egui::{FontFamily, FontId};

    fn sans(size: f32) -> FontId {
        FontId::new(size, FontFamily::Proportional)
    }
    fn mono(size: f32) -> FontId {
        FontId::new(size, FontFamily::Monospace)
    }

    /// Крупный заголовок страницы.
    pub fn display() -> FontId {
        sans(30.0)
    }
    /// Обычный текст интерфейса.
    pub fn body() -> FontId {
        sans(14.0)
    }
    /// Текст в редакторе реплики.
    pub fn editor() -> FontId {
        sans(19.0)
    }
    /// Микро-лейбл капсом: «ВВОД», «ТАЙМИНГ», «01».
    pub fn eyebrow() -> FontId {
        mono(10.0)
    }
    /// Разметка на выходе и подписи внутри чипов.
    pub fn code() -> FontId {
        mono(13.0)
    }
    /// Значение внутри чипа.
    pub fn chip() -> FontId {
        mono(11.0)
    }
    /// Игровой текстбокс превью.
    pub fn stage() -> FontId {
        mono(16.0)
    }
}

/// Микро-лейблы на рефах набраны капсом с большим трекингом. egui не умеет
/// letter-spacing, поэтому разряжаем строку вручную — иначе капс выглядит сбитым.
pub fn eyebrow_text(s: &str) -> String {
    s.to_uppercase()
        .chars()
        .flat_map(|c| [c, '\u{2009}'])
        .collect()
}

// ---------------------------------------------------------------- формы

pub const R_PANEL: CornerRadius = CornerRadius::same(16);
pub const R_CTRL: CornerRadius = CornerRadius::same(10);
pub const R_CHIP: CornerRadius = CornerRadius::same(8);

pub fn shadow_card() -> Shadow {
    Shadow {
        offset: [0, 10],
        blur: 24,
        spread: 0,
        color: Color32::from_black_alpha(30),
    }
}

pub fn shadow_control() -> Shadow {
    Shadow {
        offset: [0, 3],
        blur: 9,
        spread: 0,
        color: Color32::from_black_alpha(24),
    }
}

pub fn shadow_pop() -> Shadow {
    Shadow {
        offset: [0, 16],
        blur: 40,
        spread: 0,
        color: Color32::from_black_alpha(48),
    }
}

/// Карточка-панель: светлая плашка со скруглением, тонкой рамкой и мягкой тенью.
pub fn panel_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(CARD)
        .stroke(egui::Stroke::new(1.0, LINE_2))
        .corner_radius(R_PANEL)
        .shadow(shadow_card())
        .inner_margin(Margin::same(0))
}

// ---------------------------------------------------------------- установка

/// Вшитые шрифты. Оба под OFL и оба с полной кириллицей — интерфейс русский,
/// а на системные шрифты Windows полагаться нельзя.
pub fn install_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();

    let mut add = |name: &str, bytes: &'static [u8]| {
        fonts.font_data.insert(
            name.to_owned(),
            std::sync::Arc::new(FontData::from_static(bytes)),
        );
    };
    add("golos", include_bytes!("../assets/GolosText-Regular.ttf"));
    add(
        "golos-medium",
        include_bytes!("../assets/GolosText-Medium.ttf"),
    );
    add(
        "golos-semibold",
        include_bytes!("../assets/GolosText-SemiBold.ttf"),
    );
    add(
        "jetbrains",
        include_bytes!("../assets/JetBrainsMono-Regular.ttf"),
    );

    // Свои шрифты первыми, дефолтные egui остаются позади как запасной вариант
    // для символов, которых нет в Golos и JetBrains.
    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .insert(0, "golos".to_owned());
    fonts
        .families
        .entry(FontFamily::Monospace)
        .or_default()
        .insert(0, "jetbrains".to_owned());

    // Отдельные семейства для насыщенных начертаний: egui не синтезирует
    // жирность, её нужно подключать настоящим файлом.
    fonts.families.insert(
        FontFamily::Name("medium".into()),
        vec!["golos-medium".to_owned(), "golos".to_owned()],
    );
    fonts.families.insert(
        FontFamily::Name("semibold".into()),
        vec!["golos-semibold".to_owned(), "golos".to_owned()],
    );

    ctx.set_fonts(fonts);
}

/// Полное перекрытие стиля egui: по умолчанию он выглядит как отладочный
/// инструмент, а нам нужна светлая типографская схема с рефов.
pub fn install_style(ctx: &egui::Context) {
    // В egui 0.36 стиль задаётся отдельно для светлой и тёмной темы;
    // приложение намеренно однотемное, поэтому пишем один и тот же в обе.
    ctx.all_styles_mut(|style| {
        let v = &mut style.visuals;
        v.dark_mode = false;
        v.override_text_color = Some(INK);
        v.panel_fill = BG;
        v.window_fill = WHITE;
        v.extreme_bg_color = WHITE;
        v.faint_bg_color = CARD;
        v.window_stroke = egui::Stroke::new(1.0, LINE_2);
        v.window_corner_radius = R_PANEL;
        v.window_shadow = shadow_pop();
        v.popup_shadow = shadow_pop();
        v.selection.bg_fill = ACCENT_SOFT;
        v.selection.stroke = egui::Stroke::new(1.0, ACCENT_INK);

        for w in [
            &mut v.widgets.noninteractive,
            &mut v.widgets.inactive,
            &mut v.widgets.hovered,
            &mut v.widgets.active,
            &mut v.widgets.open,
        ] {
            w.corner_radius = R_CTRL;
            w.bg_stroke = egui::Stroke::new(1.0, LINE_2);
            w.fg_stroke = egui::Stroke::new(1.0, INK);
            w.expansion = 0.0;
        }
        v.widgets.noninteractive.bg_fill = CARD;
        v.widgets.inactive.bg_fill = WHITE;
        v.widgets.inactive.weak_bg_fill = WHITE;
        v.widgets.hovered.bg_fill = WHITE;
        v.widgets.hovered.weak_bg_fill = WHITE;
        v.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, INK_4);
        v.widgets.active.bg_fill = INK;
        v.widgets.active.weak_bg_fill = INK;

        let s = &mut style.spacing;
        s.item_spacing = egui::vec2(8.0, 8.0);
        s.button_padding = egui::vec2(12.0, 9.0);
        s.window_margin = Margin::same(12);
        s.menu_margin = Margin::same(8);
        s.interact_size.y = 34.0;

        style.text_styles = [
            (egui::TextStyle::Heading, font::display()),
            (egui::TextStyle::Body, font::body()),
            (egui::TextStyle::Button, font::body()),
            (egui::TextStyle::Small, font::eyebrow()),
            (egui::TextStyle::Monospace, font::code()),
        ]
        .into();
    });
}

pub fn install(ctx: &egui::Context) {
    install_fonts(ctx);
    install_style(ctx);
}
