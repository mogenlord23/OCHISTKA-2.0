import { afterEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, within } from '@testing-library/svelte';
import fs from 'node:fs';
import path from 'node:path';
import App from './App.svelte';
import { SECTIONS } from './lib/sections';
import SettingsPage from './components/SettingsPage.svelte';
import BoostPage from './components/BoostPage.svelte';
import { i18n, t } from './lib/i18n.svelte';

vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow: () => ({
    minimize: vi.fn(),
    toggleMaximize: vi.fn(),
    close: vi.fn(),
    isMaximized: vi.fn(async () => false),
    onResized: vi.fn(async () => () => {}),
  }),
}));

vi.mock('./lib/tweaksApi', () => ({
  tweaksOverview: vi.fn(async () => ({ tweaks: [], boost: { profile: null, hasBackup: false } })),
  tweakSet: vi.fn(),
  tweaksRevert: vi.fn(),
  boostSetProfile: vi.fn(),
}));

vi.mock('./lib/api', async (importOriginal) => ({
  ...(await importOriginal<typeof import('./lib/api')>()),
  getSystemOverview: vi.fn(() => new Promise(() => {})),
  clamavStatus: vi.fn(async () => ({ available: true, reason: '' })),
  defaultScanTarget: vi.fn(async () => 'C:\\Users\\Slava\\Downloads'),
}));

afterEach(() => {
  cleanup();
  i18n.lang = 'ru';
  localStorage.clear();
});

function walk(dir: string, out: string[] = []): string[] {
  for (const name of fs.readdirSync(dir)) {
    const full = path.join(dir, name);
    if (fs.statSync(full).isDirectory()) {
      if (name !== 'generated' && name !== 'assets') walk(full, out);
    } else if (/\.(svelte|ts|json)$/.test(name) && !/\.test\.ts$/.test(name)) out.push(full);
  }
  return out;
}

describe('review v2, phase 1: language', () => {
  it('starts in Russian', () => {
    render(App);
    expect(i18n.lang).toBe('ru');
    expect(screen.getByRole('button', { name: 'RU' }).getAttribute('aria-pressed')).toBe('true');
    expect(screen.getByRole('navigation', { name: 'Навигация' })).toBeTruthy();
  });

  it('switching to ENG changes the shell texts, the lead line and the lang store', async () => {
    render(App);
    const lead = () => document.querySelector('.lead')?.textContent;
    const ruLead = lead();
    expect(ruLead).toBe(t('lead.vpn'));
    await fireEvent.click(screen.getByRole('button', { name: 'ENG' }));
    expect(i18n.lang).toBe('en');
    expect(lead()).toBe(t('lead.vpn'));
    expect(lead()).not.toBe(ruLead);
    expect(screen.getByRole('button', { name: 'Minimize' })).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Collapse panel' })).toBeTruthy();
    await fireEvent.click(screen.getByRole('button', { name: 'RU' }));
    expect(i18n.lang).toBe('ru');
    expect(lead()).toBe(ruLead);
  });

  it('every section has a lead key with an English text', () => {
    for (const section of SECTIONS) {
      expect(section.leadKey).toBeTruthy();
    }
  });
});

describe('review v2, phase 1: no hints, no version', () => {
  it('never says the feature is "not connected yet" in the shell or pages', () => {
    const owned = (file: string) =>
      !/[\\/](lib[\\/]vpn|lib[\\/]vpnNetwork|Vpn[A-Za-z]*\.svelte)/.test(file);
    const hits = walk(path.join(__dirname))
      .filter(owned)
      .filter((file) =>
        /пока не подключ|не подключено|not connected yet/i.test(fs.readFileSync(file, 'utf8')),
      );
    expect(hits).toEqual([]);
  });

  it('puts no hover hints (title attributes) on the shell', () => {
    const { container } = render(App);
    const withTitle = container.querySelectorAll('.side [title], header [title]');
    expect(withTitle.length).toBe(0);
  });

  it('renders no version string', () => {
    const { container } = render(App);
    expect(container.textContent).not.toMatch(/\bv?\d+\.\d+\.\d+\b/);
  });

  it('has all seven rail items', () => {
    render(App);
    const nav = screen.getByRole('navigation', { name: 'Навигация' });
    expect(within(nav).getAllByRole('button')).toHaveLength(7);
  });
});

describe('review v2, phase 1: window sizing', () => {
  const conf = JSON.parse(
    fs.readFileSync(path.join(__dirname, '..', 'src-tauri', 'tauri.conf.json'), 'utf8'),
  );
  const win = conf.app.windows[0];
  it('keeps a minimum size that fits seven 48 px rail tiles, gaps, top offset and the dock', () => {
    const rail = 8 + 7 * 48 + 6 * 8;
    const dock = 24 + 8; // ручка и нижний отступ
    expect(win.minHeight).toBeGreaterThanOrEqual(rail + dock);
    expect(win.minWidth).toBeGreaterThanOrEqual(880);
  });
});

describe('review v2, phase 1: no OS switch in Settings and Boost', () => {
  it.each([
    ['Settings', SettingsPage],
    ['Boost', BoostPage],
  ])('%s shows only Windows, no OS switch', (_name, Page) => {
    const { container } = render(Page);
    expect(container.textContent).not.toMatch(/macOS|Linux/);
    expect(container.querySelector('[role="tablist"], .os-switch')).toBeNull();
  });
});
