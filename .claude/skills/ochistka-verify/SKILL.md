---
name: ochistka-verify
description: Run narrow non-destructive checks for the current change in OCHISTKA
disable-model-invocation: true
---

# Minimal verification

1. Inspect `git diff --stat` / `git diff --check` and identify the touched crate or Svelte component.
2. Run relevant tests only: e.g. `cargo test -p ochistka-core -q` for core, targeted Vitest test for Svelte, `npm run check` for TS/Svelte when appropriate.
3. For Rust formatting, prefer `cargo fmt --all --check`; for changed files only report needed changes. Avoid printing huge log output; show the first actionable failure and short summary.
4. Full `cargo test --workspace`, clippy and project build only at a major stage boundary or explicitly requested.
5. Don't modify Windows state or promote simulated tests to `verified` on Windows.
6. Report PASS / FAIL / NOT RUN; distinguish tested and untested areas.
