import type { RequestedStart, ServiceInfo, ServiceState, StartType } from './servicesApi';

export const NEEDS_ADMIN_NOTE =
  'Чтобы запускать, останавливать и менять запуск служб, запусти OCHISTKA от имени администратора. ' +
  'Отдельный запрос прав на одно действие пока не поддерживается.';

const STATE_LABELS: Record<ServiceState, string> = {
  running: 'Работает',
  stopped: 'Остановлена',
  startPending: 'Запускается',
  stopPending: 'Останавливается',
  continuePending: 'Возобновляется',
  pausePending: 'Приостанавливается',
  paused: 'Приостановлена',
  unknown: 'Неизвестно',
};

const START_LABELS: Record<StartType, string> = {
  boot: 'Загрузка драйвера',
  system: 'Системная',
  auto: 'Автоматически',
  autoDelayed: 'Автоматически (отложенный)',
  manual: 'Вручную',
  disabled: 'Отключена',
  unknown: 'Неизвестно',
};

export const stateLabel = (state: ServiceState): string => STATE_LABELS[state];
export const startLabel = (start: StartType): string => START_LABELS[start];

/** Состояния «в процессе»: выключатель ждёт, пока служба дойдёт до конца. */
export function isPending(state: ServiceState): boolean {
  return (
    state === 'startPending' ||
    state === 'stopPending' ||
    state === 'continuePending' ||
    state === 'pausePending'
  );
}

/** Три доступных типа запуска и подпись каждого. */
export const START_OPTIONS: Array<{ value: RequestedStart; label: string }> = [
  { value: 'auto', label: START_LABELS.auto },
  { value: 'manual', label: START_LABELS.manual },
  { value: 'disabled', label: START_LABELS.disabled },
];

/** Выбранное значение списка по текущему типу: отложенный автозапуск показывается как «Автоматически». */
export function startOptionValue(start: StartType): RequestedStart | null {
  if (start === 'auto' || start === 'autoDelayed') return 'auto';
  if (start === 'manual') return 'manual';
  if (start === 'disabled') return 'disabled';
  return null;
}

/** Нужно ли подтверждение с последствиями: остановка или отключение службы «с осторожностью». */
export function needsConfirm(service: ServiceInfo, action: 'stop' | RequestedStart): boolean {
  return service.guard === 'careful' && (action === 'stop' || action === 'disabled');
}

/** Причина, по которой управлять службой нельзя; `null`, если можно. */
export function blockReason(service: ServiceInfo, elevated: boolean): string | null {
  if (service.guard === 'locked') {
    return service.note ?? 'Эту службу менять нельзя.';
  }
  if (!elevated) return NEEDS_ADMIN_NOTE;
  return null;
}

/** Фильтр: подстрока в имени или отображаемом имени; «только работающие». */
export function filterServices(
  list: readonly ServiceInfo[],
  query: string,
  onlyRunning: boolean,
): ServiceInfo[] {
  const needle = query.trim().toLowerCase();
  return list.filter((service) => {
    if (onlyRunning && service.state !== 'running' && !isPending(service.state)) return false;
    if (needle === '') return true;
    return (
      service.name.toLowerCase().includes(needle) ||
      service.displayName.toLowerCase().includes(needle)
    );
  });
}

/** Фраза-итог после действия по фактическому состоянию. */
export function resultNote(service: ServiceInfo): string {
  return `«${service.displayName}»: ${stateLabel(service.state).toLowerCase()}, запуск: ${startLabel(service.startType).toLowerCase()}.`;
}
