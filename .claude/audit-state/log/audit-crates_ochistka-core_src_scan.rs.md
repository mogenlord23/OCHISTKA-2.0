File: crates/ochistka-core/src/scan.rs
Declared Lines: 307
Lines Inspected: 307 (plus callers clamav.rs, commands.rs, files.rs, system.rs read)
Items Analyzed: 13 test functions, 4 functions, 5 declarations, 3 constants, 1 trait

Execution Traces: validate_target boundary table (byte indexing safe; "/" and "//" accepted, FND-0052; Unicode format chars accepted, FND-0054); parse_clamscan_output (CRLF clean; integer conversions unreachable truncation; memory FND-0055); score_suspicion (indexing guarded; trailing-dot evasion FND-0053; cap dead in practice since max sum is 100); run_scan ordering verified by test.
Already covered, not duplicated: FND-0006, 0007, 0008, 0009, 0010, 0026, 0027, 0044, 0047.
UNVERIFIED: Rust std argv quoting on Windows; clamscan handling of "\\" and "//"; clamscan bare CR output; Win32 trailing dot and space normalisation; ERROR-line volume.

STATUS: COMPLETE
(Recorded by the orchestrator from the auditor's read-only report.)
