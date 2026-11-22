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

/** Подпись сервера в списке: только название страны. Если у кода нет флага (например, «EU»), берётся
    имя из подписки без флага-эмодзи и кода страны в начале. Исходное имя не меняется. */
export function serverLabel(name: string, code: string | null | undefined, lang: string): string {
  const country = countryName(code, lang);
  if (country) return country;
  return cleanName(name);
}

/** Имя без флага-эмодзи и кода страны в начале: «ЕУ Европа» → «Европа», «[NL] Amsterdam» → «Amsterdam». */
function cleanName(name: string): string {
  const cleaned = name
    .replace(/[\u{1F1E6}-\u{1F1FF}]{2}/gu, ' ')
    .replace(/^\s*[[(]?\p{Lu}{2,3}[\])]?(?=\s|[-_·:|])\s*[-_·:|]?\s*/u, '')
    .replace(/\s+/g, ' ')
    .trim();
  return cleaned || name;
}

/** Одинаковые подписи получают номер со второго повтора: «Нидерланды», «Нидерланды 2». */
export function numberedLabels(labels: string[]): string[] {
  const seen = new Map<string, number>();
  return labels.map((label) => {
    const n = (seen.get(label) ?? 0) + 1;
    seen.set(label, n);
    return n === 1 ? label : `${label} ${n}`;
  });
}
