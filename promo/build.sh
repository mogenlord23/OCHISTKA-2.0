#!/usr/bin/env bash
# Собирает promo/out/ochistka-promo.mp4. Ключ ElevenLabs: переменная ELEVENLABS_API_KEY (необязательно).
# SKIP_RENDER=1 и SKIP_REC=1 пропускают кадры сцен и запись интерфейса (берутся из promo/out).
set -euo pipefail
cd "$(dirname "$0")"
ROOT="$(cd .. && pwd)"
[[ "$(fc-list)" == *Inter* ]] || { echo "нет шрифта Inter" >&2; exit 1; }
for e in broom wastebasket shield rocket lightning sparkles; do
  [ -f "assets/emoji/$e.png" ] || { echo "нет эмодзи $e: запустите assets/fetch.sh" >&2; exit 1; }
done
mkdir -p out
[ -n "${ELEVENLABS_API_KEY:-}" ] || echo "ВНИМАНИЕ: ключа ElevenLabs нет, используется временный голос" >&2

# 1. запись настоящего интерфейса (vite + подменённый бэкенд, виртуальный курсор)
if [ "${SKIP_REC:-0}" != "1" ]; then
  (cd "$ROOT/apps/desktop" && npx vite --port 1420 --strictPort >"$ROOT/promo/out/vite.log" 2>&1) &
  VITE=$!
  trap 'kill $VITE 2>/dev/null || true' EXIT
  for _ in $(seq 1 40); do curl -fs http://127.0.0.1:1420/ >/dev/null && break; sleep 0.5; done
  node app/record.mjs out/rec
  kill $VITE 2>/dev/null || true
fi
[ -f out/rec/frames.txt ] || { echo "нет записи интерфейса out/rec" >&2; exit 1; }

# 2. кадры сцен (фон, заголовки, эмодзи)
if [ "${SKIP_RENDER:-0}" != "1" ]; then rm -rf out/frames; node render.mjs out/frames; fi
NF=$(ls out/frames | wc -l); [ "$NF" = 900 ] || { echo "ожидалось 900 кадров, найдено $NF" >&2; exit 1; }

# 3. звук
python3 - <<'PY'
import os, sys
sys.path.insert(0, "."); sys.path.insert(0, "audio")
from music import make_music
from voice import make_voice
from timeline import load_timeline
make_music("out/music.wav")
rep = make_voice(load_timeline("timeline.json"), "out/voice.wav", api_key=os.environ.get("ELEVENLABS_API_KEY") or None)
print("голос:", rep["engine"], rep["warnings"])
PY

# 4. склейка: запись интерфейса в скруглённой рамке поверх сцен 6–25 с
ffmpeg -v error -y -framerate 30 -i out/frames/frame_%05d.png \
  -i out/voice.wav -i out/music.wav -f concat -safe 0 -i out/rec/frames.txt -loop 1 -framerate 30 -i app/mask.png \
  -filter_complex "[3:v]fps=30,scale=960:1036:flags=lanczos,tpad=stop_mode=clone:stop_duration=2,trim=duration=19,setpts=PTS-STARTPTS+6/TB,format=rgba[r];\
[4:v]scale=960:1036,format=gray,trim=duration=19,setpts=PTS-STARTPTS+6/TB[m];[r][m]alphamerge[rm];[0:v][rm]overlay=60:520:eof_action=pass:repeatlast=0:enable='between(t,6,24.99)'[v];\
[1:a]asplit=2[v1][v2];[2:a][v1]sidechaincompress=threshold=0.02:ratio=8:attack=20:release=300[mu];[mu]volume=0.8[m2];[v2]volume=1.6[vv];\
[vv][m2]amix=inputs=2:duration=first:normalize=0,aformat=sample_rates=44100:channel_layouts=stereo,alimiter=limit=0.95[a]" \
  -map "[v]" -map "[a]" -c:v libx264 -crf 16 -preset medium -pix_fmt yuv420p -r 30 -c:a aac -b:a 192k -t 30 -movflags +faststart out/ochistka-promo.mp4
echo "готово: out/ochistka-promo.mp4"
