import os, sys, wave, tempfile
import numpy as np
import pytest
sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "audio"))
from timeline import load_timeline
from voice import make_voice

SC = load_timeline(os.path.join(os.path.dirname(__file__), "..", "timeline.json"))
SR = 44100


def read(p):
    with wave.open(p) as w:
        assert w.getframerate() == SR and w.getnchannels() == 1
        return np.frombuffer(w.readframes(w.getnframes()), dtype=np.int16)


def test_fallback_without_key_warns_and_fits():
    out = os.path.join(tempfile.mkdtemp(), "v.wav")
    rep = make_voice(SC, out, api_key=None)
    assert rep["engine"] == "fallback"
    assert any("временный" in w for w in rep["warnings"])
    x = read(out)
    assert abs(len(x) / SR - 30.0) < 0.02
    for s in SC:  # в интервале каждой сцены есть речь
        seg = x[int(s["start"] * SR):int(s["end"] * SR)]
        assert np.abs(seg).max() > 1000, s["id"]


def test_elevenlabs_path_uses_key_and_never_leaks_it():
    calls = []

    def fake_post(url, headers, body):
        calls.append((url, headers, body))
        return (np.sin(np.arange(SR) * 0.05) * 8000).astype("<i2").tobytes()  # 1 с PCM

    out = os.path.join(tempfile.mkdtemp(), "v.wav")
    rep = make_voice(SC, out, api_key="sk_secret_123", http_post=fake_post)
    assert rep["engine"] == "elevenlabs" and len(calls) == len(SC)
    assert calls[0][1]["xi-api-key"] == "sk_secret_123"
    assert "pcm_44100" in calls[0][0]
    assert "sk_secret_123" not in repr(rep)
    assert abs(len(read(out)) / SR - 30.0) < 0.02


def test_too_long_phrase_fails_loudly():
    def long_post(url, headers, body):
        return np.zeros(SR * 40, dtype="<i2").tobytes()

    with pytest.raises(ValueError):
        make_voice(SC, os.path.join(tempfile.mkdtemp(), "v.wav"), api_key="k", http_post=long_post)
