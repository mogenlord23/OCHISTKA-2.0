<script lang="ts">
  import { onMount, tick } from 'svelte';
  import ContextMenu from './ContextMenu.svelte';
  import { exePath, revealExe, toProcessesError } from '../lib/processesApi';
  import { showProcess } from '../lib/navigation.svelte';
  import {
    CONNECTIONS_REFRESH_MS,
    connectionKey,
    connectionMenuItems,
    endpointText,
    type ConnectionAction,
    errorMessage,
    formatEndpoint,
    listConnections,
    nextSort,
    sortConnections,
    type ConnectionSort,
    type ConnectionSortKey,
    type TcpConnection,
  } from '../lib/vpnNetwork';

  let connections = $state<TcpConnection[] | null>(null);
  let error = $state<string | null>(null);
  let sort = $state<ConnectionSort>({ key: 'process', direction: 'asc' });

  const rows = $derived(connections ? sortConnections(connections, sort) : []);

  /** Виртуализация: в DOM только видимые строки плюс запас, остальное заменяют пустые строки-распорки. */
  const ROW_H = 30;
  const OVERSCAN = 8;
  function measure(node: HTMLElement) {
    viewH = node.clientHeight || viewH;
    if (typeof ResizeObserver === 'undefined') return;
    const ro = new ResizeObserver(() => (viewH = node.clientHeight || viewH));
    ro.observe(node);
    return { destroy: () => ro.disconnect() };
  }
  let scrollTop = $state(0);
  let viewH = $state(480);
  const first = $derived(Math.max(0, Math.floor(scrollTop / ROW_H) - OVERSCAN));
  const last = $derived(Math.min(rows.length, Math.ceil((scrollTop + viewH) / ROW_H) + OVERSCAN));
  const visible = $derived(rows.slice(first, last));

  const COLUMNS: { key: ConnectionSortKey; label: string }[] = [
    { key: 'local', label: 'Локальный адрес' },
    { key: 'remote', label: 'Удалённый адрес' },
    { key: 'state', label: 'Состояние' },
    { key: 'process', label: 'Процесс' },
    { key: 'pid', label: 'PID' },
  ];

  /** Выбранная строка держится по ключу соединения, а не по номеру: список обновляется. */
  let selectedKey = $state<string | null>(null);
  const selected = $derived(
    connections?.find((connection) => connectionKey(connection) === selectedKey) ?? null,
  );
  let notice = $state('');
  let filePath = $state<{ key: string; path: string | null } | null>(null);

  function toggle(row: TcpConnection) {
    const key = connectionKey(row);
    selectedKey = selectedKey === key ? null : key;
  }

  // Путь файла процесса для подробностей: читаем только у выбранной строки.
  $effect(() => {
    const row = selected;
    if (!row || row.pid === null) {
      filePath = null;
      return;
    }
    const key = connectionKey(row);
    if (filePath?.key === key) return;
    filePath = null;
    void exePath(row.pid)
      .then((path) => {
        if (selectedKey === key) filePath = { key, path };
      })
      .catch(() => {
        if (selectedKey === key) filePath = { key, path: null };
      });
  });

  let menu = $state<{ row: TcpConnection; x: number; y: number } | null>(null);
  let menuReturnFocus: HTMLElement | null = null;

  function openMenu(event: MouseEvent, row: TcpConnection) {
    event.preventDefault();
    selectedKey = connectionKey(row);
    const target = event.currentTarget as HTMLElement;
    menuReturnFocus = target.querySelector<HTMLElement>('button.row-btn') ?? target;
    let { clientX: x, clientY: y } = event;
    if (x === 0 && y === 0) {
      const rect = menuReturnFocus.getBoundingClientRect();
      x = rect.left + 24;
      y = rect.bottom;
    }
    menu = { row, x, y };
  }

  async function closeMenu() {
    menu = null;
    await tick();
    menuReturnFocus?.focus();
  }

  async function copy(text: string) {
    try {
      await navigator.clipboard.writeText(text);
      notice = `Скопировано: ${text}`;
    } catch {
      notice = 'Не удалось скопировать';
    }
  }

  async function onMenuSelect(id: string) {
    const row = menu?.row;
    menu = null;
    if (!row) return;
    const action = id as ConnectionAction;
    if (action === 'copy-remote') await copy(endpointText(row, 'remote'));
    else if (action === 'copy-local') await copy(endpointText(row, 'local'));
    else if (action === 'show-process' && row.pid !== null) showProcess(row.pid);
    else if (action === 'open-location' && row.pid !== null) {
      try {
        await revealExe(row.pid);
        notice = 'Расположение файла открыто в Проводнике';
      } catch (err) {
        notice = toProcessesError(err).message;
      }
    }
  }

  async function load() {
    try {
      connections = await listConnections();
      error = null;
    } catch (err) {
      error = errorMessage(err);
    }
  }

  function ariaSort(key: ConnectionSortKey): 'ascending' | 'descending' | 'none' {
    if (sort.key !== key) return 'none';
    return sort.direction === 'asc' ? 'ascending' : 'descending';
  }

  onMount(() => {
    void load();
    const timer = setInterval(() => void load(), CONNECTIONS_REFRESH_MS);
    return () => clearInterval(timer);
  });
