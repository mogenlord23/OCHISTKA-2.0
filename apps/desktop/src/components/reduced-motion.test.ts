import { describe, expect, it } from 'vitest';
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

describe('VpnPage reduced motion', () => {
  it('turns the tab-switch animation off under prefers-reduced-motion: reduce', async () => {
    const source = (await import('./VpnPage.svelte?raw')).default;
    const css = source.slice(source.indexOf('<style>'));
    const match = css.match(/@media \(prefers-reduced-motion: reduce\)\s*\{([\s\S]*?)\n\s{2}\}/);
    expect(match?.[1] ?? '').toMatch(/\.pane\s*\{[^}]*animation:\s*none/);
  });
});
