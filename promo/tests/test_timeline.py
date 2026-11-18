import os, sys
sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))
from timeline import load_timeline, total_duration

PATH = os.path.join(os.path.dirname(__file__), "..", "timeline.json")


def test_timeline_covers_30s_without_gaps():
    s = load_timeline(PATH)
    assert len(s) == 7
    assert s[0]["start"] == 0
    assert s[-1]["end"] == 30.0
    for a, b in zip(s, s[1:]):
        assert a["end"] == b["start"]
    assert total_duration(s) == 30.0


def test_every_scene_has_voice_text():
    for sc in load_timeline(PATH):
        assert sc["voice"].strip() and sc["title"].strip() and sc["accent"] in sc["title"]
