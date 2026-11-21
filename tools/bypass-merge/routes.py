"""Маршруты RixyPow/netzapret (rules.yaml + config/lists) в один файл маршрутов.

Только данные. Режимы правил RixyPow: direct (не вмешиваться), proxy (через VPN), desync (оставить
десинку, у RixyPow — с рецептом pass). «Обход блокировок» VPN не использует (ADR 0014), поэтому
proxy-маршруты записываются, но этим переключателем не применяются.
"""
from __future__ import annotations

import re
from dataclasses import dataclass, field

from zpreset import Section

GAME_WORDS = (
    "arc-raiders", "battlenet", "ea", "epicgames", "fortnite", "gog", "hoyoverse", "itch", "lol",
    "minecraft", "nintendo", "photon", "playstation", "riot", "roblox", "rockstar", "steam", "tankix",
    "ubisoft", "valheim", "wardogs", "xbox", "game-net",
)
APP_WORDS = (
    "anydesk", "discord", "docker", "dropbox", "figma", "github", "jetbrains", "kick", "linkedin", "miro", "notion",
    "npm", "obsidian", "pypi", "signal", "slack", "soundcloud", "spotify", "telegram", "twitch", "vencord",
    "viber", "whatsapp", "zoom", "chatgpt", "claude", "gemini", "grok", "deepseek", "mistral", "perplexity",
    "huggingface", "proton", "speedtest", "virustotal",
)


def category(name: str, kind: str) -> str:
    base = re.sub(r"^(nz-|ipset-)", "", name.removesuffix(".txt"))
    tokens = set(re.split(r"[-_.]", base)) | {base}
    if tokens & set(GAME_WORDS) or any(base.startswith(w) for w in GAME_WORDS if len(w) > 3):
        return "game"
    if tokens & set(APP_WORDS) or any(base.startswith(w) for w in APP_WORDS if len(w) > 4):
        return "app"
    return "network" if kind == "ipset" else "domain"


def norm(name: str) -> str:
    n = name.rsplit("/", 1)[-1]
    return re.sub(r"^(nz-|ipset-)", "", n)


def parse_rules(text: str) -> list[dict]:
    rules: list[dict] = []
    cur: dict | None = None
    in_rules = False
    for raw in text.splitlines():
        line = raw.split("#", 1)[0].rstrip() if not raw.lstrip().startswith('"') else raw.rstrip()
        s = line.strip()
        if s == "rules:":
            in_rules = True
            continue
        if not in_rules or not s:
            continue
        m = re.match(r"-\s*match:\s*(\w+)", s)
        if m:
            cur = {"match": m.group(1)}
            rules.append(cur)
            continue
        m = re.match(r"(value|mode|server):\s*(.*)$", s)
        if m and cur is not None:
            cur[m.group(1)] = m.group(2).strip().strip('"')
    return [r for r in rules if "value" in r and "mode" in r]


@dataclass
class Result:
    entries: list[dict] = field(default_factory=list)
    sections: list[Section] = field(default_factory=list)
    conflicts: list[dict] = field(default_factory=list)
    ship: dict[str, str] = field(default_factory=dict)  # куда положить -> имя файла в config/lists


def build(rules: list[dict], config_lists: dict[str, tuple[str, int]], existing_by_sha: dict[str, str],
          preset_lists: set[str], kinds: dict[str, str] | None = None) -> Result:
    kinds = kinds or {}
    res = Result()
    covered = {norm(x) for x in preset_lists}
    dest_by_sha: dict[str, str] = dict(existing_by_sha)
    dest_of: dict[str, str] = {}
    ruled = {r["value"].rsplit("/", 1)[-1] for r in rules if r["match"] in ("hostlist", "ipset")}
    order = sorted(config_lists, key=lambda n: (n not in ruled, n))
    for name in order:
        sha = config_lists[name][0]
        if sha in dest_by_sha:
            dest_of[name] = dest_by_sha[sha]
        else:
            dest_of[name] = f"lists/nz-{name}"
            dest_by_sha[sha] = dest_of[name]
            res.ship[dest_of[name]] = name
    rule_for: dict[str, dict] = {}
    for r in rules:
        if r["match"] in ("hostlist", "ipset"):
            rule_for.setdefault(r["value"].rsplit("/", 1)[-1], r)
    entries: dict[tuple[str, str], dict] = {}

    def add(entry: dict) -> None:
        key = (entry["kind"], entry.get("file") or entry["value"])
        if key in entries:
            entries[key].setdefault("aliases", []).append(entry.get("source_list", ""))
            return
        entries[key] = entry

    for name in order:
        rule = rule_for.get(name)
        kind = (rule["match"] if rule else kinds.get(name, "hostlist"))
        entry = {"kind": kind, "file": dest_of[name], "source_list": name, "lines": config_lists[name][1],
                 "category": category(name, kind), "mode": rule["mode"] if rule else "catalog"}
        if rule:
            add_rule(entry, rule, covered, name, res)
        else:
            entry["applied"] = "preset" if norm(name) in covered else "catalog"
        add(entry)
    for r in rules:
        if r["match"] not in ("domain",):
            continue
        value = r["value"].removeprefix("*.")
        entry = {"kind": "domain", "value": value, "mode": r["mode"], "category": "domain"}
        add_rule(entry, r, covered, value, res)
        add(entry)
    res.entries = [entries[k] for k in sorted(entries, key=lambda k: (k[0], k[1]))]
    return res


def add_rule(entry: dict, rule: dict, covered: set[str], name: str, res: Result) -> None:
    mode = rule["mode"]
    if mode == "proxy":
        entry["applied"] = "vpn-only"
        entry["note"] = "нужен VPN; «Обход блокировок» VPN не использует"
    elif mode == "direct":
        entry["applied"] = "direct"
        entry["note"] = "не вмешиваться"
        if entry["kind"] != "domain" and norm(name) in covered:
            res.conflicts.append({
                "value": entry["file"],
                "detail": f"маршрут direct для «{norm(name).removesuffix('.txt')}», но в пресете есть секция с этим списком; "
                          "оставлено поведение пресета (основа не меняется), проверить на Windows",
            })
    elif mode == "desync":
        if entry["kind"] != "domain" and norm(name) in covered:
            entry["applied"] = "preset"
        else:
            entry["applied"] = "shield"
            entry["note"] = "рецепт pass, как у RixyPow; действует на трафик, не пойманный более ранними секциями"
            scope = (f"--hostlist={entry['file']}" if entry["kind"] == "hostlist"
                     else f"--ipset={entry['file']}" if entry["kind"] == "ipset"
                     else f"--hostlist-domains={entry['value']}")
            label = entry.get("file") or entry["value"]
            res.sections.append(Section([
                f"--name=Маршрут (pass): {label.removeprefix('lists/')}", "--filter-tcp=80,443-65535",
                scope, "--out-range=-d8", "--lua-desync=pass",
            ]))
    else:
        entry["applied"] = "catalog"
