"""Слияние пресетов в один без повторов. Только данные; код RixyPow не используется.

Принцип наименьшего риска:
* основа — один пресет целиком (порядок и содержимое не меняются), его трафик ведёт себя как раньше;
* из остальных берутся только секции, ограниченные списками адресов или доменов, трафик которых
  основа не обрабатывает; перехват (порты WinDivert) остаётся как в основе;
* всё, что не вошло, записывается с причиной (конфликт или исключение), ничего не пропадает молча.
"""
from __future__ import annotations

import re
from dataclasses import dataclass, field

import zpreset
from zpreset import Preset, Section, key_of, value_of

SINGLE_VALUE = (
    "ctrack-disable", "ipcache-lifetime", "ipcache-hostname", "wf-tcp-out", "wf-udp-out",
    "wf-tcp-in", "wf-udp-in", "wf-tcp-empty", "wf-raw-filter", "writable",
)


@dataclass
class SectionRecord:
    preset: str
    index: int
    name: str
    identity: str
    status: str  # base | duplicate | added | conflict | excluded
    detail: str = ""
    target: int | None = None  # индекс в итоговом пресете
    item_id: str = ""


@dataclass
class GlobalRecord:
    preset: str
    arg: str
    status: str  # base | duplicate | added | conflict | excluded
    detail: str = ""
    item_id: str = ""


@dataclass
class Result:
    title: str
    base: str
    globals: list[str]
    sections: list[Section]
    section_records: list[SectionRecord]
    global_records: list[GlobalRecord]
    resources: set[str] = field(default_factory=set)
    shared: list[SectionRecord] = field(default_factory=list)  # секции, где решение владельца затрагивает не всё

    def coverage(self) -> dict:
        return {
            "base": self.base,
            "sections": [
                {"preset": r.preset, "index": r.index, "name": r.name, "identity": r.identity,
                 "status": r.status, "target": r.target, "item_id": r.item_id}
                for r in self.section_records
            ],
            "globals": [
                {"preset": r.preset, "arg": r.arg, "status": r.status, "item_id": r.item_id}
                for r in self.global_records
            ],
        }


def _blobs(globals_: list[str]) -> dict[str, str]:
    out = {}
    for a in globals_:
        if key_of(a) == "blob" and ":" in value_of(a):
            name, file = value_of(a).split(":", 1)
            out[name] = file
    return out


def _used_blobs(sec: Section, declared: dict[str, str]) -> set[str]:
    used = set()
    for a in sec.body():
        for name in declared:
            if re.search(r"(?:(?<=[=:,])|^)" + re.escape(name) + r"(?=[:,]|$)", a.split("=", 1)[-1] if a.startswith("--lua-desync") else a):
                used.add(name)
    return used


def _only_broad(sec: Section, broad: set[str]) -> bool:
    """Единственный охват секции — огромный список адресов (почти весь интернет)."""
    if sec.values("hostlist-domains") or sec.values("ipset-ip"):
        return False
    scope = sec.values("hostlist") + sec.values("ipset")
    return bool(scope) and all(v in broad for v in scope)


def _scope_items(sec: Section) -> list[str]:
    items = [v.rsplit("/", 1)[-1] for v in sec.values("hostlist") + sec.values("ipset")
             + sec.values("hostlist-exclude") + sec.values("ipset-exclude")]
    for v in sec.values("hostlist-domains"):
        items += [d for d in v.split(",") if d]
    return items


def owner_match(sec: Section, rules: dict[str, str]) -> tuple[str, str, bool] | None:
    """Решение владельца для секции: (слово, причина, целиком?). `целиком` = весь охват секции — это сервис."""
    if not rules:
        return None
    items = [i.lower() for i in _scope_items(sec)]
    for word, reason in rules.items():
        hit = [i for i in items if word in i]
        if hit:
            return word, reason, len(hit) == len(items)
        if not items and word in sec.name.lower():
            return word, reason, True
    return None


def _funcs(sec: Section) -> set[str]:
    return {v.split(":", 1)[0] for v in sec.values("lua-desync")}


