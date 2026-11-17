import { afterEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, within } from '@testing-library/svelte';
import App from './App.svelte';
import { SECTIONS } from './lib/sections';

vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow: () => ({
    minimize: vi.fn(),
    toggleMaximize: vi.fn(),
    close: vi.fn(),
  }),
}));

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
      .map((button) => button.getAttribute('aria-label'));
    expect(titles).toEqual(SECTIONS.map((section) => section.title));
  });

  it('opens VPN first, with its subsections', () => {
    render(App);
    expect(screen.getByRole('heading', { level: 1 }).textContent).toBe('VPN');
    const tabs = screen.getByRole('navigation', { name: 'Подразделы' });
    expect(within(tabs).getByRole('button', { name: 'Подписка и импорт' })).toBeTruthy();
    expect(within(tabs).getByRole('button', { name: 'Серверы' })).toBeTruthy();
  });

  it('opens the virus section from the rail', async () => {
    render(App);
    await fireEvent.click(screen.getByRole('button', { name: 'Вирусы' }));
    expect(screen.getByRole('heading', { level: 1 }).textContent).toBe('Вирусы');
    const tabs = screen.getByRole('navigation', { name: 'Подразделы' });
    expect(within(tabs).getByRole('button', { name: 'Журнал проверки' })).toBeTruthy();
  });

  it('shows a placeholder for a section that is not built yet', async () => {
    render(App);
    await fireEvent.click(screen.getByRole('button', { name: 'Процессы' }));
    expect(screen.getByRole('heading', { level: 1 }).textContent).toBe('Процессы');
    expect(screen.getByText('В разработке')).toBeTruthy();
  });

  it('has window controls for minimise, maximise and close', () => {
    render(App);
    expect(screen.getByRole('button', { name: 'Свернуть' })).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Развернуть' })).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Закрыть' })).toBeTruthy();
  });
});
