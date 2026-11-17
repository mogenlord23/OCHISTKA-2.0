File: apps/desktop/src/components/Home.test.ts
Declared Lines: 116
Lines Inspected: 116
Functions Analyzed: 9 (flush, afterEach, 7 tests)

Execution Traces: mocks keep real POLL_INTERVAL_MS and toCommandError; sample data matches types.ts; formatted strings verified against format.ts (12 %, 4.0 ГБ из 8.0 ГБ). Render from @testing-library/svelte flushes effects synchronously; cleanup runs teardown (active=false, clearInterval). afterEach order (cleanup, useRealTimers, mockReset) is correct. Findings: no polling assertion (FND-0170, medium); response-after-unmount test cannot fail (FND-0171, medium); uncovered branches (FND-0172, low); misleading test titles (FND-0173, FND-0174); flush and fake-timer fragility (FND-0175).
Cross-references: FND-0102 (stuck pending), FND-0103 (unmount does not cancel backend job; consistent), FND-0105 (error hides data; no test).
UNVERIFIED: that Svelte 5.57 never logs when state is written after unmount (Svelte runtime not read); Svelte tick interaction with async batching; vitest not run.

STATUS: COMPLETE
(Recorded by the orchestrator from the auditor's read-only report.)
