import unittest

import merge
import zpreset

BASE = """# Preset: Base
--lua-init=@lua/a.lua
--blob=tls1:@bin/t1.bin
--ctrack-disable=0
--wf-tcp-out=80,443-65535
--wf-udp-out=443,19294-19344

--new
--name=Discord
--filter-tcp=443
--hostlist=lists/discord.txt
--payload=tls_client_hello
--lua-desync=fake:blob=tls1

--new
--name=Games UDP
--filter-udp=19300-19310
--ipset=lists/ipset-g.txt
--lua-desync=fake:blob=tls1
"""

OTHER = """# Preset: Other
--lua-init=@lua/a.lua
--blob=tls1:@bin/t1.bin
--blob=tls2:@bin/t2.bin
--ctrack-disable=1
--wf-tcp-out=80,443-65535
--wf-udp-out=443-65535

--new
--name=Discord (same, other name)
--filter-tcp=443
--hostlist=lists/discord.txt
--payload=tls_client_hello
--lua-desync=fake:blob=tls1

--new
--name=Discord other recipe
--filter-tcp=443
--hostlist=lists/discord.txt
--payload=tls_client_hello
--lua-desync=multisplit:pos=1

--new
--name=New scoped
--filter-tcp=443
--hostlist=lists/new.txt
--lua-desync=fake:blob=tls2

--new
--name=Broad
--filter-tcp=443
--lua-desync=fake:blob=tls1

--new
--name=Wide UDP
--filter-udp=27000-27100
--ipset=lists/ipset-g.txt
--lua-desync=fake:blob=tls1

--new
--name=Needs missing module
--filter-tcp=443
--hostlist=lists/new2.txt
--lua-desync=nz_first2:x=1

--new
--name=Missing list
--filter-tcp=443
--hostlist=lists/absent.txt
--lua-desync=fake:blob=tls1
"""

AVAILABLE = {"lua/a.lua", "bin/t1.bin", "bin/t2.bin", "lists/discord.txt", "lists/ipset-g.txt", "lists/new.txt", "lists/new2.txt"}
FUNCS = {"fake", "multisplit"}


def run():
    inputs = [("base.txt", zpreset.parse(BASE)), ("other.txt", zpreset.parse(OTHER))]
    return merge.merge(inputs, base="base.txt", available=AVAILABLE, defined_funcs=FUNCS)


class MergeTests(unittest.TestCase):
    def test_base_sections_come_first_and_unchanged(self):
        r = run()
        base = zpreset.parse(BASE)
        self.assertEqual([s.args for s in r.sections[:2]], [s.args for s in base.sections])

    def test_no_duplicate_sections_and_no_duplicate_global_arguments(self):
        r = run()
        ids = [s.identity() for s in r.sections]
        self.assertEqual(len(ids), len(set(ids)))
        self.assertEqual(len(r.globals), len(set(r.globals)))

    def test_every_input_section_has_a_recorded_outcome(self):
        r = run()
        inputs = {("base.txt", i) for i in range(2)} | {("other.txt", i) for i in range(7)}
        self.assertEqual({(x.preset, x.index) for x in r.section_records}, inputs)
        status = {(x.preset, x.index): x.status for x in r.section_records}
        self.assertEqual(status[("other.txt", 0)], "duplicate")  # то же правило, другое имя
        self.assertEqual(status[("other.txt", 1)], "conflict")  # тот же трафик, другое действие
        self.assertEqual(status[("other.txt", 2)], "added")
        self.assertEqual(status[("other.txt", 3)], "excluded")  # без списка: слишком широкий
        self.assertEqual(status[("other.txt", 4)], "excluded")  # UDP вне базового перехвата
        self.assertEqual(status[("other.txt", 5)], "excluded")  # нет модуля
        self.assertEqual(status[("other.txt", 6)], "excluded")  # нет списка

    def test_a_huge_address_list_counts_as_broad_not_scoped(self):
        other = OTHER + "\n--new\n--name=All nets\n--filter-tcp=443\n--ipset=lists/ipset-g.txt\n--lua-desync=fake:blob=tls2\n"
        inputs = [("base.txt", zpreset.parse(BASE)), ("other.txt", zpreset.parse(other))]
        r = merge.merge(inputs, base="base.txt", available=AVAILABLE, defined_funcs=FUNCS,
                        broad_lists={"lists/ipset-g.txt"})
        rec = next(x for x in r.section_records if x.name == "All nets")
        self.assertEqual(rec.status, "excluded")
        self.assertIn("весь трафик", rec.detail)

    def test_conflict_keeps_base_and_says_so(self):
        r = run()
        rec = next(x for x in r.section_records if (x.preset, x.index) == ("other.txt", 1))
        self.assertIn("Discord", rec.detail)
        self.assertTrue(rec.item_id.startswith("X"))
        self.assertNotIn("multisplit:pos=1", "\n".join(a for s in r.sections for a in s.args))

    def test_added_section_pulls_its_blob_declaration(self):
        r = run()
        self.assertIn("--blob=tls2:@bin/t2.bin", r.globals)
        self.assertIn("bin/t2.bin", r.resources)

    def test_capture_scope_and_single_value_options_stay_as_in_base(self):
        r = run()
        self.assertIn("--wf-udp-out=443,19294-19344", r.globals)
        self.assertNotIn("--wf-udp-out=443-65535", r.globals)
        self.assertIn("--ctrack-disable=0", r.globals)
        self.assertNotIn("--ctrack-disable=1", r.globals)
        kinds = {x.arg: x.status for x in r.global_records if x.preset == "other.txt"}
        self.assertEqual(kinds["--wf-udp-out=443-65535"], "conflict")
        self.assertEqual(kinds["--ctrack-disable=1"], "conflict")
        self.assertEqual(kinds["--lua-init=@lua/a.lua"], "duplicate")

    def test_conflicts_markdown_lists_every_non_merged_item(self):
        r = run()
        md = merge.conflicts_markdown(r, source="src@1")
        for x in r.section_records + r.global_records:
            if x.status in ("conflict", "excluded"):
                self.assertIn(x.item_id, md)
        self.assertIn("наименьшим риском", md)

    def test_coverage_manifest_is_consistent_with_the_merged_preset(self):
        r = run()
        problems = merge.check_coverage(r.coverage(), r.sections, r.globals)
        self.assertEqual(problems, [])
        # поломанный манифест обнаруживается
        bad = r.coverage()
        bad["sections"][2]["status"] = "added"
        bad["sections"][2]["identity"] = "0" * 64
        self.assertTrue(merge.check_coverage(bad, r.sections, r.globals))


if __name__ == "__main__":
    unittest.main()
