import { describe, expect, it } from 'vitest';
import {
  buttonLabel,
  fillPercent,
  IDLE,
  isBusy,
  stateFromProgress,
  type ButtonState,
} from './driverInstall';

describe('driver button states', () => {
  it('idle label depends on the action', () => {
    expect(buttonLabel('install', IDLE)).toBe('Установить');
    expect(buttonLabel('openSite', IDLE)).toBe('Открыть сайт');
  });

  it('shows download percent, with a fallback when the size is unknown', () => {
    expect(buttonLabel('install', { kind: 'downloading', percent: 42 })).toBe('Загрузка 42%');
    expect(buttonLabel('install', { kind: 'downloading', percent: null })).toBe('Загрузка…');
    expect(fillPercent({ kind: 'downloading', percent: 42 })).toBe(42);
    expect(fillPercent({ kind: 'downloading', percent: 400 })).toBe(100);
    expect(fillPercent({ kind: 'downloading', percent: null })).toBe(0);
    expect(fillPercent(IDLE)).toBe(0);
  });

  it('labels verifying, launching and done', () => {
    expect(buttonLabel('install', { kind: 'verifying' })).toBe('Проверка подписи…');
    expect(buttonLabel('install', { kind: 'launching' })).toBe('Запуск установщика…');
    expect(buttonLabel('install', { kind: 'done' })).toBe('Установщик запущен');
    expect(buttonLabel('openSite', { kind: 'done' })).toBe('Страница открыта');
  });

  it('error state offers a retry and carries the reason', () => {
    const state = stateFromProgress({
      id: 'x',
      stage: 'error',
      percent: null,
      message: 'Подпись принадлежит другому издателю: Somebody',
    });
    expect(state).toEqual({
      kind: 'error',
      reason: 'Подпись принадлежит другому издателю: Somebody',
    });
    expect(buttonLabel('install', state)).toBe('Повторить');
    expect(stateFromProgress({ id: 'x', stage: 'error', percent: null, message: null })).toEqual({
      kind: 'error',
      reason: 'Не удалось установить драйвер.',
    });
  });

  it('maps backend stages to states and tells busy from idle', () => {
    const map = (stage: Parameters<typeof stateFromProgress>[0]['stage'], percent = null) =>
      stateFromProgress({ id: 'x', stage, percent, message: null });
    expect(map('downloading', 7 as never)).toEqual({ kind: 'downloading', percent: 7 });
    expect(map('verifying')).toEqual({ kind: 'verifying' });
    expect(map('launching')).toEqual({ kind: 'launching' });
    expect(map('done')).toEqual({ kind: 'done' });
    const busy: ButtonState[] = [
      { kind: 'downloading', percent: 1 },
      { kind: 'verifying' },
      { kind: 'launching' },
    ];
    for (const state of busy) expect(isBusy(state)).toBe(true);
    for (const state of [IDLE, { kind: 'done' }, { kind: 'error', reason: 'x' }] as ButtonState[]) {
      expect(isBusy(state)).toBe(false);
    }
  });
});
