/** Сайты пользователя для «Обхода блокировок». Проверка ввода и хранение — в Rust. */
import { invoke } from '@tauri-apps/api/core';

export const bypassSitesList = () => invoke<string[]>('bypass_sites_list');
/** Одна строка — один адрес (домен или https-ссылка). */
export const bypassSitesAdd = (text: string) => invoke<string[]>('bypass_sites_add', { text });
export const bypassSitesRemove = (domain: string) =>
  invoke<string[]>('bypass_sites_remove', { domain });

export const MAX_SITES = 200;
