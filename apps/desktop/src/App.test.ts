import { afterEach, describe, expect, it } from 'vitest';
import { cleanup, fireEvent, render, screen, within } from '@testing-library/svelte';
import App from './App.svelte';
import { SECTIONS } from './lib/sections';

afterEach(() => cleanup());

describe('App navigation', () => {
  it('renders ten navigation buttons with section titles', () => {
    render(App);
    const nav = screen.getByRole('navigation', { name: 'Разделы' });
    const buttons = within(nav).getAllByRole('button');
    expect(buttons.map((button) => button.textContent?.trim())).toEqual(
      SECTIONS.map((section) => section.title),
    );
  });

  it('shows Главная by default', () => {
    render(App);
    expect(screen.getByRole('heading', { level: 1 }).textContent).toBe('Главная');
  });

  it('shows a placeholder for other sections', async () => {
    render(App);
    await fireEvent.click(screen.getByRole('button', { name: 'Очистка' }));
    expect(screen.getByRole('heading', { level: 1 }).textContent).toBe('Очистка');
    expect(screen.getByText('В разработке')).toBeTruthy();
  });

  it('marks the active section with aria-current', async () => {
    render(App);
    await fireEvent.click(screen.getByRole('button', { name: 'Сеть' }));
    expect(screen.getByRole('button', { name: 'Сеть' }).getAttribute('aria-current')).toBe('page');
    expect(screen.getByRole('button', { name: 'Главная' }).getAttribute('aria-current')).toBeNull();
  });
});
