import { describe, expect, it } from 'vitest';
import pageSource from './VirusesPage.svelte?raw';

// jsdom does not apply Svelte component CSS, so the rule is checked in the source text.
// Plan 2026-11-18-design-v2, Review Focus 2: the progress transition is switched off under
// prefers-reduced-motion, and the state change is still shown as text.
describe('VirusesPage reduced motion', () => {
  const css = pageSource.slice(pageSource.indexOf('<style>'));

  it('turns the progress transition off under prefers-reduced-motion: reduce', () => {
    const match = css.match(/@media \(prefers-reduced-motion: reduce\)\s*\{([\s\S]*?)\n\s{2}\}/);
    expect(match).not.toBeNull();
    const block = match?.[1] ?? '';
    expect(block).toMatch(/\.sweep\s*\{[^}]*transition:\s*none/);
  });

  it('keeps the scanning state visible as text, not only as motion', () => {
    expect(pageSource).toContain('Подготавливаем список файлов');
    expect(pageSource).toContain('Обработано файлов');
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

describe('VirusesPage scan screen layout (round 28)', () => {
  it('uses two tokenised columns that stack at 980px and tighten at 720px', () => {
    const css = pageSource.slice(pageSource.indexOf('<style>'));
    expect(css).toMatch(
      /\.scan-layout\s*\{[^}]*grid-template-columns:\s*minmax\(0,\s*1fr\)\s+minmax\(0,\s*1fr\)/,
    );
    expect(css).toMatch(
      /@media \(max-width: 980px\)\s*\{[\s\S]*?grid-template-columns:\s*minmax\(0,\s*1fr\)/,
    );
    expect(css).toMatch(/@media \(max-width: 720px\)/);
  });

  it('scrolls only the findings list, not the station', () => {
    const css = pageSource.slice(pageSource.indexOf('<style>'));
    const scroll = css.match(/\.list-scroll\s*\{([^}]*)\}/)?.[1] ?? '';
    expect(scroll).toMatch(/overflow:\s*hidden auto/);
    const station = css.match(/\.station\s*\{([^}]*)\}/)?.[1] ?? '';
    expect(station).not.toMatch(/overflow/);
  });

  it('draws the bin as inline vector shapes, without raster images', async () => {
    const source = (await import('./VirusBin.svelte?raw')).default;
    expect(source).toContain('<svg');
    expect(source).not.toMatch(/<img|<image/);
  });
});
