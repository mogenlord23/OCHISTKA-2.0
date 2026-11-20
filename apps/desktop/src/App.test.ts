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
    // У VPN своя панель вкладок внутри страницы (общая в оболочке скрыта), поэтому берём последнюю.
    const tabs = screen.getAllByRole('tablist', { name: 'Подразделы' }).at(-1)!;
    expect(within(tabs).getByRole('tab', { name: 'VPN' })).toBeTruthy();
    expect(within(tabs).getByRole('tab', { name: 'Соединения' })).toBeTruthy();
  });

  it('opens the virus section from the rail', async () => {
    render(App);
    await fireEvent.click(screen.getByRole('button', { name: 'Вирусы' }));
    expect(screen.getByRole('heading', { level: 1 }).textContent).toBe('Вирусы');
    const tabs = screen.getByRole('tablist', { name: 'Подразделы' });
    expect(within(tabs).getByRole('tab', { name: 'Журнал проверки' })).toBeTruthy();
  });

  it('opens the appearance section with its four tabs and the first grid', async () => {
    render(App);
    await fireEvent.click(screen.getByRole('button', { name: 'Оформление' }));
    expect(screen.getByRole('heading', { level: 1 }).textContent).toBe('Оформление');
    const tabs = screen.getByRole('tablist', { name: 'Подразделы' });
    expect(
      within(tabs)
        .getAllByRole('tab')
        .map((button) => button.textContent?.trim()),
    ).toEqual(['Обои', 'Курсоры', 'Цвета', 'Настройки приложения']);
    expect(screen.getByRole('button', { name: 'Обои 1' })).toBeTruthy();
  });

  it('has window controls for minimise, maximise and close', () => {
    render(App);
    expect(screen.getByRole('button', { name: 'Свернуть' })).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Развернуть' })).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Закрыть' })).toBeTruthy();
  });
});

describe('FND-0020: sound control toggles and persists', () => {
  afterEach(() => localStorage.removeItem('soundEnabled'));

  it('FND-0020: the sound button is enabled and on by default', () => {
    localStorage.removeItem('soundEnabled');
    render(App);
    const sound = screen.getByRole('button', { name: /ЗВУК/ }) as HTMLButtonElement;
    expect(sound.disabled).toBe(false);
    expect(sound.getAttribute('aria-pressed')).toBe('true');
    expect(within(sound).getByText('ВКЛ')).toBeTruthy();
    expect(sound.textContent).not.toContain('в разработке');
  });

  it('FND-0020: clicking toggles on/off and saves the state', async () => {
    render(App);
    const sound = screen.getByRole('button', { name: /ЗВУК/ });
    await fireEvent.click(sound);
    expect(sound.getAttribute('aria-pressed')).toBe('false');
    expect(within(sound).getByText('ВЫКЛ')).toBeTruthy();
    expect(localStorage.getItem('soundEnabled')).toBe('0');
    await fireEvent.click(sound);
    expect(sound.getAttribute('aria-pressed')).toBe('true');
    expect(localStorage.getItem('soundEnabled')).toBe('1');
  });

  it('FND-0020: a saved off state is restored', () => {
    localStorage.setItem('soundEnabled', '0');
    render(App);
    const sound = screen.getByRole('button', { name: /ЗВУК/ });
    expect(sound.getAttribute('aria-pressed')).toBe('false');
  });

  it('switches the interface language with RU and ENG', async () => {
    render(App);
    const footer = () =>
      screen.getByRole('button', { name: /^(Вирусы|Viruses)$/ }).textContent?.trim();
    expect(footer()).toBe('Вирусы');
    await fireEvent.click(screen.getByRole('button', { name: 'ENG' }));
    expect(footer()).toBe('Viruses');
    expect(screen.getByRole('button', { name: 'ENG' }).getAttribute('aria-pressed')).toBe('true');
    await fireEvent.click(screen.getByRole('button', { name: 'RU' }));
    expect(footer()).toBe('Вирусы');
  });

  it('shows English section and subsection titles after switching to ENG', async () => {
    render(App);
    await fireEvent.click(screen.getByRole('button', { name: 'ENG' }));
    expect(screen.getByRole('navigation', { name: 'Navigation' })).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Viruses' })).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Appearance' })).toBeTruthy();

    await fireEvent.click(screen.getByRole('button', { name: 'Viruses' }));
    expect(screen.getByRole('heading', { level: 1 }).textContent).toBe('Viruses');
    const tabs = screen.getByRole('tablist', { name: 'Subsections' });
    expect(within(tabs).getByRole('tab', { name: 'Scan' })).toBeTruthy();
    expect(within(tabs).getByRole('tab', { name: 'Scan log' })).toBeTruthy();

    await fireEvent.click(screen.getByRole('button', { name: 'RU' }));
    expect(screen.getByRole('heading', { level: 1 }).textContent).toBe('Вирусы');
  });
});
