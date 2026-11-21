import os
import tempfile
import unittest

import slim
import zpreset


def secs(text):
    return zpreset.parse("# Preset: T\n--x=1\n" + text).sections


def loader(files):
    def load(rel):
        return files.get(rel)
    return load


FILES = {
    "lists/a.txt": ["example.com", "other.org"],
    "lists/b.txt": ["www.example.com", "sub.other.org"],
    "lists/c.txt": ["new.net"],
    "lists/empty.txt": [],
    "lists/n1.txt": ["10.0.0.0/8"],
    "lists/n2.txt": ["10.1.0.0/16"],
    "lists/n3.txt": ["11.0.0.0/8"],
}


def dead(text):
    return slim.find_dead(secs(text), loader(FILES))


def sec(name, *lines):
    return "\n--new\n--name=" + name + "\n" + "\n".join(lines) + "\n"


class SlimTests(unittest.TestCase):
    def test_subdomain_list_after_its_zone_list_is_dead_even_with_another_action(self):
        d = dead(sec("A", "--filter-tcp=443", "--hostlist=lists/a.txt", "--lua-desync=fake")
                 + sec("B", "--filter-tcp=443", "--hostlist=lists/b.txt", "--lua-desync=multisplit"))
        self.assertEqual([(x.index, x.covered_by) for x in d], [(1, 0)])

    def test_different_traffic_is_never_dead(self):
        base = sec("A", "--filter-tcp=443", "--hostlist=lists/a.txt", "--lua-desync=fake")
        for other in [
            sec("B", "--filter-tcp=443", "--hostlist=lists/c.txt", "--lua-desync=fake"),            # другие имена
            sec("B", "--filter-tcp=80,443", "--hostlist=lists/b.txt", "--lua-desync=fake"),         # шире по портам
            sec("B", "--filter-udp=443", "--hostlist=lists/b.txt", "--lua-desync=fake"),            # другой протокол
            sec("B", "--filter-tcp=443", "--filter-l7=tls", "--hostlist=lists/b.txt"),              # другой l7
            sec("B", "--filter-tcp=443", "--out-range=-d8", "--hostlist=lists/b.txt"),              # другой out-range
            sec("B", "--filter-tcp=443", "--hostlist=lists/b.txt", "--ipset=lists/n1.txt"),         # два вида охвата
        ]:
            self.assertEqual(dead(base + other), [], other)

    def test_order_matters_only_an_earlier_section_can_shadow(self):
        a = sec("A", "--filter-tcp=443", "--hostlist=lists/a.txt")
        b = sec("B", "--filter-tcp=443", "--hostlist=lists/b.txt")
        self.assertEqual(dead(b + a), [])

    def test_port_and_network_containment(self):
        d = dead(sec("A", "--filter-tcp=80,443-65535", "--ipset=lists/n1.txt")
                 + sec("B", "--filter-tcp=443", "--ipset=lists/n2.txt")
                 + sec("C", "--filter-tcp=443", "--ipset=lists/n3.txt"))
        self.assertEqual([x.index for x in d], [1])

    def test_inline_domains_and_empty_scope(self):
        d = dead(sec("A", "--filter-tcp=443", "--hostlist-domains=example.com")
                 + sec("B", "--filter-tcp=443", "--hostlist-domains=a.example.com")
                 + sec("E", "--filter-tcp=443", "--hostlist=lists/empty.txt"))
        self.assertEqual({(x.index, x.reason) for x in d}, {(1, "covered"), (2, "empty")})

    def test_excludes_must_be_identical_for_a_cover(self):
        a = sec("A", "--filter-tcp=443", "--hostlist=lists/a.txt", "--hostlist-exclude=lists/c.txt")
        b = sec("B", "--filter-tcp=443", "--hostlist=lists/b.txt")
        self.assertEqual(dead(a + b), [])

    def test_unknown_list_is_never_removed(self):
        d = dead(sec("A", "--filter-tcp=443", "--hostlist=lists/a.txt")
                 + sec("B", "--filter-tcp=443", "--hostlist=lists/missing.txt"))
        self.assertEqual(d, [])

    def test_slim_removes_only_dead_sections_and_is_idempotent(self):
        text = (sec("A", "--filter-tcp=443", "--hostlist=lists/a.txt")
                + sec("B", "--filter-tcp=443", "--hostlist=lists/b.txt")
                + sec("C", "--filter-tcp=443", "--hostlist=lists/c.txt"))
        kept, removed = slim.slim(secs(text), loader(FILES))
        self.assertEqual([s.name for s in kept], ["A", "C"])
        self.assertEqual(len(removed), 1)
        again, removed2 = slim.slim(kept, loader(FILES))
        self.assertEqual(removed2, [])
        self.assertEqual([s.name for s in again], ["A", "C"])

    def test_equivalence_check_proves_every_removed_section_is_shadowed(self):
        before = secs(sec("A", "--filter-tcp=443", "--hostlist=lists/a.txt")
                      + sec("B", "--filter-tcp=443", "--hostlist=lists/b.txt"))
        kept, removed = slim.slim(before, loader(FILES))
        self.assertEqual(slim.check_equivalence(before, kept, removed, loader(FILES)), [])
        # неверное удаление обнаруживается
        wrong = secs(sec("A", "--filter-tcp=443", "--hostlist=lists/c.txt")
                     + sec("B", "--filter-tcp=443", "--hostlist=lists/b.txt"))
        self.assertTrue(slim.check_equivalence(before, wrong[:1], [slim.Dead(1, 0, "covered")], loader(FILES)))


if __name__ == "__main__":
    unittest.main()
