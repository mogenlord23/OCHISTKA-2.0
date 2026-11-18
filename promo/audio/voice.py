"""Озвучка: ElevenLabs (если есть ключ) или временный голос espeak-ng."""
import json
import os
import subprocess
import urllib.request
import wave

import numpy as np

SR = 44100
LEAD = 0.25  # пауза после начала сцены, пока выезжает заголовок
TAIL = 0.15
MAX_SPEEDUP = 1.15
DEFAULT_VOICE = "21m00Tcm4TlvDq8ikWAM"


def _http_post(url, headers, body):
    req = urllib.request.Request(url, data=json.dumps(body).encode(), headers=headers, method="POST")
    with urllib.request.urlopen(req, timeout=60) as r:
        return r.read()


def _espeak_pcm(text):
    sp = subprocess.run(["espeak-ng", "-v", "ru", "-s", "165", "--stdout", text], check=True, capture_output=True)
    ff = subprocess.run(
        ["ffmpeg", "-v", "error", "-i", "-", "-f", "s16le", "-ar", str(SR), "-ac", "1", "-"],
        input=sp.stdout, check=True, capture_output=True,
    )
    return ff.stdout


def _speed(pcm, factor):
    ff = subprocess.run(
        ["ffmpeg", "-v", "error", "-f", "s16le", "-ar", str(SR), "-ac", "1", "-i", "-",
         "-filter:a", f"atempo={factor}", "-f", "s16le", "-ar", str(SR), "-ac", "1", "-"],
        input=pcm, check=True, capture_output=True,
    )
    return ff.stdout


def make_voice(scenes, out_path, api_key=None, http_post=None):
    post = http_post or _http_post
    voice_id = os.environ.get("ELEVENLABS_VOICE_ID", DEFAULT_VOICE)
    report = {"engine": "elevenlabs" if api_key else "fallback", "warnings": []}
    if not api_key:
        report["warnings"].append("ключ ElevenLabs не задан: используется временный голос espeak-ng")
    total = int(round((scenes[-1]["end"] - scenes[0]["start"]) * SR))
    track = np.zeros(total, dtype=np.int16)
    for s in scenes:
        if api_key:
            url = f"https://api.elevenlabs.io/v1/text-to-speech/{voice_id}?output_format=pcm_44100"
            headers = {"xi-api-key": api_key, "Content-Type": "application/json"}
            pcm = post(url, headers, {"text": s["voice"], "model_id": "eleven_multilingual_v2"})
        else:
            pcm = _espeak_pcm(s["voice"])
        room = (s["end"] - s["start"]) - LEAD - TAIL
        length = len(pcm) / 2 / SR
        if length > room:
            if length / room > MAX_SPEEDUP:
                raise ValueError(f"фраза сцены {s['id']} ({length:.2f} с) не помещается в {room:.2f} с")
            pcm = _speed(pcm, round(length / room + 0.005, 3))
        x = np.frombuffer(pcm, dtype="<i2")
        i = int((s["start"] + LEAD) * SR)
        j = min(total, i + len(x))
        track[i:j] = x[: j - i]
    with wave.open(out_path, "wb") as w:
        w.setnchannels(1)
        w.setsampwidth(2)
        w.setframerate(SR)
        w.writeframes(track.astype("<i2").tobytes())
    return report
