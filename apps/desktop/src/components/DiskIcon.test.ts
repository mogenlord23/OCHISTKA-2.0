import { afterEach, describe, expect, it } from 'vitest';
import { cleanup, render } from '@testing-library/svelte';
import DiskIcon from './DiskIcon.svelte';

afterEach(cleanup);

describe('DiskIcon', () => {
  it('draws a 3.5-inch HDD with a platter and a round spindle', () => {
    const { container } = render(DiskIcon, { kind: 'hdd' });
    expect(container.querySelector('svg')?.getAttribute('data-kind')).toBe('hdd');
    expect(container.querySelector('[data-part="platter"]')).not.toBeNull();
    expect(container.querySelector('circle[data-part="spindle"]')).not.toBeNull();
    expect(container.querySelector('[data-part="arm"]')).not.toBeNull();
    expect(container.querySelectorAll('[data-part="chip"]').length).toBe(0);
  });

  it('draws a 2.5-inch SSD board with memory chips and a connector', () => {
    const { container } = render(DiskIcon, { kind: 'ssd' });
    expect(container.querySelector('svg')?.getAttribute('data-kind')).toBe('ssd');
    expect(container.querySelectorAll('[data-part="chip"]').length).toBeGreaterThanOrEqual(3);
    expect(container.querySelector('[data-part="connector"]')).not.toBeNull();
    expect(container.querySelector('[data-part="platter"]')).toBeNull();
  });

  it('falls back to the HDD drawing when the type is unknown and hides from screen readers', () => {
    const { container } = render(DiskIcon, { kind: 'unknown' });
    const svg = container.querySelector('svg');
    expect(svg?.getAttribute('data-kind')).toBe('hdd');
    expect(svg?.getAttribute('aria-hidden')).toBe('true');
  });
});
