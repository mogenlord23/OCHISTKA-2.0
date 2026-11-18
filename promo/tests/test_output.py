import json, os, re, subprocess
import pytest

OUT = os.path.join(os.path.dirname(__file__), "..", "out", "ochistka-promo.mp4")


def probe():
    r = subprocess.run(["ffprobe", "-v", "error", "-show_streams", "-show_format", "-of", "json", OUT],
                       check=True, capture_output=True, text=True)
    return json.loads(r.stdout)


def test_output_spec():
    assert os.path.exists(OUT), "сначала bash promo/build.sh"
    d = probe()
    v = next(s for s in d["streams"] if s["codec_type"] == "video")
    a = next(s for s in d["streams"] if s["codec_type"] == "audio")
    assert (v["width"], v["height"], v["codec_name"]) == (1080, 1920, "h264")
    assert v["r_frame_rate"] == "30/1" and v["pix_fmt"] == "yuv420p"
    assert a["codec_name"] == "aac"
    assert abs(float(d["format"]["duration"]) - 30.0) < 0.05


def test_audio_not_silent():
    r = subprocess.run(["ffmpeg", "-i", OUT, "-af", "volumedetect", "-vn", "-f", "null", "-"],
                       capture_output=True, text=True)
    mean = float(re.search(r"mean_volume: (-?[\d.]+) dB", r.stderr).group(1))
    assert mean > -40
