"""Чтение пресетов Zapret/NetZapret (только данные, без запуска чего-либо).

Формат: до первого `--new` идут общие аргументы, затем секции (профили) через `--new`.
Код RixyPow/netzapret не используется: инструмент только читает файлы настроек.
"""
from __future__ import annotations

import hashlib
import re
from dataclasses import dataclass, field

# Аргументы, которые выбирают трафик секции (селектор). Остальное — действие.
SELECTOR_KEYS = (
    "filter-tcp", "filter-udp", "filter-l7", "hostlist", "hostlist-domains", "hostlist-exclude",
    "ipset", "ipset-ip", "ipset-exclude", "ipset-exclude-ip", "out-range", "in-range", "skip",
)
# Аргументы, которые ограничивают секцию конкретными адресами или именами.
SCOPE_KEYS = ("hostlist", "hostlist-domains", "ipset", "ipset-ip")


def key_of(argument: str) -> str:
    return argument[2:].split("=", 1)[0]


def value_of(argument: str) -> str:
    return argument.split("=", 1)[1] if "=" in argument else ""


@dataclass
class Section:
    args: list[str]

    @property
    def name(self) -> str:
        for a in self.args:
            if key_of(a) == "name":
                return value_of(a)
        return ""

    def body(self) -> tuple[str, ...]:
        return tuple(a for a in self.args if key_of(a) != "name")

    def selector(self) -> tuple[str, ...]:
        return tuple(a for a in self.body() if key_of(a) in SELECTOR_KEYS)

    def action(self) -> tuple[str, ...]:
        return tuple(a for a in self.body() if key_of(a) not in SELECTOR_KEYS)

    def identity(self) -> str:
        """Одинаковая секция = одинаковые аргументы в том же порядке; имя не учитывается."""
        return hashlib.sha256("\n".join(self.body()).encode("utf-8")).hexdigest()

    def is_scoped(self) -> bool:
        return any(key_of(a) in SCOPE_KEYS for a in self.body())

    def values(self, key: str) -> list[str]:
        return [value_of(a) for a in self.args if key_of(a) == key]


@dataclass
class Preset:
    name: str
    builtin: str | None
    globals: list[str]
    sections: list[Section] = field(default_factory=list)


def parse(text: str) -> Preset:
    name = None
    builtin = None
    glob: list[str] = []
    sections: list[Section] = []
    current = glob
    for raw in text.splitlines():
        line = raw.strip()
        if not line:
            continue
        if line.startswith("#"):
            comment = line[1:].strip()
            if comment.startswith("Preset:") and name is None:
                name = comment.split(":", 1)[1].strip()
            elif comment.startswith("BuiltinVersion:") and builtin is None:
                builtin = comment.split(":", 1)[1].strip()
            continue
        for argument in re.split(r"\s+(?=--[A-Za-z])", line):
            if argument == "--new":
                section = Section([])
                sections.append(section)
                current = section.args
            else:
                current.append(argument)
    return Preset(name or "", builtin, glob, sections)


def _expand(spec: str) -> list[tuple[int, int]]:
    out = []
    for part in spec.split(","):
        part = part.strip()
        if not part:
            continue
        if "-" in part:
            a, b = part.split("-", 1)
            out.append((int(a) if a else 0, int(b)))
        else:
            out.append((int(part), int(part)))
    return out


def ports_cover(wanted: str, captured: str) -> bool:
    """Все порты `wanted` лежат внутри `captured` (оба — списки вида `80,443-65535`)."""
    cov = _expand(captured)
    return all(any(a >= c0 and b <= c1 for c0, c1 in cov) for a, b in _expand(wanted))


_RES = re.compile(r"(?:@|=)((?:lua|lists|bin|windivert\.filter)/[A-Za-z0-9_.\-]+)")


def referenced_resources(arguments: list[str]) -> set[str]:
    found: set[str] = set()
    for a in arguments:
        for m in _RES.finditer(a):
            found.add(m.group(1))
        # blob=name:@bin/x.bin
        for m in re.finditer(r":@((?:lua|lists|bin|windivert\.filter)/[A-Za-z0-9_.\-]+)", a):
            found.add(m.group(1))
    return found


def render(title: str, description: list[str], glob: list[str], sections: list[Section]) -> str:
    lines = [f"# Preset: {title}"]
    lines += [f"# {d}" for d in description]
    lines.append("")
    lines += glob
    for s in sections:
        lines.append("")
        lines.append("--new")
        lines += s.args
    return "\n".join(lines) + "\n"
