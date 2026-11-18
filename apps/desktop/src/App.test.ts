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

  it('opens the appearance section with its three subsections and the first grid', async () => {
    render(App);
    await fireEvent.click(screen.getByRole('button', { name: 'Оформление' }));
    expect(screen.getByRole('heading', { level: 1 }).textContent).toBe('Оформление');
    const tabs = screen.getByRole('navigation', { name: 'Подразделы' });
    expect(
      within(tabs)
        .getAllByRole('button')
        .map((button) => button.textContent?.trim()),
    ).toEqual(['Обои', 'Курсоры', 'Цвета']);
    expect(screen.getByRole('button', { name: /Аврора/ })).toBeTruthy();
  });

  it('has window controls for minimise, maximise and close', () => {
    render(App);
    expect(screen.getByRole('button', { name: 'Свернуть' })).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Развернуть' })).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Закрыть' })).toBeTruthy();
  });
});

describe('FND-0020: sound control is disabled with a visible reason', () => {
  it('FND-0020: the sound button is disabled', () => {
    render(App);
    const sound = screen.getByRole('button', { name: /ЗВУК/ }) as HTMLButtonElement;
    expect(sound.disabled).toBe(true);
  });

  it('FND-0020: the reason is visible text inside the button and in its title', () => {
    render(App);
    const sound = screen.getByRole('button', { name: /ЗВУК/ });
    expect(within(sound).getByText('в разработке')).toBeTruthy();
    expect(sound.getAttribute('title') ?? '').toContain('в разработке');
  });

  it('FND-0020: clicking the sound button changes nothing (no fake toggle)', async () => {
    render(App);
    const sound = screen.getByRole('button', { name: /ЗВУК/ });
    const before = sound.textContent;
    expect(sound.hasAttribute('aria-pressed')).toBe(false);

    await fireEvent.click(sound);

    expect(sound.textContent).toBe(before);
    expect(sound.hasAttribute('aria-pressed')).toBe(false);
    expect(sound.textContent).not.toMatch(/ВКЛ|ВЫКЛ/);
    expect(within(sound).getByText('в разработке')).toBeTruthy();
  });

  it('switches the interface language with RU and ENG', async () => {
    render(App);
    const footer = () => document.querySelector('footer')?.textContent;
    expect(footer()).toBe('ЛОКАЛЬНО. ПОД ТВОИМ КОНТРОЛЕМ.');
    await fireEvent.click(screen.getByRole('button', { name: 'ENG' }));
    expect(footer()).toBe('LOCAL. UNDER YOUR CONTROL.');
    expect(screen.getByRole('button', { name: 'ENG' }).getAttribute('aria-pressed')).toBe('true');
    await fireEvent.click(screen.getByRole('button', { name: 'RU' }));
    expect(footer()).toBe('ЛОКАЛЬНО. ПОД ТВОИМ КОНТРОЛЕМ.');
  });

  it('shows English section and subsection titles after switching to ENG', async () => {
    render(App);
    await fireEvent.click(screen.getByRole('button', { name: 'ENG' }));
    expect(screen.getByRole('navigation', { name: 'Navigation' })).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Viruses' })).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Appearance' })).toBeTruthy();

    await fireEvent.click(screen.getByRole('button', { name: 'Viruses' }));
    expect(screen.getByRole('heading', { level: 1 }).textContent).toBe('Viruses');
    const tabs = screen.getByRole('navigation', { name: 'Subsections' });
    expect(within(tabs).getByRole('button', { name: 'Scan' })).toBeTruthy();
    expect(within(tabs).getByRole('button', { name: 'Scan log' })).toBeTruthy();

    await fireEvent.click(screen.getByRole('button', { name: 'RU' }));
    expect(screen.getByRole('heading', { level: 1 }).textContent).toBe('Вирусы');
  });
});
