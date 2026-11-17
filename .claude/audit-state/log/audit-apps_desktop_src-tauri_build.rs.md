File: apps/desktop/src-tauri/build.rs
Declared Lines: 3
Lines Inspected: 3 (tauri-build 2.7.5, tauri-utils 2.10.3, embed-resource 3.0.12, tauri-winres 0.3.7 traced read-only)
Functions Analyzed: 1 (main)

Execution Traces: main calls tauri_build::build() with default attributes; errors exit 1 loudly. Writes to OUT_DIR and gen/schemas (FND-0064). Default AppManifest gives has_app_manifest=false: root cause of FND-0030 and FND-0033 (already recorded). rerun-if-changed gaps (FND-0061). Env var gaps (FND-0062). RC path resolution (FND-0063). Default manifest (FND-0065). Icon, version, identifier consistent with config.
UNVERIFIED: static_vcruntime::metabuild source; Windows-only branches (built from source only, Linux host); some acl/tauri-utils internals.

STATUS: COMPLETE
(Recorded by the orchestrator from the auditor's read-only report.)
