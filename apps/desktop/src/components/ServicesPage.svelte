<script lang="ts">
  import { untrack } from 'svelte';
  import FloatingField from './FloatingField.svelte';
  import Switch from './Switch.svelte';
  import ExpandButton from './ExpandButton.svelte';
  import { ACTION_ICONS } from '../lib/actionIcons';
  import ConfirmDialog from './ConfirmDialog.svelte';
  import {
    blockReason,
    filterServices,
    isPending,
    needsConfirm,
    NEEDS_ADMIN_NOTE,
    resultNote,
    stateLabel,
    startLabel,
    START_OPTIONS,
    startOptionValue,
  } from '../lib/services';
  import {
    listServices,
    restoreServiceStart,
    setServiceStart,
    startService,
    stopService,
    toServicesError,
    type RequestedStart,
    type ServiceInfo,
    type ServicesErrorInfo,
  } from '../lib/servicesApi';

  /** Как часто перечитываем состояния, пока подраздел открыт и ничего не меняется. */
  const REFRESH_MS = 6000;

  let services = $state<ServiceInfo[]>([]);
  let elevated = $state(false);
  let loaded = $state(false);
  let query = $state('');
  let onlyRunning = $state(false);
  let listError = $state<ServicesErrorInfo | null>(null);
  let actionError = $state<{ name: string; message: string } | null>(null);
  let notice = $state('');
  /** Службы, над которыми идёт действие. Выключатель в это время ждёт. */
  let busy = $state<Record<string, true>>({});
  /** Желаемое значение выключателя до ответа системы (показ в тот же кадр). */
  let wanted = $state<Record<string, boolean>>({});
  /** Действие, ждущее подтверждения последствий. */
  let pending = $state<{
    service: ServiceInfo;
    action: 'stop' | RequestedStart;
  } | null>(null);
  let listing = false;

  const visible = $derived(filterServices(services, query, onlyRunning));

  async function refresh() {
    if (listing || Object.keys(busy).length > 0 || pending) return;
    listing = true;
    try {
      const snapshot = await listServices();
      services = snapshot.services;
      elevated = snapshot.elevated;
      listError = null;
    } catch (err) {
      listError = toServicesError(err);
    } finally {
      loaded = true;
      listing = false;
    }
  }

  // Эффект не должен следить за состоянием действий: иначе каждое действие вызвало бы перечитывание.
  $effect(() => {
    untrack(() => void refresh());
    const timer = setInterval(() => void refresh(), REFRESH_MS);
    return () => clearInterval(timer);
  });

  function isOn(service: ServiceInfo): boolean {
    return (
      wanted[service.name] ?? (service.state === 'running' || service.state === 'startPending')
    );
  }

  function replace(updated: ServiceInfo) {
    services = services.map((item) => (item.name === updated.name ? updated : item));
  }

  /** Выполняет действие: показ мгновенный, результат берём из фактического состояния. */
  async function run(
    service: ServiceInfo,
    call: () => Promise<ServiceInfo>,
    optimisticOn?: boolean,
  ) {
    actionError = null;
    notice = '';
    busy = { ...busy, [service.name]: true };
    if (optimisticOn !== undefined) wanted = { ...wanted, [service.name]: optimisticOn };
    try {
      const updated = await call();
      replace(updated);
      notice = resultNote(updated);
    } catch (err) {
      actionError = { name: service.name, message: toServicesError(err).message };
    } finally {
      const { [service.name]: _busy, ...restBusy } = busy;
      busy = restBusy;
      const { [service.name]: _wanted, ...restWanted } = wanted;
      wanted = restWanted;
    }
  }

  function toggle(service: ServiceInfo, next: boolean) {
    if (next) {
      void run(service, () => startService(service.name), true);
    } else if (needsConfirm(service, 'stop')) {
      pending = { service, action: 'stop' };
    } else {
      void run(service, () => stopService(service.name, false), false);
    }
  }

  function changeStart(service: ServiceInfo, value: RequestedStart) {
    if (startOptionValue(service.startType) === value) return;
    if (needsConfirm(service, value)) {
      pending = { service, action: value };
      return;
    }
    void run(service, () => setServiceStart(service.name, value, false));
  }

  function confirm() {
    const current = pending;
    if (!current) return;
    pending = null;
    const { service, action } = current;
    if (action === 'stop') {
      void run(service, () => stopService(service.name, true), false);
    } else {
      void run(service, () => setServiceStart(service.name, action, true));
    }
  }

  function dialogTitle(item: NonNullable<typeof pending>): string {
    return item.action === 'stop'
      ? `Остановить «${item.service.displayName}»?`
      : `Отключить «${item.service.displayName}»?`;
  }

  function dialogText(item: NonNullable<typeof pending>): string {
    const result = item.service.note ?? 'Последствия остановки не определены.';
    return item.action === 'stop'
      ? `${result} Запустить службу обратно можно в любой момент.`
      : `${result} Прежний тип запуска запомним: вернуть его можно кнопкой «Вернуть».`;
  }

  function rowReason(service: ServiceInfo): string | null {
    return blockReason(service, elevated);
  }
