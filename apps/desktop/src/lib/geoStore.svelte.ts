import { invoke } from '@tauri-apps/api/core';
import type { GeoConsent } from './generated/GeoConsent';
import type { GeoInfo } from './generated/GeoInfo';

export type { GeoInfo };

/** Нейтральный текст, когда сведений нет. */
export const GEO_UNDETERMINED = 'Не определено';

export const GEO_VPN_NOTE =
  'Если включён VPN, будет виден адрес VPN-сервера: провайдер и расположение покажут его, а не твои.';

export const GEO_CONSENT_TEXT =
  'Чтобы определить провайдера и расположение, приложение один раз отправит запрос во внешний сервис определения IP (по https, без личных данных). Разрешение можно отозвать.';

interface GeoState {
  /** `null` — ещё не узнали, есть ли разрешение. */
  consent: boolean | null;
  info: GeoInfo | null;
  loading: boolean;
  error: string | null;
}

export const geo = $state<GeoState>({ consent: null, info: null, loading: false, error: null });

/** Один автоматический запрос за запуск, если разрешение уже есть. */
let autoDone = false;

function message(err: unknown): string {
  if (typeof err === 'object' && err !== null && 'message' in err) {
    return String((err as { message: unknown }).message);
  }
  return String(err);
}

/** Читает разрешение (запрос в сеть не уходит). Если оно есть, один раз определяет сразу. */
export async function initGeo(): Promise<void> {
  if (geo.consent === null) {
    try {
      geo.consent = (await invoke<GeoConsent>('geo_consent')).granted;
    } catch {
      geo.consent = false;
    }
  }
  if (geo.consent && !autoDone && !geo.info) {
    autoDone = true;
    await lookupGeo();
  }
}

/** Сам запрос; бэкенд без разрешения его не выполнит. */
export async function lookupGeo(): Promise<void> {
  if (geo.loading) return;
  geo.loading = true;
  geo.error = null;
  try {
    geo.info = await invoke<GeoInfo>('geo_lookup');
  } catch (err) {
    geo.error = message(err);
  } finally {
    geo.loading = false;
  }
}

/** Кнопка «Определить»: выдаёт разрешение (запоминается) и определяет. */
export async function determineGeo(): Promise<void> {
  geo.error = null;
  try {
    geo.consent = (await invoke<GeoConsent>('geo_set_consent', { granted: true })).granted;
  } catch (err) {
    geo.error = message(err);
    return;
  }
  autoDone = true;
  await lookupGeo();
}

/** Отзывает разрешение и забывает показанное. */
export async function revokeGeo(): Promise<void> {
  try {
    geo.consent = (await invoke<GeoConsent>('geo_set_consent', { granted: false })).granted;
    geo.info = null;
    geo.error = null;
  } catch (err) {
    geo.error = message(err);
  }
}

/** Для тестов. */
export function resetGeo(): void {
  geo.consent = null;
  geo.info = null;
  geo.loading = false;
  geo.error = null;
  autoDone = false;
}
