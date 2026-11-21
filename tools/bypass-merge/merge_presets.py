#!/usr/bin/env python3
"""Инструмент слияния пресетов RixyPow/netzapret в один пресет OCHISTKA (только данные, офлайн).

Использование:
  python3 tools/bypass-merge/merge_presets.py inventory --source <клон RixyPow/netzapret>
Ничего не запускает и не обращается к сети; пишет файлы в docs/ и assets/ репозитория.
"""
from __future__ import annotations

import argparse
import os
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
REPO = os.path.abspath(os.path.join(HERE, "..", ".."))

import hashlib  # noqa: E402
import json  # noqa: E402
import re  # noqa: E402
import shutil  # noqa: E402

import inventory  # noqa: E402
import merge  # noqa: E402
import zpreset  # noqa: E402

BASE = "Universal V13 Lite.txt"
# Приоритет при конфликтах: от более проверенных к более старым и особым.
PRIORITY = [
    "Universal V13 Lite", "Universal V13", "Universal V13 YT FIX", "Universal V12 Lite", "Universal V12",
    "Universal V11 Lite", "Universal V11", "Universal V10", "Universal V9", "Universal V8",
    "Default + YouTube no QUIC + wssize", "Default v1 (game filter)", "Default v2 (game filter)",
    "Default v3 (game filter)", "Default v4 (game filter)", "Default v5 (game filter)",
    "ALL TCP & UDP multisplit_stun", "white sni (circular)", "shadow probes (circular)",
]
# Список адресов длиннее этого порога почти не ограничивает трафик (например ipset-all: ~33 тыс. сетей).
BROAD_LIST_LINES = 5000
RUNTIME = os.path.join(REPO, "assets/desync/runtime")
EXTRA = os.path.join(REPO, "assets/desync/runtime-extra")
MERGED_PRESET = os.path.join(REPO, "assets/desync/presets/OCHISTKA Merged.txt")


def our_resources() -> set[str]:
    out = set()
    for root, _, files in os.walk(RUNTIME):
        for f in files:
            out.add(os.path.relpath(os.path.join(root, f), RUNTIME).replace("\\", "/"))
    return out


def source_file(source: str, rel: str) -> str | None:
    """Где в RixyPow лежит ресурс `rel`. Только данные: списки и образцы; Lua и фильтры не берём."""
    kind, _, name = rel.partition("/")
    if kind == "lists":
        base = name.removeprefix("nz-")
        cands = [f"config/lists/{base}"] if name.startswith("nz-") else [f"vendor/zapret-gui/lists/{name}"]
    elif kind == "bin":
        base = name.removeprefix("nz-")
        cands = [f"config/blobs/{base}"] if name.startswith("nz-") else [f"vendor/zapret-gui/bin/{name}"]
    else:
        return None
    for c in cands:
        if os.path.isfile(os.path.join(source, c)):
            return c
    return None


def defined_functions() -> set[str]:
    funcs: set[str] = set()
    lua = os.path.join(RUNTIME, "lua")
    for f in os.listdir(lua):
        with open(os.path.join(lua, f), encoding="utf-8", errors="ignore") as fh:
            funcs |= set(re.findall(r"function\s+(\w+)", fh.read()))
    return funcs



def source_id(source: str) -> str:
    sha = subprocess.check_output(["git", "-C", source, "rev-parse", "HEAD"], text=True).strip()
    return f"RixyPow/netzapret@{sha}"


def cmd_inventory(args: argparse.Namespace) -> None:
    md = inventory.render_markdown(inventory.load_presets(args.source), source=source_id(args.source))
    with open(os.path.join(REPO, "docs/status/bypass-merge-inventory.md"), "w", encoding="utf-8", newline="\n") as fh:
        fh.write(md)


