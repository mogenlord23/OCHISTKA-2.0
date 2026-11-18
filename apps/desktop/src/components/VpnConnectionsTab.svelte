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
  <div class="table-wrap">
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
        {#each rows as row, index (index)}
          <tr>
            <td>{formatEndpoint(row.localAddr, row.localPort)}</td>
            <td>{formatEndpoint(row.remoteAddr, row.remotePort)}</td>
            <td><span class="state-chip">{row.state}</span></td>
            <td>{row.processName ?? '—'}</td>
            <td>{row.pid ?? '—'}</td>
          </tr>
        {:else}
          <tr><td colspan="5" class="none">Соединений нет.</td></tr>
        {/each}
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
    max-height: calc(100vh - 330px);
    min-height: 160px;
    overflow: auto;
    border-radius: var(--radius-lg);
    background: var(--surface-alt);
    box-shadow: 0 0 0 2px #000;
    scrollbar-width: thin;
    scrollbar-color: var(--accent) transparent;
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

  td {
    padding: 7px 12px;
    border-top: 1px solid var(--border);
    color: var(--ink);
    white-space: nowrap;
  }

  .state-chip {
    padding: 2px 8px;
    border-radius: 999px;
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
