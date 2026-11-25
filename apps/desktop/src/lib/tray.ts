import { invoke, isTauri } from '@tauri-apps/api/core';
import { listen, type EventCallback } from '@tauri-apps/api/event';
import type { SectionId } from './sections';

export type TrayAction = SectionId | 'open' | 'toggleVpn' | 'exit';

export interface TrayStatus {
  vpnConnected: boolean;
}

export function trayAction(action: TrayAction): Promise<void> {
  return invoke('tray_action', { action });
}

export function trayStatus(): Promise<TrayStatus> {
  return invoke('tray_status');
}

export function trayHover(inside: boolean): void {
  void invoke('tray_hover', { inside }).catch(() => {});
}

export function listenDesktop<T>(event: string, handler: EventCallback<T>): () => void {
  if (!isTauri()) return () => {};
  const subscription = listen(event, handler).catch(() => undefined);
  return () => { void subscription.then((stop) => stop?.()); };
}
