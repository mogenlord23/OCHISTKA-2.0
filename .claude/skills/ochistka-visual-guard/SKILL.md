---
name: ochistka-visual-guard
description: Manual visual acceptance and layout regression checks for OCHISTKA 2.0; one screen or shared shell at a time
argument-hint: "[раздел, экран, конкретный дефект]"
disable-model-invocation: true
---

# OCHISTKA visual guard (manual, no autonomous invocation)

Run only when the owner explicitly requests UI verification, either via `/ochistka-visual-guard` OR by requesting that you open and follow this file in the prompt. This file must not be read for unrelated tasks. Do not launch subagents, install plugins/MCP, or rewrite the app.

**Scope:** the named target (or `$ARGUMENTS` when invoked with a command). Do not inspect all seven screens unless a shared layout, token, rail or shell changed.

1. Read `CLAUDE.md`, only relevant current clauses of `docs/REQUIREMENTS.md`, `docs/design/design-system.md`, `apps/desktop/src/lib/tokens.ts`, and affected Svelte/CSS files. The owner's latest approved appearance overrides superseded screenshots. Keep approved rounded window and no unnecessary shadow.
2. Before editing, capture actual layout geometry (bounding rectangles, computed CSS) and a screenshot of affected state if a browser is available. Find the approved reference (e.g. commit `f15de11` where relevant); do not fabricate one. List the alignment and interaction invariants to preserve.
3. Fix the root cause using the smallest change. Reuse existing tokens and components. No arbitrary offsets, hardcoded radii, `!important`, `overflow:hidden` hiding defects, or universal style overhaul without reason.
4. Inspect the affected interaction in default / hover / selected / narrow viewport. Check 900×520, 1179×803 and 1920×1080 where host supports these sizes; confirm actual minimal window constraints rather than assuming 900×520 is feasible. Check no cropping, unreadable labels, page scroll, z-index leaks, unexpected stretching, changing baselines or unintended selection.
5. Prefer the existing `apps/desktop/scripts/layout-check.mjs`, targeted Vitest, and `npm run check`. The existing script mostly checks the outer shell; for recurrent inner-layout bugs, ADD a small deterministic check of the affected geometry/interaction instead of merely promising screenshot review.
6. Compare before and after screenshots VISUALLY whenever possible; do not mistake mocks for Windows verification. Browser screenshots using mocked Tauri cannot prove native Windows system integration, real window shadows, DPI, cursor or VPN behavior.
7. If introducing animation, prefer transform/opacity/compositor-safe effects, fixed alignment throughout, no flicker, no duplicate callbacks; respect `prefers-reduced-motion` and immediate selection. Only claim FPS gains when measured.
8. Return concise Russian PASS / FAIL / NOT RUN report, screenshots/evidence, changed files, tests and remaining real-Windows checks. One logical commit in active work branch; never push directly to main.

Optional deeper checklist: `references/visual-acceptance.md`. Read only the relevant portion, not by default.
