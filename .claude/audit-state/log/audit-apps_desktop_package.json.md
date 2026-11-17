File: apps/desktop/package.json
Declared Lines: 28
Lines Inspected: 28
Functions Analyzed: 0 (JSON object, 5 scripts, 11 dependency entries traced)

Execution Traces: scripts dev, build, tauri, test, check chain consistently with root package.json, src-tauri/tauri.conf.json (beforeDevCommand, beforeBuildCommand, devUrl 1420, frontendDist ../dist). All declared ranges match the lockfile workspace block; peer checks for vite-plugin-svelte, vitest, vite and @testing-library/svelte are satisfied. svelte-check peer conflict with typescript ^7.0.2 is FND-0146 (cross-referenced, evidence appended, not re-filed). New: FND-0154 (@types/node 26 vs Node 22 README claim, latent). Engines absence covered by FND-0151.
UNVERIFIED: installed node_modules tree not inspected (peer resolution from lockfile only); Tauri CLI working-directory and version-check behaviour from documentation, not source; svelte-check --fail-on-warnings flag validity in 4.7.8 not checked against source; no type-check run.

STATUS: COMPLETE
(Recorded by the orchestrator from the auditor's read-only report.)
