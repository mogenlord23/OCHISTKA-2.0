import { describe, expect, it } from 'vitest';
import { render, screen } from '@testing-library/svelte';
import VpnPage from './VpnPage.svelte';

describe('VpnPage', () => {
  it('shows the subscription import card with its controls disabled', () => {
    render(VpnPage, { props: { subsection: 'subscription' } });
    expect(screen.getByText('Ссылка подписки / импорта')).toBeTruthy();
    const importButton = screen.getByRole('button', { name: 'Импорт' }) as HTMLButtonElement;
    expect(importButton.disabled).toBe(true);
    expect(screen.getByText('Подключение пока не реализовано.')).toBeTruthy();
  });

  it('shows a placeholder for the servers subsection', () => {
    render(VpnPage, { props: { subsection: 'servers' } });
    expect(screen.getByText('В разработке')).toBeTruthy();
  });
});