def merge(inputs: list[tuple[str, Preset]], base: str, available: set[str], defined_funcs: set[str],
          title: str = "OCHISTKA Merged", broad_lists: set[str] | None = None,
          owner_excluded: dict[str, str] | None = None) -> Result:
    broad_lists = broad_lists or set()
    owner_excluded = owner_excluded or {}
    shared: list[SectionRecord] = []
    by_id = dict(inputs)
    base_preset = by_id[base]
    order = [base] + [i for i, _ in inputs if i != base]

    merged_globals = list(base_preset.globals)
    merged_sections: list[Section] = []
    seen_identity: dict[str, int] = {}
    seen_selector: dict[tuple, int] = {}
    srecs: list[SectionRecord] = []
    grecs: list[GlobalRecord] = []
    resources: set[str] = set()
    counter = 0

    def next_id() -> str:
        nonlocal counter
        counter += 1
        return f"X{counter:03d}"

    wf_tcp = next((value_of(a) for a in base_preset.globals if key_of(a) == "wf-tcp-out"), "")
    wf_udp = next((value_of(a) for a in base_preset.globals if key_of(a) == "wf-udp-out"), "")
    merged_blobs = _blobs(merged_globals)

    # Основа целиком.
    for i, sec in enumerate(base_preset.sections):
        hit = owner_match(sec, owner_excluded)
        if hit and hit[2]:
            rec = SectionRecord(base, i, sec.name, sec.identity(), "removed", item_id=next_id(),
                                detail=f"{hit[1]} (слово «{hit[0]}»)")
            srecs.append(rec)
            continue
        merged_sections.append(sec)
        seen_identity.setdefault(sec.identity(), len(merged_sections) - 1)
        seen_selector.setdefault(sec.selector(), len(merged_sections) - 1)
        srecs.append(SectionRecord(base, i, sec.name, sec.identity(), "base", target=len(merged_sections) - 1))
        if hit:
            shared.append(SectionRecord(base, i, sec.name, sec.identity(), "base",
                                        detail=f"оставлена: затрагивает и другие сервисы ({hit[0]})"))
    for a in base_preset.globals:
        grecs.append(GlobalRecord(base, a, "base"))
    base_funcs = set().union(*(_funcs(s) for s in base_preset.sections)) if base_preset.sections else set()
    funcs_ok = defined_funcs | base_funcs

    pending_globals: list[tuple[str, str]] = []  # (preset, arg) — решаются после секций

    for pid in order[1:]:
        p = by_id[pid]
        declared = _blobs(p.globals)
        for i, sec in enumerate(p.sections):
            ident = sec.identity()
            rec = SectionRecord(pid, i, sec.name, ident, "excluded")
            srecs.append(rec)
            hit = owner_match(sec, owner_excluded)
            if hit and hit[2]:
                rec.status, rec.item_id = "removed", next_id()
                rec.detail = f"{hit[1]} (слово «{hit[0]}»)"
                continue
            if ident in seen_identity:
                rec.status, rec.target = "duplicate", seen_identity[ident]
                continue
            sel = sec.selector()
            if sel in seen_selector:
                t = seen_selector[sel]
                rec.status, rec.target = "conflict", t
                rec.item_id = next_id()
                rec.detail = (
                    f"тот же трафик, другое действие; оставлено действие секции «{merged_sections[t].name}» "
                    f"(№{t + 1} итогового пресета), как в основе или более раннем пресете"
                )
                continue
            if not sec.is_scoped() or _only_broad(sec, broad_lists):
                rec.item_id = next_id()
                rec.detail = "секция без списка адресов или доменов (или со списком на почти весь интернет): перехватывала бы весь трафик; риск, не включена"
                continue
            bad_port = None
            for key, spec in (("filter-tcp", wf_tcp), ("filter-udp", wf_udp)):
                for v in sec.values(key):
                    if spec and not zpreset.ports_cover(v, spec):
                        bad_port = f"{key}={v} вне базового перехвата ({spec})"
            if bad_port:
                rec.item_id = next_id()
                rec.detail = f"{bad_port}: расширять перехват WinDivert не стали (производительность, риск для игр)"
                continue
            missing_fn = sorted(f for f in _funcs(sec) if f not in funcs_ok)
            if missing_fn:
                rec.item_id = next_id()
                rec.detail = f"функция {', '.join(missing_fn)} не входит в поставляемые Lua-модули (это код, не данные)"
                continue
            res = {r for r in zpreset.referenced_resources(list(sec.body())) if not r.startswith("bin/") or True}
            used = _used_blobs(sec, declared)
            for name in used:
                res.add(declared[name].removeprefix("@"))
            missing_res = sorted(r for r in res if r not in available)
            if missing_res:
                rec.item_id = next_id()
                rec.detail = f"нет ресурса: {', '.join(missing_res)}"
                continue
            clash = sorted(n for n in used if n in merged_blobs and merged_blobs[n] != declared[n])
            if clash:
                rec.item_id = next_id()
                rec.detail = f"образец {', '.join(clash)} в основе указывает на другой файл"
                continue
            for name in sorted(used):
                if name not in merged_blobs:
                    merged_blobs[name] = declared[name]
                    arg = f"--blob={name}:{declared[name]}"
                    merged_globals.append(arg)
                    grecs.append(GlobalRecord(pid, arg, "added", "нужен добавленной секции"))
            resources |= res
            merged_sections.append(sec)
            idx = len(merged_sections) - 1
            seen_identity[ident] = idx
            seen_selector[sel] = idx
            rec.status, rec.target = "added", idx

    # Образцы (blob) удалённых сервисов, которые не использует ни одна оставшаяся секция.
    if owner_excluded:
        used_now: set[str] = set()
        for sec in merged_sections:
            used_now |= _used_blobs(sec, _blobs(merged_globals))
        for a in list(merged_globals):
            if key_of(a) == "blob":
                name, _, file = value_of(a).partition(":")
                word = next((w for w in owner_excluded if w in name.lower() or w in file.lower()), None)
                if word and name not in used_now:
                    merged_globals.remove(a)
                    for rec in grecs:
                        if rec.arg == a and rec.status == "base":
                            rec.status, rec.item_id = "removed", next_id()
                            rec.detail = f"{owner_excluded[word]}: образец объявлен, но не используется"

    # Общие аргументы остальных пресетов: повтор, конфликт значения или не нужен.
    have = set(merged_globals)
    for pid in order[1:]:
        for a in by_id[pid].globals:
            if a in have:
                if not any(r.arg == a and r.preset == pid and r.status == "added" for r in grecs):
                    grecs.append(GlobalRecord(pid, a, "duplicate"))
                continue
            k = key_of(a)
            word = next((w for w in owner_excluded if w in a.lower()), None) if k == "blob" else None
            if word:
                rec = GlobalRecord(pid, a, "removed", item_id=next_id(), detail=f"{owner_excluded[word]}: образец не нужен")
            elif k in SINGLE_VALUE:
                kept = next((x for x in merged_globals if key_of(x) == k), None)
                rec = GlobalRecord(pid, a, "conflict", item_id=next_id())
                rec.detail = f"оставлено значение основы: {kept}" if kept else "в основе опции нет; не добавлена (перехват и настройки как в основе)"
            elif k == "blob":
                rec = GlobalRecord(pid, a, "excluded", item_id=next_id(), detail="образец не нужен ни одной включённой секции")
                name = value_of(a).split(":", 1)[0]
                if name in merged_blobs and f"--blob={name}:{merged_blobs[name]}" != a:
                    rec = GlobalRecord(pid, a, "conflict", item_id=next_id(), detail=f"образец {name} в основе указывает на другой файл; оставлен файл основы")
            elif k == "lua-init":
                rec = GlobalRecord(pid, a, "excluded", item_id=next_id(), detail="Lua-модуль не входит в поставляемые (код, не данные) или не нужен включённым секциям")
            else:
                rec = GlobalRecord(pid, a, "excluded", item_id=next_id(), detail="не нужен включённым секциям; перехват как в основе")
            grecs.append(rec)

    return Result(title, base, merged_globals, merged_sections, srecs, grecs, resources, shared)


