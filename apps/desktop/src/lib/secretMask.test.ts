import { describe, expect, it } from 'vitest';
import { decoy, segments, splitSecret, visualLine } from './secretMask';

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
