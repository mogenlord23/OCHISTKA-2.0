import os
from PIL import Image

D = os.path.join(os.path.dirname(__file__), "..", "assets", "emoji")
NAMES = ["broom", "wastebasket", "shield", "rocket", "lightning", "sparkles"]


def test_required_emoji_exist_and_are_rgba():
    for n in NAMES:
        im = Image.open(os.path.join(D, n + ".png"))
        assert im.mode == "RGBA" and min(im.size) >= 256, n
