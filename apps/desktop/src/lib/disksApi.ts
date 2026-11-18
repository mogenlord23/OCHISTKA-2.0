import { invoke } from '@tauri-apps/api/core';

/** Ошибка раздела «Диски». `message` уже на русском. */
export interface DisksErrorInfo {
  kind: string;
  message: string;
}

export interface CleanupCategory {
  id: string;
  title: string;
  found: boolean;
  fileCount: number;
  bytes: number;
  complete: boolean;
}

export interface MoveFailure {
  path: string;
  message: string;
}

export interface MoveOutcome {
  moved: number;
  bytes: number;
  failed: number;
  failures: MoveFailure[];
}

export interface QuarantineEntry {
  id: string;
  originalPath: string;
  size: number;
  sha256: string;
  movedAtUnix: number;
  reason: string;
  purgeToken: string;
}

export interface LargeFile {
  path: string;
  size: number;
  modifiedUnix: number;
}

export interface LargeFilesReport {
  files: LargeFile[];
  scanned: number;
  complete: boolean;
}

export interface FolderSize {
  name: string;
  path: string;
  bytes: number;
  files: number;
  complete: boolean;
}

export interface SpaceReport {
  root: string;
  folders: FolderSize[];
  looseBytes: number;
  complete: boolean;
}

export interface PrivacyItem {
  id: string;
  title: string;
  path: string;
  found: boolean;
  files: number;
  bytes: number;
  complete: boolean;
}

/** Ответ бэкенда: тип в поле `type` (см. `DisksResponse` в disks.rs). */
export type DisksResponse =
  | { type: 'cleanup'; categories: CleanupCategory[] }
  | { type: 'stopped' }
  | { type: 'cleaned'; outcome: MoveOutcome }
  | { type: 'quarantine'; entries: QuarantineEntry[] }
  | { type: 'restored'; entry: QuarantineEntry }
  | { type: 'purged'; entry: QuarantineEntry }
  | { type: 'largeFiles'; report: LargeFilesReport }
  | { type: 'space'; report: SpaceReport }
  | { type: 'privacy'; items: PrivacyItem[] }
  | { type: 'drives'; drives: string[] };

type Request =
  | { action: 'analyzeCleanup' }
  | { action: 'stopAnalysis' }
  | { action: 'cleanSelected'; ids: string[] }
  | { action: 'listQuarantine' }
  | { action: 'restore'; id: string; alternative: string | null }
  | { action: 'purge'; id: string; token: string }
  | { action: 'largeFiles' }
  | { action: 'spaceReport'; drive: string }
  | { action: 'privacyReport' }
  | { action: 'listDrives' };

async function request(payload: Request): Promise<DisksResponse> {
  return invoke<DisksResponse>('disks_request', { request: payload });
}

/** Берёт ответ нужного типа. Другой тип — ошибка контракта, а не молчаливый пустой список. */
function expect<T extends DisksResponse['type']>(
  response: DisksResponse,
  type: T,
): Extract<DisksResponse, { type: T }> {
  if (response.type !== type) {
    throw { kind: 'io', message: `неожиданный ответ: ${response.type}` } satisfies DisksErrorInfo;
  }
  return response as Extract<DisksResponse, { type: T }>;
}

/** Приводит любую ошибку вызова к виду для интерфейса. */
export function toDisksError(err: unknown): DisksErrorInfo {
  if (typeof err === 'object' && err !== null && 'kind' in err && 'message' in err) {
    const value = err as { kind: unknown; message: unknown };
    return { kind: String(value.kind), message: String(value.message) };
  }
  return { kind: 'io', message: String(err) };
}

export async function analyzeCleanup(): Promise<CleanupCategory[]> {
  return expect(await request({ action: 'analyzeCleanup' }), 'cleanup').categories;
}

export async function stopAnalysis(): Promise<void> {
  expect(await request({ action: 'stopAnalysis' }), 'stopped');
}

export async function cleanSelected(ids: string[]): Promise<MoveOutcome> {
  return expect(await request({ action: 'cleanSelected', ids }), 'cleaned').outcome;
}

export async function listQuarantine(): Promise<QuarantineEntry[]> {
  return expect(await request({ action: 'listQuarantine' }), 'quarantine').entries;
}

export async function restoreItem(
  id: string,
  alternative: string | null,
): Promise<QuarantineEntry> {
  return expect(await request({ action: 'restore', id, alternative }), 'restored').entry;
}

export async function purgeItem(id: string, token: string): Promise<QuarantineEntry> {
  return expect(await request({ action: 'purge', id, token }), 'purged').entry;
}

export async function largeFiles(): Promise<LargeFilesReport> {
  return expect(await request({ action: 'largeFiles' }), 'largeFiles').report;
}

export async function spaceReport(drive: string): Promise<SpaceReport> {
  return expect(await request({ action: 'spaceReport', drive }), 'space').report;
}

export async function privacyReport(): Promise<PrivacyItem[]> {
  return expect(await request({ action: 'privacyReport' }), 'privacy').items;
}

export async function listDrives(): Promise<string[]> {
  return expect(await request({ action: 'listDrives' }), 'drives').drives;
}
