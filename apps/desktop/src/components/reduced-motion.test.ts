import { describe, expect, it } from 'vitest';
import pageSource from './VirusesPage.svelte?raw';

// jsdom does not apply Svelte component CSS, so the rule is checked in the source text.
// Plan 2026-11-18-design-v2, Review Focus 2: the sweep animation is switched off under
// prefers-reduced-motion, and the state change is still shown as text.
describe('VirusesPage reduced motion', () => {
  const css = pageSource.slice(pageSource.indexOf('<style>'));

  it('turns the sweep animation off under prefers-reduced-motion: reduce', () => {
    const match = css.match(/@media \(prefers-reduced-motion: reduce\)\s*\{([\s\S]*?)\n\s{2}\}/);
    expect(match).not.toBeNull();
    const block = match?.[1] ?? '';
    expect(block).toMatch(/\.sweep\s*\{[^}]*animation:\s*none/);
  });

  it('keeps the scanning state visible as text, not only as motion', () => {
    expect(pageSource).toContain('Проверяем файлы');
  });
});

describe('VpnPage reduced motion', () => {
  it('turns the tab-switch animation off under prefers-reduced-motion: reduce', async () => {
    const source = (await import('./VpnPage.svelte?raw')).default;
    const css = source.slice(source.indexOf('<style>'));
    const match = css.match(/@media \(prefers-reduced-motion: reduce\)\s*\{([\s\S]*?)\n\s{2}\}/);
    expect(match?.[1] ?? '').toMatch(/\.vpn\s*\{[^}]*animation:\s*none/);
  });
});
