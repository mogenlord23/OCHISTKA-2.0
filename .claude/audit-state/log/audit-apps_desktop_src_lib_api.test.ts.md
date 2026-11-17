File: apps/desktop/src/lib/api.test.ts
Declared Lines: 85
Lines Inspected: 85 (api.ts, types.ts, commands.rs, system.rs, scan.rs, tsconfig, vite.config also read)
Functions Analyzed: 8 tests in 3 describe groups, module-level mock setup

Execution Traces: five command names and argument keys (target, path) match commands.rs and lib.rs exactly. Error message strings match Rust Display text. Mock isolation holds: six invoke tests call mockReset; toCommandError tests do not touch invoke. Findings: no Rust-side DTO check despite ADR 0001 (FND-0191, medium); toCommandError coverage gaps (FND-0192); no call-count assertions (FND-0193); rejection and POLL_INTERVAL_MS coverage gaps (FND-0194, FND-0195, info); empty findings fixture (FND-0196, info).
Cross-references: FND-0057, FND-0170, ADR 0001 (docs/adr/0001-stack.md:35).
UNVERIFIED: whether svelte-check type-checks *.test.ts; Tauri invoke argument key case (irrelevant for single-word keys); UI handling of empty default_scan_target; whether other files assert POLL_INTERVAL_MS; vitest trailing-undefined argument handling in toHaveBeenCalledWith; vitest not run.

STATUS: COMPLETE
(Recorded by the orchestrator from the auditor's read-only report.)
