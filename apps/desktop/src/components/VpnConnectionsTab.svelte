<script lang="ts">
  import { onMount } from 'svelte';
  import {
    CONNECTIONS_REFRESH_MS,
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
        {#each visible as row, index (first + index)}
          <tr class="row selectable">
            <td>{formatEndpoint(row.localAddr, row.localPort)}</td>
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
{/if}

<style>
  .count {
    margin: 0 0 8px;
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
    margin: 8px 2px;
    color: #000;
  }
</style>
