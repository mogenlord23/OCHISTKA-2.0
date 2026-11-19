import { afterEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import Check from './Check.svelte';

afterEach(cleanup);

describe('Check', () => {
  it('shows the label, toggles and reports the next value', async () => {
    const onchange = vi.fn();
    render(Check, { label: 'Временные файлы', onchange });
    const box = screen.getByRole('checkbox', { name: 'Временные файлы' }) as HTMLInputElement;
    expect(box.checked).toBe(false);
    await fireEvent.click(box);
    expect(box.checked).toBe(true);
    expect(onchange).toHaveBeenCalledWith(true);
  });

  it('in controlled mode returns to the parent value until the parent changes it', async () => {
    const onchange = vi.fn();
    render(Check, { label: 'Параметр', checked: false, controlled: true, onchange });
    const box = screen.getByRole('checkbox') as HTMLInputElement;
    await fireEvent.click(box);
    expect(onchange).toHaveBeenCalledWith(true);
    expect(box.checked).toBe(false);
  });

  it('is disabled for the keyboard and the screen reader when disabled', async () => {
    render(Check, { label: 'Недоступно', disabled: true });
    const box = screen.getByRole('checkbox') as HTMLInputElement;
    expect(box.disabled).toBe(true);
  });
});
