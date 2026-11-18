import { describe, expect, it } from 'vitest';
import { isHardwareOverview } from './api';

const board = { vendor: null, name: 'Board', version: null };
const gpu = {
  vendor: null,
  model: 'GPU',
  usagePercent: null,
  totalMemoryBytes: null,
  usedMemoryBytes: null,
};

describe('isHardwareOverview', () => {
  it('accepts a payload with missing optional fields as null', () => {
    expect(isHardwareOverview({ gpus: [gpu], motherboard: board })).toBe(true);
    expect(isHardwareOverview({ gpus: [], motherboard: board })).toBe(true);
  });

  it('rejects a payload with the wrong shape', () => {
    expect(isHardwareOverview({ gpus: [gpu] })).toBe(false);
    expect(
      isHardwareOverview({ gpus: [{ ...gpu, usagePercent: 'high' }], motherboard: board }),
    ).toBe(false);
    expect(isHardwareOverview({ gpus: [gpu], motherboard: { name: 1 } })).toBe(false);
    expect(isHardwareOverview(null)).toBe(false);
  });
});
