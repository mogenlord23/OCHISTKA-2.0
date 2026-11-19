import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, render, screen } from '@testing-library/svelte';
import ComputerPage from './ComputerPage.svelte';
import { getHardware, getSystemOverview } from '../lib/api';
import { tweaksOverview } from '../lib/tweaksApi';

vi.mock('../lib/api', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../lib/api')>();
  return {
    ...actual,
    getSystemOverview: vi.fn(() => new Promise(() => {})),
    getHardware: vi.fn(() => new Promise(() => {})),
  };
});
vi.mock('../lib/tweaksApi', () => ({
  tweaksOverview: vi.fn(() => new Promise(() => {})),
  tweakSet: vi.fn(),
  tweaksRevert: vi.fn(),
  boostSetProfile: vi.fn(),
  windowsActivation: vi.fn(() => new Promise(() => {})),
  openActivationSettings: vi.fn(),
}));

vi.mock('../lib/driversApi', () => ({
  getDriversCatalog: vi.fn(() => new Promise(() => {})),
  installDriver: vi.fn(),
  openDriverPage: vi.fn(),
  openWindowsUpdate: vi.fn(),
}));

beforeEach(() => {
  vi.mocked(getSystemOverview).mockClear();
  vi.mocked(getHardware).mockClear();
  vi.mocked(tweaksOverview).mockClear();
});

afterEach(() => {
  cleanup();
});

describe('ComputerPage subsections', () => {
  it('renders the components page structure at once, without a loading screen', () => {
    render(ComputerPage, { props: { subsection: 'components' } });
    expect(screen.queryByText('Загрузка…')).toBeNull();
    expect(screen.getByRole('group', { name: 'Комплектующие' })).toBeTruthy();
  });

  it('renders the Settings page without an OS switch', () => {
    render(ComputerPage, { props: { subsection: 'settings' } });
    expect(screen.getByRole('heading', { name: 'Настройки Windows' })).toBeTruthy();
    expect(screen.queryByRole('group', { name: 'Операционная система' })).toBeNull();
  });

  it('renders the Boost page without an OS switch', () => {
    render(ComputerPage, { props: { subsection: 'boost' } });
    expect(screen.getByRole('heading', { name: 'Boost / Игры' })).toBeTruthy();
    expect(screen.queryByRole('group', { name: 'Операционная система' })).toBeNull();
  });

  it('renders the Drivers page without the OS switch', () => {
    render(ComputerPage, { props: { subsection: 'drivers' } });
    expect(screen.getByRole('heading', { name: 'Драйверы / твоё оборудование' })).toBeTruthy();
    expect(screen.queryByRole('group', { name: 'Операционная система' })).toBeNull();
  });
});
