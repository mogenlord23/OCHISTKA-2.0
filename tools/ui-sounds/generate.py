#!/usr/bin/env python3
"""Генерирует свои короткие звуки интерфейса: мягкий тактильный щелчок (ASMR). Чужих пакетов нет.

Синтез (по числам образца, сам образец не используется): почти чистый синус 480-560 Гц с лёгким
падением тона, атака 6-8 мс, быстрое затухание (-20 дБ за ~25 мс), чуть тёплого шума, длительность
~70 мс, пик -18 dBFS. Без металла: гармоник и высоких частот нет, хвоста нет.
Запуск: python3 tools/ui-sounds/generate.py
Результат: apps/desktop/src/assets/sounds/click.wav и switch.wav (моно, 22,05 кГц, 16 бит).
Проверка (числа): python3 tools/ui-sounds/generate.py --check
"""

import math
import random
import struct
import sys
import wave
from pathlib import Path

RATE = 22050
OUT = Path(__file__).resolve().parents[2] / "apps/desktop/src/assets/sounds"
PEAK_DBFS = -18.0


def render(name: str, freq: float, ms: int, decay: float, noise: float, lowpass: float, attack_ms: float = 7.0) -> None:
    rnd = random.Random(11)  # детерминированно: файл воспроизводим
    n = int(RATE * ms / 1000)
    samples = []
    low = 0.0
    low2 = 0.0
    for i in range(n):
        t = i / RATE
        env = math.exp(-t * decay)
        attack = min(1.0, i / (RATE * attack_ms / 1000))  # мягкое нарастание: без треска
        tone = math.sin(2 * math.pi * (freq * (1 + 0.06 * math.exp(-t * 60)) * t))  # тон чуть опускается
        low += lowpass * ((rnd.random() * 2 - 1) - low)  # два ФНЧ подряд: шум тёплый, без шипения
        low2 += lowpass * (low - low2)
        value = (tone * (1 - noise) + low2 * noise * 5) * env * attack
        fade = min(1.0, (n - i) / (RATE * 0.008))  # 8 мс затухания в конце
        samples.append(value * fade)
    top = max(abs(s) for s in samples) or 1.0
    gain = 10 ** (PEAK_DBFS / 20)
    OUT.mkdir(parents=True, exist_ok=True)
    with wave.open(str(OUT / f"{name}.wav"), "wb") as f:
        f.setnchannels(1)
        f.setsampwidth(2)
        f.setframerate(RATE)
        f.writeframes(b"".join(struct.pack("<h", int(s / top * gain * 32767)) for s in samples))


def check(name: str) -> None:
    import numpy as np

    path = OUT / f"{name}.wav"
    with wave.open(str(path), "rb") as f:
        raw = f.readframes(f.getnframes())
        rate = f.getframerate()
    x = np.frombuffer(raw, dtype="<i2").astype(float) / 32768
    peak = 20 * math.log10(np.abs(x).max())
    spec = np.abs(np.fft.rfft(x * np.hanning(len(x)), 8192)) ** 2
    freqs = np.fft.rfftfreq(8192, 1 / rate)
    total = spec.sum()
    above6k = spec[freqs > 6000].sum() / total
    centroid = (spec * freqs).sum() / total
    print(
        f"{name}: {len(x) / rate * 1000:.1f} ms, peak {peak:.1f} dBFS, "
        f"centroid {centroid:.0f} Hz, energy>6kHz {above6k * 100:.4f}%, size {path.stat().st_size} B"
    )


if __name__ == "__main__":
    if "--check" not in sys.argv:
        render("click", freq=560, ms=70, decay=90, noise=0.12, lowpass=0.12)
        render("switch", freq=470, ms=76, decay=80, noise=0.1, lowpass=0.1, attack_ms=8)
    for n in ("click", "switch"):
        check(n)
