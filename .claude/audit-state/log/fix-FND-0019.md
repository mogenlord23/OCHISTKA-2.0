# Fix Record: FND-0019

**Date**: 2026-11-18T05:41:15Z
**Fix unit**: single-file (U06)
**Findings**: FND-0019

## Original Finding

Focus ring was `outline: 3px solid var(--page-text)`. `--page-text` = `textOn(accent)` = `#080808`, the same colour as the right rail background (`#080808`), so the ring is 1:1 on the rail (WCAG 2.4.7, 1.4.11). The comment claiming text-like contrast was wrong for the rail.

**Snippet (before):**
```css
  /* Видимый фокус с клавиатуры: цвет текста страницы, контраст как у текста. */
  :global(button:focus-visible, input:focus-visible) {
    outline: 3px solid var(--page-text);
    outline-offset: 2px;
  }
```

## Change Plan

- Change: add a second, light ring `box-shadow: 0 0 0 2px var(--ink)` (fills the 2px outline-offset gap) to the existing dark outline, giving a two-tone ring.
- Not changing: the existing outline (still visible on the coloured page), the sound toggle, font variables, tokens.ts (no new token needed: `--ink` = `NEUTRAL.ink` `#fafafa` is already exposed in `.shell`).
- Risk: box-shadow on focused buttons; no other component sets box-shadow on focus (grep for focus/outline found none). Inner ring is 2px wide, ink vs rail `#080808` is about 19:1 (>= 3:1).

## Edits Applied

### apps/desktop/src/App.svelte:117-123

**Before:**
```css
  /* Видимый фокус с клавиатуры: цвет текста страницы, контраст как у текста. */
  :global(button:focus-visible, input:focus-visible) {
    outline: 3px solid var(--page-text);
    outline-offset: 2px;
  }
```

**After:**
```css
  /* Видимый фокус с клавиатуры, двойное кольцо. Внешнее (цвет текста страницы) читается
     на цветной странице, но на правой панели сливается с её фоном (оба #080808).
     Внутреннее светлое (--ink, 2px, ровно на величину отступа) читается на панели. */
  :global(button:focus-visible, input:focus-visible) {
    outline: 3px solid var(--page-text);
    outline-offset: 2px;
    box-shadow: 0 0 0 2px var(--ink);
  }
```

Rationale: on the rail the light inner ring gives >= 3:1 against the rail; on the coloured page the dark outer ring (as before) plus the light inner ring remain visible. Comment corrected.

## Sanity Checks

- `npm run check -w apps/desktop`: PASS (0 errors, 0 warnings)
- `npm test -w apps/desktop`: PASS (9 files, 45 tests)
- Format: not-applicable

## Next

Handing off to @test-engineer for regression test + @re-verifier for independent trace.
Note: the comment contains a literal `#080808` in a CSS comment (not a code literal).

## Tests

**Framework**: vitest 5.1.0 (jsdom) + @testing-library/svelte
**Test file**: apps/desktop/src/focus-ring.test.ts (created; no other test file touched)

Why the source is parsed: vitest/jsdom does not apply Svelte component CSS, so `getComputedStyle` cannot see the rule. The test imports `App.svelte?raw` (types come from `vite/client`, no new `declare module` needed), extracts the `:global(button:focus-visible, input:focus-visible) { ... }` block, strips CSS comments, and asserts on the declarations. Colours are checked via real tokens, and the `--ink` variable is checked on the rendered `.shell`.

### Regression tests

- `FND-0019: the focus-visible rule declares an outline`
- `FND-0019: the focus-visible rule adds a light second ring via box-shadow using var(--ink)` (also requires the solid-ring form `0 0 0 <N>px var(--ink)`)
- `FND-0019: the inner ring colour has >= 3:1 contrast against the rail background (WCAG 1.4.11)` (`contrastRatio(NEUTRAL.ink, NEUTRAL.black) >= 3`)
- `FND-0019: the .shell element sets --ink to NEUTRAL.ink so the ring colour is defined` (renders App, reads inline style)

### Commands run

- `npm test -w apps/desktop`
- `npm run check -w apps/desktop`
- `npx prettier --write apps/desktop/src/focus-ring.test.ts` (formatting of the new test file only)

### Results

- Phase A (new tests on fixed code): PASS (4 new tests)
- Phase A (new tests on original code): by inspection, the box-shadow tests FAIL on the original snippet (it had no `box-shadow` declaration, so the `box-shadow` match is null). The outline, contrast and `--ink` tests pass on both, as they guard preconditions. Mutation proof to be run by the orchestrator (production code not mutated by test-engineer).
- Phase B (full desktop suite, existing tests): PASS (10 files, 49 tests; was 9 files / 45 tests before)
- `npm run check -w apps/desktop`: PASS (0 errors, 0 warnings, 389 files)

**test_status**: `passing`

## Mutation proof (orchestrator, 2026-11-18)
- Remove `box-shadow` line from App.svelte: focus-ring.test.ts FAILS (killed).
- Change box-shadow colour to `var(--page-text)`: FAILS (killed).
- Set `--ink` to `NEUTRAL.black` on `.shell`: `.shell sets --ink` test FAILS (killed).

## Re-verification (verdict recorded by orchestrator; re-verifier tools are read-only)
Verdict: CONFIRM. Original path (outline #080808 on rail #080808, 1:1) no longer reaches the bad state: the 2px `--ink` (#fafafa) box-shadow fills the outline-offset gap and gives about 19:1 on the rail. No clipping (rail padding 8px, ring reaches 5px; no overflow on rail/side/body/shell). No other outline or box-shadow rules override it. Not run by the re-verifier: tests/build (fix log results relied on).
Residual (low): on `.pill[aria-pressed='true']` (background #fafafa) the inner ring merges with the pill, so focus reads as a larger pill rather than a distinct ring; still 19:1 so WCAG 2.4.7 and 1.4.11 are met. Tracked as FND-0210.
