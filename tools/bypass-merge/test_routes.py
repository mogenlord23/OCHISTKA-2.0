import json
import os
import unittest

import routes
import zpreset

RULES = """# комментарий
mode: selective
capture:
  - config/lists/ipset-telegram.txt
rules:
  # direct
  - match: hostlist
    value: config/lists/steam.txt
    mode: direct
  - match: domain
    value: "*.avito.ru"
    mode: direct
  - match: domain
    value: "*.avito.ru"
    mode: direct
  - match: hostlist
    value: config/lists/chatgpt.txt
    mode: proxy
    server: "auto"
  - match: hostlist
    value: config/lists/seriesgraph.txt
    mode: desync
  - match: domain
    value: "*.wardogs.example.com"
    mode: desync
  - match: hostlist
    value: config/lists/github.txt
    mode: desync
"""

LISTS = {  # имя -> (sha, строк)
    "steam.txt": ("s1", 3), "chatgpt.txt": ("s2", 5), "seriesgraph.txt": ("s3", 1),
    "github.txt": ("s4", 8), "battlenet.txt": ("s5", 4), "ipset-telegram.txt": ("s6", 9),
    "dup-of-github.txt": ("s4", 8),
}


def run(covered):
    return routes.build(
        rules=routes.parse_rules(RULES), config_lists=LISTS,
        existing_by_sha={"s6": "lists/ipset-telegram.txt"}, preset_lists=covered,
    )


class RoutesTests(unittest.TestCase):
    def test_parse_rules_keeps_mode_value_and_server(self):
        r = routes.parse_rules(RULES)
        self.assertEqual(len(r), 7)
        self.assertEqual((r[0]["match"], r[0]["value"], r[0]["mode"]), ("hostlist", "config/lists/steam.txt", "direct"))
        self.assertEqual(r[3]["server"], "auto")

    def test_duplicate_rules_and_identical_lists_are_merged(self):
        res = run({"lists/nz-steam.txt"})
        values = [(e["kind"], e.get("value") or e.get("file")) for e in res.entries]
        self.assertEqual(len(values), len(set(values)))
        self.assertEqual(sum(1 for e in res.entries if e.get("value") == "avito.ru"), 1)
        # github.txt и dup-of-github.txt совпадают по содержимому: один файл
        files = [e["file"] for e in res.entries if "file" in e]
        self.assertEqual(len(files), len(set(files)))

    def test_modes_decide_what_is_applied(self):
        res = run({"lists/nz-steam.txt", "lists/nz-github.txt"})
        by = {(e.get("file") or e.get("value")): e for e in res.entries}
        self.assertEqual(by["lists/nz-chatgpt.txt"]["applied"], "vpn-only")
        self.assertEqual(by["lists/nz-github.txt"]["applied"], "preset")
        self.assertEqual(by["lists/nz-seriesgraph.txt"]["applied"], "shield")
        self.assertEqual(by["wardogs.example.com"]["applied"], "shield")
        self.assertEqual(by["avito.ru"]["applied"], "direct")
        self.assertEqual(by["lists/nz-battlenet.txt"]["applied"], "catalog")
        self.assertEqual(by["lists/nz-battlenet.txt"]["category"], "game")

    def test_direct_rule_overlapping_a_preset_section_is_a_recorded_conflict(self):
        res = run({"lists/nz-steam.txt"})
        self.assertEqual([c["value"] for c in res.conflicts], ["lists/nz-steam.txt"])
        self.assertIn("steam", res.conflicts[0]["detail"])

    def test_shield_sections_use_the_pass_recipe_and_are_scoped(self):
        res = run({"lists/nz-github.txt"})
        self.assertEqual(len(res.sections), 2)
        for s in res.sections:
            self.assertTrue(s.is_scoped())
            self.assertIn("--lua-desync=pass", s.args)
        self.assertTrue(any("--hostlist-domains=wardogs.example.com" in s.args for s in res.sections))
        self.assertTrue(any("--hostlist=lists/nz-seriesgraph.txt" in s.args for s in res.sections))

    def test_every_config_list_is_in_the_routes(self):
        res = run(set())
        names = {e["source_list"] for e in res.entries if "source_list" in e}
        self.assertTrue(set(LISTS) - {"dup-of-github.txt"} <= names)


if __name__ == "__main__":
    unittest.main()
