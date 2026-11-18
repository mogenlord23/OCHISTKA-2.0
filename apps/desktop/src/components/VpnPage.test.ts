import { afterEach, describe, expect, it } from 'vitest';
import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import VpnPage from './VpnPage.svelte';
import {
  IMPORT_REASON,
  NETWORK_CHANGE_REASON,
  VPN_CONNECT_REASON,
  VPN_TUN_MODE_REASON,
} from '../lib/vpn';

afterEach(cleanup);

const SAMPLE = [
  'vless://uuid@1.2.3.4:443#Germany',
  'trojan://pass@5.6.7.8:443#Finland',
  'banner line',
].join('\n');

describe('VpnPage: подписка и импорт', () => {
  it('shows the subscription card with an enabled import button', () => {
    render(VpnPage, { props: { subsection: 'subscription' } });
    expect(screen.getByText('Ссылка подписки / импорта')).toBeTruthy();
    const importButton = screen.getByRole('button', { name: 'Импорт' }) as HTMLButtonElement;
    expect(importButton.disabled).toBe(false);
  });

  it('rejects a non-https link with the https reason', async () => {
    render(VpnPage, { props: { subsection: 'subscription' } });
    const input = screen.getByLabelText('Ссылка подписки') as HTMLInputElement;
    await fireEvent.input(input, { target: { value: 'http://example.com/sub' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Импорт' }));
    expect(screen.getByText('Разрешены только ссылки https://')).toBeTruthy();
    expect(screen.queryByText(IMPORT_REASON)).toBeNull();
  });

  it('shows the not-connected import reason for a valid https link', async () => {
    render(VpnPage, { props: { subsection: 'subscription' } });
    const input = screen.getByLabelText('Ссылка подписки') as HTMLInputElement;
    await fireEvent.input(input, { target: { value: 'https://example.com/sub' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Импорт' }));
    expect(screen.getByText(IMPORT_REASON)).toBeTruthy();
  });

  it('parses pasted subscription text and lists the servers on the servers page', async () => {
    const { unmount } = render(VpnPage, { props: { subsection: 'subscription' } });
    const textarea = screen.getByLabelText('Текст подписки') as HTMLTextAreaElement;
    await fireEvent.input(textarea, { target: { value: SAMPLE } });
    await fireEvent.click(screen.getByRole('button', { name: 'Разобрать' }));
    expect(screen.getByText('Серверов: 2. Не распознано строк: 1.')).toBeTruthy();
    unmount();
  });
});

describe('VpnPage: серверы', () => {
  it('shows the connect button off, the timer at zero, and the TUN reason on click', async () => {
    render(VpnPage, { props: { subsection: 'servers' } });
    const toggle = screen.getByRole('button', { name: 'Подключить' });
    expect(toggle.getAttribute('aria-pressed')).toBe('false');
    expect(screen.getByLabelText('Время подключения').textContent).toBe('00:00:00');

    await fireEvent.click(toggle);
    expect(screen.getByText(VPN_CONNECT_REASON)).toBeTruthy();
    expect(toggle.getAttribute('aria-pressed')).toBe('false');
    expect(screen.getByLabelText('Время подключения').textContent).toBe('00:00:00');
  });

  it('shows TUN as a disabled mode with its reason', () => {
    render(VpnPage, { props: { subsection: 'servers' } });
    const tun = screen.getByRole('radio', { name: 'TUN' }) as HTMLInputElement;
    expect(tun.disabled).toBe(true);
    expect(screen.getByText(VPN_TUN_MODE_REASON)).toBeTruthy();
  });

  it('explains how to get servers when none were parsed', () => {
    render(VpnPage, { props: { subsection: 'servers' } });
    expect(screen.getByText(/Серверов пока нет/)).toBeTruthy();
  });
});

describe('VpnPage: маршруты и «Дополнительно»', () => {
  it('shows Zapret and Xbox DNS switches, both off with status text', () => {
    render(VpnPage, { props: { subsection: 'routes' } });
    expect(screen.getByText('Дополнительно')).toBeTruthy();
    for (const name of ['Zapret', 'Xbox DNS']) {
      const sw = screen.getByRole('switch', { name }) as HTMLButtonElement;
      expect(sw.getAttribute('aria-checked')).toBe('false');
      expect(sw.closest('.switch-row')?.textContent).toContain('выключено');
    }
  });

  it('does not show GoodbyeDPI', () => {
    render(VpnPage, { props: { subsection: 'routes' } });
    expect(screen.queryByText(/GoodbyeDPI/i)).toBeNull();
  });

  it('turning a switch on shows the network reason and leaves it off', async () => {
    render(VpnPage, { props: { subsection: 'routes' } });
    const zapret = screen.getByRole('switch', { name: 'Zapret' });
    await fireEvent.click(zapret);
    expect(screen.getByText(NETWORK_CHANGE_REASON)).toBeTruthy();
    expect(zapret.getAttribute('aria-checked')).toBe('false');
  });
});
