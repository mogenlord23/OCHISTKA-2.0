File: apps/desktop/src-tauri/Cargo.toml
Declared Lines: 21
Lines Inspected: 21
Functions Analyzed: 0 (4 sections)

Execution Traces: package name matches Cargo.lock:2209; lib target ochistka_desktop_lib matches lib.rs run(); bin target matches main.rs. Dependencies ochistka-core, ochistka-platform, serde (derive), tauri 2.12.6 and build-dependency tauri-build 2.7.5 are each used in source; no missing or unused entries; Cargo.lock entry matches. Tauri 2.12.x Rust and @tauri-apps/api 2.12.5 major.minor agree. custom-protocol is not enabled, so dev and workspace checks build in dev mode. Findings: release without custom-protocol loads devUrl (FND-0160); missing license and description (FND-0161); redundant bin and empty features (FND-0162). Cross-references: FND-0155, FND-0156, FND-0158 (not re-filed); FND-0061 to FND-0065 cover build.rs.
Correction to the auditor's cross-reference: FND-0037 concerns mutex poisoning, not version alignment.
UNVERIFIED: whether tauri-cli 2.12.5 adds tauri/custom-protocol automatically; Windows bundler use of Cargo description and license; MSRV of tauri 2.12.6 and tauri-build 2.7.5; whether ochistka_platform::windows::files compiles on non-Windows hosts (decided by crates/platform).

STATUS: COMPLETE
(Recorded by the orchestrator from the auditor's read-only report.)
