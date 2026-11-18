import { invoke } from '@tauri-apps/api/core';
import type { TweaksOverview } from './types';

/** Настройки Windows и Boost: читает состояние; все изменения идут только через `confirmed: true`. */
export function tweaksOverview(): Promise<TweaksOverview> {
  return invoke<TweaksOverview>('tweaks_overview');
}

export function tweakSet(id: string, enabled: boolean): Promise<TweaksOverview> {
  return invoke<TweaksOverview>('tweak_set', { id, enabled, confirmed: true });
}

export function tweaksRevert(group: 'settings' | 'boost'): Promise<TweaksOverview> {
  return invoke<TweaksOverview>('tweaks_revert', { group, confirmed: true });
}

export function boostSetProfile(profile: string | null): Promise<TweaksOverview> {
  return invoke<TweaksOverview>('boost_set_profile', { profile, confirmed: true });
}
