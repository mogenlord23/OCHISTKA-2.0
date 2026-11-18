# Fix Record: FND-0180, FND-0181, FND-0182, FND-0183, FND-0185, FND-0186

**Fix unit**: U13 (test-only), plus behaviour tests for U47 (FND-0093, FND-0094, FND-0097)
**File**: apps/desktop/src/components/VirusesPage.test.ts (VirusesPage.svelte not changed)

## Tests

**Framework**: vitest + @testing-library/svelte (jsdom)
**Test file**: apps/desktop/src/components/VirusesPage.test.ts (rewritten/extended; the two existing tests kept, the first now also asserts reason/hint)

FND-0185: the fixed five-microtask `flush()` and the `tick` import are removed. All waiting uses `findBy*` / `waitFor` on the expected text or button state (helper `renderReady()` waits for the start button to be enabled). No fixed count remains.

### Test names
VirusesPage scan
- FND-0182: blocks scanning and shows the engine reason and install hint
- shows the sweep while scanning, then the result and suspicion score
- FND-0181: shows an alert when clamavStatus rejects
- FND-0094: scanStart rejecting shows the error message, not silence
- FND-0181/FND-0094: assessFile failing for one finding shows a notice and the others still score
- FND-0183: while scanning the button and input are disabled and scanStart is called once
- FND-0183: empty or whitespace target does not call scanStart
- FND-0097: shows the visible reason when the engine is unavailable
- FND-0097: shows the visible reason when the target is empty, and hides it when valid

VirusesPage findings
- FND-0186: shows the no-report state before any scan
- FND-0186: shows "Угроз не найдено" for an empty findings list
- FND-0186: meter aria-valuenow and the reason text under the score
- FND-0186: shows a fallback when the score has no reasons

VirusesPage journal
- FND-0180: shows the empty state before any scan
- FND-0180: a completed scan appears in the journal list
- FND-0093: two runs of the same target in the same second both render (no each_key_duplicate) (Date frozen; console.error spy asserted not called)

### Results
- npx prettier --write: applied
- npm test -w apps/desktop: PASS (11 files, 78 tests; VirusesPage 16)
- npm run check -w apps/desktop: PASS (0 errors, 0 warnings)

No `it.fails` / `it.skip` were needed: no test exposed missing production behaviour (no U05 / U02 blockers hit). The duplicate-path test (FND-0092) was intentionally not added (deferred).
Not verified by mutation against the original svelte code (production file must not be touched).
Note: mock errors must use a valid ErrorKind (probe, invalidTarget, ...), otherwise toCommandError yields "[object Object]".

**test_status**: passing
