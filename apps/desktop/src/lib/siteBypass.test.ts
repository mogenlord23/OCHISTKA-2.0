import { describe, expect, it } from 'vitest';
import { bypassView, errorText } from './siteBypass';

const OFF = { active: false, error: false, message: 'Выключено' };
const ON = { active: true, error: false, message: 'Сайты открываются через обход' };

describe('bypassView', () => {
  it('shows the three plain states', () => {
    expect(bypassView(OFF, null, null).text).toBe('Выключено');
    expect(bypassView(ON, null, null)).toMatchObject({
      checked: true,
      text: 'Сайты открываются через обход',
    });
    expect(
      bypassView({ active: false, error: true, message: 'Ошибка: причина' }, null, null),
    ).toMatchObject({ checked: false, error: true, text: 'Ошибка: причина' });
  });

  it('flips at once on press and goes back with one reason on failure', () => {
    expect(bypassView(OFF, 'on', null)).toMatchObject({ checked: true, busy: true });
    expect(bypassView(ON, 'off', null)).toMatchObject({ checked: false, busy: true });
    expect(bypassView(OFF, null, 'не удалось включить')).toMatchObject({
      checked: false,
      error: true,
      text: 'Ошибка: не удалось включить',
    });
  });

  it('turns an unknown error into a short reason', () => {
    expect(errorText({ message: 'x' })).toBe('x');
    expect(errorText(null)).toBe('не удалось выполнить действие');
  });
});
