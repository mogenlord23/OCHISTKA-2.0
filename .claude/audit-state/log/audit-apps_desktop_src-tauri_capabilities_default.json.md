File: apps/desktop/src-tauri/capabilities/default.json
Declared Lines: 13
Lines Inspected: 13
Functions/Items Analyzed: 1 capability with 5 permission entries

Execution Traces: schema path valid; identifier unique; window label "main" resolves to the only window; local-only; window grants (minimize, toggle-maximize, close, start-dragging) are needed and used by TitleBar; core:default is broader than needed (FND-0032); app commands ungoverned (FND-0030 verified, FND-0033).
CSP (tauri.conf.json:24): no script-src override, inline and remote scripts blocked.
UNVERIFIED: IPC sources added to CSP at build time; full list of @tauri-apps/api imports; real Windows behaviour of the undecorated window (cannot test on Linux).

STATUS: COMPLETE
(Recorded by the orchestrator from the auditor's read-only report.)
