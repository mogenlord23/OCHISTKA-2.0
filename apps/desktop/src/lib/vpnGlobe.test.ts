import { describe, expect, it } from 'vitest';
import { globeBufferSize, globeCountry, globeMarkerColor, globeDotSize, globeViewport, shortestTurn } from './vpnGlobe';

import map from '../assets/vpn-globe.json';

describe('offline VPN globe', () => {
  it('provides stable positions for continent labels and a generic global endpoint', () => {
    expect(globeCountry('Европа 2')).toBe(globeCountry('EU'));
    expect(globeCountry('South America')).toEqual([-58, -15]);
    expect(globeCountry('Unknown endpoint')).toBe(globeCountry('GLOBAL'));
    expect(globeCountry(null)).toBeNull();
  });
  it('contrasts the marker against the country and scales the dot coverage with the card', () => {
    expect(globeMarkerColor(globeCountry('LV')!)).toBe('#ff009d');
    expect(globeMarkerColor(globeCountry('RU')!)).toBe('#b4ff00');
    expect(globeDotSize(1068, 436)).toBeCloseTo(globeDotSize(534, 218) * 2);
  });
  it('keeps the country palette on both sides of the Baltic and Asian borders', () => {
    const bits = atob(map.lime);
    for (const [code, expected] of Object.entries({ LV: true, SE: true, RU: false, CN: true, US: false, BR: true })) {
      const [lon, lat] = globeCountry(code)!;
      const cell = Math.floor((lat + 90) / 180 * map.gridHeight) * map.gridWidth +
        Math.floor((lon + 180) / 360 * map.gridWidth);
      expect(Boolean(bits.charCodeAt(cell >> 3) & (1 << (cell % 8))), code).toBe(expected);
    }
  });
  it('uses local country coordinates and accepts subscription aliases', () => {
    const latvia = globeCountry(' lv ');
    expect(latvia?.[0]).toBeCloseTo(25.46);
    expect(latvia?.[1]).toBeCloseTo(57.07);
    expect(globeCountry('UK')).toEqual(globeCountry('GB'));
    expect(globeCountry('EU')).toEqual([13, 50]);
    expect(globeCountry(null)).toBeNull();
  });

  it('moves across the date line via the shorter arc', () => {
    const toRad = Math.PI / 180;
    expect(shortestTurn(179 * toRad, -179 * toRad)).toBeCloseTo(2 * toRad);
    expect(shortestTurn(-179 * toRad, 179 * toRad)).toBeCloseTo(-2 * toRad);
  });

  it('crops the spinning globe at the horizon and centres the zoomed map', () => {
    const rest = globeViewport(538, 222, 1);
    expect(rest.y).toBeGreaterThan(222);
    expect(rest.radius * 2).toBeGreaterThanOrEqual(538);
    expect(rest.y - rest.radius).toBeGreaterThanOrEqual(0);
    expect(rest.y + rest.radius).toBeGreaterThan(222);
    const close = globeViewport(538, 222, 4.25);
    expect(close.y).toBeCloseTo(111);
    expect(close.radius).toBeGreaterThan(538);
  });

  it('reuses the backing buffer across small resizes and caps its dimensions', () => {
    expect(globeBufferSize(600, 222, 1)).toEqual(globeBufferSize(610, 225, 1));
    expect(globeBufferSize(1920, 1080, 2).every((size, i) => size <= [1024, 512][i])).toBe(true);
  });
});
