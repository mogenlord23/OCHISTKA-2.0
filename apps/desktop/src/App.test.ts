import { afterEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
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
    const tabs = screen.getByRole('tablist', { name: 'Подразделы' });
    const bar = tabs.closest('header')!;
    expect(within(bar).getByRole('button', { name: 'Закрыть' })).toBeTruthy();
    expect(screen.queryByLabelText('OCHISTKA')).toBeNull();
    expect(within(tabs).getByRole('tab', { name: 'VPN' })).toBeTruthy();
    expect(within(tabs).getByRole('tab', { name: 'Соединения' })).toBeTruthy();
  });

  it('keeps the same portrait when switching between all VPN tabs', async () => {
    render(App);
    const poster = screen.getByRole('button', { name: 'Постер' });
    for (const name of ['Дополнительно', 'Сеть', 'Соединения']) {
      const selected = screen.getByRole('tab', { selected: true });
      await fireEvent.keyDown(selected, { key: 'ArrowRight' });
      expect(screen.getByRole('tab', { name, selected: true })).toBeTruthy();
      expect(screen.getByRole('button', { name: 'Постер' })).toBe(poster);
    }
  });

  it('keeps a dedicated non-interactive strip for dragging the frameless window', () => {
    const { container } = render(App);
    const strip = container.querySelector('.window-drag-strip');
    expect(strip?.hasAttribute('data-tauri-drag-region')).toBe(true);
    expect(strip?.querySelector('button, input, a')).toBeNull();
  });

  it('keeps the Network page mounted across VPN tabs and section changes', async () => {
    render(App);
    await fireEvent.click(screen.getByRole('tab', { name: 'Сеть' }));
    const speedTest = screen.getByRole('region', { name: 'Тест скорости интернета' });
    await fireEvent.click(screen.getByRole('tab', { name: 'Дополнительно' }));
    await fireEvent.click(screen.getByRole('tab', { name: 'Сеть' }));
    expect(screen.getByRole('region', { name: 'Тест скорости интернета' })).toBe(speedTest);
    await fireEvent.click(screen.getByRole('button', { name: 'Вирусы' }));
    expect(screen.queryByRole('region', { name: 'Тест скорости интернета' })).toBeNull();
    await fireEvent.click(screen.getByRole('button', { name: 'VPN' }));
    expect(screen.getByRole('region', { name: 'Тест скорости интернета' })).toBe(speedTest);
  });

  it('opens the virus section from the rail', async () => {
    render(App);
    await fireEvent.click(screen.getByRole('button', { name: 'Вирусы' }));
    expect(screen.getByRole('heading', { level: 1 }).textContent).toBe('Вирусы');
    const tabs = screen.getByRole('tablist', { name: 'Подразделы' });
    expect(within(tabs).getByRole('tab', { name: 'Журнал проверки' })).toBeTruthy();
    expect(within(tabs.closest('header')!).getByRole('button', { name: 'Закрыть' })).toBeTruthy();
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
    expect(sound.querySelector('svg')).toBeTruthy();
    expect(sound.textContent?.trim()).toBe('');
  });

  it('FND-0020: clicking toggles on/off and saves the state', async () => {
    render(App);
    const sound = screen.getByRole('button', { name: /ЗВУК/ });
    await fireEvent.click(sound);
    expect(sound.getAttribute('aria-pressed')).toBe('false');
    expect(sound.getAttribute('title')).toBe('Звук выключен');
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

describe('App subsection memory', () => {
  it('returns to the last subsection of each section', async () => {
    try {
      localStorage.removeItem('ochistka.subsections');
    } catch {
      /* нет хранилища */
    }
    render(App);
    // VPN → Сеть (у VPN вкладки внутри страницы), затем Оформление → Курсоры, затем обратно.
    await fireEvent.click(screen.getAllByRole('tab', { name: 'Сеть' }).at(-1)!);
    await fireEvent.click(screen.getByRole('button', { name: 'Оформление' }));
    await fireEvent.click(screen.getAllByRole('tab', { name: 'Курсоры' }).at(-1)!);
    await fireEvent.click(screen.getByRole('button', { name: 'VPN' }));
    expect(screen.getAllByRole('tab', { name: 'Сеть' }).at(-1)!.getAttribute('aria-selected')).toBe(
      'true',
    );
    await fireEvent.click(screen.getByRole('button', { name: 'Оформление' }));
    expect(
      screen.getAllByRole('tab', { name: 'Курсоры' }).at(-1)!.getAttribute('aria-selected'),
    ).toBe('true');
    await waitFor(() => {
      expect(JSON.parse(localStorage.getItem('ochistka.subsections') ?? '{}')).toMatchObject({
        vpn: 'network',
        design: 'cursors',
      });
    });
  });
});
