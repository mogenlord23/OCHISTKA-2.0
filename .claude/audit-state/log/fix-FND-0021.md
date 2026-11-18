# Fix Record: FND-0021

**Date**: 2026-11-18
**Fix unit**: U23 (single-file)
**Findings**: FND-0021

## Original Finding

`--font-body` is set via `style:--font-body={BODY_FONT}` on `.shell` (App.svelte line 30). The rule that reads it, `font-family: var(--font-body, system-ui, sans-serif)`, was on `:global(body)`. `body` is an ancestor of `.shell`, and custom properties inherit downward only, so `body` never saw the variable. `BODY_FONT` had no effect and the fallback was always used.

**Snippet (before):**
```css
  :global(body) {
    margin: 0;
    font-family: var(--font-body, system-ui, sans-serif);
    background: #080808;
    color: #fafafa;
  }

  .shell {
    display: flex;
    flex-direction: column;
    min-height: 100vh;
    background-color: var(--accent);
```

## Change Plan

- Change: move the `font-family: var(--font-body, system-ui, sans-serif)` declaration from `:global(body)` to `.shell`, the same element that defines `--font-body`. Descendants inherit the font from there.
- Not changing: other variables, `HEADING_FONT` handling (`h1` uses `--font-heading`), focus ring rule, sound button, the fallback stack.
- Risk: low. `.shell` is `min-height: 100vh` and wraps all app content, so nothing visible sits outside it. Buttons use `font: inherit` and now inherit BODY_FONT (Segoe UI) rather than system-ui. This is the intended effect.

## Edits Applied

### apps/desktop/src/App.svelte (style block, lines ~97-109)

**Before:**
```css
    margin: 0;
    font-family: var(--font-body, system-ui, sans-serif);
    background: #080808;
```
```css
    min-height: 100vh;
    background-color: var(--accent);
```

**After:**
```css
    margin: 0;
    background: #080808;
```
```css
    min-height: 100vh;
    font-family: var(--font-body, system-ui, sans-serif);
    background-color: var(--accent);
```

Rationale: the declaration now sits on the element where `--font-body` is defined, so `var()` resolves to `BODY_FONT`. The fallback is kept.

## Sanity Checks

- `npm run check -w apps/desktop`: PASS (389 files, 0 errors, 0 warnings)
- `npm test -w apps/desktop`: PASS (10 files, 53 tests)
- Lint: NOT CONFIGURED separately
- Visual check on Windows: not performed (Linux environment)

## Next

Handing off to @test-engineer for regression test + @re-verifier for independent trace.

## Tests

**Framework**: vitest 5.1.0 (jsdom) + @testing-library/svelte
**Test file**: apps/desktop/src/font-scope.test.ts (created; Prettier-formatted)

jsdom does not apply Svelte component CSS, so (same technique as focus-ring.test.ts) the `<style>` block of App.svelte is read via `?raw`, comments are stripped, and flat rules are parsed (`:global()` unwrapped, selector lists split on top-level commas). Runtime check renders `App` with the same Tauri/api mocks as focus-ring.test.ts.

### Regression tests
1. `FND-0021: the .shell rule declares font-family using var(--font-body`
2. `FND-0021: no body/html/:root rule reads var(--font-body (it can never reach them)` (also asserts the parser found at least one body/html/:root rule, so it cannot pass vacuously)
3. `FND-0021: at runtime the .shell element has --font-body set to BODY_FONT`
4. `FND-0021: no body/html/:root rule reads any variable that is set on .shell (whole bug class)` (list of `style:--name=` vars parsed from the `.shell` tag; asserts it contains `--font-body` and has more than 5 entries)

### Results

- Phase A (new file, fixed code): PASS (4/4)
- Phase A on original code: verified by mutation on a scratch copy of apps/desktop/src (production files untouched, copy deleted):
  - M1 font-family moved back to `:global(body)` (original bug): FAIL tests 1, 2, 4 (3 failed)
  - M2 font-family deleted from `.shell`: FAIL test 1
  - M3 `style:--font-body={HEADING_FONT}`: FAIL test 3
  - M4 `:global(body)` reads another `.shell` var (`var(--radius-sm)`): FAIL test 4
  - M5 added `:global(html)` rule reading `var(--font-body)`: FAIL tests 2, 4
- Phase B (full desktop suite, 11 files, fast): PASS, 57 tests (53 existing + 4 new)
- `npm run check -w apps/desktop`: PASS (390 files, 0 errors, 0 warnings)

Commands:
- `npx prettier --write apps/desktop/src/font-scope.test.ts`
- `npm test -w apps/desktop`
- `npm run check -w apps/desktop`

Limitation: tests assert the CSS invariant from source text and the inline style at runtime. The actual rendered font on Windows was not verified (Linux, jsdom has no component CSS).

**test_status**: `passing`

## Mutation proof (orchestrator, 2026-11-18; green baseline required, run via `npm test -w apps/desktop -- font-scope`)
- font-family moved back onto `:global(body)`: "no body/html/:root rule reads var(--font-body" and "...reads any variable that .shell sets" FAIL (killed).
- font-family deleted from `.shell`: "the .shell rule declares font-family" FAILS (killed).
- `--font-body` rebound to HEADING_FONT: "at runtime the .shell element has --font-body" FAILS (killed).

## Re-verification (verdict recorded by orchestrator; re-verifier tools are read-only)
Verdict: CONFIRM. Reader and definer of --font-body are now the same element; no text renders outside `.shell` (index.html has only `#app`, main.ts mounts only App); headings unaffected (`--font-heading`); body keeps margin/background/colour; no other ancestor-reads-descendant-variable case. Side effect: buttons now Segoe UI (intended). Not observed: real rendering on Windows (jsdom has no component CSS).
