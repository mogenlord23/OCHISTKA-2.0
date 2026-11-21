import { describe, expect, it } from 'vitest';
import { maskSubscriptionUrl, scrambleFrame } from './secretMask';

describe('maskSubscriptionUrl', () => {
  it('keeps scheme and host, hides the path with the token', () => {
    const masked = maskSubscriptionUrl(
      'https://sub.example.com/api/v1/client/subscribe?token=SECRET123',
    );
    expect(masked.startsWith('https://sub.example.com/')).toBe(true);
    expect(masked).not.toContain('SECRET123');
    expect(masked).not.toContain('token');
    expect(masked).toMatch(/•{6,16}$/);
  });

  it('hides user and password before @', () => {
    const masked = maskSubscriptionUrl('https://user:pass@host.test/x');
    expect(masked).not.toContain('user');
    expect(masked).not.toContain('pass');
    expect(masked).toContain('@host.test');
  });

  it('hides anything that is not a URL and handles empty input', () => {
    expect(maskSubscriptionUrl('vless://' + 'a'.repeat(5))).toContain('vless://');
    expect(maskSubscriptionUrl('просто текст секрета')).toMatch(/^•+$/);
    expect(maskSubscriptionUrl('')).toBe('');
    expect(maskSubscriptionUrl('https://host.test')).toBe('https://host.test');
  });

  it('never grows with the secret length beyond the cap', () => {
    expect(maskSubscriptionUrl('https://h.test/' + 'a'.repeat(500)).length).toBeLessThan(40);
  });
});

describe('scrambleFrame', () => {
  it('reveals symbols from the left and keeps the length', () => {
    const frame = scrambleFrame('https://abc', 4, () => 0);
    expect(frame.startsWith('http')).toBe(true);
    expect(frame.length).toBe('https://abc'.length);
    expect(scrambleFrame('abc', 3)).toBe('abc');
  });
});
