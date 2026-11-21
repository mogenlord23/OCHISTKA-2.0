import { describe, expect, it } from 'vitest';
import { decoy, decoyChars, mutateDecoy, segments, splitSecret, visualLine } from './secretMask';

const SECRET = 'SECRET-TOKEN-123';

describe('splitSecret', () => {
  it('keeps scheme and host, hides the rest', () => {
    const { open, secretLength } = splitSecret(`https://sub.test/api/v1/sub?token=${SECRET}`);
    expect(open).toBe('https://sub.test/');
    expect(secretLength).toBeGreaterThan(0);
    expect(open).not.toContain('token');
  });

  it('hides user and password before @', () => {
    const { open } = splitSecret('https://user:pass@host.test');
    expect(open).not.toContain('user');
    expect(open).not.toContain('pass');
    expect(open.startsWith('https://host.test')).toBe(true);
  });

  it('hides everything that is not a URL, and handles empty input', () => {
    expect(splitSecret('просто текст секрета').open).toBe('');
    expect(splitSecret('').secretLength).toBe(0);
    expect(splitSecret('https://host.test')).toEqual({
      open: 'https://host.test',
      secretLength: 0,
    });
  });

  it('caps how much the visible length can tell about the secret', () => {
    expect(splitSecret('https://h.test/' + 'a'.repeat(500)).secretLength).toBe(28);
  });
});

describe('visualLine', () => {
  it('never contains the secret characters beyond the safe prefix', () => {
    const line = visualLine(`https://sub.test/path/${SECRET}`);
    expect(line.startsWith('https://sub.test/')).toBe(true);
    expect(line).not.toContain('SECRET');
    expect(line).not.toContain('TOKEN');
    expect(line).not.toMatch(/•/);
  });

  it('draws decoy characters from the random source only', () => {
    expect(decoy(4, () => 0)).toBe('aaaa');
    expect(visualLine('https://h.test/xyz', () => 0)).toBe('https://h.test/aaaaaaaa');
  });
});

describe('segments', () => {
  it('splits a long line into three parts and keeps the text', () => {
    const parts = segments('https://sub.test/abcdefghijklmnop', 3);
    expect(parts).toHaveLength(3);
    expect(parts.join('')).toBe('https://sub.test/abcdefghijklmnop');
  });

  it('keeps short text in one part', () => {
    expect(segments('abc', 3)).toEqual(['abc']);
    expect(segments('', 3)).toEqual([]);
  });
});

describe('decoyChars / mutateDecoy', () => {
  it('replaces only the requested number of positions and keeps the length', () => {
    const before = decoyChars(20);
    const after = mutateDecoy(before, 2);
    expect(after).toHaveLength(20);
    const changed = after.filter((c, i) => c !== before[i]);
    expect(changed.length).toBeGreaterThanOrEqual(1);
    expect(changed.length).toBeLessThanOrEqual(2);
    changed.forEach((c) => expect(/^[A-Za-z0-9]$/.test(c.ch)).toBe(true));
  });

  it('gives a changed position a new id and never equals the old letter', () => {
    const before = decoyChars(1);
    const after = mutateDecoy(before, 1, () => 0);
    expect(after[0].id).not.toBe(before[0].id);
    expect(mutateDecoy([], 1)).toEqual([]);
  });
});
