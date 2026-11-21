/** Локальные SVG-флаги (flag-icons, MIT; см. THIRD-PARTY.md) и названия стран для списка серверов.
    Только отображение: код страны приходит из данных VPN, ничего не определяется по сети. */
const FLAG_FILES = import.meta.glob('../assets/flags/*.svg', {
  eager: true,
  query: '?url',
  import: 'default',
}) as Record<string, string>;

const FLAGS: Record<string, string> = {};
for (const [path, url] of Object.entries(FLAG_FILES)) {
  const code = /\/([a-z]{2})\.svg$/.exec(path)?.[1];
  if (code) FLAGS[code] = url;
}

/** Код как у ISO 3166: `UK` в именах серверов означает Великобританию (`GB`). */
function iso(code: string): string {
  const up = code.trim().toUpperCase();
  return up === 'UK' ? 'GB' : up;
}

/** Адрес картинки флага или `null`, если флага нет (покажем глобус). */
export function flagUrl(code: string | null | undefined): string | null {
  if (!code) return null;
  return FLAGS[iso(code).toLowerCase()] ?? null;
}

/** Привычные короткие русские названия вместо длинных официальных. */
const RU_SHORT: Record<string, string> = { US: 'США', AE: 'ОАЭ' };

/** Название страны на языке интерфейса (`LV` → «Латвия»); `null`, если код не распознан. */
export function countryName(code: string | null | undefined, lang: string): string | null {
  if (!code || !flagUrl(code)) return null;
  try {
    if (lang === 'ru' && RU_SHORT[iso(code)]) return RU_SHORT[iso(code)];
    const name = new Intl.DisplayNames([lang], { type: 'region' }).of(iso(code));
    return name && name !== iso(code) ? name : null;
  } catch {
    return null;
  }
}

/** Частые сокращения страны в именах серверов. */
const ALIASES: Record<string, string[]> = {
  US: ['USA', 'United States', 'США'],
  GB: ['UK', 'United Kingdom', 'Britain', 'England'],
  RU: ['Russian Federation'],
  NL: ['Holland'],
  DE: ['Deutschland'],
};

function escapeRe(text: string): string {
  return text.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

/** Подпись сервера: «Страна» или «Страна · остаток имени» (номер, город). Флаг-эмодзи, код страны
    и название страны (русское или английское) внутри имени не повторяются. Исходное имя не меняется. */
export function serverLabel(name: string, code: string | null | undefined, lang: string): string {
  const country = countryName(code, lang);
  if (!country || !code) return name;
  const words = [
    country,
    countryName(code, 'ru'),
    countryName(code, 'en'),
    iso(code),
    code.toUpperCase(),
    ...(ALIASES[iso(code)] ?? []),
  ].filter((w): w is string => !!w);
  let rest = name.replace(/[\u{1F1E6}-\u{1F1FF}]{2}/gu, ' ');
  for (const word of new Set(words)) {
    rest = rest.replace(
      new RegExp(`(^|[^\\p{L}\\p{N}])${escapeRe(word)}(?![\\p{L}\\p{N}])`, 'giu'),
      '$1 ',
    );
  }
  rest = rest
    .replace(/[[\]()|·:,;]/g, ' ')
    .replace(/\s+/g, ' ')
    .replace(/^[\s\-_.#]+|[\s\-_.#]+$/g, '');
  return rest ? `${country} · ${rest}` : country;
}
