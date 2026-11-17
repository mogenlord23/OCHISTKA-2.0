File: apps/desktop/src/components/VirusesPage.test.ts
Declared Lines: 75
Lines Inspected: 75
Functions Analyzed: 2 tests, 1 helper (flush), 1 afterEach

Execution Traces: mocks spread the real module and keep toCommandError; shapes match types.ts and Rust camelCase serde. Test 1 (engine missing) verifies the disabled scan button. Test 2 (scan flow) traces sweep, result, score and findings through VirusesPage.svelte:42-79 and 122-155. cleanup and resetAllMocks isolate tests; no order dependence. Findings: log subsection untested (FND-0180, medium); error paths untested (FND-0181, medium); weak engine-missing assertion (FND-0182); guard coverage (FND-0183); tautological Удалить assertion (FND-0184); flush microtask budget (FND-0185); empty and mismatch states (FND-0186); sweep limited to presence (FND-0187, info); fixture username (FND-0188, info).
Cross-references: FND-0092, FND-0093, FND-0094, FND-0095, FND-0099, FND-0100; FND-0168 contrast (this file resets mocks, unlike App.test.ts).
UNVERIFIED: whether the suite passes (vitest not run); exact microtask margin for flush; jsdom delivery of click to disabled buttons; vi.resetAllMocks semantics in vitest 5.1.0 (no impact here).

STATUS: COMPLETE
(Recorded by the orchestrator from the auditor's read-only report.)
