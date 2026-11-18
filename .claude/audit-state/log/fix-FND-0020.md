# Fix Record: FND-0020

**Date**: 2026-11-18
**Fix unit**: single-file (U22)
**Findings**: FND-0020

## Original Finding

The bottom-bar ЗВУК toggle flipped a `soundOn` $state label and did nothing else. It was enabled and not persisted, and no audio code exists. This breaks the rule that unavailable features are shown disabled with a reason. Owner decision D12: sound is not planned now, so disable it with a visible reason.

**Snippet (before):**
```svelte
  let soundOn = $state(true);
  ...
        <button
          type="button"
          class="pill"
          onclick={() => (soundOn = !soundOn)}
          aria-pressed={soundOn}
        >
          ЗВУК: {soundOn ? 'ВКЛ' : 'ВЫКЛ'}
        </button>
```

## Change Plan

- Change: ЗВУК becomes a native disabled `<button>` with no handler and no state. The reason «в разработке» is visible text on a second line and also in `title`.
- Not changing: the focus-ring rule (U06), `--font-body` (U23), the ENG button and its `.pill:disabled` opacity rule, and the `.pill` base styles.
- Risk: low. The label is two lines because the side panel is 124px wide and a one-line «ЗВУК: В РАЗРАБОТКЕ» would not fit. The `.pill.sound:disabled` rule overrides the generic `opacity: 0.4`, which would drop the text below 4.5:1.

## Edits Applied

### apps/desktop/src/App.svelte (script, line 16)

**Before:** `let soundOn = $state(true);`
**After:** line removed.

### apps/desktop/src/App.svelte (markup, bottom bar)

**After:**
```svelte
        <button type="button" class="pill sound" disabled title="Звук: в разработке">
          ЗВУК
          <span class="reason">в разработке</span>
        </button>
```

### apps/desktop/src/App.svelte (style)

**After (added before `.lang`):**
```css
  .pill.sound:disabled {
    opacity: 1;
    background: var(--surface);
    color: var(--muted);
  }

  .reason {
    display: block;
    font-size: var(--text-caption);
    font-weight: 700;
    text-transform: none;
  }
```

Rationale: this removes the dead state and handler and shows the unavailability as visible text plus a tooltip. Contrast of `--muted` (#8b8b8b) on `--surface` (#141414) is about 5.4:1 by WCAG (luminances about 0.258 and 0.0070), which is at least 4.5:1. No new hex literals were added, only the existing CSS variables `--surface`, `--muted` and `--text-caption`.

## Sanity Checks

- `npm run check -w apps/desktop`: PASS (0 errors, 0 warnings)
- `npm test -w apps/desktop`: PASS (10 files, 49 tests). No existing test referenced the old label or behaviour (grep for `ЗВУК`, `soundOn` found nothing outside App.svelte), so no assertion was changed.
- Visual and layout check in a real window: NOT DONE (no GUI here). The two-line fit in the 124px panel is reasoned from the width, not observed.

## Next

Handing off to @test-engineer for regression test + @re-verifier for independent trace.

## Tests

**Framework**: vitest 5.1 + @testing-library/svelte (jsdom)
**Test file**: apps/desktop/src/App.test.ts (added a new `describe`; existing tests untouched)

### Regression tests (new `describe('FND-0020: sound control is disabled with a visible reason')`)

1. `FND-0020: the sound button is disabled` - `getByRole('button', { name: /ЗВУК/ })` has `disabled === true`.
2. `FND-0020: the reason is visible text inside the button and in its title` - `within(button).getByText('в разработке')` exists and `title` contains `в разработке`.
3. `FND-0020: clicking the sound button changes nothing (no fake toggle)` - no `aria-pressed` before or after `fireEvent.click`; `textContent` unchanged; text has no ВКЛ/ВЫКЛ; reason still present.
4. `FND-0020: the ENG pill is still disabled` - asserted only if a button named /ENG/ exists (it does).

### Results

- Phase A (new tests on fixed code): PASS (4 new tests)
- Phase A (new tests on original code): not run (production code must not be edited). By inspection the original was an enabled button with `aria-pressed`, label "ЗВУК: ВКЛ" and no reason text, so tests 1, 2 and 3 would fail. Test 4 passes on both (guard for unchanged ENG behaviour). Mutation proof to be run by the caller.
- Phase B (full desktop suite): PASS (10 files, 53 tests)
- `npm run check -w apps/desktop`: PASS (0 errors, 0 warnings)
- `npx prettier --write apps/desktop/src/App.test.ts`: unchanged (already formatted)

Commands: `npm test -w apps/desktop`; `npm run check -w apps/desktop`; `npx prettier --write apps/desktop/src/App.test.ts`

Limitation: jsdom does not dispatch click handlers on a disabled button, so test 3 alone would not catch "toggle state re-added while `disabled` kept"; test 1 and the aria-pressed/label assertions cover removal of `disabled`. Visual two-line fit in 124px is not tested (no GUI).

**test_status**: `passing`

## Mutation proof (orchestrator, 2026-11-18; harness: baseline run must be green, then mutate, then `npm test -w apps/desktop -- App.test`)
- Remove `disabled`: "the sound button is disabled" FAILS (killed).
- Drop the visible reason span: "reason is visible text..." and "clicking changes nothing" FAIL (killed).
- Drop `title`: "reason is visible text inside the button and in its title" FAILS (killed).
- Fake-toggle label/aria-pressed returns: "clicking changes nothing (no fake toggle)" FAILS (killed).

## Re-verification (verdict recorded by orchestrator; re-verifier tools are read-only)
Verdict: CONFIRM. No `soundOn` left; native `disabled` button with visible reason (display:block span) and title; muted #8b8b8b on surface #141414 = 5.41:1 (>= 4.5:1); `.pill.sound:disabled` (0,3,0) beats `.pill:disabled` (0,2,0) and comes later; ENG keeps opacity .4. Not observed: the two-line label fit in the 124px rail (reasoned: ~96px of text width vs ~66-72px needed). Weakness noted: jsdom does not dispatch clicks on disabled buttons, so the click test alone would not catch a re-added toggle; the disabled/aria-pressed/label assertions cover it.
