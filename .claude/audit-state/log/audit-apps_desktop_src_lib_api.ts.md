File: apps/desktop/src/lib/api.ts
Declared Lines: 60
Lines Inspected: 60
Functions Analyzed: 7 plus 2 constants

Execution Traces: backend contract verified: five command names and argument keys (target, path) match generate_handler; DTO casing matches types.ts; error kinds match ERROR_KINDS. Only invoke call site is api.ts. POLL_INTERVAL_MS guarded by pending flag in Home.svelte.
Findings: FND-0057 (error loss for unknown kinds, low), FND-0058 (doc drift on pass-through, info), FND-0059 (String throw, info), FND-0060 (unchecked cast, info).
UNVERIFIED: Tauri argument-key renaming rules (no effect here); Tauri runtime rejection strings; SystemOverview sub-struct field names; null-prototype rejection over IPC.

STATUS: COMPLETE
(Recorded by the orchestrator from the auditor's read-only report.)
