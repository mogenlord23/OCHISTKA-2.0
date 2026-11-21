import { afterEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import FloatingField from './FloatingField.svelte';

afterEach(cleanup);

describe('FloatingField', () => {
  it('keeps the label inside the field and lifts it on focus or value', async () => {
    const { container } = render(FloatingField, { label: 'Название или PID', type: 'search' });
    expect(container.querySelector('.lbl')?.textContent).toBe('Название или PID');
    expect(container.querySelector('.field.lifted')).toBeNull();
    const input = screen.getByRole('searchbox');
    await fireEvent.focus(input);
    expect(container.querySelector('.field.lifted')).not.toBeNull();
    await fireEvent.blur(input);
    expect(container.querySelector('.field.lifted')).toBeNull();
    await fireEvent.input(input, { target: { value: '1234' } });
    expect(container.querySelector('.field.lifted')).not.toBeNull();
  });

  it('uses the given accessible name and a searchbox role for search fields', () => {
    render(FloatingField, {
      label: 'Название или PID',
      ariaLabel: 'Поиск по имени или PID',
      type: 'search',
    });
    expect(screen.getByRole('searchbox', { name: 'Поиск по имени или PID' })).toBeTruthy();
  });

  it('reports Enter and shows no overlay unless asked', async () => {
    const onenter = vi.fn();
    const { container } = render(FloatingField, { label: 'x', onenter });
    await fireEvent.keyDown(screen.getByLabelText('x'), { key: 'Enter' });
    expect(onenter).toHaveBeenCalledTimes(1);
    expect(container.querySelector('.overlay')).toBeNull();
  });
});
