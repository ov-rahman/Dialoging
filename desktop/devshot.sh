#!/usr/bin/env bash
# Запускает приложение на виртуальном экране и снимает кадр.
# Нужен, чтобы проверять GUI по-настоящему, а не «оно скомпилировалось».
#
#   ./devshot.sh out.png [ширина] [высота] [секунд-до-снимка]
set -u

OUT=${1:-/tmp/dialoging.png}
W=${2:-1200}
H=${3:-860}
WAIT=${4:-5}
BIN=${BIN:-target/debug/dialoging}
DISP=:99

cleanup() { kill "${APID:-}" "${XPID:-}" 2>/dev/null; wait 2>/dev/null; }
trap cleanup EXIT

[ -x "$BIN" ] || { echo "нет бинаря: $BIN"; exit 1; }

Xvfb "$DISP" -screen 0 "${W}x${H}x24" -nolisten tcp >/dev/null 2>&1 &
XPID=$!
sleep 1

# llvmpipe: в контейнере нет GPU, glow должен идти через программный Mesa
DISPLAY=$DISP LIBGL_ALWAYS_SOFTWARE=1 GALLIUM_DRIVER=llvmpipe \
  "$BIN" >/tmp/dialoging-run.log 2>&1 &
APID=$!

sleep "$WAIT"

if ! kill -0 "$APID" 2>/dev/null; then
  echo "ПРИЛОЖЕНИЕ УПАЛО. Лог:"
  tail -25 /tmp/dialoging-run.log
  exit 1
fi

DISPLAY=$DISP import -window root "$OUT" 2>/dev/null
echo "снято: $OUT ($(stat -c%s "$OUT" 2>/dev/null) байт)"
