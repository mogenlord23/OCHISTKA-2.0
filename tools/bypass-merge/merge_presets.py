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
import routes  # noqa: E402
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
ROUTES_FILE = os.path.join(REPO, "assets/desync/routes/OCHISTKA Routes.json")
# Решения владельца: убрать из обхода целиком (слово в имени списка, домена или секции -> причина).
OWNER_EXCLUDED = {
    "telegram": "Telegram: прокси в программе нет, записи удалены (решение владельца)",
    "steam": "Steam: исключён по решению владельца",
    "riot": "Riot / Valorant: исключён по решению владельца",
    "valorant": "Riot / Valorant: исключён по решению владельца",
}


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


RUST_ASSETS = os.path.join(REPO, "crates/platform/src/desync/merged_assets.rs")


def render_rust_assets(files: list[dict]) -> str:
    """Встроенные дополнительные данные для объединённого пресета (с SHA-256 каждого файла)."""
    out = [
        "//! Дополнительные данные объединённого пресета «Обход блокировок»: списки адресов и доменов.",
        "//! Создано tools/bypass-merge/merge_presets.py из docs/status/bypass-merge-resources.json; вручную не править.",
        "//! 135 закреплённых файлов Universal V13 Lite лежат в `preset_assets.rs` и здесь не повторяются.",
        "",
        "pub(crate) static MERGED_EXTRA_RESOURCES: &[(&str, &[u8], &str)] = &[",
    ]
    for f in files:
        out += [
            "    (",
            f"        \"{f['destination']}\",",
            f"        include_bytes!(\"../../../../assets/desync/runtime-extra/{f['destination']}\"),",
            f"        \"{f['sha256']}\",",
            "    ),",
        ]
    out.append("];")
    return "\n".join(out) + "\n"


def removal_markdown(result: merge.Result, rt: routes.Result, src: str) -> str:
    out = [
        "# Удаление Telegram, Steam и Riot / Valorant из «Обхода блокировок»",
        "",
        f"Источник данных: {src}. Решение владельца; создано `tools/bypass-merge/merge_presets.py`, вручную не правится.",
        "",
        "Правило: секция убирается, только если весь её охват (списки адресов и домены) — это удаляемый сервис. "
        "Если секция нужна ещё и другим сервисам, она остаётся и записывается ниже.",
        "",
    ]
    groups = {"Telegram": "telegram", "Steam": "steam", "Riot / Valorant": "riot|valorant"}
    for title, pat in groups.items():
        rx = re.compile(pat)
        secs = [x for x in result.section_records if x.status == "removed" and rx.search(x.detail.lower())]
        out += [f"## {title}", "", f"Удалено секций пресетов: {len(secs)}.", ""]
        base = [x for x in secs if x.preset == result.base]
        if base:
            out.append("Из основы (Universal V13 Lite, в самом исходном файле остаётся без изменений): "
                       + ", ".join(f"№{x.index + 1} «{x.name}»" for x in base) + ".")
        others = [x for x in secs if x.preset != result.base]
        if others:
            out.append(f"Из других пресетов: {len(others)} секций (подробно в `bypass-merge-coverage.json`, статус `removed`).")
        gl = [x for x in result.global_records if x.status == "removed" and rx.search(x.arg.lower())]
        if gl:
            out.append(f"Образцы, объявленные, но не используемые ни одной оставшейся секцией: {len(gl)} (например `{gl[0].arg}`).")
        rl = [x for x in rt.removed if rx.search(x["source"].lower())]
        if rl:
            out.append("Маршруты и списки: " + ", ".join(f"`{x['source']}`" for x in rl) + ".")
        out.append("")
    out += [
        "## Общее использование (проверка перед удалением)",
        "",
        (f"Оставлено секций, затрагивающих удаляемые сервисы вместе с другими: {len(result.shared)}."
         if result.shared else
         "Секций, которые одновременно нужны другим сервисам, не найдено: удалены только секции, "
         "целиком относящиеся к удаляемым сервисам."),
    ]
    for x in result.shared:
        out.append(f"- {x.preset} №{x.index + 1} «{x.name}»: {x.detail}")
    out += [
        "",
        "## Что НЕ менялось",
        "",
        "- 135 закреплённых файлов NetZapret (в том числе `lists/ipset-telegram.txt`, `lists/steam.txt`, `lists/riot-valorant.txt`, "
        "`lists/russia-blacklist.txt`) и их SHA-256: файлы остаются встроенными, но объединённый пресет на них больше не ссылается.",
        "- `assets/desync/presets/Universal V13 Lite.txt` (SHA-256 `e679901475d9…`).",
        "- Маршруты Claude и других сервисов «только через VPN» (`vpn-only`) оставлены как есть; отдельного обхода Claude нет.",
    ]
    return "\n".join(out) + "\n"


