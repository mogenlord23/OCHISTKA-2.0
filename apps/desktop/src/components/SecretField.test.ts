import { afterEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import SecretField from './SecretField.svelte';

const URL_TEXT = 'https://sub.test/api/SECRET-TOKEN-123';

afterEach(() => {
  cleanup();
  vi.useRealTimers();
});

describe('SecretField', () => {
  it('shows the label with an empty field and lifts it when there is a value', async () => {
    const { container } = render(SecretField, { value: '' });
    expect(container.querySelector('.field.lifted')).toBeNull();
    await fireEvent.input(screen.getByLabelText('Ссылка подписки'), { target: { value: 'x' } });
    expect(container.querySelector('.field.lifted')).not.toBeNull();
  });

  it('hides the secret part on screen but keeps the real value', () => {
    const { container } = render(SecretField, { value: URL_TEXT });
    const input = screen.getByLabelText('Ссылка подписки') as HTMLInputElement;
    expect(input.value).toBe(URL_TEXT);
    const overlay = container.querySelector('.overlay')?.textContent ?? '';
    expect(overlay.startsWith('https://sub.test/')).toBe(true);
    expect(overlay).not.toContain('SECRET-TOKEN');
    expect(container.textContent).not.toContain('SECRET-TOKEN');
  });

  it('shows the link on request and hides it again', async () => {
    vi.useFakeTimers();
    const { container } = render(SecretField, { value: URL_TEXT });
    const toggle = screen.getByRole('button', { name: 'Показать ссылку' });
    await fireEvent.click(toggle);
    await vi.advanceTimersByTimeAsync(50);
    expect(screen.getByRole('button', { name: 'Скрыть ссылку' }).getAttribute('aria-pressed')).toBe(
      'true',
    );
    expect(container.querySelector('.overlay')).toBeNull();
    await fireEvent.click(screen.getByRole('button', { name: 'Скрыть ссылку' }));
    // Пока маска «собирается» эффектом, ни на одном кадре в страницу не попадает секретная часть.
    for (let i = 0; i < 14; i++) {
      expect(container.querySelector('.overlay')?.textContent ?? '').not.toContain('SECRET');
      expect(container.querySelector('.overlay')?.textContent ?? '').not.toContain('TOKEN');
      await vi.advanceTimersByTimeAsync(40);
    }
    expect(container.querySelector('.overlay')?.textContent ?? '').not.toContain('SECRET-TOKEN');
    expect((screen.getByLabelText('Ссылка подписки') as HTMLInputElement).value).toBe(URL_TEXT);
  });

  it('pasting a full link works while hidden and Enter triggers the import', async () => {
    const onenter = vi.fn();
    render(SecretField, { value: '', onenter });
    const input = screen.getByLabelText('Ссылка подписки') as HTMLInputElement;
    await fireEvent.input(input, { target: { value: URL_TEXT } });
    expect(input.value).toBe(URL_TEXT);
    await fireEvent.keyDown(input, { key: 'Enter' });
    expect(onenter).toHaveBeenCalledTimes(1);
  });
});
