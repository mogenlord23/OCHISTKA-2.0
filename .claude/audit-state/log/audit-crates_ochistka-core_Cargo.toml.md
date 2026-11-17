File: crates/ochistka-core/Cargo.toml
Declared Lines: 12
Lines Inspected: 12
Functions Analyzed: 0 (3 tables)

Execution Traces: package fields inherit workspace values (version 0.1.0, edition 2021, publish false) matching Cargo.lock:2200. Dependencies serde (derive; used by Serialize derives in scan.rs and system.rs), thiserror 2 (used at system.rs:95-103) and dev-dependency serde_json (used only in the test module at system.rs:162) are each used in source. No platform-specific dependency table and no OS crates; the only cfg uses are cfg(test). Serde requirement "1" versus workspace pins is FND-0156 (cross-referenced, not re-filed). thiserror 1.0.69 in the lock comes from another part of the graph (not this file).
UNVERIFIED: minimal-versions support for rename_all camelCase; source of thiserror 1.0.69 in the lock; no build run (cargo not run, compile-time use shown by reading only).

Result: no new findings.

STATUS: COMPLETE
(Recorded by the orchestrator from the auditor's read-only report.)
