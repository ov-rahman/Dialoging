---
name: watch
description: |
  Смотрит любое видео по ссылке (YouTube, Loom, запись Zoom, прямой mp4) — не только читает
  транскрипт, но и ВИДИТ, что на экране. Кадры берёт по сменам сцен (ffmpeg scene detection),
  а не по таймеру, поэтому не пропускает графику, демо, монтаж и b-roll. Звук: субтитры через
  yt-dlp, fallback — Whisper. Работает и со ссылкой, и с прикреплённым/локальным видеофайлом.
  Используй, когда пользователь пишет /watch <url>, присылает /watch с вложенным видео, или
  просит «посмотри это видео», «разбери ролик/созвон/лекцию», «что показано на экране».
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
- `<slug>` — короткий безопасный идентификатор ролика (например, ID видео или имя файла).
- **Определи тип источника** — от него зависят шаги 2–4:
  - **Ссылка** (YouTube / Loom / Zoom-share / прямой видеофайл по http) → шаги 2, 3, 4 как есть.
  - **Локальный файл** (пользователь прикрепил видео или дал путь) → шаг 1b, затем сразу шаг 5.
    yt-dlp тут не нужен и не сработает: на локальный путь он отвечает `is not a valid URL`.
    Путь к вложению приходит вместе с сообщением; в веб-сессии файлы лежат в
    `~/.claude/uploads/<session-id>/`.

### 1b. Локальный файл (прикреплённое видео) — вместо шагов 2–4

Скачивать нечего, работает один ffmpeg. `<FILE>` — путь к видео.

```bash
# что внутри файла: есть ли дорожки субтитров и звука
ffprobe -v error -show_entries stream=index,codec_type,codec_name -of csv=p=0 "<FILE>"
```

```bash
# субтитры: если в файле есть дорожка (s) — вынимаем её, это готовый транскрипт с тайм-кодами
if ffprobe -v error -select_streams s -show_entries stream=index -of csv=p=0 "<FILE>" | grep -q .; then
  ffmpeg -v error -y -i "<FILE>" -map 0:s:0 "/tmp/watch/<slug>/subs.srt"
fi
```

```bash
# иначе — звук в mp3 и Whisper (как в шаге 3).
# Если звуковой дорожки (a) нет — Whisper запускать нечего, скажи это пользователю, не падай.
ffmpeg -v error -y -i "<FILE>" -vn -acodec libmp3lame -ar 16000 -ac 1 "/tmp/watch/<slug>/audio.mp3"

whisper "/tmp/watch/<slug>/audio.mp3" --model small --language ru \
  --output_format srt --output_dir "/tmp/watch/<slug>/"
```

```bash
# кадры по сменам сцен — та же команда, что в шаге 4, просто по файлу напрямую
ffmpeg -i "<FILE>" -vf "select='gt(scene,0.3)',showinfo" -vsync vfr \
  "/tmp/watch/<slug>/frames/frame_%04d.png" 2> "/tmp/watch/<slug>/scenes.log"
```

Дальше — шаг 5 (анализ) без изменений; подбор порога и разбор `scene_score` из шага 4 применимы
и здесь.

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
