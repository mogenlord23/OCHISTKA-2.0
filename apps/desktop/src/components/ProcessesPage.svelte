<script lang="ts">
  import { formatBytes } from '../lib/format';
  import {
    filterProcesses,
    formatCpu,
    iconColor,
    isProtectedPid,
    REFRESH_MS,
  } from '../lib/processes';
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

  const visible = $derived(filterProcesses(processes, query));
  const selected = $derived(processes.find((proc) => proc.pid === selectedPid) ?? null);

  async function refresh() {
    if (listing) return;
    listing = true;
    try {
      processes = await listProcesses();
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
    <p class="lead">
      CPU — доля всех ядер · память — рабочий набор процесса · правый клик по строке — выбрать
    </p>

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
      Процессов: {processes.length}{updatedAt ? ` · обновлено ${updatedAt}` : ''} · обновление каждые
      3 секунды
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
      <div class="table-wrap">
        <table>
          <thead>
            <tr>
              <th scope="col" class="col-icon"><span class="sr-only">Значок</span></th>
              <th scope="col">Процесс</th>
              <th scope="col" class="num">PID</th>
              <th scope="col" class="num">CPU</th>
              <th scope="col" class="num">Память</th>
            </tr>
          </thead>
          <tbody>
            {#each visible as proc (proc.pid)}
              <tr
                class:is-selected={proc.pid === selectedPid}
                oncontextmenu={(event) => {
                  event.preventDefault();
                  selectedPid = proc.pid;
                }}
              >
                <td class="col-icon">
                  <span class="icon" style:background={iconColor(proc.name)} aria-hidden="true"
                    >{proc.name.charAt(0).toUpperCase()}</span
                  >
                </td>
                <td>
                  <button
                    type="button"
                    class="row-btn"
                    aria-pressed={proc.pid === selectedPid}
                    onclick={() => (selectedPid = proc.pid)}>{proc.name}</button
                  >
                </td>
                <td class="num">{proc.pid}</td>
                <td class="num">{formatCpu(proc.cpuPercent)}</td>
                <td class="num">{formatBytes(proc.memoryBytes)}</td>
              </tr>
            {/each}
          </tbody>
        </table>
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
    padding: 0;
    border: 0;
    background: none;
    color: var(--ink);
    text-align: left;
    font-weight: 500;
  }
  .icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 22px;
    height: 22px;
    border-radius: 5px;
    color: #080808;
    font-size: 11px;
    font-weight: 800;
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
    max-height: calc(100vh - 486px);
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
  .col-icon {
    width: 22px;
    padding-right: 0;
  }
  .num {
    width: 90px;
    text-align: left;
    font-variant-numeric: tabular-nums;
  }
  tbody tr:hover {
    background: #181818;
  }
  tr.is-selected {
    background: #222;
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
