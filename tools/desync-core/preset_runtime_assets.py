#!/usr/bin/env python3
"""Offline verification and one-time reproducible import of pinned NetZapret assets.

Import downloads exactly one immutable GitHub source snapshot. It validates the
Git *blob* SHA-1 and byte size against our checked-in source map BEFORE writing
any files. It then records SHA-256 for every imported runtime resource.
No executable or driver is run. No Windows network setting is changed.
"""
from __future__ import annotations

import argparse
import hashlib
import io
import json
import pathlib
import re
import tarfile
import urllib.request

PROJECT = pathlib.Path(__file__).resolve().parents[2]
MANIFEST = PROJECT / "docs/status/desync-preset-resources.json"
PRESET = PROJECT / "assets/desync/presets/Universal V13 Lite.txt"
RUNTIME = PROJECT / "assets/desync/runtime"
COMMIT = "883ba05b9531a283e0d6c83fdc3536016fbd51d3"
PRESET_SHA256 = "e679901475d998cb0c7e5ed5c51a5286c3b97f02d4b6c0d947f4527830cacae7"
ALLOWED = ("lua", "lists", "bin", "windivert.filter")
MAX_ARCHIVE = 100 * 1024 * 1024


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def git_blob_sha1(data: bytes) -> str:
    return hashlib.sha1(b"blob " + str(len(data)).encode("ascii") + b"\x00" + data).hexdigest()


def safe_parts(value: str) -> tuple[str, ...]:
    # Reject traversal, Windows drives/ADS, absolute paths and ambiguous names.
    parts = value.split("/")
    if (
        not parts
        or any(not part or part in (".", "..") for part in parts)
        or any(re.fullmatch(r"[A-Za-z0-9_.-]+", part) is None for part in parts)
        or "\\" in value
        or ":" in value
    ):
        raise ValueError(f"Unsafe asset path: {value!r}")
    return tuple(parts)


def referenced_resources(text: str) -> set[str]:
    names: set[str] = set()
    for line in text.splitlines():
        line = line.strip()
        if not line.startswith("--"):
            continue
        for argument in re.split(r"\s+(?=--[A-Za-z])", line):
            for token in re.split(r"[:=,\s]+", argument):
                name = token.removeprefix("@")
                if any(name.startswith(prefix + "/") for prefix in ALLOWED):
                    safe_parts(name)
                    names.add(name)
    return names


def read_map() -> dict:
    config = json.loads(MANIFEST.read_text(encoding="utf-8"))
    if config.get("source_commit") != COMMIT or config.get("preset_sha256") != PRESET_SHA256:
        raise ValueError("Source commit/preset pin changed")
    if sha256(PRESET.read_bytes()) != PRESET_SHA256:
        raise ValueError("Original upstream preset SHA-256 does not match")
    entries = config["files"]
    names = [entry["destination"] for entry in entries]
    if len(entries) != 135 or len(set(names)) != 135:
        raise ValueError("Source inventory must contain 135 distinct resources")
    if set(names) != referenced_resources(PRESET.read_text(encoding="utf-8")):
        raise ValueError("Preset references and source inventory differ")
    for entry in entries:
        dest = entry["destination"]
        parts = safe_parts(dest)
        if parts[0] not in ALLOWED:
            raise ValueError(f"Unknown runtime directory: {dest}")
        source = entry["source_path"]
        source_parts = safe_parts(source)
        if source_parts[:2] not in [
            ("vendor", "zapret-gui"), ("config", "lists"),
            ("config", "blobs"), ("config", "lua"),
        ]:
            raise ValueError(f"Unexpected upstream source path: {source}")
        if re.fullmatch(r"[a-f0-9]{40}", entry["git_blob_sha1"]) is None:
            raise ValueError(f"Invalid Git blob pin: {dest}")
        if not isinstance(entry["size_bytes"], int) or entry["size_bytes"] < 0:
            raise ValueError(f"Invalid pinned size: {dest}")
    return config


