import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { cleanup, render, waitFor } from '@testing-library/svelte';
import { mockIPC, clearMocks } from '@tauri-apps/api/mocks';
import AppIcon from './AppIcon.svelte';
import { getIcon, getProgramIcon } from '../lib/programsApi';
import { resetIconCache } from '../lib/iconLoader';

/** Настоящий PNG 1x1: проверяем весь путь «ответ бэкенда → адрес → img». */
const PNG_URL =
  'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==';

beforeEach(() => resetIconCache());
afterEach(() => {
  cleanup();
  clearMocks();
});

describe('AppIcon', () => {
  it('shows the picture from a real data URL returned by the backend', async () => {
    mockIPC((cmd, args) => {
      expect(cmd).toBe('programs_request');
      expect(args).toEqual({ request: { action: 'programIcon', id: 'chrome' } });
      return { type: 'icon', dataUrl: PNG_URL };
    });
    const { container } = render(AppIcon, {
      name: 'Chrome',
      cacheKey: 'program:chrome',
      resolve: () => getProgramIcon('chrome'),
    });
    await waitFor(() => expect(container.querySelector('img')?.getAttribute('src')).toBe(PNG_URL));
  });

  it('asks for a file icon by path for processes', async () => {
    mockIPC((_cmd, args) => {
      expect(args).toEqual({ request: { action: 'icon', path: 'C:\\A\\a.exe' } });
      return { type: 'icon', dataUrl: PNG_URL };
    });
    expect(await getIcon('C:\\A\\a.exe')).toBe(PNG_URL);
  });

  it('draws a glyph placeholder, never an empty box, when there is no icon', async () => {
    const { container } = render(AppIcon, {
      name: 'Old Tool',
      cacheKey: 'program:none',
      resolve: async () => null,
    });
    await waitFor(() => expect(container.querySelector('svg.glyph')).not.toBeNull());
    expect(container.querySelector('img')).toBeNull();
  });

  it('falls back to the placeholder when the picture fails to decode', async () => {
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
    await waitFor(() => expect(container.querySelector('svg.glyph')).not.toBeNull());
  });
});
