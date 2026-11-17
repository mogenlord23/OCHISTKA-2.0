File: apps/desktop/src/components/VirusesPage.svelte
Declared Lines: 311
Lines Inspected: 311
Functions Analyzed: 2 (onMount, start) plus declarations and markup

Execution Traces: onMount sequence and error handling (FND-0095, FND-0096); start guard and race checks (double-click safe within instance; cross-instance FND-0018/0026); markup escaped, no {@html}; keyed each duplicates (FND-0092, FND-0093); silent assess failures (FND-0094); misleading scanning state (FND-0098); accessibility of disabled buttons (FND-0097); doc drift and dead class (FND-0099); hidden errors and count mismatch (FND-0100); journal omits failures (FND-0101). Checked clean: runs growth, reduced-motion rule, live region, bind:value guard.
Already covered: FND-0005, 0009, 0018, 0025, 0026, 0047, 0057, 0059, 0085, 0086.
UNVERIFIED: clamscan duplicate paths; screen reader handling of title on disabled; IPC latency; resolved Svelte version.

STATUS: COMPLETE
(Recorded by the orchestrator from the auditor's read-only report.)