def conflicts_markdown(r: Result, source: str) -> str:
    out = [
        "# Конфликты и исключения при слиянии пресетов",
        "",
        f"Источник: {source}. Основа слияния: `{r.base}` (Universal V13 Lite без изменений — порядок, аргументы и поведение сохранены). "
        "Файл создан `tools/bypass-merge/merge_presets.py`, вручную не правится.",
        "",
        "## Правила выбора («вариант с наименьшим риском»)",
        "",
        "1. **Основа первой.** Секции основы идут первыми и не меняются: при совпадении трафика действует основа.",
        "2. **Дубли** (тот же набор аргументов, имя не учитывается) оставлены в одном экземпляре; в этот файл не попадают.",
        "3. **Конфликт:** тот же трафик (одинаковый селектор), но другие действия. Не удалено молча: записано здесь, "
        "выбрано действие основы или более раннего пресета (приоритет: Universal V13 Lite, V13, V13 YT FIX, V12…V8, Default, остальные).",
        "4. **Исключение «слишком широко»:** секция без списка адресов или доменов не включена (перехватывала бы весь трафик).",
        "5. **Исключение «вне перехвата»:** порты секции вне `--wf-tcp-out` / `--wf-udp-out` основы; перехват не расширяется.",
        "6. **Исключение «нет кода/ресурса»:** нужна Lua-функция или файл, которых нет среди поставляемых; код RixyPow не копируется.",
        "7. **Общие аргументы:** значения одиночных опций (порты перехвата, ctrack, ipcache) остаются как в основе.",
        "",
        "Каждый пункт ниже имеет номер Xnnn; тот же номер стоит в `docs/status/bypass-merge-coverage.json`.",
        "",
    ]
    rem = [x for x in r.section_records if x.status == "removed"] + [x for x in r.global_records if x.status == "removed"]
    out += [f"## Исключено по решению владельца ({len(rem)})", "",
            "Telegram, Steam и Riot / Valorant убраны из обхода; подробности: `docs/status/bypass-remove-steam-riot.md`.", ""]
    for x in rem:
        label = getattr(x, "name", None) or f"`{x.arg}`"
        out.append(f"- {x.item_id}: {x.preset} — {label}: {x.detail}")
    out.append("")
    secs = [x for x in r.section_records if x.status in ("conflict", "excluded")]
    out += [f"## Секции ({len(secs)})", "", "| № | Пресет | Секция | Вид | Что выбрано и почему |", "|---|---|---|---|---|"]
    for x in secs:
        out.append(f"| {x.item_id} | {x.preset} №{x.index + 1} | {x.name.replace('|', '/')} | {'конфликт' if x.status == 'conflict' else 'исключена'} | {x.detail.replace('|', '/')} |")
    gl = [x for x in r.global_records if x.status in ("conflict", "excluded")]
    out += ["", f"## Общие аргументы ({len(gl)})", "", "| № | Пресет | Аргумент | Вид | Что выбрано и почему |", "|---|---|---|---|---|"]
    for x in gl:
        out.append(f"| {x.item_id} | {x.preset} | `{x.arg.replace('|', '/')}` | {'конфликт' if x.status == 'conflict' else 'не взято'} | {x.detail.replace('|', '/')} |")
    return "\n".join(out) + "\n"


