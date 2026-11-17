File: apps/desktop/src/App.test.ts
Declared Lines: 62
Lines Inspected: 62
Functions Analyzed: 5 tests, 3 mock blocks, 1 afterEach

Execution Traces: mocks match api.ts and types.ts (window controls, getSystemOverview, clamavStatus, defaultScanTarget). Five tests traced to App.svelte: rail labels and count (pinned with sections.test.ts), VPN default h1 and subtabs, Вирусы navigation resets h1 and subtab presence, Процессы falls to PlaceholderPage, window control presence. Findings: routing not asserted (FND-0163, low); activeSub reset not asserted (FND-0164, low); sound toggle, RU and ENG pills and Subtabs onSelect untested (FND-0165); dead getSystemOverview mock (FND-0166); window test duplicates TitleBar.test.ts (FND-0167); no mock reset and un-awaited onMount (FND-0168); weak section and subtab assertions (FND-0169).
Cross-references: FND-0020, FND-0022, FND-0079, FND-0108, FND-0111, FND-0113, FND-0115, FND-0125, FND-0129 (not re-filed).
UNVERIFIED: runtime behaviour of fireEvent and cleanup with Svelte 5 async onMount (vitest not run); VirusesPage beyond line 48 (only branch lines confirmed by grep); jsdom accessible-name calculation taken from WAI-ARIA rules, not run.

STATUS: COMPLETE
(Recorded by the orchestrator from the auditor's read-only report.)
