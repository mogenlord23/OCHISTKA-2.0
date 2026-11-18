#!/usr/bin/env bash
# Собирает promo/out/ochistka-promo.mp4. Ключ ElevenLabs: переменная ELEVENLABS_API_KEY (необязательно).
set -euo pipefail
cd "$(dirname "$0")"
[[ "$(fc-list)" == *Inter* ]] || { echo "нет шрифта Inter" >&2; exit 1; }
for e in broom wastebasket shield rocket lightning sparkles; do
  [ -f "assets/emoji/$e.png" ] || { echo "нет эмодзи $e: запустите assets/fetch.sh" >&2; exit 1; }
done
mkdir -p out
if [ -z "${ELEVENLABS_API_KEY:-}" ]; then echo "ВНИМАНИЕ: ключа ElevenLabs нет, используется временный голос" >&2; fi
if [ "${SKIP_RENDER:-0}" != "1" ]; then rm -rf out/frames; node render.mjs out/frames; fi
NF=$(ls out/frames | wc -l); [ "$NF" = 900 ] || { echo "ожидалось 900 кадров, найдено $NF" >&2; exit 1; }
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
ffmpeg -v error -y -framerate 30 -i out/frames/frame_%05d.png -i out/voice.wav -i out/music.wav \
  -filter_complex "[1:a]asplit=2[v1][v2];[2:a][v1]sidechaincompress=threshold=0.02:ratio=8:attack=20:release=300[m];[m]volume=0.8[m2];[v2]volume=1.6[vv];[vv][m2]amix=inputs=2:duration=first:normalize=0,aformat=sample_rates=44100:channel_layouts=stereo,alimiter=limit=0.95[a]" \
  -map 0:v -map "[a]" -c:v libx264 -crf 16 -preset medium -pix_fmt yuv420p -r 30 -c:a aac -b:a 192k -t 30 -movflags +faststart out/ochistka-promo.mp4
echo "готово: out/ochistka-promo.mp4"
