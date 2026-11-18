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
  <section class="card" aria-labelledby="services-title">
    <h2 id="services-title">Службы</h2>
    {#if servicesError}
      <p class="alert" role="alert">{servicesError.message}</p>
    {:else if services.length === 0}
      <p class="muted">Служб нет.</p>
    {:else}
      <div class="table-wrap">
        <table>
          <thead>
            <tr><th>Имя</th><th>Отображаемое имя</th><th>Состояние</th></tr>
          </thead>
          <tbody>
            {#each services as service (service.name)}
              <tr>
                <td>{service.name}</td>
                <td>{service.displayName}</td>
                <td>{service.state}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}
    <p class="muted">Только чтение: службы здесь не запускаются и не останавливаются.</p>
  </section>
{:else}
  <section class="card" aria-labelledby="processes-title">
    <h2 id="processes-title" class="sr-only">Процессы</h2>

    <div class="toolbar">
      <input
        type="search"
        class="search"
        placeholder="Поиск по имени или PID"
        aria-label="Поиск по имени или PID"
        bind:value={query}
      />
      <span class="muted">Обновляется каждые 3 с</span>
    </div>

    {#if listError}
      <p class="alert" role="alert">{listError.message}</p>
    {/if}

    <div class="actions" aria-label="Действия с выбранным процессом">
      {#if selected}
        <span class="selected-name">
          <span class="icon" style:background={iconColor(selected.name)} aria-hidden="true"></span>
          {selected.name} · PID {selected.pid}
        </span>
        <button type="button" onclick={() => showPath(selected)}>Расположение файла</button>
        <button
          type="button"
          class="danger"
          disabled={isProtectedPid(selected.pid)}
          title={isProtectedPid(selected.pid)
            ? 'Этот процесс системный, завершать его нельзя'
            : 'Завершить задачу'}
          onclick={() => askEnd(selected)}>Завершить задачу</button
        >
      {:else}
        <span class="muted">Выберите процесс в списке.</span>
      {/if}
    </div>

    {#if pathInfo}
      <p class="path">
        PID {pathInfo.pid}:
        {pathInfo.path ?? 'путь не отдан системой (нет доступа к процессу)'}
      </p>
    {/if}

    {#if actionError}
      <p class="alert" role="alert">{actionError.message}</p>
    {/if}
    {#if notice}
      <p class="notice" role="status">{notice}</p>
    {/if}

    {#if !loaded && !listError}
      <p class="muted">Загружаем список процессов…</p>
    {:else if visible.length === 0 && processes.length > 0}
      <p class="muted">Ничего не найдено по запросу «{query}».</p>
    {:else}
      <div class="table-wrap">
        <table>
          <thead>
            <tr>
              <th scope="col"><span class="sr-only">Значок</span></th>
              <th scope="col">Имя</th>
              <th scope="col" class="num">PID</th>
              <th scope="col" class="num">ЦП</th>
              <th scope="col" class="num">Память</th>
            </tr>
          </thead>
          <tbody>
            {#each visible as proc (proc.pid)}
              <tr class:is-selected={proc.pid === selectedPid}>
                <td>
                  <span class="icon" style:background={iconColor(proc.name)} aria-hidden="true"
                  ></span>
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
  .card {
    margin-top: 16px;
    padding: 16px;
    border-radius: var(--radius-lg);
    background: var(--surface);
    color: var(--ink);
  }
  h2 {
    margin: 0 0 8px;
    font-size: var(--text-title);
  }
  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
  }
  .muted {
    color: var(--muted);
    font-size: var(--text-small);
  }
  .toolbar {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 12px;
  }
  .search {
    flex: 1;
    max-width: 320px;
    padding: 8px 10px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    font: inherit;
    font-size: var(--text-small);
    background: var(--surface-alt);
    color: var(--ink);
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
    min-height: 36px;
    margin-bottom: 8px;
  }
  .selected-name {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-weight: 700;
  }
  button {
    padding: 6px 12px;
    border: 0;
    border-radius: var(--radius-sm);
    background: var(--black);
    color: #fafafa;
    font: inherit;
    font-size: var(--text-small);
    font-weight: 700;
    cursor: pointer;
  }
  button:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }
  button.danger {
    background: #9b1c1c;
  }
  .row-btn {
    padding: 0;
    background: none;
    color: var(--ink);
    text-align: left;
    font-weight: 600;
  }
  .icon {
    display: inline-block;
    width: 12px;
    height: 12px;
    border-radius: 3px;
  }
  .path {
    margin: 4px 0 8px;
    font-family: ui-monospace, Consolas, monospace;
    font-size: var(--text-small);
    word-break: break-all;
  }
  .alert {
    color: #9b1c1c;
    font-weight: 700;
  }
  .notice {
    color: var(--ink-soft);
    font-weight: 700;
  }
  .table-wrap {
    max-height: 60vh;
    overflow: auto;
  }
  table {
    width: 100%;
    border-collapse: collapse;
    font-size: var(--text-small);
  }
  th,
  td {
    padding: 6px 8px;
    border-bottom: 1px solid var(--border);
    text-align: left;
  }
  th {
    position: sticky;
    top: 0;
    background: var(--surface);
  }
  .num {
    text-align: right;
    font-variant-numeric: tabular-nums;
  }
  tr.is-selected {
    background: var(--surface-alt);
  }
  .backdrop {
    position: fixed;
    inset: 0;
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
