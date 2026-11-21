import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import BypassSitesPanel from './BypassSitesPanel.svelte';

const invokeMock = vi.hoisted(() => vi.fn());
vi.mock('@tauri-apps/api/core', () => ({ invoke: invokeMock }));

let stored: string[] = [];

beforeEach(() => {
  stored = [];
  invokeMock.mockReset();
  invokeMock.mockImplementation(async (cmd: string, args?: Record<string, string>) => {
    if (cmd === 'bypass_sites_list') return [...stored];
    if (cmd === 'bypass_sites_add') {
      const lines = args!.text
        .split('\n')
        .map((l) => l.trim())
        .filter(Boolean);
      if (lines.some((l) => !/^[a-z0-9.-]+\.[a-z]{2,}$/.test(l))) {
        throw { kind: 'invalidTarget', message: 'Введите адрес сайта, например example.com' };
      }
      for (const l of lines) if (!stored.includes(l)) stored.push(l);
      return [...stored];
    }
    if (cmd === 'bypass_sites_remove') {
      stored = stored.filter((d) => d !== args!.domain);
      return [...stored];
    }
    return undefined;
  });
});
afterEach(cleanup);

describe('BypassSitesPanel', () => {
  it('is hidden when closed and visible when open, and the close button asks to close', async () => {
    const onclose = vi.fn();
    const { container, rerender } = render(BypassSitesPanel, { props: { open: false, onclose } });
    const root = container.querySelector('.sites') as HTMLElement;
    expect(root.getAttribute('aria-hidden')).toBe('true');
    expect((root as HTMLElement & { inert: boolean }).inert).toBe(true);
    await rerender({ open: true, onclose });
    expect(root.getAttribute('aria-hidden')).toBe('false');
    expect((root as HTMLElement & { inert: boolean }).inert).toBe(false);
    await fireEvent.click(screen.getByRole('button', { name: 'Закрыть' }));
    expect(onclose).toHaveBeenCalledTimes(1);
  });

  it('adds addresses, shows them in the list and removes them', async () => {
    render(BypassSitesPanel, { props: { open: true, onclose: () => {} } });
    const field = screen.getByLabelText('Адреса сайтов') as HTMLTextAreaElement;
    const add = screen.getByRole('button', { name: 'Добавить' }) as HTMLButtonElement;
    expect(add.disabled).toBe(true);
    await fireEvent.input(field, { target: { value: 'example.com\nsub.example.com' } });
    await fireEvent.click(add);
    await waitFor(() => expect(screen.getByText('example.com')).toBeTruthy());
    expect(screen.getByText('sub.example.com')).toBeTruthy();
    expect(invokeMock).toHaveBeenCalledWith('bypass_sites_add', {
      text: 'example.com\nsub.example.com',
    });
    expect(field.value).toBe('');
    await fireEvent.click(screen.getByRole('button', { name: 'Удалить example.com' }));
    await waitFor(() => expect(screen.queryByText('example.com')).toBeNull());
    expect(screen.getByText('sub.example.com')).toBeTruthy();
  });

  it('shows the plain error for rubbish and keeps what was typed', async () => {
    render(BypassSitesPanel, { props: { open: true, onclose: () => {} } });
    const field = screen.getByLabelText('Адреса сайтов') as HTMLTextAreaElement;
    await fireEvent.input(field, { target: { value: '1.2.3.4' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Добавить' }));
    expect((await screen.findByRole('alert')).textContent).toBe(
      'Введите адрес сайта, например example.com',
    );
    expect(field.value).toBe('1.2.3.4');
    expect(stored).toEqual([]);
  });

  it('keeps the list when the panel is closed and opened again', async () => {
    stored = ['example.com'];
    const onclose = () => {};
    const { rerender } = render(BypassSitesPanel, { props: { open: true, onclose } });
    await screen.findByText('example.com');
    await rerender({ open: false, onclose });
    expect(screen.getByText('example.com')).toBeTruthy();
    await rerender({ open: true, onclose });
    expect(await screen.findByText('example.com')).toBeTruthy();
  });

  it('says in one line that domains are not applied yet', () => {
    render(BypassSitesPanel, { props: { open: true, onclose: () => {} } });
    expect(screen.getByText('Домены пока не применяются')).toBeTruthy();
    expect(screen.queryByText(/вступят в силу/)).toBeNull();
  });

  it('uses plain words only', () => {
    const { container } = render(BypassSitesPanel, { props: { open: true, onclose: () => {} } });
    const text = (container.textContent ?? '').toLowerCase();
    for (const bad of [
      'замер',
      'базов',
      'служб',
      'порт',
      'драйвер',
      'sha',
      'пресет',
      'движок',
      'ip',
    ]) {
      expect(text.includes(bad), bad).toBe(false);
    }
  });
});
