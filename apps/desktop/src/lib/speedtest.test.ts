import { describe, expect, it } from 'vitest';
import {
  IDLE,
  START,
  easeToward,
  failureMessage,
  formatMbps,
  formatMs,
  gaugeFraction,
  markNoteSeen,
  noteSeen,
  reduce,
  summarize,
} from './speedtest';
import type { SpeedEvent, SpeedResult } from './speedtestApi';

const RESULT: SpeedResult = {
  downloadMbps: 120,
  uploadMbps: 40,
  pingMs: 12,
  jitterMs: 2,
  loadedPingMs: 40,
};

describe('speedtest reduce', () => {
  it('follows a whole run through the events', () => {
    const events: SpeedEvent[] = [
      { type: 'phase', phase: 'ping' },
      { type: 'ping', ms: 14 },
      { type: 'phase', phase: 'download' },
      { type: 'progress', phase: 'download', mbps: 80, fraction: 0.5 },
    ];
    let s = START;
    for (const e of events) s = reduce(s, e);
    expect(s).toMatchObject({ status: 'running', phase: 'download', liveMbps: 80, pingMs: 14 });
    s = reduce(s, { type: 'phase', phase: 'upload' });
    expect(s.downloadMbps).toBe(80);
    expect(s.liveMbps).toBe(0);
    s = reduce(s, { type: 'done', result: RESULT });
    expect(s).toMatchObject({
      status: 'done',
      downloadMbps: 120,
      uploadMbps: 40,
      pingMs: 12,
      jitterMs: 2,
      loadedMs: 40,
    });
  });

  it('failure and cancel end the run without a result', () => {
    expect(reduce(START, { type: 'failed', kind: 'offline' })).toMatchObject({
      status: 'failed',
      failure: 'offline',
    });
    expect(reduce({ ...START, liveMbps: 50 }, { type: 'cancelled' })).toMatchObject({
      status: 'cancelled',
      liveMbps: 0,
    });
    expect(IDLE.status).toBe('idle');
  });

  it('does not mutate the previous state', () => {
    const before = { ...START };
    reduce(START, { type: 'ping', ms: 5 });
    expect(START).toEqual(before);
  });
});

describe('speedtest formats and scale', () => {
  it('gauge scale is logarithmic and clamped', () => {
    expect(gaugeFraction(0)).toBe(0);
    expect(gaugeFraction(-5)).toBe(0);
    expect(gaugeFraction(Number.NaN)).toBe(0);
    expect(gaugeFraction(1000)).toBeCloseTo(1, 6);
    expect(gaugeFraction(5000)).toBe(1);
    expect(gaugeFraction(100)).toBeGreaterThan(gaugeFraction(10));
    expect(gaugeFraction(10)).toBeGreaterThan(0.3);
  });

  it('formats numbers for display', () => {
    expect(formatMbps(null)).toBe('—');
    expect(formatMbps(5.234)).toBe('5.2');
    expect(formatMbps(99.94)).toBe('99.9');
    expect(formatMbps(412.6)).toBe('413');
    expect(formatMs(4.26)).toBe('4.3');
    expect(formatMs(37.4)).toBe('37');
    expect(formatMs(null)).toBe('—');
  });

  it('smooths numbers towards the target and settles on it', () => {
    const step = easeToward(0, 100, 16);
    expect(step).toBeGreaterThan(0);
    expect(step).toBeLessThan(100);
    expect(easeToward(99.99, 100, 16)).toBe(100);
    let v = 0;
    for (let i = 0; i < 200; i++) v = easeToward(v, 100, 16);
    expect(v).toBe(100);
  });

  it('error messages are plain, with no codes', () => {
    expect(failureMessage('offline')).toMatch(/Нет подключения к интернету/);
    expect(failureMessage('server')).toMatch(/Сервер измерения/);
    expect(failureMessage(null)).toMatch(/Сервер измерения/);
  });
});

describe('speedtest summary', () => {
  it('rates the connection in words', () => {
    expect(summarize(RESULT).title).toBe('Отличное соединение');
    expect(summarize({ ...RESULT, downloadMbps: 40 }).title).toBe('Хорошее соединение');
    expect(summarize({ ...RESULT, downloadMbps: 10 }).title).toBe('Скромная скорость');
    expect(summarize({ ...RESULT, downloadMbps: 2 }).title).toBe('Медленное соединение');
  });

  it('warns about load latency, jitter and weak upload', () => {
    const text = summarize({
      ...RESULT,
      uploadMbps: 1,
      jitterMs: 50,
      loadedPingMs: 400,
    }).text;
    expect(text).toMatch(/Отдача низкая/);
    expect(text).toMatch(/задержка сильно растёт/);
    expect(text).toMatch(/нестабильна/);
    expect(summarize(RESULT).text).not.toMatch(/нестабильна|лагать/);
  });
});

describe('speedtest note', () => {
  it('is remembered once seen', () => {
    localStorage.clear();
    expect(noteSeen()).toBe(false);
    markNoteSeen();
    expect(noteSeen()).toBe(true);
  });
});
