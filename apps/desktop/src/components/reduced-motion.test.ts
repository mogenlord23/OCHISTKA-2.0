import { describe, expect, it, vi } from 'vitest';
import { slideInOnChange } from '../lib/slideInOnChange';
import pageSource from './VirusesPage.svelte?raw';

// jsdom does not apply Svelte component CSS, so the rule is checked in the source text.
// Plan 2026-11-18-design-v2, Review Focus 2: the progress transition is switched off under
// prefers-reduced-motion, and the state change is still shown as text.
describe('VirusesPage reduced motion', () => {
  const css = pageSource.slice(pageSource.indexOf('<style>'));

  it('disables scan transitions for system and in-app motion preferences', () => {
    const match = css.match(/@media \(prefers-reduced-motion: reduce\)\s*\{([\s\S]*?)\n  \}/);
    expect(match?.[1]).toMatch(/\.sweep,\s*\.chevron,\s*\.folder-reveal\s*\{[^}]*transition:\s*none/);
    expect(css).toMatch(/:root\[data-reduce-motion='true'\]\)\s*\.sweep,[\s\S]*?transition:\s*none/);
  });

  it('keeps the scanning state visible as text, not only as motion', () => {
    expect(pageSource).toContain('Подготавливаем список файлов');
    expect(pageSource).toContain('Идёт проверка');
  });
});

it('slides the next section and VPN pane without changing their layout', async () => {
  const [app, vpn] = await Promise.all([import('../App.svelte?raw'), import('./VpnPage.svelte?raw')]);
  expect(app.default).toMatch(/<main[\s\S]*?use:slideInOnChange/);
  expect(vpn.default).toMatch(/class="pane-slot" use:slideInOnChange/);
});

it('restarts a running slide on a fast change and skips it in reduced-motion mode', () => {
  const cancel = vi.fn();
  const animate = vi.fn(() => ({ cancel }));
  const node = Object.assign(document.createElement('div'), { animate });
  const slide = slideInOnChange(node, 'vpn');
  try {
    slide.update('vpn');
    expect(animate).not.toHaveBeenCalled();
    slide.update('viruses');
    slide.update('design');
    expect(cancel).toHaveBeenCalledOnce();
    expect(animate).toHaveBeenCalledTimes(2);
    expect(animate).toHaveBeenLastCalledWith(
      [expect.objectContaining({ transform: 'translateY(-8px)' }), expect.objectContaining({ transform: 'none' })],
      expect.any(Object),
    );

    document.documentElement.dataset.reduceMotion = 'true';
    slide.update('computer');
    expect(cancel).toHaveBeenCalledTimes(2);
    expect(animate).toHaveBeenCalledTimes(2);
  } finally {
    delete document.documentElement.dataset.reduceMotion;
    slide.destroy();
  }
});
