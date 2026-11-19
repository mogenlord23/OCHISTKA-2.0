import { invoke } from '@tauri-apps/api/core';
import type { ActivationInfo, TweaksOverview } from './types';

/** Настройки Windows и Boost: все изменения применяются сразу. Подтверждение нужно только для
    опасной настройки (VBS/HVCI), её окно показывает интерфейс и передаёт `confirmed: true`. */
export function tweaksOverview(): Promise<TweaksOverview> {
  return invoke<TweaksOverview>('tweaks_overview');
}

export function tweakSet(id: string, enabled: boolean, confirmed = false): Promise<TweaksOverview> {
  return invoke<TweaksOverview>('tweak_set', { id, enabled, confirmed });
}

export function tweaksRevert(group: 'settings' | 'boost'): Promise<TweaksOverview> {
  return invoke<TweaksOverview>('tweaks_revert', { group });
}

export function boostSetProfile(profile: string | null): Promise<TweaksOverview> {
  return invoke<TweaksOverview>('boost_set_profile', { profile });
}

/** Статус активации Windows (запрос идёт в фоне). */
export function windowsActivation(): Promise<ActivationInfo> {
  return invoke<ActivationInfo>('windows_activation');
}

/** Открывает «Параметры → Активация». */
export function openActivationSettings(): Promise<void> {
  return invoke<void>('open_activation_settings');
}
