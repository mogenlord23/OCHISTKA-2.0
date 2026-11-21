import { afterEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import SecretField from './SecretField.svelte';

const URL_TEXT = 'https://sub.test/api/SECRET-TOKEN-123';

afterEach(() => {
  cleanup();
  vi.useRealTimers();
});

describe('SecretField', () => {
  it('has no show/hide button and no extra controls', () => {
    render(SecretField, { value: URL_TEXT });
    expect(screen.queryByRole('button')).toBeNull();
  });

  it('lifts the label when there is a value or focus', async () => {
    const { container } = render(SecretField, { value: '' });
    expect(container.querySelector('.field.lifted')).toBeNull();
    const input = screen.getByLabelText('Ссылка подписки');
    await fireEvent.focus(input);
    expect(container.querySelector('.field.lifted')).not.toBeNull();
    await fireEvent.blur(input);
    await fireEvent.input(input, { target: { value: 'x' } });
    expect(container.querySelector('.field.lifted')).not.toBeNull();
  });

  it('shows a decoy line instead of the secret and keeps the real value intact', () => {
    const { container } = render(SecretField, { value: URL_TEXT });
    const input = screen.getByLabelText('Ссылка подписки') as HTMLInputElement;
    expect(input.value).toBe(URL_TEXT);
    const overlay = container.querySelector('.overlay')?.textContent ?? '';
    expect(overlay).toContain('https://sub.test/');
    expect(overlay).not.toContain('SECRET');
    expect(overlay).not.toContain('TOKEN');
    expect(overlay).not.toContain('•');
  });

  it('never exposes the secret on any animation frame', async () => {
    vi.useFakeTimers();
    const { container } = render(SecretField, { value: URL_TEXT });
    for (let i = 0; i < 80; i++) {
      const text = container.querySelector('.overlay')?.textContent ?? '';
      expect(text).not.toMatch(/SECRET|TOKEN|123/);
      await vi.advanceTimersByTimeAsync(120);
    }
  });

  it('lets the user edit the real link in focus and brings the decoy back after blur', async () => {
    const { container } = render(SecretField, { value: URL_TEXT });
    const input = screen.getByLabelText('Ссылка подписки') as HTMLInputElement;
    await fireEvent.focus(input);
    expect(container.querySelector('.overlay')).toBeNull();
    expect(input.classList.contains('hide-text')).toBe(false);
    await fireEvent.input(input, { target: { value: URL_TEXT + '-NEW' } });
    expect(input.value).toBe(URL_TEXT + '-NEW');
    await fireEvent.blur(input);
    expect(container.querySelector('.overlay')).not.toBeNull();
    expect(container.querySelector('.overlay')?.textContent).not.toContain('NEW');
  });

  it('accepts a pasted link and Enter triggers the import', async () => {
    const onenter = vi.fn();
    render(SecretField, { value: '', onenter });
    const input = screen.getByLabelText('Ссылка подписки') as HTMLInputElement;
    await fireEvent.input(input, { target: { value: URL_TEXT } });
    expect(input.value).toBe(URL_TEXT);
    await fireEvent.keyDown(input, { key: 'Enter' });
    expect(onenter).toHaveBeenCalledTimes(1);
  });

  it('keeps the host still and changes only a few secret letters per tick', async () => {
    vi.useFakeTimers();
    const { container } = render(SecretField, { value: URL_TEXT });
    const open = () => container.querySelector('.open')?.textContent;
    const letters = () =>
      Array.from(container.querySelectorAll('.secret .ch'), (e) => e.textContent);
    const first = letters();
    expect(open()).toBe('https://sub.test/');
    let maxChanged = 0;
    let prev = first;
    for (let i = 0; i < 30; i++) {
      await vi.advanceTimersByTimeAsync(700);
      expect(open()).toBe('https://sub.test/');
      const now = letters();
      expect(now).toHaveLength(first.length);
      maxChanged = Math.max(maxChanged, now.filter((c, j) => c !== prev[j]).length);
      prev = now;
    }
    expect(maxChanged).toBeGreaterThan(0);
    expect(maxChanged).toBeLessThanOrEqual(2);
  });

  it('stops animating while focused and keeps the real value for import', async () => {
    vi.useFakeTimers();
    const { container } = render(SecretField, { value: URL_TEXT });
    const input = screen.getByLabelText('Ссылка подписки') as HTMLInputElement;
    await fireEvent.focus(input);
    expect(container.querySelector('.secret')).toBeNull();
    await vi.advanceTimersByTimeAsync(5000);
    expect(input.value).toBe(URL_TEXT);
  });

  it('stays empty for an empty field', () => {
    const { container } = render(SecretField, { value: '' });
    expect(container.querySelector('.overlay')).toBeNull();
  });
});