STATUS_TEXT = {
    "preset": "в пресете обхода",
    "shield": "щит pass (обход не вмешивается)",
    "direct": "напрямую (обход не вмешивается)",
    "vpn-only": "только через VPN (этим переключателем не применяется)",
    "catalog": "справочно: действие не задано",
}


def routes_status_markdown(rt: routes.Result, sections: list, src: str) -> str:
    out = [
        "# Статус маршрутов «Обхода блокировок»",
        "",
        f"Источник: {src}. Создано `tools/bypass-merge/merge_presets.py`. **Не проверено на Windows у провайдера:** "
        "ни один сервис не объявляется работающим.",
        "",
        "## Сервис → статус (решения владельца)",
        "",
        "| Сервис | Статус |",
        "|---|---|",
        "| Steam | исключён по решению владельца (секции, списки и маршруты удалены) |",
        "| Riot / Valorant | исключён по решению владельца (секции, списки и маршруты удалены) |",
        "| Telegram | исключён: прокси в программе нет, записи удалены |",
        "| Claude | только через VPN (`vpn-only`), отдельного обхода нет |",
        "",
        f"## Все маршруты ({len(rt.entries)})",
        "",
        "| Сервис / адрес | Вид | Категория | Режим RixyPow | Статус |",
        "|---|---|---|---|---|",
    ]
    for e in rt.entries:
        name = (e.get("source_list") or e.get("value") or e.get("file", "")).removesuffix(".txt")
        out.append(f"| {name} | {e['kind']} | {e['category']} | {e['mode']} | {STATUS_TEXT[e['applied']]} |")
    out += ["", f"## Секции итогового пресета ({len(sections)})", "",
            "Основа Universal V13 Lite (без удалённых сервисов) + секции из других пресетов + щиты pass. "
            "Состав по секциям: `assets/desync/presets/OCHISTKA Merged.txt`; причины исключений: `bypass-merge-conflicts.md`."]
    return "\n".join(out) + "\n"


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
    result = merge.merge(inputs, base=BASE, available=available, defined_funcs=defined_functions(), broad_lists=broad,
                         owner_excluded=OWNER_EXCLUDED)
    description = [
        f"Объединённый пресет OCHISTKA: основа {BASE[:-4]} без изменений + секции из других пресетов {src}.",
        "Создан tools/bypass-merge/merge_presets.py; конфликты и исключения: docs/status/bypass-merge-conflicts.md.",
        "НЕ ПРОВЕРЕН на Windows у провайдера: обход не объявляется рабочим.",
    ]
    # Маршруты: правила и списки RixyPow, один файл.
    cfg_dir = os.path.join(args.source, "config/lists")
    config_lists, kinds = {}, {}
    for name in sorted(os.listdir(cfg_dir)):
        if not name.endswith(".txt"):
            continue
        raw = open(os.path.join(cfg_dir, name), "rb").read()
        lines = [x.strip() for x in raw.decode("utf-8", "ignore").splitlines() if x.strip() and not x.strip().startswith("#")]
        config_lists[name] = (hashlib.sha256(raw).hexdigest(), len(lines))
        kinds[name] = "ipset" if lines and re.match(r"^[0-9a-fA-F:.]+(/\d+)?$", lines[0]) else "hostlist"
    existing_by_sha = {}
    for rel in sorted(ours):
        if rel.startswith("lists/"):
            existing_by_sha.setdefault(hashlib.sha256(open(os.path.join(RUNTIME, rel), "rb").read()).hexdigest(), rel)
    used_lists = set()
    for sec in result.sections:
        for a in sec.args:
            if zpreset.key_of(a) in ("hostlist", "ipset", "hostlist-exclude", "ipset-exclude"):
                used_lists.add(zpreset.value_of(a))
    rules = routes.parse_rules(open(os.path.join(args.source, "config/rules.yaml"), encoding="utf-8").read())
    rt = routes.build(rules, config_lists, existing_by_sha, used_lists, kinds, exclude=OWNER_EXCLUDED)
    final_sections = result.sections + rt.sections
    text = zpreset.render("OCHISTKA Merged", description, result.globals, final_sections)
    with open(MERGED_PRESET, "w", encoding="utf-8", newline="\n") as fh:
        fh.write(text)
    md = merge.conflicts_markdown(result, src)
    md += f"\n## Маршруты ({len(rt.conflicts)} конфликтов с пресетом)\n\n"
    md += ("Маршруты RixyPow (`config/rules.yaml`, 31 правило + все списки `config/lists`) собраны в "
           "`assets/desync/routes/OCHISTKA Routes.json`. Режим `proxy` требует VPN и этим переключателем не применяется "
           "(ADR 0014, п. 1); `direct` — не вмешиваться; `desync` — в пресете или секцией pass (щит, как у RixyPow). "
           "Списки без правила остаются в файле как справочные (`catalog`), пока на Windows не подтвердится нужное действие.\n\n")
    if rt.conflicts:
        md += "| № | Маршрут | Что выбрано и почему |\n|---|---|---|\n"
        for i, c in enumerate(rt.conflicts, 1):
            md += f"| R{i:03d} | `{c['value']}` | {c['detail']} |\n"
    with open(os.path.join(REPO, "docs/status/bypass-merge-conflicts.md"), "w", encoding="utf-8", newline="\n") as fh:
        fh.write(md)
    os.makedirs(os.path.dirname(ROUTES_FILE), exist_ok=True)
    counts_applied = {}
    for e in rt.entries:
        counts_applied[e["applied"]] = counts_applied.get(e["applied"], 0) + 1
    with open(ROUTES_FILE, "w", encoding="utf-8", newline="\n") as fh:
        json.dump({"schema_version": 1, "source": src, "license": "MIT (NetZapret © 2026 RixyPow)",
                   "applied_counts": counts_applied, "routes": rt.entries}, fh, ensure_ascii=False, indent=1)
        fh.write("\n")
    with open(os.path.join(REPO, "docs/status/bypass-remove-steam-riot.md"), "w", encoding="utf-8", newline="\n") as fh:
        fh.write(removal_markdown(result, rt, src))
    with open(os.path.join(REPO, "docs/status/bypass-routes-status.md"), "w", encoding="utf-8", newline="\n") as fh:
        fh.write(routes_status_markdown(rt, final_sections, src))
    cov = result.coverage()
    cov["source"] = src
    cov["route_sections"] = len(rt.sections)
    problems = merge.check_coverage(cov, final_sections, result.globals)
    if problems:
        sys.exit("Манифест не сходится: " + "; ".join(problems[:5]))
    with open(os.path.join(REPO, "docs/status/bypass-merge-coverage.json"), "w", encoding="utf-8", newline="\n") as fh:
        json.dump(cov, fh, ensure_ascii=False, indent=1, sort_keys=True)
        fh.write("\n")
    # Дополнительные данные (списки и образцы), которых нет среди 135 закреплённых файлов.
    shutil.rmtree(EXTRA, ignore_errors=True)
    files = []
    needed = set(zpreset.referenced_resources(result.globals + [a for s in final_sections for a in s.args])) | set(rt.ship)
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
    with open(RUST_ASSETS, "w", encoding="utf-8", newline="\n") as fh:
        fh.write(render_rust_assets(files))
    counts = {}
    for r in result.section_records:
        counts[r.status] = counts.get(r.status, 0) + 1
    print("sections:", counts, "merged:", len(final_sections), "route sections:", len(rt.sections), "routes:", counts_applied, "extra files:", len(files))


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