</script>

{#if error}
  <p class="state" role="alert">{error}</p>
{:else if connections === null}
  <p class="state" role="status">Читаем соединения…</p>
{:else}
  <p class="count">TCP-соединений: {connections.length}</p>
  <div class="table-wrap" use:measure onscroll={(e) => (scrollTop = e.currentTarget.scrollTop)}>
    <table>
      <thead>
        <tr>
          {#each COLUMNS as column (column.key)}
            <th scope="col" aria-sort={ariaSort(column.key)}>
              <button type="button" onclick={() => (sort = nextSort(sort, column.key))}>
                {column.label}
                <span class="arrow" aria-hidden="true"
                  >{sort.key === column.key ? (sort.direction === 'asc' ? '▲' : '▼') : ''}</span
                >
              </button>
            </th>
          {/each}
        </tr>
      </thead>
      <tbody>
        {#if first > 0}
          <tr class="spacer" aria-hidden="true" style:height="{first * ROW_H}px"
            ><td colspan="5"></td></tr
          >
        {/if}
        {#each visible as row (connectionKey(row))}
          <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions -->
          <tr
            class="row selectable"
            class:is-selected={connectionKey(row) === selectedKey}
            onclick={() => toggle(row)}
            oncontextmenu={(event) => openMenu(event, row)}
          >
            <td
              ><button
                type="button"
                class="row-btn"
                aria-pressed={connectionKey(row) === selectedKey}
                onclick={(event) => {
                  event.stopPropagation();
                  toggle(row);
                }}>{formatEndpoint(row.localAddr, row.localPort)}</button
              ></td
            >
            <td>{formatEndpoint(row.remoteAddr, row.remotePort)}</td>
            <td><span class="state-chip">{row.state}</span></td>
            <td>{row.processName ?? '—'}</td>
            <td>{row.pid ?? '—'}</td>
          </tr>
        {:else}
          <tr><td colspan="5" class="none">Соединений нет.</td></tr>
        {/each}
        {#if last < rows.length}
          <tr class="spacer" aria-hidden="true" style:height="{(rows.length - last) * ROW_H}px"
            ><td colspan="5"></td></tr
          >
        {/if}
      </tbody>
    </table>
  </div>
  <div class="details selectable" aria-live="polite">
    {#if selected}
      <span class="d-main"
        >{formatEndpoint(selected.localAddr, selected.localPort)} → {formatEndpoint(
          selected.remoteAddr,
          selected.remotePort,
        )}</span
      >
      <span class="d-sub"
        >{selected.state} · {selected.processName ?? 'процесс не определён'}{selected.pid !== null
          ? ` · PID ${selected.pid}`
          : ''}</span
      >
      {#if filePath?.path}<span class="d-sub path">{filePath.path}</span>{/if}
    {:else}
      <span class="d-sub"
        >Выбери соединение: покажем подробности. Правая кнопка мыши — действия.</span
      >
    {/if}
    {#if notice}<span class="d-note" role="status">{notice}</span>{/if}
  </div>
  {#if menu}
    <ContextMenu
      label="Действия с соединением"
      items={connectionMenuItems(menu.row)}
      x={menu.x}
      y={menu.y}
      onselect={(id) => void onMenuSelect(id)}
      onclose={() => void closeMenu()}
    />
  {/if}
{/if}

<style>
  .count {
    margin: 0;
    font-size: var(--text-caption);
    font-weight: 800;
    color: #000;
  }

  .table-wrap {
    /* Таблица тянется до нижней линии постера: колонка и постер заканчиваются на одной линии. */
    flex: 1;
    min-height: 160px;
    overflow: auto;
    border-radius: var(--radius-md);
    background: var(--surface-alt);
    box-shadow: 0 0 0 2px #000;
    contain: layout paint;
    overscroll-behavior: contain;
  }

  table {
    width: 100%;
    border-collapse: collapse;
    font-size: var(--text-caption);
    font-variant-numeric: tabular-nums;
  }

  th {
    position: sticky;
    top: 0;
    padding: 0;
    background: #000;
    text-align: left;
  }

  th button {
    width: 100%;
    min-height: 38px;
    padding: 8px 12px;
    border: 0;
    background: transparent;
    color: var(--ink);
    font: inherit;
    font-size: 0.7rem;
    font-weight: 800;
    letter-spacing: 0.1em;
    text-align: left;
    text-transform: uppercase;
    cursor: pointer;
  }

  th[aria-sort='ascending'] button,
  th[aria-sort='descending'] button {
    color: var(--accent);
  }

  .arrow {
    font-size: 0.6rem;
  }

  tr.row {
    height: 30px;
  }

  tr.spacer td {
    padding: 0;
    border: 0;
  }

  td {
    padding: 0 12px;
    border-top: 1px solid var(--border);
    color: var(--ink);
    white-space: nowrap;
  }

  tr.row {
    cursor: pointer;
  }

  tr.row:hover td {
    background: color-mix(in srgb, var(--accent) 8%, transparent);
  }

  tr.row.is-selected td {
    background: color-mix(in srgb, var(--accent) 16%, #111);
  }

  tr.row.is-selected td:first-child {
    box-shadow: inset 3px 0 0 var(--accent);
  }

  .row-btn {
    padding: 0;
    border: 0;
    background: none;
    color: inherit;
    font: inherit;
    font-variant-numeric: tabular-nums;
    cursor: pointer;
  }

  .row-btn:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 3px;
    border-radius: var(--radius-xs);
  }

  .details {
    display: flex;
    flex-direction: column;
    flex-shrink: 0;
    gap: 2px;
    min-height: 64px;
    margin-top: 0;
    padding: 10px 14px;
    border-radius: var(--radius-md);
    background: var(--surface-alt);
    box-shadow: 0 0 0 2px #000;
    font-size: var(--text-caption);
    font-variant-numeric: tabular-nums;
  }

  .d-main {
    font-weight: 800;
    color: var(--ink);
  }

  .d-sub,
  .d-note {
    color: var(--muted);
    overflow-wrap: anywhere;
  }

  .d-note {
    color: var(--accent);
  }

  .state-chip {
    padding: 2px 8px;
    border-radius: var(--radius-pill);
    background: #222;
    color: var(--accent);
    font-size: 0.7rem;
    font-weight: 700;
  }

  .none,
  .state {
    color: var(--muted);
    font-weight: 700;
  }

  .state {
    margin: 0;
    color: #000;
  }
</style>
