---
name: watch
description: |
  Смотрит любое видео по ссылке (YouTube, Loom, запись Zoom, прямой mp4) — не только читает
  транскрипт, но и ВИДИТ, что на экране. Кадры берёт по сменам сцен (ffmpeg scene detection),
  а не по таймеру, поэтому не пропускает графику, демо, монтаж и b-roll. Звук: субтитры через
  yt-dlp, fallback — Whisper. Используй, когда пользователь пишет /watch <url> или просит
  «посмотри это видео», «разбери ролик/созвон/лекцию», «что показано на экране».
  Триггеры: /watch, посмотри видео, разбери ролик, проанализируй видео, transcribe.
---

# watch — Claude смотрит видео целиком

## Зависимости (проверить при первом запуске)

- `yt-dlp`, `ffmpeg`, `whisper`. Если чего-то нет — установить и сообщить пользователю.

```bash
command -v yt-dlp; command -v ffmpeg; command -v whisper
```

Установка, если чего-то не хватает:

- **macOS:** `brew install yt-dlp ffmpeg` + `pip install -U openai-whisper`
- **Linux:** `pipx install yt-dlp`, `sudo apt install -y ffmpeg`, `pip install -U openai-whisper`
- **Windows:** `winget install yt-dlp.yt-dlp`, `winget install Gyan.FFmpeg`, `pip install -U openai-whisper`

## Workflow

### 1. Подготовка

- Сделай рабочую папку: `mkdir -p /tmp/watch/<slug>/frames`
- Распарси URL (YouTube / Loom / Zoom-share / прямой видеофайл).
- `<slug>` — короткий безопасный идентификатор ролика (например, ID видео).

### 2. Транскрипт — сначала субтитры (бесплатно, если есть)

```bash
yt-dlp --write-subs --write-auto-subs --sub-langs "ru,en" --skip-download \
  --convert-subs srt -o "/tmp/watch/<slug>/subs.%(ext)s" "<URL>"
```

Если `.srt` появился — это транскрипт с тайм-кодами, используй его.

### 3. Транскрипт — fallback Whisper (если субтитров нет)

```bash
yt-dlp -x --audio-format mp3 -o "/tmp/watch/<slug>/audio.mp3" "<URL>"

whisper "/tmp/watch/<slug>/audio.mp3" --model small --language ru \
  --output_format srt --output_dir "/tmp/watch/<slug>/"
```

### 4. Кадры ПО СМЕНАМ СЦЕН (ключевой шаг — не по таймеру)

```bash
yt-dlp -f "bv*[height<=720]+ba/b[height<=720]" \
  -o "/tmp/watch/<slug>/video.mp4" "<URL>"

ffmpeg -i "/tmp/watch/<slug>/video.mp4" \
  -vf "select='gt(scene,0.3)',showinfo" -vsync vfr \
  "/tmp/watch/<slug>/frames/frame_%04d.png" 2> "/tmp/watch/<slug>/scenes.log"
```

Порог `0.3` — крути `0.2`–`0.4` по плотности сцен. Если кажется, что склейки потерялись — не
угадывай порог, посмотри реальные оценки:

```bash
ffmpeg -v info -i "/tmp/watch/<slug>/video.mp4" \
  -vf "select='gt(scene,0.01)',metadata=print:file=-" -f null - 2>/dev/null
```

Низкоконтрастные склейки (тёмное на тёмное, один слайд сменился похожим) дают `scene_score`
около `0.15`, и весь диапазон `0.2`–`0.4` их пропускает — тогда опускай порог до `0.1`–`0.15`.
Проверено: на тестовом ролике порог `0.2` терял склейку со score `0.165`, `0.15` ловил все.

Тайм-коды кадров бери из showinfo-лога (`pts_time`), чтобы привязать кадр к моменту:

```bash
grep -o 'pts_time:[0-9.]*' "/tmp/watch/<slug>/scenes.log" | cut -d: -f2 | nl
```

`n`-я строка соответствует `frame_<n>.png`. Если кадров слишком много — оставь до ~150 самых
информативных, прореживая близкие по времени.

### 5. Анализ

- Прочитай транскрипт (`.srt`).
- Просмотри извлечённые кадры (vision): слайды, код, демо, графика.
- Свяжи визуал с транскриптом по тайм-кодам.

### 6. Отчёт

Выдай:

- **TL;DR** — 3–5 строк;
- **Ключевые концепции** с тайм-кодами;
- **Что показано НА ЭКРАНЕ, чего нет в тексте**;
- **Заметные моменты / цитаты** с тайм-кодами.

### 7. Ингест в базу знаний (спросить)

Спроси: «Сохранить разбор в твою базу знаний (Obsidian)?». Если да — запиши markdown-заметку
в указанную папку со ссылками на связанные заметки. Без подтверждения — не сохраняй.

## Правила

- Рабочие файлы — во временную папку, проект не засоряй.
- Ничего не публикуй и не отправляй наружу без явной просьбы.
- Приватное видео с логином не обходи — сообщи об этом.