def check_coverage(cov: dict, sections: list[Section], globals_: list[str]) -> list[str]:
    """Проверка манифеста: что помечено включённым — есть в итоговом пресете; что нет — есть номер Xnnn."""
    problems = []
    ids = [s.identity() for s in sections]
    for rec in cov["sections"]:
        if rec["status"] in ("base", "added"):
            t = rec["target"]
            if t is None or t >= len(ids) or ids[t] != rec["identity"]:
                problems.append(f"{rec['preset']}#{rec['index']}: нет в итоговом пресете")
        elif rec["status"] == "duplicate":
            if rec["identity"] not in ids:
                problems.append(f"{rec['preset']}#{rec['index']}: дубль без оригинала")
        elif rec["status"] in ("conflict", "excluded", "removed"):
            if not rec["item_id"]:
                problems.append(f"{rec['preset']}#{rec['index']}: нет номера записи")
        else:
            problems.append(f"{rec['preset']}#{rec['index']}: неизвестный статус")
    gset = set(globals_)
    for rec in cov["globals"]:
        if rec["status"] in ("base", "added", "duplicate") and rec["arg"] not in gset:
            problems.append(f"общий аргумент {rec['arg']}: нет в итоговом пресете")
        if rec["status"] in ("conflict", "excluded", "removed") and not rec["item_id"]:
            problems.append(f"общий аргумент {rec['arg']}: нет номера записи")
    if len(ids) != len(set(ids)):
        problems.append("в итоговом пресете есть повторяющиеся секции")
    if len(globals_) != len(gset):
        problems.append("в итоговом пресете есть повторяющиеся общие аргументы")
    return problems
