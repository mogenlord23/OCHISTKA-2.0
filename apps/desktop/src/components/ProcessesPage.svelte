<script lang="ts">
  import FloatingField from './FloatingField.svelte';
  import ExpandButton from './ExpandButton.svelte';
  import { ACTION_ICONS } from '../lib/actionIcons';
  import { tick } from 'svelte';
  import AppIcon from './AppIcon.svelte';
  import ServicesPage from './ServicesPage.svelte';
  import ContextMenu, { type MenuItem } from './ContextMenu.svelte';
  import {
    filterProcesses,
    formatCpu,
    isProtectedProcess,
    protectedReason,
    REFRESH_MS,
    sortProcesses,
    type ProcessSortKey,
  } from '../lib/processes';
  import { ariaSort, nextSort, type SortState } from '../lib/sortState';
  import { formatMemory } from '../lib/units';
  import { getIcon } from '../lib/programsApi';
  import { prefetchProcessIcons } from '../lib/processIcons';
  import { cachedList, rememberList } from '../lib/listCache';
  import { takeProcessRequest } from '../lib/navigation.svelte';
  import {
    endTask,
    exePath,
    revealExe,
    listProcesses,
    toProcessesError,
    type ProcessInfo,
    type ProcessesErrorInfo,
  } from '../lib/processesApi';

  let { subsection }: { subsection: string } = $props();

  /** Последний список из памяти: страница не пустеет при повторном открытии. */
  const cachedProcesses = cachedList<ProcessInfo>('processes');
  let processes = $state<ProcessInfo[]>(cachedProcesses ?? []);
  let query = $state('');
  /** Запрос из другого раздела («Показать процесс»): выбираем процесс и прокручиваем к нему. */
  const requestedPid = takeProcessRequest();
  let selectedPid = $state<number | null>(requestedPid);
  let listError = $state<ProcessesErrorInfo | null>(null);
  let actionError = $state<ProcessesErrorInfo | null>(null);
  let notice = $state('');
  /** Результат «Расположение файла»: показанный путь или причина, почему показать нельзя. */
  let pathInfo = $state<{ pid: number; path: string | null; reason: string | null } | null>(null);
  let revealing = $state(false);
  /** Процесс, ожидающий подтверждения завершения. Без подтверждения ядро не вызывается. */
  let pending = $state<ProcessInfo | null>(null);

  let cancelButton = $state<HTMLButtonElement | null>(null);

  /** Защита от наложения опросов. Обычная переменная, а не $state: не должна влиять на эффект. */
  let listing = false;

  /** Сортировка по клику на заголовок. `null` — по имени, как в списке Windows. */
  /** При открытии: по загрузке CPU от большего к меньшему; дальше сортирует пользователь. */
  let sort = $state<SortState<ProcessSortKey> | null>({ key: 'cpu', dir: 'desc' });
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
      rememberList('processes', processes);
      prefetchProcessIcons(processes);
      listError = null;
    } catch (err) {
      listError = toProcessesError(err);
    } finally {
      listing = false;
    }
  }

  /** Выбор пришёл из другого раздела: один раз прокручиваем список к выбранной строке. */
  let scrollPending = requestedPid !== null;
  $effect(() => {
    if (!scrollPending || visible.length === 0) return;
    void tick().then(() => {
      const row = document.querySelector('.row.is-selected');
      if (row && typeof row.scrollIntoView === 'function') {
        row.scrollIntoView({ block: 'center' });
        scrollPending = false;
      }
    });
  });

  // Список обновляется раз в REFRESH_MS, пока открыт подраздел «Процессы».
  $effect(() => {
    if (subsection !== 'processes') return;
    void refresh();
    const timer = setInterval(() => void refresh(), REFRESH_MS);
    return () => clearInterval(timer);
  });

  // Фокус на «Отмена» при открытии диалога: так случайный Enter не завершает процесс.
  $effect(() => {
    if (pending && cancelButton) cancelButton.focus();
  });

  /** «Расположение файла»: Проводник с выделенным файлом. Нет пути — причина человеческим языком. */
  async function showPath(proc: ProcessInfo) {
    if (revealing) return;
    revealing = true;
    actionError = null;
    notice = '';
    try {
      const path = await revealExe(proc.pid);
      pathInfo = { pid: proc.pid, path, reason: null };
    } catch (err) {
      const info = toProcessesError(err);
      pathInfo = { pid: proc.pid, path: null, reason: info.message };
    } finally {
      revealing = false;
    }
  }

  /** Контекстное меню строки: у точки нажатия. */
  let menu = $state<{ proc: ProcessInfo; x: number; y: number } | null>(null);
  let menuReturnFocus: HTMLElement | null = null;

  function openMenu(event: MouseEvent, proc: ProcessInfo) {
    event.preventDefault();
    selectedPid = proc.pid;
    const target = event.currentTarget as HTMLElement;
    menuReturnFocus = target.querySelector<HTMLElement>('button.row-btn') ?? target;
    let { clientX: x, clientY: y } = event;
    if (x === 0 && y === 0) {
      const rect = menuReturnFocus.getBoundingClientRect();
      x = rect.left + 24;
      y = rect.bottom;
    }
    menu = { proc, x, y };
  }

  function menuItems(proc: ProcessInfo): MenuItem[] {
    const reason = protectedReason(proc);
    return [
      { id: 'path', label: 'Открыть расположение файла', disabled: revealing },
      { id: 'end', label: 'Завершить задачу', disabled: reason !== null, reason },
    ];
  }

  async function closeMenu() {
    menu = null;
    await tick();
    menuReturnFocus?.focus();
  }

  function onMenuSelect(id: string) {
    const target = menu?.proc;
    menu = null;
    if (!target) return;
    if (id === 'path') void showPath(target);
    else if (id === 'end') askEnd(target);
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
    if (isProtectedProcess(proc)) return;
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
  /** Почему «Завершить задачу» недоступна (для скринридера; видимой подсказки нет). */
  const endReason = $derived(
    !selected
      ? 'Сначала выбери процесс в списке'
      : isProtectedProcess(selected)
        ? (protectedReason(selected) ?? '')
        : '',
  );
</script>

{#if subsection === 'services'}
  <ServicesPage />
{:else}
  <section class="page" aria-labelledby="processes-title">
    <h2 id="processes-title" class="sr-only">Процессы</h2>

    <div class="search-row">
      <FloatingField
        id="proc-search"
        type="search"
        label="Название или PID"
        ariaLabel="Поиск по имени или PID"
        bind:value={query}
      />
    </div>

    <div class="buttons" aria-label="Действия с выбранным процессом">
      <ExpandButton
        icon={ACTION_ICONS.refresh}
        label="Обновить"
        width={96}
        name="Обновить"
        onclick={() => void refresh()}
      />
      <ExpandButton
        icon={ACTION_ICONS.explorer}
        label="Расположение"
        width={120}
        name="Расположение файла"
        disabled={!selected || revealing}
        aria-describedby={selected ? undefined : 'proc-end-reason'}
        onclick={() => selected && showPath(selected)}
      />
      <ExpandButton
        icon={ACTION_ICONS.endTask}
        label="Завершить"
        width={104}
        name="Завершить задачу"
        accent
        disabled={!selected || isProtectedProcess(selected)}
        aria-describedby={endReason ? 'proc-end-reason' : undefined}
        onclick={() => selected && askEnd(selected)}
      />
      <span id="proc-end-reason" class="sr-only">{endReason}</span>
    </div>

    {#if listError}
      <p class="alert" role="alert">{listError.message}</p>
    {/if}
    {#if actionError}
      <p class="alert" role="alert">{actionError.message}</p>
    {/if}
    {#if notice}
      <p class="notice" role="status">{notice}</p>
    {/if}

    {#if visible.length === 0 && processes.length > 0}
      <p class="status-line">Ничего не найдено по запросу «{query}».</p>
    {:else}
      <div class="grid" role="table" aria-label="Процессы" aria-rowcount={visible.length + 1}>
        <div class="grid-head" role="row">
          <span class="c-icon" role="columnheader">
            <span class="sr-only">Значок. Процессов:</span>
            <span class="count-head">{processes.length}</span>
          </span>
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
              oncontextmenu={(event) => openMenu(event, proc)}
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

    <div class="path-box selectable" aria-live="polite">
      {#if pathInfo}
        <span class="path">
          {#if pathInfo.path}
            Показан в Проводнике (PID {pathInfo.pid}): {pathInfo.path}
          {:else}
            PID {pathInfo.pid}: {pathInfo.reason}
          {/if}
        </span>
      {:else if selected}
        {selected.name} · PID {selected.pid} — нажми «Расположение файла»
      {:else}
        Выбери процесс в списке
      {/if}
    </div>
  </section>

  {#if menu}
    <ContextMenu
      label="Действия с процессом «{menu.proc.name}»"
      items={menuItems(menu.proc)}
      x={menu.x}
      y={menu.y}
      onselect={onMenuSelect}
      onclose={() => void closeMenu()}
    />
  {/if}

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
    flex: 1;
    flex-direction: column;
    gap: 8px;
    min-height: 0;
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
    gap: 8px;
  }
  button {
    padding: 10px 14px;
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: var(--radius-sm);
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
    border-radius: var(--radius-xs);
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
  /* Список процессов: сетка из div с ролями таблицы (выделение — скруглённая рамка). */
  /* Список заполняет всю высоту страницы до низа, прокрутка внутри; строки фиксированной высоты. */
  .grid {
    flex: 1;
    min-height: 160px;
    overflow: auto;
    border-radius: var(--radius-lg);
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
  /* Число над значками: тот же вид, что у заголовков, без разрядки: строго по центру колонки. */
  .count-head {
    display: inline-flex;
    align-items: center;
    padding: 6px 4px;
    color: var(--accent);
    font-size: 10px;
    font-weight: 800;
    text-transform: uppercase;
  }
  .sort {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 6px 4px;
    border: 0;
    border-radius: var(--radius-xs);
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
    border-radius: var(--radius-md);
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
    border-radius: var(--radius-sm);
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
