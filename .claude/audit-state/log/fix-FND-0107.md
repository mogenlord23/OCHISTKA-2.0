# Fix Record: FND-0107 through FND-0108

**Date**: 2026-11-18T05:53:13.068255+00:00
**Fix unit**: U46 (single-file)
**Findings**: FND-0107, FND-0108

## Original Finding

- FND-0107: at the 600px minimum height the rail, bottom controls and title bar total about 733px (estimated), so the page scrolls and the title bar leaves the screen.
- FND-0108: `<nav aria-label="Навигация">` was nested in `<aside aria-label="Разделы приложения">`.

**Snippet (before):**
```svelte
<aside class="rail" aria-label="Разделы приложения">
  <nav aria-label="Навигация">
    ...
  </nav>
</aside>
.item { gap: 6px; padding: 12px 4px; }  svg { width: 26px; height: 26px; }
```

## Change Plan

- Change: drop the aside, make the rail itself `<nav class="rail" aria-label="Навигация">`; shrink item spacing (padding 12 to 6, gap 6 to 2, rail gap 4, icon 26 to 22) and add `min-height: 0; overflow-y: auto` on the rail with `flex-shrink: 0` on items.
- Not changing: sound button, focus ring, font, section data, App.svelte.
- Risk: label sizes unchanged; each item is about 52px tall (6+22+2+about 16+6), above the 44px target. Estimated rail about 7*52+6*4+8 = 396px, so total about 396+117+68 = 581px, under 600.

## Edits Applied

### apps/desktop/src/components/Navigation.svelte

Markup: `aside` + inner `nav` replaced by a single `nav.rail` (accessible name "Навигация", still matched by App.test.ts). CSS: the `nav` rule merged into `.rail`; item and icon sizes reduced as above.

Rationale: single navigation landmark (FND-0108); compact rail fits at 600px (FND-0107).

Limitation: `.shell` in App.svelte uses `min-height: 100vh`, so the column is not height-bounded and the rail's `overflow-y: auto` is only a safety net. The fix relies on the reduced spacing. App.svelte was not touched.

## Sanity Checks

- `npm run check -w apps/desktop`: PASS (0 errors, 0 warnings)
- `npm test -w apps/desktop`: PASS (11 files, 58 tests)

## Verification status

The 600px height check on real Windows is UNVERIFIED (Linux environment, heights estimated, not rendered).

## Next

Handing off to @test-engineer and @re-verifier.

## Tests

**Framework**: vitest + jsdom (apps/desktop)
**Test file**: none added (height/scroll is not testable in jsdom; production code and tests unchanged)

### Confirmed by reading `apps/desktop/src/components/Navigation.svelte`

- Markup is a single `<nav class="rail" aria-label="Навигация">` (line 24); there is no wrapping `<aside>` (FND-0108).
- `.rail` has `min-height: 0; overflow-y: auto`; `.item` has `flex-shrink: 0`, padding 6px 4px, gap 2px; icons 22px (FND-0107).
- `apps/desktop/src/App.test.ts:26` still queries `screen.getByRole('navigation', { name: 'Навигация' })`.

### Commands run

- `npm test -w apps/desktop`: PASS (64 tests passed, 64 total; the fix log said 58, the count has grown since, no failures)

### Manual Windows check (FND-0107)

1. Build and run the app on Windows 10/11 x64 (`npm run tauri dev` or a release build).
2. Resize the window to 600px height (the minimum).
3. Expect the custom title bar to stay on screen and the page not to scroll vertically.
4. Expect all 7 rail items and the bottom controls (sound button) visible, with the rail not clipped (no cut-off item).
5. Repeat at 100%, 125% and 150% display scaling. Note: `.shell` uses `min-height: 100vh`, so `overflow-y: auto` on the rail is only a safety net; if it scrolls at 125%+, report it.
6. Check with a screen reader or devtools accessibility tree: one navigation landmark named "Навигация", no "Разделы приложения" region.

**test_status**: `manual`  (applies to FND-0107 and FND-0108; FND-0108 is also covered by the existing App.test.ts query)
