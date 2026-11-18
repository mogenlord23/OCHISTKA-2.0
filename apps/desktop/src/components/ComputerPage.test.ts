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
  it('renders the components page with the loading state', () => {
    render(ComputerPage, { props: { subsection: 'components' } });
    expect(screen.getByText('Загрузка…')).toBeTruthy();
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

  it('renders the Drivers page with the OS switch', () => {
    render(ComputerPage, { props: { subsection: 'drivers' } });
    expect(screen.getByRole('group', { name: 'Операционная система' })).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Linux' })).toBeTruthy();
  });
});
