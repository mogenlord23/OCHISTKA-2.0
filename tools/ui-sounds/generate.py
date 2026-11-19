#!/usr/bin/env python3
"""Генерирует свои короткие звуки интерфейса (мягкий щелчок). Чужих пакетов нет.

Запуск: python3 tools/ui-sounds/generate.py
Результат: apps/desktop/src/assets/sounds/click.wav и switch.wav (моно, 22,05 кГц, 16 бит, ~60 мс).
"""

import math
import random
import struct
import wave
from pathlib import Path

RATE = 22050
OUT = Path(__file__).resolve().parents[2] / "apps/desktop/src/assets/sounds"


def render(name: str, freq: float, ms: int, decay: float, noise: float, peak: float) -> None:
    rnd = random.Random(7)  # детерминированно: файл воспроизводим
    n = int(RATE * ms / 1000)
    samples = []
    low = 0.0
    for i in range(n):
        t = i / RATE
        env = math.exp(-t * decay)
        attack = min(1.0, i / (RATE * 0.0015))  # 1.5 мс нарастания: без треска
        tone = math.sin(2 * math.pi * freq * t) + 0.35 * math.sin(2 * math.pi * freq * 2.01 * t)
        low += 0.25 * ((rnd.random() * 2 - 1) - low)  # приглушённый шум: «мягкий» удар
        value = (tone * (1 - noise) + low * noise * 4) * env * attack
        fade = min(1.0, (n - i) / (RATE * 0.008))  # 8 мс затухания в конце
        samples.append(value * fade)
    top = max(abs(s) for s in samples) or 1.0
    OUT.mkdir(parents=True, exist_ok=True)
    with wave.open(str(OUT / f"{name}.wav"), "wb") as f:
        f.setnchannels(1)
        f.setsampwidth(2)
        f.setframerate(RATE)
        f.writeframes(b"".join(struct.pack("<h", int(s / top * peak * 32767)) for s in samples))


render("click", freq=1900, ms=60, decay=70, noise=0.45, peak=0.55)
render("switch", freq=1250, ms=70, decay=55, noise=0.3, peak=0.5)
