import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, render, screen } from '@testing-library/svelte';
import ComputerPage from './ComputerPage.svelte';
import { getSystemOverview } from '../lib/api';

vi.mock('../lib/api', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../lib/api')>();
  return { ...actual, getSystemOverview: vi.fn(() => new Promise(() => {})) };
});

const SYSTEM_REASON = 'Раздел в разработке: системные настройки меняют Windows, пока не подключено';

beforeEach(() => {
  vi.mocked(getSystemOverview).mockClear();
});

afterEach(() => {
  cleanup();
});

describe('ComputerPage subsections', () => {
  it('renders the components page with the loading state', () => {
    render(ComputerPage, { props: { subsection: 'components' } });
    expect(screen.getByText('Загрузка…')).toBeTruthy();
  });

  it('renders the Settings placeholder with its reason', () => {
    render(ComputerPage, { props: { subsection: 'settings' } });
    expect(screen.getByText(SYSTEM_REASON)).toBeTruthy();
  });

  it('renders the Boost placeholder with its reason', () => {
    render(ComputerPage, { props: { subsection: 'boost' } });
    expect(screen.getByText(SYSTEM_REASON)).toBeTruthy();
  });

  it('renders the Drivers page with the OS switch', () => {
    render(ComputerPage, { props: { subsection: 'drivers' } });
    expect(screen.getByRole('group', { name: 'Операционная система' })).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Linux' })).toBeTruthy();
  });
});
