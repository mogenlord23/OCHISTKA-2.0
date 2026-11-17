File: package.json (root)
Declared Lines: 20
Lines Inspected: 20
Functions Analyzed: 0 (JSON object)

Execution Traces: name, private, type, workspaces match package-lock.json. dev and build delegate to the workspace Tauri CLI; beforeDevCommand and beforeBuildCommand do not recurse into root scripts. test chains vitest and cargo test; check chains svelte-check, cargo fmt and clippy. No global installs required. Missing dist does not break test or check (tauri-codegen skips frontendDist check in dev with devUrl; depends on tauri-macros, UNVERIFIED). Findings: check lacks Prettier (FND-0148); README drift (FND-0149); fail-fast masking (FND-0150); no engines field (FND-0151); major-only prettier ranges (FND-0152); format scope includes .claude (FND-0153). Cross-file: FND-0146 (TypeScript version split) is not re-filed.
UNVERIFIED: the CLAUDE.md global-install rule was not located in project sources; jsdom and vitest runtime failure on out-of-range Node not run; tauri-macros dev flag; cargo clippy --all-targets output not run.

STATUS: COMPLETE
(Recorded by the orchestrator from the auditor's read-only report.)
