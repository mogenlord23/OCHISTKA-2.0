"""Синтез фоновой музыки: бас, кик, хэт, пэд; 110 BPM, нарастание к 21 с."""
import wave
import numpy as np


def make_music(path, duration=30.0, bpm=110, sr=44100):
    n = int(duration * sr)
    t = np.arange(n) / sr
    beat = 60.0 / bpm
    rng = np.random.default_rng(7)
    out = np.zeros(n)
    # нарастание: тихо в хуке, полная громкость с 6 с, пик в финале
    energy = np.interp(t, [0, 3, 6, 21, 25, 30], [0.5, 0.6, 0.9, 1.0, 1.1, 1.1])

    def add(sig, start):
        i = int(start * sr)
        j = min(n, i + len(sig))
        if i < n:
            out[i:j] += sig[: j - i]

    # кик на каждую долю
    k = np.arange(int(0.35 * sr)) / sr
    kick = np.sin(2 * np.pi * (55 + 90 * np.exp(-k * 30)) * k) * np.exp(-k * 9)
    # хэт на каждую восьмую
    h = np.arange(int(0.05 * sr)) / sr
    hat = rng.standard_normal(len(h)) * np.exp(-h * 90) * 0.25
    # бас: прогрессия Am F C G, по такту
    roots = [55.0, 43.65, 65.41, 49.0]
    bar = beat * 4
    for b in range(int(duration / beat) + 1):
        s = b * beat
        if s >= 3.0:
            add(kick * 0.9, s)
        else:
            add(kick * 0.5, s) if b % 2 == 0 else None
        add(hat, s + beat / 2)
        if s >= 6.0 and b % 4 == 2:
            add(hat * 1.5, s)
    for bi in range(int(duration / bar) + 1):
        f = roots[bi % 4]
        s = bi * bar
        m = np.arange(int(bar * sr)) / sr
        env = np.minimum(1, m * 40) * np.exp(-m * 1.2)
        bass = (np.sin(2 * np.pi * f * m) + 0.3 * np.sin(2 * np.pi * 2 * f * m)) * env * 0.35
        add(bass if s >= 3.0 else bass * 0.5, s)
        pad = sum(np.sin(2 * np.pi * f * r * m + i) for i, r in enumerate([4, 5, 6])) * 0.05
        add(pad * np.minimum(1, m * 2) * np.minimum(1, (bar - m) * 4), s)
    out *= energy
    # затухание последних 1.5 с
    fade = np.clip((duration - t) / 1.5, 0, 1) ** 2
    out *= fade
    out = np.tanh(out * 1.4) * 0.7
    st = np.stack([out, np.roll(out, int(0.0004 * sr))], axis=1)
    st = np.clip(st / max(1e-9, np.abs(st).max()) * 0.9, -0.95, 0.95)
    pcm = (st * 32767).astype("<i2")
    with wave.open(path, "wb") as w:
        w.setnchannels(2)
        w.setsampwidth(2)
        w.setframerate(sr)
        w.writeframes(pcm.tobytes())
