import { afterEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import Switch from './Switch.svelte';

afterEach(cleanup);

describe('Switch', () => {
  it('is a labelled switch that reports the wanted value and keeps the parent state', async () => {
    const onchange = vi.fn();
    render(Switch, { label: 'Параметр', checked: false, onchange });
    const sw = screen.getByRole('switch', { name: 'Параметр' });
    expect(sw.getAttribute('aria-checked')).toBe('false');
    await fireEvent.click(sw);
    expect(onchange).toHaveBeenCalledWith(true);
    expect(sw.getAttribute('aria-checked')).toBe('false');
  });

  it('asks to turn off when it is on', async () => {
    const onchange = vi.fn();
    render(Switch, { label: 'Параметр', checked: true, onchange });
    await fireEvent.click(screen.getByRole('switch'));
    expect(onchange).toHaveBeenCalledWith(false);
  });

  it('does nothing when disabled', async () => {
    const onchange = vi.fn();
    render(Switch, { label: 'Недоступно', disabled: true, onchange });
    const sw = screen.getByRole('switch') as HTMLButtonElement;
    expect(sw.disabled).toBe(true);
    await fireEvent.click(sw);
    expect(onchange).not.toHaveBeenCalled();
  });

  it('shows the waiting state to the screen reader', () => {
    render(Switch, { label: 'Параметр', pending: true });
    expect(screen.getByRole('switch').getAttribute('aria-busy')).toBe('true');
  });
});
