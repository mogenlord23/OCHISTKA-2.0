import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { cleanup, render, waitFor } from '@testing-library/svelte';
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks';
import AppIcon from './AppIcon.svelte';
import { getProgramIcon, prefetchProgramIcons, programIconKey } from '../lib/programsApi';
import { resetIconCache } from '../lib/iconLoader';

/** Настоящий PNG 1x1: проверяем весь путь «ответ бэкенда -> адрес -> img». */
const PNG_URL =
  'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==';

beforeEach(() => resetIconCache());
afterEach(() => {
  cleanup();
  clearMocks();
});

describe('AppIcon', () => {
  it('shows the real picture returned by the backend and no letter', async () => {
    mockIPC((cmd, args) => {
      expect(cmd).toBe('programs_request');
      expect(args).toEqual({ request: { action: 'programIcons', ids: ['qbittorrent'] } });
      return { type: 'iconBatch', items: [{ key: 'qbittorrent', dataUrl: PNG_URL }] };
    });
    const { container } = render(AppIcon, {
      name: 'qBittorrent',
      cacheKey: programIconKey('qbittorrent'),
      resolve: () => getProgramIcon('qbittorrent'),
    });
    await waitFor(() => expect(container.querySelector('img')?.getAttribute('src')).toBe(PNG_URL));
    expect(container.textContent?.trim()).toBe('');
  });

  it('draws the letter only when the backend returns no icon', async () => {
    mockIPC(() => ({ type: 'iconBatch', items: [{ key: 'old', dataUrl: null }] }));
    const { container } = render(AppIcon, {
      name: 'Old Tool',
      cacheKey: programIconKey('old'),
      resolve: () => getProgramIcon('old'),
    });
    await waitFor(() => expect(container.textContent?.trim()).toBe('O'));
    expect(container.querySelector('img')).toBeNull();
  });

  it('draws the letter when the picture fails to decode', async () => {
    const { container } = render(AppIcon, {
      name: 'Broken',
      cacheKey: 'program:broken',
      resolve: async () => 'data:image/png;base64,AAAA',
    });
    const img = await waitFor(() => {
      const found = container.querySelector('img');
      expect(found).not.toBeNull();
      return found as HTMLImageElement;
    });
    img.dispatchEvent(new Event('error'));
    await waitFor(() => expect(container.textContent?.trim()).toBe('B'));
  });

  it('draws the letter when no source is given', () => {
    const { container } = render(AppIcon, { name: 'Zeta' });
    expect(container.textContent?.trim()).toBe('Z');
  });

  it('uses an icon prefetched in the background without asking again', async () => {
    let calls = 0;
    mockIPC((_cmd, args) => {
      calls++;
      const ids = (args as { request: { ids: string[] } }).request.ids;
      return { type: 'iconBatch', items: ids.map((key) => ({ key, dataUrl: PNG_URL })) };
    });
    prefetchProgramIcons([{ id: 'a' }, { id: 'b' }]);
    const { container } = render(AppIcon, {
      name: 'A',
      cacheKey: programIconKey('a'),
      resolve: () => getProgramIcon('a'),
    });
    await waitFor(() => expect(container.querySelector('img')?.getAttribute('src')).toBe(PNG_URL));
    expect(calls).toBe(1);
  });
});
