File: apps/desktop/src-tauri/src/lib.rs
Declared Lines: 26
Lines Inspected: 26
Functions/Methods Analyzed: 1 (run)

Execution Traces:
- run: manage(AppState{ current_system_info(), current_virus_scanner() }) -> generate_handler! of 5 commands (all registered, all invoke names match) -> run(generate_context!()) -> Ok returns, Err panics at L25 [findings: FND-0001, FND-0002, FND-0003, FND-0004]

Bugs / Observations: see FND-0001 to FND-0004 (no critical or high).

Cross-file notes (not counted for lib.rs):
- assess_file and clamav_status are synchronous commands; assess_file does fs::metadata, which can block on an unreachable UNC path. To check when commands.rs is audited.

STATUS: COMPLETE
(Recorded by the orchestrator from the auditor's read-only report.)
