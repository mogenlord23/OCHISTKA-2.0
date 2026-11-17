import { describe, expect, it } from 'vitest';
import { SECTIONS } from './sections';
import { BODY_FONT, HEADING_FONT, SECTION_COLORS, contrastRatio, textOn } from './tokens';

describe('SECTION_COLORS', () => {
  it('matches the design brief for each section', () => {
    expect(SECTION_COLORS).toEqual({
      vpn: '#ff009d',
      viruses: '#b4ff00',
      processes: '#147cff',
      programs: '#ff7a00',
      disks: '#f5ff00',
      computer: '#cf00ff',
      design: '#ff5a5f',
    });
  });

  it('gives every section its colour from the tokens', () => {
    for (const section of SECTIONS) {
      expect(section.accent).toBe(SECTION_COLORS[section.id]);
    }
  });
});

describe('contrastRatio', () => {
  it('is 21 for white on black, in either order', () => {
    expect(contrastRatio('#ffffff', '#000000')).toBeCloseTo(21, 5);
    expect(contrastRatio('#000000', '#ffffff')).toBeCloseTo(21, 5);
  });
});

describe('textOn', () => {
  it('gives every section page text that reads at WCAG AA (4.5:1)', () => {
    for (const color of Object.values(SECTION_COLORS)) {
      expect(contrastRatio(textOn(color), color)).toBeGreaterThanOrEqual(4.5);
    }
  });

  it('keeps black page text on every section colour, the purple one included (4.86:1)', () => {
    expect(textOn(SECTION_COLORS.computer)).toBe('#080808');
    expect(contrastRatio('#080808', SECTION_COLORS.computer)).toBeGreaterThanOrEqual(4.5);
  });
});

describe('font tokens', () => {
  it('prefers Arial Black for headings with a system fallback', () => {
    expect(HEADING_FONT.startsWith("'Arial Black'")).toBe(true);
    expect(HEADING_FONT).toContain('system-ui');
  });

  it('uses the system UI font for body text', () => {
    expect(BODY_FONT).toContain('system-ui');
  });
});
