"""Sample Natural Earth land and country label points for the offline VPN globe."""

import hashlib
import json
import math
import sys
from pathlib import Path

SOURCE_SHA256 = "3e458fc036ad0a66411f2c1e6cac49c5d7bfb81cb1123bc513b22511a2b7fdeb"
STEP = 2.2


def inside(ring, x, y):
    hit = False
    ax, ay = ring[-1]
    for bx, by in ring:
        if (ay > y) != (by > y) and x < ax + (bx - ax) * (y - ay) / (by - ay):
            hit = not hit
        ax, ay = bx, by
    return hit


def main():
    raw = Path(sys.argv[1]).read_bytes()
    if hashlib.sha256(raw).hexdigest() != SOURCE_SHA256:
        raise ValueError("Expected the Natural Earth source listed in THIRD-PARTY.md")
    countries = {}
    polygons = []
    for feature in json.loads(raw)["features"]:
        properties = feature["properties"]
        code = properties["ISO_A2_EH"]
        if len(code) == 2 and code.isalpha():
            countries[code] = [properties["LABEL_X"], properties["LABEL_Y"]]
        geometry = feature["geometry"]
        rings = geometry["coordinates"]
        for polygon in (rings if geometry["type"] == "MultiPolygon" else [rings]):
            xx, yy = zip(*polygon[0])
            polygons.append((min(xx), max(xx), min(yy), max(yy), polygon))

    land = []
    for row in range(math.floor(180 / STEP) + 1):
        y = -89 + row * STEP
        if y > 89:
            break
        candidates = [record for record in polygons if record[2] <= y <= record[3]]
        count = max(1, round(360 * math.cos(math.radians(y)) / STEP))
        for column in range(count):
            x = -180 + (column + (row % 2) / 2) * 360 / count
            if any(left <= x <= right and inside(rings[0], x, y)
                   and not any(inside(hole, x, y) for hole in rings[1:])
                   for left, right, _, _, rings in candidates):
                land.append([round(x, 2), round(y, 2)])
    output = Path(__file__).resolve().parents[1] / "src/assets/vpn-globe.json"
    output.write_text(json.dumps({"land": land, "countries": countries}, separators=(",", ":")) + "\n",
                      encoding="utf-8", newline="\n")


if __name__ == "__main__":
    main()
