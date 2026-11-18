# Fix Record: FND-0092

**Date**: 2026-11-18
**Fix unit**: single-file
**Findings**: FND-0092

## Original Finding

`{#each scan.report.findings as finding (finding.path)}` in VirusesPage.svelte throws `each_key_duplicate` when two findings share a path; nothing contains the error, so the Находки view breaks.

**Snippet (before):**
```svelte
    <ul class="findings">
      {#each scan.report.findings as finding (finding.path)}
        {@const score = scan.suspicions[finding.path]}
        <li>
```

## Change Plan

- Change: key each row by `` `${finding.path}#${index}` `` so keys are unique per row.
- Not changing: displayed content, order, `scan.suspicions[finding.path]` lookup (still by path), dedup of the report.
- Risk: none visible; rows are re-created only if list order changes (list is static per report).

## Edits Applied

### apps/desktop/src/components/VirusesPage.svelte (each line, ~112)

**Before:** `{#each scan.report.findings as finding (finding.path)}`

**After:** `` {#each scan.report.findings as finding, index (`${finding.path}#${index}`)} ``

### apps/desktop/src/components/VirusesPage.test.ts

Added test `FND-0092: two findings with the same path both render (no each_key_duplicate)` in the `VirusesPage findings` describe. It scans with a report holding the same path twice (signatures Win.Dup.A / Win.Dup.B), opens Находки, asserts both rows and both path texts render and `console.error` is not called.

Would FAIL with the old keying: duplicate keys make Svelte throw `each_key_duplicate` during render of the findings subsection (rerender rejects / rows missing, and the error is reported). Confirmed by reasoning only; the orchestrator runs the mutation.

## Sanity Checks

- `npx prettier --write apps/desktop/src/components/VirusesPage.test.ts`: unchanged (already formatted)
- `npm test -w apps/desktop -- VirusesPage`: PASS, 20/20
- `npm test -w apps/desktop`: PASS, 11 files, 89 tests
- `npm run check -w apps/desktop`: PASS, 0 errors, 0 warnings

## Next

Handing off to @re-verifier.

## Re-verification (verdict recorded by orchestrator)
Verdict: CONFIRM. Key path#index is unique per row; rows re-created on update, which is harmless here (no local state). Scores keyed by path are shared by duplicates by design and correct.
