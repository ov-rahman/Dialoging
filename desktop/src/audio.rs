//! Звук печати и звуковые события.
//!
//! Порт `tick` и `SFX` из веб-версии, где звук синтезировался WebAudio.
//! Здесь то же самое: волна считается сэмплами на лету, звуковых файлов в
//! проекте нет и не будет — у каждого персонажа свой тембр, а не свой .wav.

use rodio::buffer::SamplesBuffer;
use rodio::MixerDeviceSink;
use std::num::NonZero;

const RATE: u32 = 44_100;

#[derive(Copy, Clone)]
enum Wave {
    Square,
    Saw,
    Sine,
    Triangle,
}

/// Голос персонажа: форма волны, частота, длительность.
struct Voice {
    wave: Wave,
    freq: f32,
    secs: f32,
}

fn voice(code: &str) -> Voice {
    match code {
        "L" => Voice {
            wave: Wave::Saw,
            freq: 180.0,
            secs: 0.045,
        },
        "H" => Voice {
            wave: Wave::Square,
            freq: 760.0,
            secs: 0.022,
        },
        "R" => Voice {
            wave: Wave::Square,
            freq: 300.0,
            secs: 0.055,
        },
        "W" => Voice {
            wave: Wave::Triangle,
            freq: 240.0,
            secs: 0.018,
        },
        // Незнакомый код голоса не должен ронять звук — берём обычный.
        _ => Voice {
            wave: Wave::Square,
            freq: 420.0,
            secs: 0.030,
        },
    }
}

/// Звуковое событие `{snd:…}`: частота, длительность, форма, скольжение вниз.
fn sfx(name: &str) -> (f32, f32, Wave) {
    match name {
        "ping" => (880.0, 0.18, Wave::Sine),
        "thud" => (90.0, 0.22, Wave::Saw),
        "coin" => (1180.0, 0.12, Wave::Square),
        _ => (160.0, 0.16, Wave::Square),
    }
}

fn sample(wave: Wave, phase: f32) -> f32 {
    let p = phase.fract();
    match wave {
        Wave::Square => {
            if p < 0.5 {
                1.0
            } else {
                -1.0
            }
        }
        Wave::Saw => 2.0 * p - 1.0,
        Wave::Sine => (p * std::f32::consts::TAU).sin(),
        Wave::Triangle => 4.0 * (p - 0.5).abs() - 1.0,
    }
}

/// Строит короткий сигнал с экспоненциальным затуханием.
/// `glide` — во сколько раз частота падает к концу (для звуков событий).
fn render(wave: Wave, freq: f32, secs: f32, gain: f32, glide: f32) -> Vec<f32> {
    let n = (secs * RATE as f32) as usize;
    let mut out = Vec::with_capacity(n);
    let mut phase = 0.0f32;
    for i in 0..n {
        let t = i as f32 / n as f32;
        let f = freq * (1.0 + (glide - 1.0) * t);
        phase += f / RATE as f32;
        // экспоненциальный спад — без него щелчок на обрыве
        let env = (-5.0 * t).exp();
        out.push(sample(wave, phase) * env * gain);
    }
    out
}

pub struct Audio {
    sink: Option<MixerDeviceSink>,
    pub enabled: bool,
}

impl Audio {
    /// Устройства может не быть (сервер, контейнер, отключённая карта) —
    /// это не ошибка приложения, просто играть нечем.
    pub fn new() -> Self {
        let sink = rodio::DeviceSinkBuilder::open_default_sink().ok();
        Self {
            sink,
            enabled: true,
        }
    }

    pub fn available(&self) -> bool {
        self.sink.is_some()
    }

    fn play(&self, data: Vec<f32>) {
        if !self.enabled {
            return;
        }
        let Some(sink) = &self.sink else { return };
        let (Some(ch), Some(rate)) = (NonZero::new(1u16), NonZero::new(RATE)) else {
            return;
        };
        sink.mixer().add(SamplesBuffer::new(ch, rate, data));
    }

    /// Щелчок печати. `n` — сколько символов вышло за кадр; играем один раз,
    /// иначе на быстрой скорости звук превращается в кашу.
    pub fn tick(&self, voice_code: &str, n: usize) {
        if n == 0 {
            return;
        }
        let v = voice(voice_code);
        self.play(render(v.wave, v.freq, v.secs, 0.16, 1.0));
    }

    pub fn event(&self, name: &str) {
        let (f, secs, wave) = sfx(name);
        self.play(render(wave, f, secs, 0.22, 0.5));
    }
}

impl Default for Audio {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn сигнал_нужной_длины_и_затухает() {
        let d = render(Wave::Square, 440.0, 0.05, 1.0, 1.0);
        assert_eq!(d.len(), (0.05 * RATE as f32) as usize);
        let first = d[..40].iter().fold(0.0f32, |m, x| m.max(x.abs()));
        let last = d[d.len() - 40..].iter().fold(0.0f32, |m, x| m.max(x.abs()));
        assert!(
            last < first * 0.2,
            "хвост должен затухать: {first} → {last}"
        );
    }

    #[test]
    fn незнакомый_голос_не_роняет_звук() {
        let v = voice("СЛУЧАЙНЫЙ");
        assert!(v.freq > 0.0 && v.secs > 0.0);
    }

    #[test]
    fn сигнал_в_пределах_громкости() {
        for (w, f) in [
            (Wave::Saw, 180.0),
            (Wave::Sine, 880.0),
            (Wave::Triangle, 240.0),
        ] {
            let d = render(w, f, 0.05, 0.16, 1.0);
            assert!(d.iter().all(|x| x.abs() <= 0.17), "перегруз у {f} Гц");
        }
    }
}
