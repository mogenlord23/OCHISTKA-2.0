File: crates/platform/Cargo.toml
Declared Lines: 9
Lines Inspected: 9
Functions Analyzed: 0 (2 sections)

Execution Traces: name ochistka-platform matches Cargo.lock:2220; workspace package fields inherit correctly; ochistka-core path dependency resolves to crates/ochistka-core and is used across lib.rs, windows, macos and linux modules. sysinfo 0.40.4 satisfies the requirement; only System, MINIMUM_CPU_UPDATE_INTERVAL, Disks and sysinfo::Error are used. No target-specific table is correct because the windows module has no cfg gate and compiles on all targets (lib.rs:5), so sysinfo cannot be made Windows-only here. Only finding: default features enabled (FND-0159). Cross-references: FND-0156 (path declaration repeated in src-tauri, not re-filed), FND-0037 and FND-0158 (no manifest-level change), FND-0155 (sysinfo 0.40.4 declares rust-version 1.95, supporting evidence only).
Code/doc drift for the lib.rs audit (not filed here): CLAUDE.md describes non-Windows modules as empty stubs, but the windows module with its live sysinfo test compiles and runs on Linux and macOS (lib.rs:5, 76-89).
UNVERIFIED: compile and binary-size cost of unused features (no build run); whether WMI/COM code from the component feature has load-time side effects on Windows.

STATUS: COMPLETE
(Recorded by the orchestrator from the auditor's read-only report.)
