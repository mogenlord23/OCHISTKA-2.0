import { describe, expect, it } from 'vitest';
import { SECTIONS } from './sections';
import {
  BODY_FONT,
  HEADING_FONT,
  NEUTRAL,
  PATTERN,
  RADIUS,
  SECTION_COLORS,
  TYPE_SCALE,
  contrastRatio,
  textOn,
} from './tokens';

describe('SECTION_COLORS', () => {
  it('matches the design brief for each section', () => {
    expect(SECTION_COLORS).toEqual({
      vpn: '#ff009d',
      viruses: '#b4ff00',
      processes: '#147cff',
      programs: '#ff7a00',
      disks: '#f5ff00',
      computer: '#cf00ff',
      design: '#f83c5c',
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

describe('neutral, radius and type tokens', () => {
  it('defines the neutral colours used by the components', () => {
    expect(NEUTRAL).toEqual({
      black: '#080808',
      ink: '#fafafa',
      inkSoft: '#d4d4d4',
      muted: '#8b8b8b',
      surface: '#111111',
      surfaceAlt: '#111111',
      border: '#343434',
    });
  });

  it('defines the radius scale', () => {
    expect(RADIUS).toEqual({ sm: '8px', md: '12px', lg: '14px' });
  });

  it('defines the type scale', () => {
    expect(TYPE_SCALE).toEqual({
      caption: '11px',
      small: '12px',
      body: '13px',
      lead: '14px',
      title: '16px',
      section: '26px',
      display: '44px',
    });
  });
});

describe('PATTERN', () => {
  it('keeps page text at 4.5:1 even where the pattern darkens the colour under it', () => {
    const k = 1 - PATTERN.opacity * PATTERN.worstCoverage;
    for (const color of Object.values(SECTION_COLORS)) {
      const n = parseInt(color.slice(1), 16);
      const dark = [(n >> 16) & 255, (n >> 8) & 255, n & 255].map((v) => Math.round(v * k));
      const under = '#' + dark.map((v) => v.toString(16).padStart(2, '0')).join('');
      expect(contrastRatio(textOn(color), under)).toBeGreaterThanOrEqual(4.5);
    }
  });
});
