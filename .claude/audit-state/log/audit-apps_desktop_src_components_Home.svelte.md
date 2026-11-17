File: apps/desktop/src/components/Home.svelte
Declared Lines: 165
Lines Inspected: 165
Functions Analyzed: effect, load plus markup and style

Execution Traces: effect runs once, teardown clean (active flag and interval); pending guard resets on settle (FND-0102 for never-settling); remount leaves backend job running (FND-0103); ARIA and style widths clamped on backend (FND-0067 caveat); keyed each duplicate via lossy conversion (FND-0104); error branch hides data (FND-0105); polling after unsupported (FND-0106). DTO names match types.ts.
Already covered: FND-0038, 0039, 0067, 0069, 0076, 0086, 0058.
UNVERIFIED: real Windows blocking duration; Windows path surrogate creation; top-level effect error impact on the shell.

STATUS: COMPLETE
(Recorded by the orchestrator from the auditor's read-only report.)
