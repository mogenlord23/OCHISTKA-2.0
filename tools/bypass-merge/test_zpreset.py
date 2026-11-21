import unittest

import zpreset

SAMPLE = """# Preset: Demo
# BuiltinVersion: 1.0
--lua-init=@lua/a.lua
--blob=tls1:@bin/t1.bin
--wf-tcp-out=80,443

--new
--name=Discord TCP
--filter-tcp=443
--hostlist=lists/discord.txt
--out-range=-d8
--payload=tls_client_hello
--lua-desync=fake:blob=tls1

--new
--name=Games UDP
--filter-udp=443
--ipset=lists/ipset-x.txt
--lua-desync=fake:blob=quic1
"""


class ParseTests(unittest.TestCase):
    def test_header_globals_and_sections(self):
        p = zpreset.parse(SAMPLE)
        self.assertEqual(p.name, "Demo")
        self.assertEqual(p.globals, ["--lua-init=@lua/a.lua", "--blob=tls1:@bin/t1.bin", "--wf-tcp-out=80,443"])
        self.assertEqual([s.name for s in p.sections], ["Discord TCP", "Games UDP"])
        self.assertEqual(p.sections[0].args[1], "--filter-tcp=443")

    def test_several_arguments_on_one_line_are_split(self):
        p = zpreset.parse("# Preset: X\n--a=1 --b=2\n--new\n--name=S --filter-tcp=80\n")
        self.assertEqual(p.globals, ["--a=1", "--b=2"])
        self.assertEqual(p.sections[0].args, ["--name=S", "--filter-tcp=80"])

    def test_selector_and_action_split_and_identity_ignores_name(self):
        a = zpreset.parse(SAMPLE).sections[0]
        self.assertEqual(
            a.selector(), ("--filter-tcp=443", "--hostlist=lists/discord.txt", "--out-range=-d8")
        )
        self.assertEqual(a.action(), ("--payload=tls_client_hello", "--lua-desync=fake:blob=tls1"))
        b = zpreset.parse(SAMPLE.replace("Discord TCP", "Other name")).sections[0]
        self.assertEqual(a.identity(), b.identity())

    def test_scope_kinds(self):
        s = zpreset.parse(SAMPLE).sections
        self.assertTrue(s[0].is_scoped() and s[1].is_scoped())
        broad = zpreset.parse("# Preset: B\n--x=1\n--new\n--name=All\n--filter-tcp=443\n--lua-desync=fake\n")
        self.assertFalse(broad.sections[0].is_scoped())

    def test_ports_cover(self):
        self.assertTrue(zpreset.ports_cover("443", "80,443-65535"))
        self.assertTrue(zpreset.ports_cover("19300-19310", "443,19294-19344"))
        self.assertFalse(zpreset.ports_cover("27000-27100", "443,19294-19344"))
        self.assertFalse(zpreset.ports_cover("443-65535", "443,6568"))

    def test_referenced_resources(self):
        s = zpreset.parse(SAMPLE)
        res = zpreset.referenced_resources(s.globals + [a for sec in s.sections for a in sec.args])
        self.assertEqual(res, {"lua/a.lua", "bin/t1.bin", "lists/discord.txt", "lists/ipset-x.txt"})

    def test_render_roundtrip_keeps_every_argument(self):
        p = zpreset.parse(SAMPLE)
        again = zpreset.parse(zpreset.render(p.name, ["demo"], p.globals, p.sections))
        self.assertEqual(again.globals, p.globals)
        self.assertEqual([s.args for s in again.sections], [s.args for s in p.sections])


if __name__ == "__main__":
    unittest.main()
