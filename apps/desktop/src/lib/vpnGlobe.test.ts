import { describe, expect, it } from 'vitest';
import { globeCountry, shortestTurn } from './vpnGlobe';

describe('offline VPN globe', () => {
  it('uses local country coordinates and accepts subscription aliases', () => {
    const latvia = globeCountry(' lv ');
    expect(latvia?.[0]).toBeCloseTo(25.46);
    expect(latvia?.[1]).toBeCloseTo(57.07);
    expect(globeCountry('UK')).toEqual(globeCountry('GB'));
    expect(globeCountry('EU')).toBeNull();
    expect(globeCountry(null)).toBeNull();
  });

  it('moves across the date line via the shorter arc', () => {
    const toRad = Math.PI / 180;
    expect(shortestTurn(179 * toRad, -179 * toRad)).toBeCloseTo(2 * toRad);
    expect(shortestTurn(-179 * toRad, 179 * toRad)).toBeCloseTo(-2 * toRad);
  });
});
