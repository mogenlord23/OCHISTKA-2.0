---
name: ochistka-feature
description: Implement exactly one small OCHISTKA 2.0 feature with tests and minimal context
disable-model-invocation: true
---

# Small feature — OCHISTKA 2.0

1. Before editing, read `CLAUDE.md`, then the relevant requirement in `docs/REQUIREMENTS.md` and only the impacted source files. Do not read all audit logs or image assets.
2. Define one acceptance criterion and a short plan (up to 5 bullets); preserve existing API boundaries and behavior.
3. Implement a small change with its relevant test. Prefer one feature per commit.
4. UI -> typed Tauri command -> core -> platform port. Keep Windows-specific operations within `crates/platform/src/windows/`; other OS adapters return explicit Unsupported until implemented and verified.
5. Run targeted tests quietly, report only failures; avoid project-wide audits and broad formatting unrelated files.
6. Do not perform real cleanup, registry changes, network changes, driver installs, system service edits, or release without explicit user approval.
7. Report changed paths, test results, Windows verification status, and exact commit SHA if a commit was requested.
