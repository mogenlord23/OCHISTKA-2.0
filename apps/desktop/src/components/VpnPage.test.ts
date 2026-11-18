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

  it('parses pasted subscription text and lists servers with count and selection', async () => {
    render(VpnPage, { props: { subsection: 'subscription' } });
    const input = screen.getByLabelText('Ссылка подписки') as HTMLInputElement;
    // Однострочное поле: строки подписки приходят через пробел.
    await fireEvent.input(input, { target: { value: SAMPLE.split('\n').slice(0, 2).join(' ') } });
    await fireEvent.click(screen.getByRole('button', { name: 'Импорт' }));
    expect(screen.getByText('Серверов: 2. Не распознано строк: 0.')).toBeTruthy();
    expect(screen.getByText('Серверы / 2')).toBeTruthy();
    const second = screen.getByRole('button', { name: /Finland/ });
    await fireEvent.click(second);
    expect(second.getAttribute('aria-pressed')).toBe('true');
  });

  it('shows the country code from a flag emoji', async () => {
    render(VpnPage, { props: { subsection: 'subscription' } });
    const input = screen.getByLabelText('Ссылка подписки') as HTMLInputElement;
    await fireEvent.input(input, { target: { value: 'vless://a@1:1#🇱🇻 Латвия' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Импорт' }));
    expect(screen.getByText('LV')).toBeTruthy();
    expect(screen.getByText('Латвия')).toBeTruthy();
  });

  it('«Маршруты и исключения» opens the advanced view', async () => {
    render(VpnPage, { props: { subsection: 'subscription' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Маршруты и исключения' }));
    expect(screen.getByRole('switch', { name: 'Zapret' })).toBeTruthy();
  });
});

describe('VpnPage: подключение и серверы', () => {
  it('shows the connect toggle off, the timer at zero, and the TUN reason on click', async () => {
    render(VpnPage, { props: { subsection: 'servers' } });
    const toggle = screen.getByRole('button', { name: 'Подключить' });
    expect(toggle.getAttribute('aria-pressed')).toBe('false');
    expect(screen.getByLabelText('Время подключения').textContent).toBe('00:00:00');

    await fireEvent.click(toggle);
    expect(screen.getByText(VPN_CONNECT_REASON)).toBeTruthy();
    expect(toggle.getAttribute('aria-pressed')).toBe('false');
    expect(screen.getByLabelText('Время подключения').textContent).toBe('00:00:00');
  });

  it('shows the TUN mode as a disabled dropdown with its reason', () => {
    render(VpnPage, { props: { subsection: 'servers' } });
    const tun = screen.getByRole('combobox', { name: 'Режим' }) as HTMLSelectElement;
    expect(tun.disabled).toBe(true);
    expect(screen.getByText(VPN_TUN_MODE_REASON)).toBeTruthy();
  });

  it('shows an honest empty state when there are no servers', () => {
    render(VpnPage, { props: { subsection: 'servers' } });
    expect(screen.getByText('Серверы / 0')).toBeTruthy();
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

  it('clicking a switch opens a confirmation dialog and changes nothing yet', async () => {
    render(VpnPage, { props: { subsection: 'routes' } });
    const zapret = screen.getByRole('switch', { name: 'Zapret' });
    await fireEvent.click(zapret);
    const dialog = screen.getByRole('dialog', { name: 'Включить Zapret?' });
    expect(dialog).toBeTruthy();
    expect(screen.queryByText(NETWORK_CHANGE_REASON)).toBeNull();
    expect(zapret.getAttribute('aria-checked')).toBe('false');
  });

  it('confirming runs the stub: shows the network reason and leaves the switch off', async () => {
    render(VpnPage, { props: { subsection: 'routes' } });
    const xbox = screen.getByRole('switch', { name: 'Xbox DNS' });
    await fireEvent.click(xbox);
    await fireEvent.click(screen.getByRole('button', { name: 'Включить' }));
    expect(screen.queryByRole('dialog')).toBeNull();
    expect(screen.getByText(NETWORK_CHANGE_REASON)).toBeTruthy();
    expect(xbox.getAttribute('aria-checked')).toBe('false');
  });

  it('cancelling closes the dialog and shows no network reason', async () => {
    render(VpnPage, { props: { subsection: 'routes' } });
    await fireEvent.click(screen.getByRole('switch', { name: 'Zapret' }));
    await fireEvent.click(screen.getByRole('button', { name: 'Отмена' }));
    expect(screen.queryByRole('dialog')).toBeNull();
    expect(screen.queryByText(NETWORK_CHANGE_REASON)).toBeNull();
  });

  it('Escape closes the dialog without any change', async () => {
    render(VpnPage, { props: { subsection: 'routes' } });
    await fireEvent.click(screen.getByRole('switch', { name: 'Zapret' }));
    await fireEvent.keyDown(screen.getByRole('dialog'), { key: 'Escape' });
    expect(screen.queryByRole('dialog')).toBeNull();
    expect(screen.queryByText(NETWORK_CHANGE_REASON)).toBeNull();
  });
});
