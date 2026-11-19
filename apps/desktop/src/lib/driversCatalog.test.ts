import { describe, expect, it } from 'vitest';
import {
  detectVendors,
  filterByVendor,
  compatibilityNote,
  layoutVendors,
  VENDOR_ORDER,
  VENDORS,
  windowsGeneration,
  type DriverItem,
} from './driversCatalog';
import type { HardwareOverview } from './types';

const hw = (
  vendor: string | null,
  model: string | null,
  cpuBrand: string | null,
  board: string | null = null,
) =>
  ({
    gpus: [{ vendor, model, usagePercent: null, totalMemoryBytes: null, usedMemoryBytes: null }],
    motherboard: { vendor: board, name: null, version: null },
    cpuBrand,
    driveModels: [],
  }) satisfies HardwareOverview;

const item = (id: string, vendor: DriverItem['vendor']): DriverItem => ({
  id,
  vendor,
  maker: 'Maker',
  device: 'Device',
  title: id,
  description: '',
  action: 'openSite',
  windowsUpdate: false,
});

describe('driver catalog', () => {
  it('has a tile description for every vendor', () => {
    for (const id of VENDOR_ORDER) {
      expect(VENDORS[id].label.length).toBeGreaterThan(0);
    }
  });

  it('filters items by vendor and shows everything without a filter', () => {
    const items = [item('a', 'nvidia'), item('b', 'amd'), item('c', 'nvidia'), item('d', 'oem')];
    expect(filterByVendor(items, 'nvidia').map((i) => i.id)).toEqual(['a', 'c']);
    expect(filterByVendor(items, 'amd').map((i) => i.id)).toEqual(['b']);
    expect(filterByVendor(items, 'audio')).toEqual([]);
    expect(filterByVendor(items, null)).toHaveLength(4);
  });

  it('marks nothing and keeps the usual order when hardware is unknown', () => {
    const layout = layoutVendors(null);
    expect(layout.ordered).toEqual([...VENDOR_ORDER]);
    expect(layout.detected.size).toBe(0);
  });

  it('puts detected vendors first and marks them', () => {
    const layout = layoutVendors(hw('NVIDIA', 'GeForce GTX 1060', 'Intel(R) Core(TM) i7-6700'));
    expect(layout.ordered.slice(0, 2)).toEqual(['nvidia', 'intel']);
    expect([...layout.detected].sort()).toEqual(['intel', 'nvidia']);
    expect(new Set(layout.ordered).size).toBe(VENDOR_ORDER.length);
  });

  it('detects AMD from a Radeon card and a Ryzen CPU, and a PC maker from the board', () => {
    expect([...detectVendors(hw(null, 'Radeon RX 6600', 'AMD Ryzen 5'))]).toEqual(['amd']);
    expect(detectVendors(hw(null, null, null, 'ASUSTeK COMPUTER INC.')).has('oem')).toBe(true);
    expect(detectVendors(hw(null, null, null, 'Unknown Maker')).has('oem')).toBe(false);
  });
});

describe('Звук и связь', () => {
  it('is one tile; there is no separate Realtek or Wi-Fi/Bluetooth tile', () => {
    expect(VENDOR_ORDER).toContain('audio');
    expect(VENDORS.audio.label).toBe('Звук и связь');
    expect((VENDOR_ORDER as readonly string[]).includes('realtek')).toBe(false);
    expect((VENDOR_ORDER as readonly string[]).includes('bluetooth')).toBe(false);
  });
});

describe('compatibility note', () => {
  it('tells the Windows generation from the build number', () => {
    expect(windowsGeneration('10.0.26200')).toBe('11');
    expect(windowsGeneration('10.0.19045')).toBe('10');
    expect(windowsGeneration('24H2')).toBeNull();
  });

  it('names the detected GPU and the Windows version', () => {
    const note = compatibilityNote(
      { id: 'nvidia-app', vendor: 'nvidia' },
      hw('NVIDIA', 'GeForce GTX 1060', null),
      { osName: 'Windows 11', osVersion: '10.0.26200' },
    );
    expect(note).toBe('Найдено: NVIDIA GeForce GTX 1060 · Windows 11 подходит');
  });

  it('warns when the card of this maker is absent and says nothing without data', () => {
    expect(
      compatibilityNote({ id: 'amd-software', vendor: 'amd' }, hw('NVIDIA', 'GeForce', null), null),
    ).toContain('не найдена');
    expect(compatibilityNote({ id: 'realtek-audio', vendor: 'audio' }, null, null)).toBeNull();
  });
});
