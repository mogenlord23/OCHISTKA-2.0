File: apps/desktop/src-tauri/tauri.conf.json
Declared Lines: 32
Lines Inspected: 32
Items Analyzed: 6 (root keys, build, app.windows[0], app.security, bundle, $schema)

Execution Traces: schema valid; identifier and productName valid; version 0.1.0 consistent (FND-0024); build devUrl matches vite config; frontendDist exists; window settings valid; CSP script-src via Tauri hashes ok; style-src unsafe-inline stays in effect; bundle icons exist; ClamAV not bundled (FND-0016 confirmed).
Findings: FND-0034 (IPC fallback, low), FND-0035 (data: SVG blocked, low), FND-0036 (hardening, info).
UNVERIFIED: dev webview bypasses get_asset so CSP may be dev-off (affects where FND-0034/0035 show); CLI working directory for beforeDevCommand; icon RGBA; WebView2 install mode default.

STATUS: COMPLETE
(Recorded by the orchestrator from the auditor's read-only report.)
