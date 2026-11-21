---
name: ochistka-performance
description: Manual targeted diagnosis and repair of lag, startup latency, animation FPS, IPC and Rust/Tauri/Svelte performance in OCHISTKA
argument-hint: "[конкретный лаг, переход или операция]"
---

# OCHISTKA performance (manual only)

Run only when explicitly requested to use this file or `/ochistka-performance`. No autonomous agents, MCP or broad profiling. Restrict scope to user-reported screen/operation and files needed to explain it. Preserve correctness, safety and visual design.

1. Name the specific symptom, device/OS and expected user-observable result. Establish lightweight baseline when feasible: time to first visible frame, interaction latency, repeated IPC count, component update count, task duration, or FPS. If a metric cannot be measured in cloud, say NOT MEASURED.
2. Inspect only relevant Svelte 5 state, subscriptions, rerenders, CSS animations, Tauri IPC and Rust OS adapters. Distinguish **instant UI feedback** from **successful Windows system action**; do not fake success or hide errors.
3. Find root cause: remounting on tab switch; unstable keys; synchronous OS operations on UI/IPC thread; duplicate listeners/timers; broad reactive dependencies; repeated polling; unnecessary disk/network calls; main-thread media decode; CSS layout/reflow transitions; excessive state updates.
4. Fix minimally: retain store/state across navigation as required, non-blocking background work with bounded concurrency, cache last verified data and invalidate correctly, throttle measurement not user input, update only changed data, virtualize only genuinely large lists. Never suppress security confirmations, errors, undo/backup, or use an unsafe process kill to 'speed up' the app.
5. For animation prefer transform/opacity with stable geometry and `prefers-reduced-motion`; avoid expensive global filters and continuous decorative effects. Keep them interruptible if user changes tabs quickly.
6. Verify with one focused regression test plus targeted check; where applicable compare baseline/after using same scenario and report numbers **only if measured**, including hardware and mock/native limitations. `apps/desktop/scripts/layout-check.mjs` handles shell geometry, not actual Windows native speed.
7. Report in Russian: specific root cause, minimal files changed, observed before/after metrics or NOT MEASURED, targeted test PASS/FAIL, native-Windows verification needed. Make one commit in current work branch, not main.

When a system action touches VPN, drivers, Windows settings or antivirus, respect `ochistka-security`, `ochistka-vpn`, `CLAUDE.md` and the real OS verification contract. Do NOT claim backend success from a frontend mock.
