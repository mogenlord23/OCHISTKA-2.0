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
    def test_owner_decisions_telegram_steam_riot_are_fully_removed(self):
        words = ("telegram", "steam", "riot", "valorant")
        m = zpreset.parse(read(PRESET))
        text = "\n".join(m.globals + [a for s in m.sections for a in s.args] + [s.name for s in m.sections]).lower()
        for w in words:
            self.assertNotIn(w, text)
        routes_text = read(os.path.join(REPO, "assets/desync/routes/OCHISTKA Routes.json")).lower()
        for w in words:
            self.assertNotIn(w, routes_text)
        for root, _, files in os.walk(EXTRA):
            for f in files:
                for w in words:
                    self.assertNotIn(w, f.lower())

    def test_slimming_is_idempotent_and_proven(self):
        import slim

        def load(rel):
            for root in (RUNTIME, EXTRA):
                path = os.path.join(root, rel)
                if os.path.isfile(path):
                    return [x.strip() for x in read(path).splitlines() if x.strip() and not x.strip().startswith("#")]
            return None

        m = zpreset.parse(read(PRESET))
        self.assertEqual(slim.find_dead(m.sections, load), [], "в итоговом пресете не осталось недостижимых секций")

    def test_protected_files_are_untouched(self):
        base = os.path.join(REPO, "assets/desync/presets/Universal V13 Lite.txt")
        self.assertEqual(
            hashlib.sha256(open(base, "rb").read()).hexdigest(),
            "e679901475d998cb0c7e5ed5c51a5286c3b97f02d4b6c0d947f4527830cacae7",
        )
        pinned = json.loads(read(os.path.join(REPO, "docs/status/desync-preset-resources.json")))
        self.assertEqual(pinned["total_resources"], 135)
        for f in pinned["files"]:
            data = open(os.path.join(REPO, f["asset_path"]), "rb").read()
            self.assertEqual(hashlib.sha256(data).hexdigest(), f["sha256"], f["destination"])
        for name in ("ipset-telegram.txt", "russia-blacklist.txt"):
            self.assertTrue(os.path.isfile(os.path.join(RUNTIME, "lists", name)), name)

    def test_claude_stays_a_vpn_only_route_without_own_bypass(self):
        data = json.loads(read(os.path.join(REPO, "assets/desync/routes/OCHISTKA Routes.json")))
        claude = [r for r in data["routes"] if "claude" in (r.get("file") or r.get("value", ""))]
        self.assertTrue(claude)
        for r in claude:
            if r["mode"] == "proxy":
                self.assertEqual(r["applied"], "vpn-only")
            self.assertNotEqual(r["applied"], "shield")
        m = zpreset.parse(read(PRESET))
        self.assertFalse(any(s.name.startswith("Маршрут (pass): ") and "claude" in s.name for s in m.sections))

    def test_merged_preset_has_no_duplicates_and_keeps_base_first(self):
        m = zpreset.parse(read(PRESET))
        base = zpreset.parse(read(BASE))
        ids = [s.identity() for s in m.sections]
        self.assertEqual(len(ids), len(set(ids)))
        self.assertEqual(len(m.globals), len(set(m.globals)))
        words = ("telegram", "steam", "riot", "valorant")
        kept = [s for s in base.sections if not any(w in " ".join(s.args).lower() for w in words)]
        # Основа идёт первой и в прежнем порядке; пропавшие секции основы — только недостижимые
        # (перекрыты более ранними), они записаны в манифесте как dead.
        cov = json.loads(read(os.path.join(REPO, "docs/status/bypass-merge-coverage.json")))
        dead_ids = {r["identity"] for r in cov["sections"] if r["status"] == "dead"}
        expected = [s for s in kept if s.identity() not in dead_ids]
        self.assertEqual([s.args for s in m.sections[: len(expected)]], [s.args for s in expected])
        for s_ in kept:
            self.assertTrue(s_.identity() in dead_ids or s_.args in [x.args for x in m.sections])
        for g in base.globals:
            if not any(w in g.lower() for w in words):
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

    def test_routes_file_has_no_duplicates_and_points_to_existing_data(self):
        data = json.loads(read(os.path.join(REPO, "assets/desync/routes/OCHISTKA Routes.json")))
        keys = [(r["kind"], r.get("file") or r.get("value")) for r in data["routes"]]
        self.assertEqual(len(keys), len(set(keys)))
        self.assertEqual(sum(data["applied_counts"].values()), len(data["routes"]))
        for r in data["routes"]:
            self.assertIn(r["applied"], ("preset", "shield", "direct", "vpn-only", "catalog"))
            self.assertIn(r["category"], ("game", "app", "domain", "network"))
            if "file" in r:
                self.assertTrue(
                    os.path.isfile(os.path.join(RUNTIME, r["file"])) or os.path.isfile(os.path.join(EXTRA, r["file"])),
                    r["file"],
                )
        # proxy-маршруты не применяются этим переключателем (VPN отдельно)
        self.assertTrue(all(r["applied"] == "vpn-only" for r in data["routes"] if r["mode"] == "proxy"))
        # щиты pass есть в пресете как секции
        m = zpreset.parse(read(PRESET))
        shields = [s for s in m.sections if s.name.startswith("Маршрут (pass)")]
        self.assertEqual(len(shields), sum(1 for r in data["routes"] if r["applied"] == "shield"))

    def test_generated_rust_assets_list_exactly_the_manifest_files(self):
        man = json.loads(read(os.path.join(REPO, "docs/status/bypass-merge-resources.json")))
        rs = read(os.path.join(REPO, "crates/platform/src/desync/merged_assets.rs"))
        for f in man["files"]:
            self.assertIn(f'"{f["destination"]}"', rs)
            self.assertIn(f'"{f["sha256"]}"', rs)
        self.assertEqual(rs.count("include_bytes!"), len(man["files"]))

    def test_no_executable_or_lua_code_is_added(self):
        for root, _, files in os.walk(EXTRA):
            for f in files:
                self.assertTrue(f.endswith(".txt"), f)


if __name__ == "__main__":
    unittest.main()
