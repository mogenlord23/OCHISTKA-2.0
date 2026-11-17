File: crates/ochistka-core/src/lib.rs
Declared Lines: 2
Lines Inspected: 2 (plus the two declared modules read in full: scan.rs 307/307, system.rs 180/180)
Items Analyzed: 2 module declarations, 14 non-test items in declared modules

Execution Traces:
- lib.rs: pub mod scan; pub mod system; - nothing to fail [findings: none in this file]
- Call graph checked by grep: run_scan (commands.rs:41), validate_target, score_suspicion, parse_clamscan_output (clamav.rs:80), FileFacts (files.rs:36), VirusScanPort (clamav.rs:56), SystemInfoPort (platform mod.rs), CoreError (commands.rs:21-33). No orphaned public item.

Findings located in declared modules (recorded against their own files):
- FND-0005 clamav.rs (high, medium confidence, UNVERIFIED clamscan exit-code behaviour)
- FND-0006, FND-0007, FND-0008, FND-0009, FND-0010: scan.rs
- FND-0011: docs/adr/0001-stack.md
- FND-0012: system.rs

Cross-file notes:
- clamav.rs decodes clamscan stdout with from_utf8_lossy; a non-UTF-8 codepage could mangle Cyrillic profile paths. UNVERIFIED, to confirm in the platform audit.

STATUS: COMPLETE
(Recorded by the orchestrator from the auditor's read-only report.)
