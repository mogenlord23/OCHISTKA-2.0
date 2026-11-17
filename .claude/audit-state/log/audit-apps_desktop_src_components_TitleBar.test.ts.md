File: apps/desktop/src/components/TitleBar.test.ts
Declared Lines: 38
Lines Inspected: 38 (module under test TitleBar.svelte, 71 lines, also read)
Functions Analyzed: 2 tests, 1 hoisted mock, 1 afterEach

Execution Traces: window mock is hoisted and shared; afterEach cleanup and mockClear isolate tests. Name and author text queries match TitleBar.svelte:9-10. Aria-label queries match TitleBar.svelte:13-15. Click counts would be 0 if wiring were removed (not a false pass for removal), but handler swapping is not detected (FND-0176). Drag region attributes are untested (FND-0177). Mock returns undefined, limiting FND-0116 testing (FND-0178). toBeTruthy is redundant (FND-0179). Cross-references: FND-0115, FND-0116, FND-0167 (App.test.ts duplication, not re-filed).
UNVERIFIED: Tauri drag behaviour on child elements (Tauri drag script not read); vitest not run; fireEvent and getByRole internals assumed standard.

STATUS: COMPLETE
(Recorded by the orchestrator from the auditor's read-only report.)
