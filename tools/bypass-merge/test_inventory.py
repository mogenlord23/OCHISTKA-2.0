import unittest

import inventory
import zpreset
from test_zpreset import SAMPLE


class InventoryTests(unittest.TestCase):
    def test_section_row_lists_ports_services_and_functions(self):
        sec = zpreset.parse(SAMPLE).sections[0]
        row = inventory.section_row(sec)
        self.assertEqual(row["tcp"], "443")
        self.assertEqual(row["udp"], "")
        self.assertEqual(row["services"], ["discord"])
        self.assertEqual(row["funcs"], ["fake"])

    def test_domains_are_counted_and_listed(self):
        p = zpreset.parse("# Preset: D\n--x=1\n--new\n--name=V\n--filter-tcp=80,443\n--hostlist-domains=a.com,b.com\n--lua-desync=hostfakesplit_multi:x=1\n")
        row = inventory.section_row(p.sections[0])
        self.assertEqual(row["domains"], ["a.com", "b.com"])
        self.assertEqual(row["funcs"], ["hostfakesplit_multi"])

    def test_markdown_has_a_table_row_per_section_and_global_summary(self):
        p = zpreset.parse(SAMPLE)
        md = inventory.render_markdown([("demo.txt", p)], source="RixyPow/netzapret@abc")
        self.assertIn("demo.txt", md)
        self.assertIn("Discord TCP", md)
        self.assertIn("Games UDP", md)
        self.assertIn("RixyPow/netzapret@abc", md)
        self.assertIn("--wf-tcp-out=80,443", md)


if __name__ == "__main__":
    unittest.main()
