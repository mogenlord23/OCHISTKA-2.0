import { describe, expect, it } from 'vitest';
import { CATALOG, detectVendors } from './driversCatalog';
import type { HardwareOverview } from './types';

const hw = (vendor: string | null, model: string | null, cpuBrand: string | null) =>
  ({
    gpus: [{ vendor, model, usagePercent: null, totalMemoryBytes: null, usedMemoryBytes: null }],
    motherboard: { vendor: null, name: null, version: null },
    cpuBrand,
    driveModels: [],
  }) satisfies HardwareOverview;

describe('driver catalog', () => {
  it('only lists https official pages with unique ids', () => {
    expect(new Set(CATALOG.map((item) => item.id)).size).toBe(CATALOG.length);
    for (const item of CATALOG) {
      expect(item.url.startsWith('https://')).toBe(true);
    }
  });

  it('shows the three main vendors when hardware is unknown', () => {
    expect(detectVendors(null)).toEqual(['nvidia', 'amd', 'intel', 'realtek', 'bluetooth', 'usb']);
  });

  it('detects the GPU and CPU vendors without duplicates', () => {
    expect(detectVendors(hw('NVIDIA', 'GeForce GTX 1060', 'Intel(R) Core(TM) i7-6700'))).toEqual([
      'nvidia',
      'intel',
      'realtek',
      'bluetooth',
      'usb',
    ]);
    expect(detectVendors(hw(null, 'Radeon RX 6600', 'AMD Ryzen 5'))).toEqual([
      'amd',
      'realtek',
      'bluetooth',
      'usb',
    ]);
  });
});
