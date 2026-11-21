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

import inventory  # noqa: E402


def source_id(source: str) -> str:
    sha = subprocess.check_output(["git", "-C", source, "rev-parse", "HEAD"], text=True).strip()
    return f"RixyPow/netzapret@{sha}"


def cmd_inventory(args: argparse.Namespace) -> None:
    md = inventory.render_markdown(inventory.load_presets(args.source), source=source_id(args.source))
    with open(os.path.join(REPO, "docs/status/bypass-merge-inventory.md"), "w", encoding="utf-8", newline="\n") as fh:
        fh.write(md)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    p = sub.add_parser("inventory")
    p.add_argument("--source", required=True)
    p.set_defaults(func=cmd_inventory)
    args = parser.parse_args()
    args.func(args)


if __name__ == "__main__":
    main()
