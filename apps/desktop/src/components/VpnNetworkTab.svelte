<script lang="ts">
  import { onMount } from 'svelte';
  import SpeedTest from './SpeedTest.svelte';
  import { formatBytes } from '../lib/format';
  import {
    NETWORK_REFRESH_MS,
    errorMessage,
    formatRate,
    listAdapters,
    type NetworkAdapter,
  } from '../lib/vpnNetwork';

  let adapters = $state<NetworkAdapter[] | null>(null);
  let error = $state<string | null>(null);

  async function load() {
    try {
      adapters = await listAdapters();
      error = null;
    } catch (err) {
      error = errorMessage(err);
    }
  }

  onMount(() => {
    void load();
    const timer = setInterval(() => void load(), NETWORK_REFRESH_MS);
    return () => clearInterval(timer);
  });
</script>

<SpeedTest />

{#if error}
  <p class="state" role="alert">{error}</p>
{:else if adapters === null}
  <p class="state" role="status">Читаем сетевые адаптеры…</p>
{:else if adapters.length === 0}
  <p class="state">Сетевых адаптеров не найдено.</p>
{:else}
  <ul class="adapters" aria-label="Сетевые адаптеры">
    {#each adapters as adapter (adapter.name)}
      <li class="card">
        <div class="head">
          <span class="name">{adapter.name}</span>
          <span class="mac selectable">{adapter.mac ?? 'MAC не задан'}</span>
        </div>
        <dl class="stats">
          <div>
            <dt>Получено</dt>
            <dd>{formatBytes(adapter.receivedBytes)}</dd>
          </div>
          <div>
            <dt>Отправлено</dt>
            <dd>{formatBytes(adapter.transmittedBytes)}</dd>
          </div>
          <div>
            <dt>Приём</dt>
            <dd>{formatRate(adapter.receiveRate)}</dd>
          </div>
          <div>
            <dt>Передача</dt>
            <dd>{formatRate(adapter.transmitRate)}</dd>
          </div>
        </dl>
        <p class="ips">
          {#if adapter.addresses.length > 0}
            {adapter.addresses.join(' · ')}
          {:else}
            Адресов нет
          {/if}
        </p>
      </li>
    {/each}
  </ul>
{/if}

<style>
  .adapters {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .card {
    padding: 16px 18px;
    border-radius: var(--radius-md);
    background: var(--surface-alt);
    box-shadow: 0 0 0 2px #000;
  }

  .head {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    justify-content: space-between;
    gap: 4px 16px;
  }

  .name {
    font-size: 1.1rem;
    font-weight: 800;
    color: var(--ink);
  }

  .mac,
  .ips {
    font-size: var(--text-caption);
    color: var(--muted);
    font-variant-numeric: tabular-nums;
  }

  .stats {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(130px, 1fr));
    gap: 8px;
    margin: 8px 0 8px;
  }

  dt {
    font-size: 0.7rem;
    font-weight: 800;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: var(--muted);
  }

  dd {
    margin: 2px 0 0;
    font-size: 1.05rem;
    font-weight: 800;
    font-variant-numeric: tabular-nums;
    color: var(--ink);
  }

  .ips {
    margin: 0;
    overflow-wrap: anywhere;
  }

  .state {
    margin: 8px 2px;
    font-weight: 700;
    color: #000;
  }
</style>
