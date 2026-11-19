import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/svelte';

const invokeMock = vi.hoisted(() => vi.fn());
vi.mock('@tauri-apps/api/core', () => ({ invoke: invokeMock }));

import GeoCards from './GeoCards.svelte';
import { resetGeo } from '../lib/geoStore.svelte';

beforeEach(() => {
  resetGeo();
  let granted = false;
  invokeMock.mockImplementation(async (cmd: string, args?: { granted?: boolean }) => {
    if (cmd === 'geo_consent') return { granted };
    if (cmd === 'geo_set_consent') {
      granted = Boolean(args?.granted);
      return { granted };
    }
    if (cmd === 'geo_lookup') return { provider: 'Example ISP', country: 'Latvia', city: null };
  });
});

afterEach(() => {
  cleanup();
  invokeMock.mockReset();
});

describe('GeoCards', () => {
  it('shows neutral text and no request before consent, with the VPN note', async () => {
    render(GeoCards);
    expect(await screen.findByRole('button', { name: 'Определить' })).toBeTruthy();
    expect(screen.getAllByText('Не определено')).toHaveLength(2);
    expect(screen.getByText(/Если включён VPN/)).toBeTruthy();
    expect(invokeMock.mock.calls.some((c) => c[0] === 'geo_lookup')).toBe(false);
  });

  it('"Определить" shows provider and place; the consent can be revoked', async () => {
    render(GeoCards);
    await fireEvent.click(await screen.findByRole('button', { name: 'Определить' }));
    await waitFor(() => expect(screen.getByText('Example ISP')).toBeTruthy());
    expect(screen.getByText('Latvia')).toBeTruthy();
    await fireEvent.click(screen.getByRole('button', { name: 'Отозвать разрешение' }));
    await waitFor(() => expect(screen.getAllByText('Не определено')).toHaveLength(2));
    expect(screen.getByRole('button', { name: 'Определить' })).toBeTruthy();
  });
});
