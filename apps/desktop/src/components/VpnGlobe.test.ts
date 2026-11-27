import { render, waitFor } from '@testing-library/svelte';
import { expect, it, vi } from 'vitest';
import VpnGlobe from './VpnGlobe.svelte';

const { mount, destroy, setVisible } = vi.hoisted(() => {
  const destroy = vi.fn();
  const setVisible = vi.fn();
  return { destroy, setVisible, mount: vi.fn(() => ({ destroy, setVisible, invalidate: vi.fn() })) };
});
vi.mock('../lib/vpnGlobe', () => ({ mountVpnGlobe: mount }));

it('keeps the same renderer across window restores and releases it on leaving VPN', async () => {
  vi.stubGlobal('ResizeObserver', class {});
  const props = { active: true, visible: true, country: null, connected: false, label: 'VPN' };
  const view = render(VpnGlobe, props);
  try {
    await waitFor(() => expect(mount).toHaveBeenCalledTimes(1));
    await view.rerender({ visible: false });
    await waitFor(() => expect(setVisible).toHaveBeenLastCalledWith(false));
    await view.rerender({ visible: true });
    await waitFor(() => expect(setVisible).toHaveBeenLastCalledWith(true));
    expect(mount).toHaveBeenCalledTimes(1);
    expect(destroy).not.toHaveBeenCalled();
    await view.rerender({ active: false });
    expect(destroy).toHaveBeenCalledTimes(1);
  } finally {
    view.unmount();
    vi.unstubAllGlobals();
  }
});
