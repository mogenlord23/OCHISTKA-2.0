import { Channel, invoke } from '@tauri-apps/api/core';
import { VENDOR_ORDER, type DriverAction, type DriverItem, type VendorId } from './driversCatalog';
import type { InstallProgress, InstallStage } from './driverInstall';

const STAGES: readonly InstallStage[] = ['downloading', 'verifying', 'launching', 'done', 'error'];

function isItem(value: unknown): value is DriverItem {
  if (typeof value !== 'object' || value === null) {
    return false;
  }
  const item = value as Record<string, unknown>;
  const actions: readonly DriverAction[] = ['install', 'openSite'];
  return (
    typeof item.id === 'string' &&
    typeof item.title === 'string' &&
    typeof item.description === 'string' &&
    typeof item.windowsUpdate === 'boolean' &&
    (VENDOR_ORDER as readonly string[]).includes(item.vendor as string) &&
    (actions as readonly string[]).includes(item.action as string)
  );
}

export function isInstallProgress(value: unknown): value is InstallProgress {
  if (typeof value !== 'object' || value === null) {
    return false;
  }
  const event = value as Record<string, unknown>;
  return (
    typeof event.id === 'string' &&
    (STAGES as readonly string[]).includes(event.stage as string) &&
    (event.percent === null || typeof event.percent === 'number') &&
    (event.message === null || typeof event.message === 'string')
  );
}

export async function getDriversCatalog(): Promise<DriverItem[]> {
  const result: unknown = await invoke('drivers_catalog');
  if (!Array.isArray(result) || !result.every(isItem)) {
    throw { kind: 'probe', message: 'Каталог драйверов имеет неожиданную форму' };
  }
  return result as DriverItem[];
}

/** Скачивает официальный установщик, проверяет подпись и запускает его. Ход — в `onProgress`. */
export async function installDriver(
  id: string,
  onProgress: (event: InstallProgress) => void,
): Promise<void> {
  const channel = new Channel<unknown>();
  channel.onmessage = (event) => {
    if (isInstallProgress(event)) {
      onProgress(event);
    }
  };
  await invoke('driver_install', { id, onProgress: channel });
}

export function openDriverPage(id: string): Promise<void> {
  return invoke('driver_open_page', { id });
}

export function openWindowsUpdate(): Promise<void> {
  return invoke('driver_open_windows_update');
}

export type { VendorId };
