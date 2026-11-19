/** Переход между разделами из другого раздела (например, «Показать процесс» во вкладке «Соединения»). */
export const navigation = $state<{ processPid: number | null }>({ processPid: null });

/** Просит открыть «Процессы» с выбранным процессом. */
export function showProcess(pid: number): void {
  navigation.processPid = pid;
}

/** Забирает запрос один раз. */
export function takeProcessRequest(): number | null {
  const pid = navigation.processPid;
  navigation.processPid = null;
  return pid;
}
