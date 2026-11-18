import json


def load_timeline(path):
    with open(path, encoding="utf-8") as f:
        return json.load(f)


def total_duration(scenes):
    return scenes[-1]["end"] - scenes[0]["start"]
