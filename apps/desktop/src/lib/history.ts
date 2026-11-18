/** Сколько последних замеров хранит график: 90 секунд при опросе раз в секунду. */
export const HISTORY_LIMIT = 90;

/** Добавляет замер в конец и обрезает старые, чтобы осталось не больше `limit` значений. */
export function pushSample(
  history: readonly number[],
  value: number,
  limit: number = HISTORY_LIMIT,
): number[] {
  const next = [...history, value];
  return next.length > limit ? next.slice(next.length - limit) : next;
}
