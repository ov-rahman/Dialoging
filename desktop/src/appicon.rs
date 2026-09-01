//! Иконка приложения. Рисуется кодом, а не картинкой: тот же язык форм,
//! что у остальных иконок, и в репозитории не заводится бинарный файл.
//!
//! Мотив — игровой текстбокс со стрелкой продолжения: ровно то, что
//! приложение делает.

const SIZE: usize = 64;

const INK: [u8; 4] = [0x0A, 0x0A, 0x0A, 0xFF];
const PAPER: [u8; 4] = [0xFF, 0xFF, 0xFF, 0xFF];

/// Точка внутри скруглённого прямоугольника?
fn in_round_rect(x: f32, y: f32, rect: (f32, f32, f32, f32), r: f32) -> bool {
    let (x0, y0, x1, y1) = rect;
    if x < x0 || x > x1 || y < y0 || y > y1 {
        return false;
    }
    let cx = x.clamp(x0 + r, x1 - r);
    let cy = y.clamp(y0 + r, y1 - r);
    let (dx, dy) = (x - cx, y - cy);
    dx * dx + dy * dy <= r * r + 0.001
}

pub fn icon() -> egui::IconData {
    let mut rgba = vec![0u8; SIZE * SIZE * 4];
    let n = SIZE as f32;

    for py in 0..SIZE {
        for px in 0..SIZE {
            let (x, y) = (px as f32 + 0.5, py as f32 + 0.5);
            let mut c = [0u8, 0, 0, 0];

            // подложка — тёмный скруглённый квадрат
            if in_round_rect(x, y, (2.0, 2.0, n - 2.0, n - 2.0), 14.0) {
                c = INK;

                // белая рамка текстбокса
                let outer = in_round_rect(x, y, (12.0, 16.0, n - 12.0, n - 16.0), 3.0);
                let inner = in_round_rect(x, y, (16.0, 20.0, n - 16.0, n - 20.0), 2.0);
                if outer && !inner {
                    c = PAPER;
                }

                // стрелка продолжения в правом нижнем углу бокса
                let (ax, ay) = (x - (n - 21.0), y - (n - 26.0));
                if (0.0..=5.0).contains(&ay) && ax.abs() <= 5.0 - ay {
                    c = PAPER;
                }
            }

            let i = (py * SIZE + px) * 4;
            rgba[i..i + 4].copy_from_slice(&c);
        }
    }

    egui::IconData {
        rgba,
        width: SIZE as u32,
        height: SIZE as u32,
    }
}
