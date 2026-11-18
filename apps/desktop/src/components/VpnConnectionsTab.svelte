<script lang="ts">
  import { onMount } from 'svelte';
  import { t, type Key } from '../lib/i18n.svelte';
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
  const ROW_H = 32;
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

  const COLUMNS: { key: ConnectionSortKey; label: Key | 'PID' }[] = [
    { key: 'local', label: 'vpn.conn.local' },
    { key: 'remote', label: 'vpn.conn.remote' },
    { key: 'state', label: 'vpn.conn.state' },
    { key: 'process', label: 'vpn.conn.process' },
    { key: 'pid', label: 'PID' },
  ];

  /** Выбранная строка (по ключу «адрес:порт:pid»): один маркер-обводка, без флажков и галочек. */
  let selectedKey = $state<string | null>(null);
  const rowKey = (row: TcpConnection) =>
    `${row.localAddr}:${row.localPort}>${row.remoteAddr}:${row.remotePort}#${row.pid ?? ''}`;

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
  <p class="state" role="status">{t('vpn.conn.reading')}</p>
{:else}
  <div class="table-wrap" use:measure onscroll={(e) => (scrollTop = e.currentTarget.scrollTop)}>
    <p class="count">{t('vpn.conn.count').replace('{n}', String(connections.length))}</p>
    <table>
      <thead>
        <tr>
          {#each COLUMNS as column (column.key)}
            <th scope="col" aria-sort={ariaSort(column.key)}>
              <button type="button" onclick={() => (sort = nextSort(sort, column.key))}>
                {column.label === 'PID' ? column.label : t(column.label)}
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
          <tr
            class="row"
            class:selected={selectedKey === rowKey(row)}
            aria-current={selectedKey === rowKey(row) ? 'true' : undefined}
            tabindex="0"
            onclick={() => (selectedKey = rowKey(row))}
            onkeydown={(e) => {
              if (e.key === 'Enter' || e.key === ' ') {
                e.preventDefault();
                selectedKey = rowKey(row);
              }
            }}
          >
            <td>{formatEndpoint(row.localAddr, row.localPort)}</td>
            <td>{formatEndpoint(row.remoteAddr, row.remotePort)}</td>
            <td><span class="state-chip">{row.state}</span></td>
            <td>{row.processName ?? '—'}</td>
            <td>{row.pid ?? '—'}</td>
          </tr>
        {:else}
          <tr><td colspan="5" class="none">{t('vpn.conn.none')}</td></tr>
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
  /* Счётчик внутри карточки, чтобы содержимое начиналось там же, где у остальных вкладок. */
  .count {
    position: sticky;
    top: 0;
    z-index: 1;
    left: 0;
    height: 40px;
    margin: 0;
    padding: 0 16px;
    line-height: 40px;
    background: var(--black);
    font-size: var(--text-caption);
    font-weight: 800;
    color: var(--ink);
  }

  .table-wrap {
    height: clamp(240px, calc(100vh - 336px), 960px);
    overflow: auto;
    border-radius: var(--radius-lg);
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
    top: 40px;
    padding: 0;
    background: #000;
    text-align: left;
  }

  th button {
    width: 100%;
    min-height: 40px;
    padding: 8px 16px;
    border: 0;
    background: transparent;
    color: var(--ink);
    font: inherit;
    font-size: var(--text-caption);
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
    font-size: var(--text-caption);
  }

  tr.row {
    height: 32px;
    cursor: pointer;
  }

  /* Выбранная строка: одна обводка в цвете раздела, без галочек и заливки-полосы. */
  tr.row.selected,
  tr.row:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: -2px;
  }

  tr.spacer td {
    padding: 0;
    border: 0;
  }

  td {
    padding: 0 16px;
    border-top: 1px solid var(--border);
    color: var(--ink);
    white-space: nowrap;
  }

  .state-chip {
    padding: 0 8px;
    border-radius: 999px;
    background: #222;
    color: var(--accent);
    font-size: var(--text-caption);
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
