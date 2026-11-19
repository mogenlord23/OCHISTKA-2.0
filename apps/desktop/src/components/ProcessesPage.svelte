<script lang="ts">
  import AppIcon from './AppIcon.svelte';
  import {
    filterProcesses,
    formatCpu,
    isProtectedPid,
    REFRESH_MS,
    sortProcesses,
    type ProcessSortKey,
  } from '../lib/processes';
  import { ariaSort, nextSort, type SortState } from '../lib/sortState';
  import { formatMemory } from '../lib/units';
  import { getIcon } from '../lib/programsApi';
  import { prefetchProcessIcons } from '../lib/processIcons';
  import {
    endTask,
    exePath,
    listProcesses,
    listServices,
    toProcessesError,
    type ProcessInfo,
    type ProcessesErrorInfo,
    type ServiceInfo,
  } from '../lib/processesApi';

  let { subsection }: { subsection: string } = $props();

  let processes = $state<ProcessInfo[]>([]);
  let loaded = $state(false);
  let query = $state('');
  let selectedPid = $state<number | null>(null);
  let listError = $state<ProcessesErrorInfo | null>(null);
  let actionError = $state<ProcessesErrorInfo | null>(null);
  let notice = $state('');
  let pathInfo = $state<{ pid: number; path: string | null } | null>(null);
  /** Процесс, ожидающий подтверждения завершения. Без подтверждения ядро не вызывается. */
  let pending = $state<ProcessInfo | null>(null);
  let updatedAt = $state('');
  let services = $state<ServiceInfo[]>([]);
  let servicesError = $state<ProcessesErrorInfo | null>(null);

  let cancelButton = $state<HTMLButtonElement | null>(null);

  /** Защита от наложения опросов. Обычная переменная, а не $state: не должна влиять на эффект. */
  let listing = false;

  /** Сортировка по клику на заголовок. `null` — по имени, как в списке Windows. */
  let sort = $state<SortState<ProcessSortKey> | null>(null);
  const COLUMNS: Array<{ key: ProcessSortKey; label: string; cls: string; first: 'asc' | 'desc' }> =
    [
      { key: 'name', label: 'Процесс', cls: 'c-name', first: 'asc' },
      { key: 'pid', label: 'PID', cls: 'c-pid', first: 'desc' },
      { key: 'cpu', label: 'CPU', cls: 'c-cpu', first: 'desc' },
      { key: 'memory', label: 'RAM / Память', cls: 'c-mem', first: 'desc' },
    ];

  const visible = $derived(sortProcesses(filterProcesses(processes, query), sort));
  const selected = $derived(processes.find((proc) => proc.pid === selectedPid) ?? null);

  async function refresh() {
    if (listing) return;
    listing = true;
    try {
      processes = await listProcesses();
      prefetchProcessIcons(processes);
      updatedAt = new Date().toLocaleTimeString('ru-RU');
      listError = null;
    } catch (err) {
      listError = toProcessesError(err);
    } finally {
      loaded = true;
      listing = false;
    }
  }

  async function loadServices() {
    try {
      services = await listServices();
      servicesError = null;
    } catch (err) {
      servicesError = toProcessesError(err);
    }
  }

  // Список обновляется раз в REFRESH_MS, пока открыт подраздел «Процессы».
  $effect(() => {
    if (subsection !== 'processes') return;
    void refresh();
    const timer = setInterval(() => void refresh(), REFRESH_MS);
    return () => clearInterval(timer);
  });

  $effect(() => {
    if (subsection !== 'services') return;
    void loadServices();
  });

  // Фокус на «Отмена» при открытии диалога: так случайный Enter не завершает процесс.
  $effect(() => {
    if (pending && cancelButton) cancelButton.focus();
  });

  async function showPath(proc: ProcessInfo) {
    actionError = null;
    notice = '';
    try {
      pathInfo = { pid: proc.pid, path: await exePath(proc.pid) };
    } catch (err) {
      actionError = toProcessesError(err);
    }
  }

  function sortBy(column: (typeof COLUMNS)[number]) {
    sort = nextSort(sort, column.key, column.first);
  }

  /** Значок процесса: путь к exe берём у системы, потом его значок. Нет пути — буква. */
  function iconSource(proc: ProcessInfo) {
    return async () => {
      const path = await exePath(proc.pid);
      return path ? getIcon(path) : null;
    };
  }

  function askEnd(proc: ProcessInfo) {
    if (isProtectedPid(proc.pid)) return;
    actionError = null;
    notice = '';
    pending = proc;
  }

  function cancelEnd() {
    pending = null;
  }

  async function confirmEnd() {
    const target = pending;
    if (!target) return;
    pending = null;
    try {
      await endTask(target, true);
      notice = `Процесс «${target.name}» (PID ${target.pid}) завершён.`;
      if (selectedPid === target.pid) selectedPid = null;
      await refresh();
    } catch (err) {
      actionError = toProcessesError(err);
    }
  }

  function onDialogKey(event: KeyboardEvent) {
    if (event.key === 'Escape') cancelEnd();
  }
