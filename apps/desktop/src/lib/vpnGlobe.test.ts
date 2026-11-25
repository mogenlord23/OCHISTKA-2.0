import { describe, expect, it } from 'vitest';
import { globeBufferSize, globeCountry, globeViewport, shortestTurn } from './vpnGlobe';

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

  it('crops the spinning globe at the horizon and centres the zoomed map', () => {
    const rest = globeViewport(538, 222, 1);
    expect(rest.y).toBeGreaterThan(222 * 0.9);
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
