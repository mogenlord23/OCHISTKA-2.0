import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import ExpandButton from './ExpandButton.svelte';

describe('ExpandButton', () => {
  it('keeps the full action name and runs the action only on click', async () => {
    const onclick = vi.fn();
    render(ExpandButton, {
      icon: 'M0 0h1v1z',
      label: 'Проводник',
      name: 'Показать в Проводнике',
      onclick,
    });
    const button = screen.getByRole('button', { name: 'Показать в Проводнике' });
    await fireEvent.mouseEnter(button);
    await fireEvent.focus(button);
    expect(onclick).not.toHaveBeenCalled();
    await fireEvent.click(button);
    expect(onclick).toHaveBeenCalledTimes(1);
  });

  it('stays disabled when the action is unavailable', async () => {
    const onclick = vi.fn();
    render(ExpandButton, {
      icon: 'M0 0h1v1z',
      label: 'Удалить',
      name: 'Удалить',
      disabled: true,
      onclick,
    });
    const button = screen.getByRole('button', { name: 'Удалить' }) as HTMLButtonElement;
    expect(button.disabled).toBe(true);
    expect(onclick).not.toHaveBeenCalled();
  });
});