</script>

{#if subsection === 'services'}
  <section class="page" aria-labelledby="services-title">
    <h2 id="services-title" class="sr-only">Службы</h2>
    <p class="lead">Только чтение: службы здесь не запускаются и не останавливаются.</p>
    {#if servicesError}
      <p class="alert" role="alert">{servicesError.message}</p>
    {:else if services.length === 0}
      <p class="status-line">Служб нет.</p>
    {:else}
      <div class="table-wrap">
        <table>
          <thead>
            <tr
              ><th scope="col">Имя</th><th scope="col">Отображаемое имя</th><th scope="col"
                >Запуск</th
              ></tr
            >
          </thead>
          <tbody>
            {#each services as service (service.name)}
              <tr>
                <td>{service.name}</td>
                <td>{service.displayName}</td>
                <td>{service.startType}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}
  </section>
{:else}
  <section class="page" aria-labelledby="processes-title">
    <h2 id="processes-title" class="sr-only">Процессы</h2>

    <label class="field-label" for="proc-search">Поиск по названию или PID</label>
    <input
      id="proc-search"
      type="search"
      class="search"
      aria-label="Поиск по имени или PID"
      bind:value={query}
    />

    <div class="buttons" aria-label="Действия с выбранным процессом">
      <button type="button" onclick={() => void refresh()}>Обновить</button>
      <button
        type="button"
        disabled={!selected || isProtectedPid(selected.pid)}
        title={!selected
          ? 'Сначала выбери процесс в списке'
          : isProtectedPid(selected.pid)
            ? 'Этот процесс системный, завершать его нельзя'
            : 'Завершить задачу'}
        onclick={() => selected && askEnd(selected)}>Завершить задачу</button
      >
      <button
        type="button"
        disabled={!selected}
        title={selected ? 'Показать путь к файлу' : 'Сначала выбери процесс в списке'}
        onclick={() => selected && showPath(selected)}>Расположение файла</button
      >
    </div>

    <p class="status-line">
      Процессов: {processes.length}{updatedAt ? ` · обновлено ${updatedAt}` : ''} · каждые 3 секунды ·
      CPU — доля всех ядер, память — рабочий набор · клик по заголовку сортирует
    </p>

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
      <p class="status-line">Загружаем список процессов…</p>
    {:else if visible.length === 0 && processes.length > 0}
      <p class="status-line">Ничего не найдено по запросу «{query}».</p>
    {:else}
      <div class="grid" role="table" aria-label="Процессы" aria-rowcount={visible.length + 1}>
        <div class="grid-head" role="row">
          <span class="c-icon" role="columnheader"><span class="sr-only">Значок</span></span>
          {#each COLUMNS as column (column.key)}
            <span class={column.cls} role="columnheader" aria-sort={ariaSort(sort, column.key)}>
              <button
                type="button"
                class="sort"
                class:is-active={sort?.key === column.key}
                onclick={() => sortBy(column)}
                title="Сортировать: {column.label}"
              >
                {column.label}
                <svg
                  class="arrow"
                  class:is-asc={sort?.key === column.key && sort.dir === 'asc'}
                  viewBox="0 0 10 10"
                  aria-hidden="true"><path d="M5 8.5 1.5 3h7z" fill="currentColor" /></svg
                >
              </button>
            </span>
          {/each}
        </div>
        <div class="grid-body" role="rowgroup">
          {#each visible as proc (proc.pid)}
            <!-- svelte-ignore a11y_click_events_have_key_events, a11y_interactive_supports_focus -->
            <!-- Клик по строке — удобство для мыши; с клавиатуры строку выбирает кнопка с названием. -->
            <div
              class="row"
              role="row"
              class:is-selected={proc.pid === selectedPid}
              onclick={() => (selectedPid = proc.pid)}
              oncontextmenu={(event) => {
                event.preventDefault();
                selectedPid = proc.pid;
              }}
            >
              <span class="c-icon" role="cell">
                <AppIcon
                  name={proc.name}
                  cacheKey={proc.name}
                  resolve={iconSource(proc)}
                  size={30}
                />
              </span>
              <span class="c-name" role="cell">
                <button
                  type="button"
                  class="row-btn"
                  aria-pressed={proc.pid === selectedPid}
                  onclick={(event) => {
                    event.stopPropagation();
                    selectedPid = proc.pid;
                  }}>{proc.name}</button
                >
              </span>
              <span class="c-pid" role="cell"><span class="pid">{proc.pid}</span></span>
              <span class="c-cpu" role="cell"
                ><span class="cpu">{formatCpu(proc.cpuPercent)}</span></span
              >
              <span class="c-mem" role="cell"
                ><span class="mem" class:is-empty={proc.memoryBytes <= 0}
                  >{formatMemory(proc.memoryBytes)}</span
                ></span
              >
            </div>
          {/each}
        </div>
      </div>
    {/if}

    <p class="field-label">Путь выбранного процесса</p>
    <div class="path-box" aria-live="polite">
      {#if pathInfo}
        <span class="path">
          PID {pathInfo.pid}:
          {pathInfo.path ?? 'путь не отдан системой (нет доступа к процессу)'}
        </span>
      {:else if selected}
        {selected.name} · PID {selected.pid} — нажми «Расположение файла»
      {:else}
        Выбери процесс в списке
      {/if}
    </div>
  </section>

  {#if pending}
    <div class="backdrop">
      <div
        class="dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="end-title"
        aria-describedby="end-text"
        tabindex="-1"
        onkeydown={onDialogKey}
      >
        <h3 id="end-title">Завершить задачу?</h3>
        <p id="end-text">
          Процесс «{pending.name}» (PID {pending.pid}) будет принудительно завершён. Несохранённые
          данные в нём пропадут.
        </p>
        <div class="dialog-actions">
          <button type="button" bind:this={cancelButton} onclick={cancelEnd}>Отмена</button>
          <button type="button" class="danger" onclick={confirmEnd}>Завершить</button>
        </div>
      </div>
    </div>
  {/if}
{/if}

<style>
  .page {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin-top: 0;
    color: var(--page-text);
  }
  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
  }
  .lead,
  .status-line {
    margin: 0;
    font-size: var(--text-small);
    font-weight: 600;
  }
  .lead {
    font-weight: 700;
  }
  .field-label {
    margin: 4px 0 0;
    font-size: var(--text-small);
    font-weight: 800;
    text-transform: uppercase;
  }
  .search {
    height: 48px;
    padding: 0 14px;
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: var(--radius-md);
    background: var(--surface);
    color: var(--ink);
    font: inherit;
  }
  .search:focus-visible {
    outline: 2px solid var(--ink);
  }
  .buttons {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  button {
    padding: 10px 14px;
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: var(--radius-md);
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
    opacity: 1;
    color: var(--muted);
    cursor: not-allowed;
  }
  button.danger {
    background: #9b1c1c;
  }
  .row-btn {
    max-width: 100%;
    padding: 0;
    border: 0;
    background: none;
    color: var(--ink);
    text-align: left;
    font-weight: 700;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .row-btn:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 3px;
    border-radius: 4px;
  }
  .path {
    font-family: ui-monospace, Consolas, monospace;
    word-break: break-all;
  }
  .path-box {
    padding: 12px 14px;
    border-radius: var(--radius-md);
    background: rgba(8, 8, 8, 0.35);
    color: var(--ink);
    font-size: var(--text-small);
  }
  .alert,
  .notice {
    margin: 0;
    padding: 8px 12px;
    border-radius: var(--radius-sm);
    background: var(--black);
    font-size: var(--text-small);
    font-weight: 700;
  }
  .alert {
    color: #ff8a8a;
  }
  .notice {
    color: var(--ink);
  }
  .table-wrap {
    max-height: calc(100vh - 456px);
    min-height: 180px;
    overflow: auto;
    border-radius: var(--radius-md);
    background: var(--surface);
  }
  table {
    width: 100%;
    border-collapse: collapse;
    font-size: var(--text-small);
    color: var(--ink);
  }
  th,
  td {
    padding: 0 12px;
    height: 41px;
    border-bottom: 1px solid #262626;
    text-align: left;
  }
  th {
    position: sticky;
    top: 0;
    z-index: 1;
    height: 36px;
    border-bottom: 0;
    background: var(--black);
    color: var(--accent);
    font-size: var(--text-caption);
    font-weight: 700;
    text-transform: uppercase;
  }
  /* Список процессов: сетка из div с ролями таблицы (выделение — скруглённая рамка). */
  .grid {
    max-height: calc(100vh - 440px);
    min-height: 180px;
    overflow: auto;
    border-radius: 16px;
    background: #111;
    color: var(--ink);
    font-size: var(--text-small);
  }
  .grid-head,
  .row {
    display: grid;
    grid-template-columns: 56px minmax(140px, 1fr) 120px 96px 132px;
    align-items: center;
    column-gap: 8px;
  }
  .grid-head {
    position: sticky;
    top: 0;
    z-index: 1;
    height: 44px;
    padding: 0 18px 0 8px;
    background: #050505;
  }
  .c-pid {
    justify-self: start;
  }
  .c-cpu,
  .c-mem {
    justify-self: end;
    text-align: right;
  }
  .c-icon {
    display: flex;
    justify-content: center;
  }
  .sort {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 6px 4px;
    border: 0;
    border-radius: 6px;
    background: none;
    color: var(--accent);
    font-size: 10px;
    font-weight: 800;
    letter-spacing: 0.18em;
    text-transform: uppercase;
    transition: background 120ms ease;
  }
  .sort:hover {
    background: color-mix(in srgb, var(--accent) 16%, transparent);
    border-color: transparent;
  }
  .sort:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }
  .arrow {
    width: 9px;
    height: 9px;
    opacity: 0;
    transform: translateY(2px) rotate(0deg);
    transition:
      opacity 140ms ease,
      transform 180ms cubic-bezier(0.2, 0, 0, 1);
  }
  .is-active .arrow {
    opacity: 1;
    transform: none;
  }
  .is-active .arrow.is-asc {
    transform: rotate(180deg);
  }
  .grid-body {
    padding: 4px 6px 6px;
  }
  .row {
    position: relative;
    height: 52px;
    padding: 0 12px 0 2px;
    border-radius: 12px;
    cursor: default;
    transition: background 120ms ease;
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
  .row:hover :global(.icon) {
    transform: scale(1.07);
  }
  .row :global(.icon) {
    transition: transform 160ms cubic-bezier(0.2, 0, 0, 1);
  }
  .row.is-selected {
    background: color-mix(in srgb, var(--accent) 13%, #111);
    box-shadow: inset 0 0 0 2px var(--accent);
  }
  .row.is-selected::after {
    display: none;
  }
  .c-name {
    min-width: 0;
  }
  .pid {
    display: inline-block;
    min-width: 62px;
    padding: 5px 10px;
    border-radius: 10px;
    background: #2a0f1e;
    color: #ff2e93;
    font-family: ui-monospace, Consolas, monospace;
    font-size: 11px;
    font-weight: 800;
    font-variant-numeric: tabular-nums;
  }
  .cpu {
    color: #b4ff00;
    font-weight: 800;
    font-variant-numeric: tabular-nums;
  }
  .mem {
    color: var(--accent);
    font-weight: 800;
    font-variant-numeric: tabular-nums;
  }
  @media (prefers-reduced-motion: reduce) {
    .row,
    .row :global(.icon),
    .sort,
    .arrow {
      transition: none;
    }
  }
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 10;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(0, 0, 0, 0.6);
  }
  .dialog {
    width: min(420px, calc(100vw - 32px));
    padding: 20px;
    border-radius: var(--radius-lg);
    background: var(--surface);
    color: var(--ink);
  }
  .dialog h3 {
    margin: 0 0 8px;
  }
  .dialog-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 16px;
  }
</style>
