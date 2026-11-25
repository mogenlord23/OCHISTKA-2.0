"""Sample Natural Earth land and country label points for the offline VPN globe."""

import base64
import hashlib
import json
import math
import sys
from pathlib import Path

SOURCE_SHA256 = "3e458fc036ad0a66411f2c1e6cac49c5d7bfb81cb1123bc513b22511a2b7fdeb"
STEP = 1.7
GRID_WIDTH, GRID_HEIGHT = 900, 450
GRID_STEP = 360 / GRID_WIDTH


def spans(rings, y):
    cuts = []
    for ring in rings:
        ax, ay = ring[-1]
        for bx, by in ring:
            if (ay > y) != (by > y):
                cuts.append(ax + (bx - ax) * (y - ay) / (by - ay))
            ax, ay = bx, by
    cuts.sort()
    return zip(cuts[::2], cuts[1::2])


def rasterize(polygons):
    labels = bytearray(GRID_WIDTH * GRID_HEIGHT)
    for row in range(GRID_HEIGHT):
        y = -90 + (row + 0.5) * GRID_STEP
        start = row * GRID_WIDTH
        for south, north, index, rings in polygons:
            if not south <= y <= north:
                continue
            for left, right in spans(rings, y):
                low = max(0, math.ceil((left + 180) / GRID_STEP - 0.5))
                high = min(GRID_WIDTH, math.ceil((right + 180) / GRID_STEP - 0.5))
                if high > low:
                    labels[start + low:start + high] = bytes([index]) * (high - low)
    return labels


def pack_detail(labels):
    packed = bytearray(len(labels) // 4)
    for index, land in enumerate(labels):
        if not land:
            continue
        row, column = divmod(index, GRID_WIDTH)
        neighbors = (
            labels[row * GRID_WIDTH + (column - 1) % GRID_WIDTH],
            labels[row * GRID_WIDTH + (column + 1) % GRID_WIDTH],
            labels[max(0, index - GRID_WIDTH)],
            labels[min(len(labels) - 1, index + GRID_WIDTH)],
        )
        point = 2 if any(neighbor != land for neighbor in neighbors) else 1
        packed[index // 4] |= point << (2 * (index % 4))
    return base64.b64encode(packed).decode("ascii")


def main():
    raw = Path(sys.argv[1]).read_bytes()
    if hashlib.sha256(raw).hexdigest() != SOURCE_SHA256:
        raise ValueError("Expected the Natural Earth source listed in THIRD-PARTY.md")
    countries = {}
    lime_ids = set()
    polygons = []
    for index, feature in enumerate(json.loads(raw)["features"], 1):
        properties = feature["properties"]
        code = properties["ISO_A2_EH"]
        continent = properties["CONTINENT"]
        if code != "RU" and (continent in {"Europe", "South America", "Oceania"}
                             or code == "CN" or (continent in {"Africa", "Asia"} and index % 2 == 0)):
            lime_ids.add(index)
        if len(code) == 2 and code.isalpha():
            countries[code] = [properties["LABEL_X"], properties["LABEL_Y"]]
        geometry = feature["geometry"]
        rings = geometry["coordinates"]
        for polygon in (rings if geometry["type"] == "MultiPolygon" else [rings]):
            yy = [point[1] for point in polygon[0]]
            polygons.append((min(yy), max(yy), index, polygon))

    labels = rasterize(polygons)
    palette = bytearray((len(labels) + 7) // 8)
    for cell, label in enumerate(labels):
        if label in lime_ids:
            palette[cell // 8] |= 1 << (cell % 8)
    land = []
    for row in range(math.floor(180 / STEP) + 1):
        y = -89 + row * STEP
        if y > 89:
            break
        grid_row = min(GRID_HEIGHT - 1, int((y + 90) / GRID_STEP))
        count = max(1, round(360 * math.cos(math.radians(y)) / STEP))
        for column in range(count):
            x = -180 + (column + (row % 2) / 2) * 360 / count
            grid_column = min(GRID_WIDTH - 1, int((x + 180) / GRID_STEP))
            if labels[grid_row * GRID_WIDTH + grid_column]:
                land.append([round(x, 2), round(y, 2)])
    output = Path(__file__).resolve().parents[1] / "src/assets/vpn-globe.json"
    data = {"land": land, "countries": countries, "detail": pack_detail(labels),
            "lime": base64.b64encode(palette).decode("ascii"), "gridWidth": GRID_WIDTH, "gridHeight": GRID_HEIGHT}
    output.write_text(json.dumps(data, separators=(",", ":")) + "\n",
                      encoding="utf-8", newline="\n")


if __name__ == "__main__":
    main()
