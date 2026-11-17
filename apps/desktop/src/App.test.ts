import { afterEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, within } from '@testing-library/svelte';
import App from './App.svelte';
import { SECTIONS } from './lib/sections';

vi.mock('./lib/api', async (importOriginal) => ({
  ...(await importOriginal<typeof import('./lib/api')>()),
  getSystemOverview: vi.fn(() => new Promise(() => {})),
  clamavStatus: vi.fn(async () => ({ available: true, reason: '' })),
  defaultScanTarget: vi.fn(async () => 'C:\\Users\\Slava\\Downloads'),
}));

afterEach(() => cleanup());

describe('App navigation', () => {
  it('renders seven sections in the right rail, in order', () => {
    render(App);
    const nav = screen.getByRole('navigation', { name: 'Навигация' });
    const titles = within(nav)
      .getAllByRole('button')
      .map((button) => button.querySelector('.title')?.textContent?.trim());
    expect(titles).toEqual(SECTIONS.map((section) => section.title));
  });

  it('opens the virus section first, with its subsections', () => {
    render(App);
    expect(screen.getByRole('heading', { level: 1 }).textContent).toBe('Вирусы');
    const tabs = screen.getByRole('navigation', { name: 'Подразделы' });
    expect(within(tabs).getByRole('button', { name: 'Проверка' })).toBeTruthy();
    expect(within(tabs).getByRole('button', { name: 'Находки' })).toBeTruthy();
    expect(within(tabs).getByRole('button', { name: 'Журнал проверки' })).toBeTruthy();
  });

  it('shows a placeholder for a section that is not built yet', async () => {
    render(App);
    await fireEvent.click(screen.getByRole('button', { name: /VPN/ }));
    expect(screen.getByRole('heading', { level: 1 }).textContent).toBe('VPN');
    expect(screen.getByText('В разработке')).toBeTruthy();
  });
});