def cmd_merge(args: argparse.Namespace) -> None:
    src = source_id(args.source)
    loaded = dict(inventory.load_presets(args.source))
    ordered = [(f"{n}.txt", loaded[f"{n}.txt"]) for n in PRIORITY if f"{n}.txt" in loaded]
    rest = [(n, p) for n, p in loaded.items() if n not in dict(ordered)]
    inputs = ordered + sorted(rest)
    ours = our_resources()
    available = set(ours)
    # Все ресурсы RixyPow, которые можно взять как данные.
    for _, p in inputs:
        for r in zpreset.referenced_resources(p.globals + [a for s in p.sections for a in s.args]):
            if r not in available and source_file(args.source, r):
                available.add(r)
    broad = set()
    for rel in available:
        sf = source_file(args.source, rel) if rel.startswith("lists/") else None
        path = os.path.join(RUNTIME, rel) if rel in ours else (os.path.join(args.source, sf) if sf else None)
        if path and os.path.isfile(path) and sum(1 for _ in open(path, encoding="utf-8", errors="ignore")) > BROAD_LIST_LINES:
            broad.add(rel)
    result = merge.merge(inputs, base=BASE, available=available, defined_funcs=defined_functions(), broad_lists=broad)
    description = [
        f"Объединённый пресет OCHISTKA: основа {BASE[:-4]} без изменений + секции из других пресетов {src}.",
        "Создан tools/bypass-merge/merge_presets.py; конфликты и исключения: docs/status/bypass-merge-conflicts.md.",
        "НЕ ПРОВЕРЕН на Windows у провайдера: обход не объявляется рабочим.",
    ]
    text = zpreset.render("OCHISTKA Merged", description, result.globals, result.sections)
    with open(MERGED_PRESET, "w", encoding="utf-8", newline="\n") as fh:
        fh.write(text)
    with open(os.path.join(REPO, "docs/status/bypass-merge-conflicts.md"), "w", encoding="utf-8", newline="\n") as fh:
        fh.write(merge.conflicts_markdown(result, src))
    cov = result.coverage()
    cov["source"] = src
    problems = merge.check_coverage(cov, result.sections, result.globals)
    if problems:
        sys.exit("Манифест не сходится: " + "; ".join(problems[:5]))
    with open(os.path.join(REPO, "docs/status/bypass-merge-coverage.json"), "w", encoding="utf-8", newline="\n") as fh:
        json.dump(cov, fh, ensure_ascii=False, indent=1, sort_keys=True)
        fh.write("\n")
    # Дополнительные данные (списки и образцы), которых нет среди 135 закреплённых файлов.
    shutil.rmtree(EXTRA, ignore_errors=True)
    files = []
    needed = set(zpreset.referenced_resources(result.globals + [a for s in result.sections for a in s.args]))
    for rel in sorted(needed - ours):
        sf = source_file(args.source, rel)
        assert sf, rel
        data = open(os.path.join(args.source, sf), "rb").read()
        dest = os.path.join(EXTRA, rel)
        os.makedirs(os.path.dirname(dest), exist_ok=True)
        with open(dest, "wb") as fh:
            fh.write(data)
        files.append({"destination": rel, "source_path": sf, "sha256": hashlib.sha256(data).hexdigest(), "size_bytes": len(data)})
    manifest = {"schema_version": 1, "source": src, "license": "MIT (NetZapret © 2026 RixyPow; Zapret GUI © censorliber/loop-uh)",
                "note": "Только данные (списки адресов и образцы). 135 закреплённых файлов не менялись.",
                "files": files}
    with open(os.path.join(REPO, "docs/status/bypass-merge-resources.json"), "w", encoding="utf-8", newline="\n") as fh:
        json.dump(manifest, fh, ensure_ascii=False, indent=1)
        fh.write("\n")
    counts = {}
    for r in result.section_records:
        counts[r.status] = counts.get(r.status, 0) + 1
    print("sections:", counts, "merged:", len(result.sections), "extra files:", len(files))


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    p = sub.add_parser("inventory")
    p.add_argument("--source", required=True)
    p.set_defaults(func=cmd_inventory)
    m = sub.add_parser("merge")
    m.add_argument("--source", required=True)
    m.set_defaults(func=cmd_merge)
    args = parser.parse_args()
    args.func(args)


if __name__ == "__main__":
    main()