</script>

<section class="page" aria-label="Службы">
  <div class="search-row">
    <FloatingField
      id="services-search"
      type="search"
      label="Поиск службы"
      ariaLabel="Поиск по названию службы"
      bind:value={query}
    />
  </div>

  <div class="buttons">
    <ExpandButton
      icon={ACTION_ICONS.refresh}
      label="Обновить"
      width={96}
      name="Обновить"
      onclick={() => void refresh()}
    />
    <ExpandButton
      icon={ACTION_ICONS.running}
      label="Только работающие"
      width={160}
      name="Только работающие"
      role="switch"
      aria-checked={onlyRunning}
      onclick={() => (onlyRunning = !onlyRunning)}
    />
  </div>

  {#if loaded && !elevated && !listError}
    <p class="alert" role="status">{NEEDS_ADMIN_NOTE}</p>
  {/if}
  {#if listError}
    <p class="alert" role="alert">{listError.message}</p>
  {/if}
  {#if actionError}
    <p class="alert" role="alert">{actionError.message}</p>
  {/if}
  {#if notice}
    <p class="notice" role="status">{notice}</p>
  {/if}

  {#if !loaded && !listError}
    <p class="status-line">Читаю список служб…</p>
  {:else if loaded && visible.length === 0}
    <p class="status-line">Ничего не найдено.</p>
  {:else if loaded}
    <div class="grid" role="table" aria-label="Службы" aria-rowcount={visible.length + 1}>
      <div class="grid-head" role="row">
        <span class="c-switch" role="columnheader">Запущена</span>
        <span class="c-name" role="columnheader">Служба</span>
        <span class="c-state" role="columnheader">Состояние</span>
        <span class="c-start" role="columnheader">Запуск</span>
      </div>
      <div class="grid-body" role="rowgroup">
        {#each visible as service (service.name)}
          {@const reason = rowReason(service)}
          <div class="row" role="row" class:is-locked={service.guard === 'locked'}>
            <span class="c-switch" role="cell">
              <Switch
                ariaLabel="Служба «{service.displayName}» запущена"
                checked={isOn(service)}
                pending={!!busy[service.name] || isPending(service.state)}
                error={actionError?.name === service.name}
                disabled={reason !== null || !!busy[service.name]}
                onchange={(next) => toggle(service, next)}
              />
            </span>
            <span class="c-name" role="cell" title={reason ?? service.note ?? undefined}>
              <span class="display">{service.displayName}</span>
              <span class="tech">
                {service.name}{service.guard === 'locked' ? ' · защищена' : ''}
              </span>
            </span>
            <span class="c-state" role="cell">
              <span class="pill" class:is-on={service.state === 'running'}
                >{stateLabel(service.state)}</span
              >
            </span>
            <span class="c-start" role="cell">
              <select
                class="start-select"
                aria-label="Запуск службы «{service.displayName}»"
                disabled={reason !== null || !!busy[service.name]}
                value={startOptionValue(service.startType) ?? ''}
                onchange={(event) =>
                  changeStart(service, event.currentTarget.value as RequestedStart)}
              >
                {#if startOptionValue(service.startType) === null}
                  <option value="" disabled>{startLabel(service.startType)}</option>
                {/if}
                {#each START_OPTIONS as option (option.value)}
                  <option value={option.value}
                    >{option.value === 'auto' && service.startType === 'autoDelayed'
                      ? startLabel('autoDelayed')
                      : option.label}</option
                  >
                {/each}
              </select>
              {#if service.previousStartType && service.guard !== 'locked'}
                <button
                  type="button"
                  class="restore"
                  disabled={!elevated || !!busy[service.name]}
                  title="Вернуть тип запуска «{startLabel(service.previousStartType)}»"
                  onclick={() => void run(service, () => restoreServiceStart(service.name))}
                  >Вернуть: {startLabel(service.previousStartType).toLowerCase()}</button
                >
              {/if}
            </span>
          </div>
        {/each}
      </div>
    </div>
  {/if}
</section>

{#if pending}
  <ConfirmDialog
    title={dialogTitle(pending)}
    text={dialogText(pending)}
    confirmLabel={pending.action === 'stop' ? 'Остановить' : 'Отключить'}
    onconfirm={confirm}
    oncancel={() => (pending = null)}
  />
{/if}

<style>
  .page {
    display: flex;
    flex: 1;
    flex-direction: column;
    gap: 8px;
    min-height: 0;
    color: var(--page-text);
  }
  .status-line {
    margin: 0;
    font-size: var(--text-small);
    font-weight: 600;
  }
  .search-row {
    display: flex;
  }
  .buttons {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
  }
  button {
    padding: 10px 14px;
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: var(--radius-adaptive);
    background: var(--black);
    color: var(--ink);
    font: inherit;
    font-size: var(--text-small);
    font-weight: 600;
    cursor: pointer;
  }
  button:hover:not(:disabled) {
    border-color: var(--ink-soft);
  }
  button:disabled {
    color: var(--muted);
    cursor: not-allowed;
  }
  .alert,
  .notice {
    margin: 0;
    padding: 8px 12px;
    border-radius: var(--radius-adaptive);
    background: var(--black);
    font-size: var(--text-small);
    font-weight: 700;
    overflow-wrap: anywhere;
  }
  .alert {
    color: #ff8a8a;
  }
  .notice {
    color: var(--ink);
  }

  /* Список заполняет всю высоту страницы до низа, прокрутка внутри; строки фиксированной высоты. */
  .grid {
    flex: 1;
    min-height: 160px;
    overflow: auto;
    border-radius: var(--radius-adaptive);
    background: #111;
    color: var(--ink);
    font-size: var(--text-small);
  }
  .grid-head,
  .row {
    display: grid;
    grid-template-columns: 84px minmax(160px, 1fr) 128px minmax(180px, 250px);
    align-items: center;
    column-gap: 8px;
  }
  .grid-head {
    position: sticky;
    top: 0;
    z-index: 1;
    height: 44px;
    padding: 0 18px 0 12px;
    background: #050505;
    color: var(--accent);
    font-size: 10px;
    font-weight: 800;
    letter-spacing: 0.18em;
    text-transform: uppercase;
  }
  .grid-body {
    padding: 4px 6px 6px;
  }
  .row {
    position: relative;
    height: 52px;
    padding: 0 12px;
    border-radius: var(--radius-adaptive);
  }
  .row::after {
    content: '';
    position: absolute;
    right: 12px;
    bottom: 0;
    left: 12px;
    height: 1px;
    background: #262626;
  }
  .row:hover {
    background: #181818;
  }
  .c-name {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .display {
    overflow: hidden;
    font-weight: 700;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .tech {
    overflow: hidden;
    color: var(--muted);
    font-family: ui-monospace, Consolas, monospace;
    font-size: 11px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .pill {
    display: inline-block;
    padding: 4px 10px;
    border-radius: var(--radius-adaptive);
    background: #1c1c1c;
    color: var(--muted);
    font-size: 11px;
    font-weight: 800;
  }
  .pill.is-on {
    background: color-mix(in srgb, var(--accent) 22%, #111);
    color: var(--ink);
  }
  .c-start {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 2px;
    min-width: 0;
  }
  .start-select {
    max-width: 100%;
    height: 30px;
    padding: 0 8px;
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: var(--radius-adaptive);
    background: var(--black);
    color: var(--ink);
    font: inherit;
    font-size: var(--text-small);
  }
  .start-select:disabled {
    color: var(--muted);
    cursor: not-allowed;
  }
  .restore {
    padding: 0;
    border: 0;
    background: none;
    color: var(--muted);
    font-size: 11px;
    text-decoration: underline;
  }
  .is-locked .display {
    color: var(--muted);
  }
</style>
