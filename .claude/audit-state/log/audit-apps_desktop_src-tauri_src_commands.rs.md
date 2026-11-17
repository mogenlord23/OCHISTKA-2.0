File: apps/desktop/src-tauri/src/commands.rs
Declared Lines: 230
Lines Inspected: 230
Functions/Methods Analyzed: 21 (9 production, 12 in test module)

Execution Traces: From/CoreError mapping complete (all four variants, kinds match frontend); load_overview; scan_with via run_scan; get_system_overview and scan_start use spawn_blocking; assess_path validation then file_facts then score; assess_file synchronous (FND-0025); default_scan_target and clamav_status synchronous, no I/O, cannot block.
Input validation: every command with input validates it before I/O or engine calls; validator accepts UNC and device paths (FND-0007, FND-0027).
Cross-file: confirms FND-0018 backend half (FND-0026); FND-0003, FND-0005, FND-0006, FND-0002 referenced, not repeated.
UNVERIFIED: Tauri sync-command thread (FND-0025); tokio JoinError text (FND-0029); job object for child process (FND-0026); default-allow for app commands (FND-0030).

STATUS: COMPLETE
(Recorded by the orchestrator from the auditor's read-only report.)
