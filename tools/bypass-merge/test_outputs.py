"""Проверки готовых файлов слияния (офлайн, без RixyPow): хэши, имена, ссылки, манифест."""
import hashlib
import json
import os
import re
import unittest

import merge
import zpreset

REPO = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
PRESET = os.path.join(REPO, "assets/desync/presets/OCHISTKA Merged.txt")
BASE = os.path.join(REPO, "assets/desync/presets/Universal V13 Lite.txt")
RUNTIME = os.path.join(REPO, "assets/desync/runtime")
EXTRA = os.path.join(REPO, "assets/desync/runtime-extra")
SAFE = re.compile(r"[A-Za-z0-9_.\-]+")
LINE = re.compile(r"^[A-Za-z0-9_.\-*/:]+$")  # домен, адрес или сеть


def read(p):
    with open(p, encoding="utf-8") as fh:
        return fh.read()


class OutputTests(unittest.TestCase):
    def test_merged_preset_has_no_duplicates_and_keeps_base_first(self):
        m = zpreset.parse(read(PRESET))
        base = zpreset.parse(read(BASE))
        ids = [s.identity() for s in m.sections]
        self.assertEqual(len(ids), len(set(ids)))
        self.assertEqual(len(m.globals), len(set(m.globals)))
        self.assertEqual([s.args for s in m.sections[: len(base.sections)]], [s.args for s in base.sections])
        for g in base.globals:
            self.assertIn(g, m.globals)

    def test_coverage_manifest_matches_preset(self):
        cov = json.loads(read(os.path.join(REPO, "docs/status/bypass-merge-coverage.json")))
        m = zpreset.parse(read(PRESET))
        self.assertEqual(merge.check_coverage(cov, m.sections, m.globals), [])
        conflicts = read(os.path.join(REPO, "docs/status/bypass-merge-conflicts.md"))
        for rec in cov["sections"] + cov["globals"]:
            if rec["status"] in ("conflict", "excluded"):
                self.assertIn(rec["item_id"], conflicts)

    def test_every_referenced_resource_exists_in_pinned_or_extra_tree(self):
        m = zpreset.parse(read(PRESET))
        for rel in zpreset.referenced_resources(m.globals + [a for s in m.sections for a in s.args]):
            self.assertTrue(
                os.path.isfile(os.path.join(RUNTIME, rel)) or os.path.isfile(os.path.join(EXTRA, rel)), rel
            )

    def test_extra_resources_are_safe_data_with_matching_hashes(self):
        man = json.loads(read(os.path.join(REPO, "docs/status/bypass-merge-resources.json")))
        on_disk = set()
        for root, _, files in os.walk(EXTRA):
            for f in files:
                on_disk.add(os.path.relpath(os.path.join(root, f), EXTRA).replace("\\", "/"))
        self.assertEqual(on_disk, {f["destination"] for f in man["files"]})
        pinned = json.loads(read(os.path.join(REPO, "docs/status/desync-preset-resources.json")))
        pinned_names = {f["destination"] for f in pinned["files"]}
        for f in man["files"]:
            rel = f["destination"]
            self.assertNotIn(rel, pinned_names, "135 закреплённых файлов не пересекаются с новыми")
            parts = rel.split("/")
            self.assertEqual(parts[0], "lists")
            self.assertEqual(len(parts), 2)
            self.assertTrue(SAFE.fullmatch(parts[1]) and ".." not in rel)
            path = os.path.join(EXTRA, rel)
            self.assertFalse(os.path.islink(path))
            data = open(path, "rb").read()
            self.assertEqual(hashlib.sha256(data).hexdigest(), f["sha256"])
            self.assertEqual(len(data), f["size_bytes"])
            for n, line in enumerate(data.decode("utf-8").splitlines(), 1):
                line = line.strip()
                self.assertTrue(not line or line.startswith("#") or LINE.match(line), f"{rel}:{n}")

    def test_no_executable_or_lua_code_is_added(self):
        for root, _, files in os.walk(EXTRA):
            for f in files:
                self.assertTrue(f.endswith(".txt"), f)


if __name__ == "__main__":
    unittest.main()
