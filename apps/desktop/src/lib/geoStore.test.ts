import { beforeEach, describe, expect, it, vi } from 'vitest';

const invokeMock = vi.hoisted(() => vi.fn());
vi.mock('@tauri-apps/api/core', () => ({ invoke: invokeMock }));

import { determineGeo, geo, initGeo, resetGeo, revokeGeo } from './geoStore.svelte';

const INFO = { provider: 'Example ISP', country: 'Latvia', city: 'Riga' };

function backend(consent: boolean) {
  let granted = consent;
  invokeMock.mockImplementation(async (cmd: string, args?: { granted?: boolean }) => {
    if (cmd === 'geo_consent') return { granted };
    if (cmd === 'geo_set_consent') {
      granted = Boolean(args?.granted);
      return { granted };
    }
    if (cmd === 'geo_lookup') {
      if (!granted) throw { kind: 'consentRequired', message: 'Сначала нужно разрешение' };
      return INFO;
    }
  });
}

const called = (cmd: string) => invokeMock.mock.calls.filter((c) => c[0] === cmd).length;

beforeEach(() => {
  invokeMock.mockReset();
  resetGeo();
});

describe('geoStore: согласие на внешний запрос', () => {
  it('without consent opening the tab sends nothing to the network', async () => {
    backend(false);
    await initGeo();
    expect(geo.consent).toBe(false);
    expect(called('geo_lookup')).toBe(0);
    expect(geo.info).toBeNull();
  });

  it('"Определить" grants consent once and looks up', async () => {
    backend(false);
    await initGeo();
    await determineGeo();
    expect(called('geo_set_consent')).toBe(1);
    expect(invokeMock).toHaveBeenCalledWith('geo_set_consent', { granted: true });
    expect(called('geo_lookup')).toBe(1);
    expect(geo.info).toEqual(INFO);
  });

  it('remembered consent looks up once per run', async () => {
    backend(true);
    await initGeo();
    await initGeo();
    expect(called('geo_lookup')).toBe(1);
    expect(geo.info).toEqual(INFO);
  });

  it('revoking forgets the data and the next open sends nothing', async () => {
    backend(true);
    await initGeo();
    await revokeGeo();
    expect(geo.consent).toBe(false);
    expect(geo.info).toBeNull();
    resetGeo();
    await initGeo();
    expect(called('geo_lookup')).toBe(1);
  });

  it('lookup errors show a message and keep data undetermined', async () => {
    invokeMock.mockImplementation(async (cmd: string) => {
      if (cmd === 'geo_consent') return { granted: true };
      throw { kind: 'offline', message: 'Нет подключения к интернету' };
    });
    await initGeo();
    expect(geo.error).toBe('Нет подключения к интернету');
    expect(geo.info).toBeNull();
  });
});
