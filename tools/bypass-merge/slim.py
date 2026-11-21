"""Разгрузка пресета: убираются только секции, которые НИКОГДА не выбираются движком.

Профили (секции) сканируются строго по порядку, выбирается первый совпавший (zapret2 manual.md).
Поэтому секция B недостижима, если ранее стоящая секция A совпадает с любым трафиком B:
одинаковые протоколы, `filter-l7`, `out-range`, `in-range`, `skip` и исключения, порты A включают
порты B, а адреса или имена A включают адреса или имена B (имя `example.com` в списке — это зона,
она покрывает и поддомены). Действия секций при этом не сравниваются: недостижимая секция
ничего не делает. Всё, что не доказано, остаётся. Также недостижима секция с пустым охватом.
"""
from __future__ import annotations

import ipaddress
from dataclasses import dataclass
from typing import Callable

import zpreset
from zpreset import Section, key_of, value_of

OTHER_KEYS = ("filter-l7", "out-range", "in-range", "skip", "hostlist-exclude", "ipset-exclude", "ipset-exclude-ip")
Loader = Callable[[str], "list[str] | None"]


@dataclass
class Dead:
    index: int
    covered_by: "int | None"  # None для пустого охвата
    reason: str  # covered | empty


@dataclass
class _Sig:
    kind: str  # "h" | "i" | ""
    hosts: set
    nets: list
    ports: dict
    other: tuple
    known: bool


def _signature(sec: Section, load: Loader) -> _Sig:
    hosts: set[str] = set()
    nets: list = []
    kinds: set[str] = set()
    ports: dict[str, str] = {}
    other: list[str] = []
    known = True
    for a in sec.body():
        k, v = key_of(a), value_of(a)
        if k == "hostlist":
            items = load(v)
            if items is None:
                known = False
            else:
                hosts |= {x.lower() for x in items}
            kinds.add("h")
        elif k == "hostlist-domains":
            hosts |= {d.lower() for d in v.split(",") if d}
            kinds.add("h")
        elif k == "ipset":
            items = load(v)
            if items is None:
                known = False
            else:
                try:
                    nets += [ipaddress.ip_network(x, strict=False) for x in items if x]
                except ValueError:
                    known = False
            kinds.add("i")
        elif k == "ipset-ip":
            try:
                nets += [ipaddress.ip_network(x, strict=False) for x in v.split(",") if x]
            except ValueError:
                known = False
            kinds.add("i")
        elif k in ("filter-tcp", "filter-udp"):
            ports[k] = v
        elif k in OTHER_KEYS:
            other.append(a)
    kind = next(iter(kinds)) if len(kinds) == 1 else ("?" if kinds else "")
    return _Sig(kind, hosts, nets, ports, tuple(sorted(other)), known)


def _host_covered(a: set, b: set) -> bool:
    for d in b:
        parts = d.split(".")
        if not any(".".join(parts[i:]) in a for i in range(len(parts))):
            return False
    return True


def _net_covered(a: list, b: list) -> bool:
    return all(any(n.version == m.version and n.subnet_of(m) for m in a) for n in b)


def _covers(a: _Sig, b: _Sig) -> bool:
    if not (a.known and b.known) or a.kind != b.kind or a.kind not in ("h", "i"):
        return False
    if a.other != b.other or set(a.ports) != set(b.ports):
        return False
    if not all(zpreset.ports_cover(b.ports[k], a.ports[k]) for k in b.ports):
        return False
    return _host_covered(a.hosts, b.hosts) if a.kind == "h" else _net_covered(a.nets, b.nets)


def find_dead(sections: list[Section], load: Loader) -> list[Dead]:
    sigs = [_signature(s, load) for s in sections]
    dead: list[Dead] = []
    gone: set[int] = set()
    for j, b in enumerate(sigs):
        if b.known and b.kind in ("h", "i") and not b.hosts and not b.nets:
            dead.append(Dead(j, None, "empty"))
            gone.add(j)
            continue
        for i in range(j):
            if i in gone:
                continue
            if _covers(sigs[i], b):
                dead.append(Dead(j, i, "covered"))
                gone.add(j)
                break
    return dead


def slim(sections: list[Section], load: Loader) -> tuple[list[Section], list[Dead]]:
    dead = find_dead(sections, load)
    drop = {d.index for d in dead}
    return [s for i, s in enumerate(sections) if i not in drop], dead


def check_equivalence(before: list[Section], after: list[Section], removed: list[Dead], load: Loader) -> list[str]:
    """Независимая проверка: каждая убранная секция перекрыта более ранней, оставшейся в `after`."""
    problems: list[str] = []
    kept_ids = {s.identity() for s in after}
    sigs = [_signature(s, load) for s in before]
    for d in removed:
        b = sigs[d.index]
        if d.reason == "empty":
            if b.hosts or b.nets:
                problems.append(f"секция {d.index + 1}: охват не пуст")
            continue
        a_index = d.covered_by
        if a_index is None or a_index >= d.index:
            problems.append(f"секция {d.index + 1}: перекрывающая секция не раньше")
            continue
        if before[a_index].identity() not in kept_ids and not any(x.index == a_index for x in removed):
            problems.append(f"секция {d.index + 1}: перекрывающая секция потеряна")
        if not _covers(sigs[a_index], b):
            problems.append(f"секция {d.index + 1}: не перекрыта секцией {a_index + 1}")
    return problems
