/**
 * Маска ссылки подписки: видны схема и адрес сервера, всё остальное (путь с токеном, параметры, логин
 * и пароль перед @) заменено точками. Настоящая ссылка не меняется: маска только для показа.
 */
const BULLET = '•';

export function maskSubscriptionUrl(raw: string): string {
  const text = raw.trim();
  if (text === '') return '';
  const match = text.match(/^([a-z][a-z0-9+.-]*:\/\/)([^/?#]*)([/?#].*)?$/i);
  if (!match) return BULLET.repeat(Math.min(text.length, 24));
  const [, scheme, authority, rest = ''] = match;
  const at = authority.lastIndexOf('@');
  const host = at >= 0 ? `${BULLET.repeat(3)}@${authority.slice(at + 1)}` : authority;
  return `${scheme}${host}${rest ? `/${BULLET.repeat(Math.min(Math.max(rest.length - 1, 6), 16))}` : ''}`;
}

const SCRAMBLE = '!#$%&*+<>?@ABCDEFGHJKLMNPQRSTUVWXYZ0123456789';

/** Кадр лёгкого эффекта «расшифровки»: первые `shown` символов настоящие, остальные случайные. */
export function scrambleFrame(
  real: string,
  shown: number,
  random: () => number = Math.random,
): string {
  let out = '';
  for (let i = 0; i < real.length; i++) {
    out +=
      i < shown || real[i] === ' ' ? real[i] : SCRAMBLE[Math.floor(random() * SCRAMBLE.length)];
  }
  return out;
}
