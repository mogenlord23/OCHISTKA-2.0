import os, sys, wave, tempfile
import numpy as np
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "audio"))
from music import make_music


def test_music_spec():
    p = os.path.join(tempfile.mkdtemp(), "m.wav")
    make_music(p)
    with wave.open(p) as w:
        assert w.getnchannels() == 2
        sr = w.getframerate()
        assert abs(w.getnframes() / sr - 30.0) < 0.02
        x = np.frombuffer(w.readframes(w.getnframes()), dtype=np.int16).astype(float) / 32768
    assert np.abs(x).max() <= 0.95
    assert np.sqrt((x ** 2).mean()) > 0.02
    tail = np.abs(x[-int(1.5 * sr):])
    assert tail[-int(0.05 * sr):].max() < 0.1 * np.abs(x).max()