def safe_output_path(destination: str) -> pathlib.Path:
    parts = safe_parts(destination)
    path = RUNTIME.joinpath(*parts)
    for parent in (RUNTIME, *path.parents):
        if parent == PROJECT.parent or parent == pathlib.Path("/"):
            break
        if parent.is_symlink():
            raise ValueError(f"Symlinked directory refused: {destination}")
        if parent == PROJECT:
            break
    if path.is_symlink():
        raise ValueError(f"Symlinked file refused: {destination}")
    return path


def verify() -> None:
    config = read_map()
    if config.get("state") != "PINNED_ASSETS_PENDING_WINWS_DRY_RUN":
        raise ValueError("Incomplete manifest; runtime assets are not pinned")
    total_bytes = 0
    for entry in config["files"]:
        name = entry["destination"]
        if entry.get("asset_path") != "assets/desync/runtime/" + name:
            raise ValueError(f"Wrong runtime path: {name}")
        pinned_sha = entry.get("sha256")
        if not isinstance(pinned_sha, str) or re.fullmatch(r"[a-f0-9]{64}", pinned_sha) is None:
            raise ValueError(f"Unpinned SHA-256: {name}")
        path = safe_output_path(name)
        if not path.is_file():
            raise ValueError(f"Missing resource: {name}")
        data = path.read_bytes()
        if len(data) != entry["size_bytes"]:
            raise ValueError(f"Bad resource size: {name}")
        if sha256(data) != pinned_sha:
            raise ValueError(f"Bad SHA-256: {name}")
        if git_blob_sha1(data) != entry["git_blob_sha1"]:
            raise ValueError(f"Bad Git source identity: {name}")
        total_bytes += len(data)
    if total_bytes != config["total_source_bytes"]:
        raise ValueError("Total source byte count does not match")
    print(f"VERIFIED: {len(config['files'])} pinned runtime files; {total_bytes} bytes; no network")


def import_original_assets() -> None:
    config = read_map()
    if config.get("state") != "UNVERIFIED_DEPENDENCIES_DO_NOT_RUN":
        raise ValueError("Import is only valid for the initial unverified source map")
    expected = {e["source_path"]: e for e in config["files"]}
    # Strict HTTPS + immutable 40-digit SHA. All archived entries are untrusted.
    url = f"https://codeload.github.com/RixyPow/netzapret/tar.gz/{COMMIT}"
    req = urllib.request.Request(url, headers={"User-Agent": "OCHISTKA-pinned-source-import/1.0"})
    with urllib.request.urlopen(req, timeout=90) as response:
        archive_bytes = response.read(MAX_ARCHIVE + 1)
    if len(archive_bytes) > MAX_ARCHIVE:
        raise ValueError("Downloaded archive exceeds safety limit")

    verified: dict[str, bytes] = {}
    with tarfile.open(fileobj=io.BytesIO(archive_bytes), mode="r:gz") as archive:
        for member in archive:
            if "/" not in member.name:
                continue
            source = member.name.split("/", 1)[1]
            if source not in expected:
                continue
            pin = expected[source]
            if source in verified or not member.isfile() or member.size != pin["size_bytes"]:
                raise ValueError(f"Duplicate, missing, or invalid archive entry: {source}")
            handle = archive.extractfile(member)
            if handle is None:
                raise ValueError(f"Cannot read original file: {source}")
            data = handle.read(pin["size_bytes"] + 1)
            if len(data) != pin["size_bytes"] or git_blob_sha1(data) != pin["git_blob_sha1"]:
                raise ValueError(f"Source Git blob verification failed: {source}")
            verified[source] = data
    if set(verified) != set(expected):
        raise ValueError(f"Missing original files: {sorted(set(expected) - set(verified))}")

    # No partial output before ALL upstream contents have been verified.
    for entry in config["files"]:
        destination = entry["destination"]
        data = verified[entry["source_path"]]
        target = safe_output_path(destination)
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(data)
        entry["sha256"] = sha256(data)
        entry["asset_path"] = "assets/desync/runtime/" + destination
    config["state"] = "PINNED_ASSETS_PENDING_WINWS_DRY_RUN"
    MANIFEST.write_text(json.dumps(config, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    verify()


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=("verify", "import"))
    args = parser.parse_args()
    if args.action == "verify":
        verify()
    else:
        import_original_assets()


if __name__ == "__main__":
    main()
