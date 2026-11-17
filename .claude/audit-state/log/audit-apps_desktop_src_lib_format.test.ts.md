File: apps/desktop/src/lib/format.test.ts
Declared Lines: 20
Lines Inspected: 20 (format.ts also read, 20 lines)
Functions Analyzed: 2 (formatBytes, formatPercent), 3 test cases

Execution Traces: every expected string checked by hand against format.ts: 0 and 500 give Б; 1536 gives 1.5 КБ; 2^30 gives 1.0 ГБ; 12.4 gives 12 %. All pass. Boundaries, cap, negative, NaN and Infinity checked by hand (FND-0073, FND-0074, FND-0075, FND-0076 already recorded). No locale dependence (no toLocaleString or Intl). Call sites in Home.svelte and Home.test.ts agree with format.ts. Findings: rounding not distinguished (FND-0197), boundaries and cap not pinned (FND-0198), fractional bytes branch (FND-0199), all info.
UNVERIFIED: byte-level identity of Cyrillic unit letters versus Latin lookalikes; f32 and u64 JSON precision (FND-0066 related); vitest not run.

STATUS: COMPLETE
(Recorded by the orchestrator from the auditor's read-only report.)
