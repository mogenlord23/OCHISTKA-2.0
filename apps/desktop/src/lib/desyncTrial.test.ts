import { describe, expect, it } from 'vitest';
import { bypassView, type TrialStatus } from './desyncTrial';

const BAD = ['замер', 'базов', 'служб', 'порт', 'sha', 'пресет', 'движок'];
const ALLOWED_DRIVER = 'Драйвер не остановлен, перезагрузите компьютер';

const base: TrialStatus = {
  active: false,
  message: 'Выключено',
  unavailable: null,
};

function plain(text: string) {
  const t = text.toLowerCase();
  for (const bad of BAD) expect(t.includes(bad), `${text}: ${bad}`).toBe(false);
  if (t.includes('драйвер')) expect(text).toBe(ALLOWED_DRIVER);
}

describe('texts of the bypass switch', () => {
  it('contain no technical words in any state', () => {
    const views = [
      bypassView(null, null, null),
      bypassView(base, null, null),
      bypassView(base, 'on', null),
      bypassView(base, 'off', null),
      bypassView({ ...base, active: true }, null, null),
      bypassView({ ...base, error: ALLOWED_DRIVER }, null, null),
      bypassView(base, null, 'Нужны права администратора'),
      bypassView({ ...base, unavailable: 'Нужны права администратора' }, null, null),
    ];
    for (const v of views) {
      plain(v.text);
      if (v.hint) plain(v.hint);
    }
  });

  it('shows «Включаем…» and «Выключаем…», never «Проверяем…», while Rust works', () => {
    expect(bypassView(base, 'on', null).text).toBe('Включаем…');
    expect(bypassView(base, 'off', null).text).toBe('Выключаем…');
  });
});
