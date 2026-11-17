File: apps/desktop/src/components/Subtabs.svelte
Declared Lines: 54
Lines Inspected: 54
Functions Analyzed: 2 (props destructure, onclick closure)

Execution Traces: props typed at single call site; keyed each throws on duplicate ids in dev and prod (FND-0112); native buttons; aria-current and class:active share one test; onSelect receives only ids from items; empty lists guarded in App; contrast ink on black both states about 19.2:1; focus ring inherited and visible on every accent.
Findings: FND-0112 (latent crash, low, settles FND-0081 UNVERIFIED step), FND-0113 (test gap, info), FND-0114 (semantics, info, design choice).
Already covered: FND-0079, FND-0081.
UNVERIFIED: rendered tab widths (Boost shortest title estimated 44px or more).

STATUS: COMPLETE
(Recorded by the orchestrator from the auditor's read-only report.)
