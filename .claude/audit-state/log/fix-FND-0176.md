# Fix Record: FND-0176 through FND-0177

**Date**: 2026-11-18T05:51:05Z
**Fix unit**: single-file (U25, test-only)
**Findings**: FND-0176, FND-0177

## Original Finding

- FND-0176: window-control click counts were asserted only after all three clicks, so handlers wired to the wrong buttons (minimise bound to close) still passed.
- FND-0177: no test covered the `data-tauri-drag-region` attributes, the only way to move the frameless window.

**Snippet (before):**
```ts
  it('minimises, maximises and closes the window', async () => {
    render(TitleBar);
    await fireEvent.click(screen.getByRole('button', { name: 'Свернуть' }));
    await fireEvent.click(screen.getByRole('button', { name: 'Развернуть' }));
    await fireEvent.click(screen.getByRole('button', { name: 'Закрыть' }));
    expect(win.minimize).toHaveBeenCalledTimes(1);
    expect(win.toggleMaximize).toHaveBeenCalledTimes(1);
    expect(win.close).toHaveBeenCalledTimes(1);
  });
```

## Change Plan

- Change: assert all three call counts after each individual click; add a test for the drag-region attributes on header, .brand, OCHISTKA and author text, and their absence on the three buttons.
- Not changing: TitleBar.svelte (production), existing first test (including its toBeTruthy lines), mocks, afterEach.
- Risk: none for production; test-only.

## Edits Applied

### apps/desktop/src/components/TitleBar.test.ts:29-60

**Before:** see snippet above.

**After:**
```ts
  it('minimises, maximises and closes the window', async () => {
    render(TitleBar);
    await fireEvent.click(screen.getByRole('button', { name: 'Свернуть' }));
    expect(win.minimize).toHaveBeenCalledTimes(1);
    expect(win.toggleMaximize).toHaveBeenCalledTimes(0);
    expect(win.close).toHaveBeenCalledTimes(0);

    await fireEvent.click(screen.getByRole('button', { name: 'Развернуть' }));
    expect(win.minimize).toHaveBeenCalledTimes(1);
    expect(win.toggleMaximize).toHaveBeenCalledTimes(1);
    expect(win.close).toHaveBeenCalledTimes(0);

    await fireEvent.click(screen.getByRole('button', { name: 'Закрыть' }));
    expect(win.minimize).toHaveBeenCalledTimes(1);
    expect(win.toggleMaximize).toHaveBeenCalledTimes(1);
    expect(win.close).toHaveBeenCalledTimes(1);
  });

  it('lets the frameless window be dragged by the bar and brand, but not by the buttons', () => {
    const { container } = render(TitleBar);
    const header = container.querySelector('header');
    const name = screen.getByText('OCHISTKA');
    const author = screen.getByText('Create: Slava Lavs');
    const brand = name.parentElement;

    expect(header?.hasAttribute('data-tauri-drag-region')).toBe(true);
    expect(brand?.classList.contains('brand')).toBe(true);
    expect(brand?.hasAttribute('data-tauri-drag-region')).toBe(true);
    expect(name.hasAttribute('data-tauri-drag-region')).toBe(true);
    expect(author.hasAttribute('data-tauri-drag-region')).toBe(true);

    for (const label of ['Свернуть', 'Развернуть', 'Закрыть']) {
      const button = screen.getByRole('button', { name: label });
      expect(button.hasAttribute('data-tauri-drag-region')).toBe(false);
    }
  });
```

Rationale: per-click assertions make a swapped handler fail at the first wrong click; the drag test pins exactly the attributes TitleBar.svelte lines 7-10 declare and forbids them on buttons.

## Sanity Checks

- npm test -w apps/desktop: PASS (11 files, 58 tests)
- npm run check -w apps/desktop (svelte-check, fail-on-warnings): PASS (0 errors, 0 warnings)
- npx prettier --check TitleBar.test.ts: PASS
- Manual mutation checks (component temporarily mutated, then restored byte-identical): swapped minimise/close handlers -> click test FAILS; removed drag attribute from header -> drag test FAILS; removed from small -> FAILS; added to a button -> FAILS.

## Next

Handing off to @re-verifier for independent trace.

## Mutation proof (orchestrator, 2026-11-18; green baseline, `npm test -w apps/desktop -- TitleBar`)
- Swap minimise and close handlers: "minimises, maximises and closes the window" FAILS (killed).
- Drop drag region from header: "lets the frameless window be dragged..." FAILS (killed).
- Drag region added to a control button: same drag test FAILS (killed).

## Re-verification (verdict recorded by orchestrator)
Verdict: CONFIRM for FND-0176 and FND-0177. Per-click counts match the real handlers (1/0/0, 1/1/0, 1/1/1); drag attributes match TitleBar.svelte lines 7-10; buttons assert absence. Isolation via afterEach cleanup and mockClear holds.
