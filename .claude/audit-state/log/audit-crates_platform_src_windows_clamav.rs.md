File: crates/platform/src/windows/clamav.rs
Declared Lines: 152
Lines Inspected: 152
Functions Analyzed: 11 (6 production, 5 tests)

Execution Traces: command built from absolute path, no shell, args vector, target last (safe via single caller); CREATE_NO_WINDOW under cfg(windows); exit gate 0/1 accepted; stdout lossy decode (FND-0047); stderr-only error (FND-0048); public scan trusts caller (FND-0049); unix fake-engine test leak and gaps (FND-0050); engine trust on existence (FND-0051).
Already covered, not duplicated: FND-0002, 0003, 0004, 0005, 0006, 0007, 0013, 0016, 0026, 0043.
UNVERIFIED: clamscan stdout encoding and message routing; CREATE_NO_WINDOW on real Windows; orphan child on exit; no test run (read-only, Linux).

STATUS: COMPLETE
(Recorded by the orchestrator from the auditor's read-only report.)
