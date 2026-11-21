"""Инвентаризация пресетов: файл -> секции -> порты, сервисы, домены, функции."""
from __future__ import annotations

import os
import re

import zpreset

_LIST = re.compile(r"lists/(?:nz-)?(?:ipset-)?([A-Za-z0-9_.\-]+?)(?:_v6)?\.txt$")


def _service(path: str) -> str:
    m = _LIST.search(path)
    return m.group(1) if m else path


def section_row(sec: zpreset.Section) -> dict:
    services: list[str] = []
    for key in ("hostlist", "ipset", "hostlist-exclude", "ipset-exclude"):
        for v in sec.values(key):
            if key.endswith("exclude"):
                continue
            s = _service(v)
            if s not in services:
                services.append(s)
    domains: list[str] = []
    for v in sec.values("hostlist-domains"):
        domains += [d for d in v.split(",") if d]
    funcs: list[str] = []
    for v in sec.values("lua-desync"):
        f = v.split(":", 1)[0]
        if f not in funcs:
            funcs.append(f)
    return {
        "name": sec.name,
        "tcp": ",".join(sec.values("filter-tcp")),
        "udp": ",".join(sec.values("filter-udp")),
        "l7": ",".join(sec.values("filter-l7")),
        "services": services,
        "domains": domains,
        "funcs": funcs,
        "scoped": sec.is_scoped(),
    }


def _cell(text: str) -> str:
    return text.replace("|", "\\|").replace("\n", " ")


def render_markdown(presets: list[tuple[str, zpreset.Preset]], source: str) -> str:
    out = [
        "# Инвентаризация пресетов десинка (RixyPow/netzapret)",
        "",
        f"Источник: {source}. Лицензия данных: MIT (NetZapret © 2026 RixyPow; Zapret GUI © censorliber/loop-uh). "
        "Код не копируется, разобраны только файлы настроек. Файл создан инструментом "
        "`tools/bypass-merge/merge_presets.py`, вручную не правится.",
        "",
        "## Сводка",
        "",
        "| Файл | Общих аргументов | Секций |",
        "|---|---:|---:|",
    ]
    for fname, p in presets:
        out.append(f"| {_cell(fname)} | {len(p.globals)} | {len(p.sections)} |")
    for fname, p in presets:
        out += ["", f"## {fname}", ""]
        out.append("Общие аргументы (порты перехвата, образцы, модули):")
        out.append("")
        for g in p.globals:
            out.append(f"- `{_cell(g)}`")
        out += [
            "",
            "| № | Секция | TCP | UDP | L7 | Сервисы (списки) | Домены | Функции |",
            "|---:|---|---|---|---|---|---|---|",
        ]
        for i, s in enumerate(p.sections, 1):
            r = section_row(s)
            domains = ", ".join(r["domains"][:6]) + (" …" if len(r["domains"]) > 6 else "")
            out.append(
                f"| {i} | {_cell(r['name'])} | {_cell(r['tcp'])} | {_cell(r['udp'])} | {_cell(r['l7'])} | "
                f"{_cell(', '.join(r['services']) or ('(без списка)' if not r['domains'] else ''))} | "
                f"{_cell(domains)} | {_cell(', '.join(r['funcs']))} |"
            )
    return "\n".join(out) + "\n"


def load_presets(source_dir: str) -> list[tuple[str, zpreset.Preset]]:
    folder = os.path.join(source_dir, "presets")
    result = []
    for fname in sorted(os.listdir(folder)):
        if fname.endswith(".txt"):
            with open(os.path.join(folder, fname), encoding="utf-8") as fh:
                result.append((fname, zpreset.parse(fh.read())))
    return result
