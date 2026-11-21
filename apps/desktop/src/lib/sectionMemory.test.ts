import { describe, expect, it } from 'vitest';
import { defaultSub, loadSubsections, saveSubsections, subFor } from './sectionMemory';

function store(initial: string | null = null) {
  let value = initial;
  return {
    getItem: () => value,
    setItem: (_k: string, v: string) => {
      value = v;
    },
  };
}

describe('section memory', () => {
  it('uses the first subsection by default', () => {
    expect(defaultSub('vpn')).toBe('vpn');
    expect(subFor('design', {})).toBe('wallpapers');
  });

  it('remembers one subsection per section and survives a restart', () => {
    const s = store();
    saveSubsections({ vpn: 'network', design: 'cursors' }, s);
    const back = loadSubsections(s);
    expect(subFor('vpn', back)).toBe('network');
    expect(subFor('design', back)).toBe('cursors');
    expect(subFor('disks', back)).toBe('cleanup');
  });

  it('drops unknown values and broken data', () => {
    expect(loadSubsections(store('{"vpn":"nope","design":"colors","x":"y"}'))).toEqual({
      design: 'colors',
    });
    expect(loadSubsections(store('{broken'))).toEqual({});
    expect(loadSubsections(null)).toEqual({});
  });
});
