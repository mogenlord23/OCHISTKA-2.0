File: apps/desktop/src/components/ComputerPage.svelte
Declared Lines: 12
Lines Inspected: 12
Functions Analyzed: 1 (component with branch)

Execution Traces: one required string prop from App.svelte:70 (types match); components renders Home; other ids render the generic placeholder; subtab switching remounts Home (FND-0103 already recorded); unknown ids fall to placeholder silently (FND-0079 root cause).
Findings: FND-0129 (routing untested, low), FND-0130 (doc drift on placeholders, info).
Already covered: FND-0079, FND-0102, FND-0103, FND-0121, FND-0125, FND-0113.
UNVERIFIED: docs/platform-status.md status of the three placeholder subsections.

STATUS: COMPLETE
(Recorded by the orchestrator from the auditor's read-only report.)
