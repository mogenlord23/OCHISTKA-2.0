import { describe, expect, it, vi } from 'vitest';

const invokeMock = vi.hoisted(() => vi.fn());
vi.mock('@tauri-apps/api/core', () => ({ invoke: invokeMock }));

import { prepareCursorSchemes } from './appearanceApi';

describe('prepareCursorSchemes', () => {
  it('asks the backend to pre-draw schemes once per launch', async () => {
    invokeMock.mockResolvedValue({ type: 'status', status: {} });
    const schemes = [{ id: 'cursor-red', color: '#f00', secondary: '#f00', pattern: 'plain' }];
    prepareCursorSchemes(schemes);
    prepareCursorSchemes(schemes);
    await Promise.resolve();
    expect(invokeMock).toHaveBeenCalledTimes(1);
    expect(invokeMock).toHaveBeenCalledWith('appearance_request', {
      request: { action: 'prepareCursorSchemes', schemes },
    });
  });
});

describe('prepareCursorScheme (hover)', () => {
  it('sends one scheme once per launch', async () => {
    vi.resetModules();
    const invoke = vi.fn().mockResolvedValue({ kind: 'status', status: {} });
    vi.doMock('@tauri-apps/api/core', () => ({ invoke }));
    const api = await import('./appearanceApi');
    const scheme = { id: 'cursor-x', color: '#FF0000', secondary: '#00FF00', pattern: 'plain' };
    api.prepareCursorScheme(scheme);
    api.prepareCursorScheme(scheme);
    expect(invoke).toHaveBeenCalledTimes(1);
    expect(invoke).toHaveBeenCalledWith('appearance_request', {
      request: { action: 'prepareCursorSchemes', schemes: [scheme] },
    });
  });
});
