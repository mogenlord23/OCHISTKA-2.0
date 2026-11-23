import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/svelte';

const invokeMock = vi.hoisted(() => vi.fn());
vi.mock('@tauri-apps/api/core', () => ({ invoke: invokeMock }));

import GeoCards from './GeoCards.svelte';
import { GEO_CONSENT_TEXT, resetGeo } from '../lib/geoStore.svelte';

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

  it('groups both cards, the full-width action and consent information in one block', async () => {
    const { container } = render(GeoCards);
    const section = screen.getByRole('region', { name: 'Провайдер и расположение' });
    const button = await screen.findByRole('button', { name: 'Определить' });
    expect(section.contains(screen.getByTestId('geo-provider'))).toBe(true);
    expect(section.contains(screen.getByTestId('geo-place'))).toBe(true);
    expect(section.contains(button)).toBe(true);
    expect(screen.getByText(GEO_CONSENT_TEXT)).toBeTruthy();
    expect(container.querySelectorAll('.icon')).toHaveLength(2);
  });

  it('shows loading, long results and an error on refresh', async () => {
    let completeLookup: (value: unknown) => void = () => {};
    invokeMock.mockImplementation(async (cmd: string) => {
      if (cmd === 'geo_consent') return { granted: false };
      if (cmd === 'geo_set_consent') return { granted: true };
      if (cmd === 'geo_lookup') return new Promise((resolve) => (completeLookup = resolve));
    });
    render(GeoCards);
    await fireEvent.click(await screen.findByRole('button', { name: 'Определить' }));
    expect(
      (await screen.findByRole('button', { name: 'Определяем…' })).hasAttribute('disabled'),
    ).toBe(true);
    const provider = 'Очень длинное название интернет-провайдера для проверки переноса';
    const city = 'Очень длинное название города для проверки переноса';
    completeLookup({ provider, city, country: 'Страна' });
    await screen.findByText(provider);
    expect(screen.getByText(`${city}, Страна`)).toBeTruthy();
    invokeMock.mockImplementation(async (cmd: string) => {
      if (cmd === 'geo_lookup') throw new Error('Сервис недоступен');
    });
    await fireEvent.click(screen.getByRole('button', { name: 'Обновить' }));
    expect((await screen.findByRole('alert')).textContent).toBe('Сервис недоступен');
    expect(screen.getByText(provider)).toBeTruthy();
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
