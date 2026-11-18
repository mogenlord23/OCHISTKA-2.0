<script lang="ts">
  import { onMount } from 'svelte';
  import { formatBytes } from '../lib/format';
  import { t } from '../lib/i18n.svelte';
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

{#if error}
  <p class="state" role="alert">{error}</p>
{:else if adapters === null}
  <p class="state" role="status">{t('vpn.net.reading')}</p>
{:else if adapters.length === 0}
  <p class="state">{t('vpn.net.none')}</p>
{:else}
  <ul class="adapters" aria-label={t('vpn.net.list')}>
    {#each adapters as adapter (adapter.name)}
      <li class="card">
        <div class="head">
          <span class="name">{adapter.name}</span>
          <span class="mac">{adapter.mac ?? t('vpn.net.noMac')}</span>
        </div>
        <dl class="stats">
          <div>
            <dt>{t('vpn.net.received')}</dt>
            <dd>{formatBytes(adapter.receivedBytes)}</dd>
          </div>
          <div>
            <dt>{t('vpn.net.sent')}</dt>
            <dd>{formatBytes(adapter.transmittedBytes)}</dd>
          </div>
          <div>
            <dt>{t('vpn.net.rx')}</dt>
            <dd>{formatRate(adapter.receiveRate)}</dd>
          </div>
          <div>
            <dt>{t('vpn.net.tx')}</dt>
            <dd>{formatRate(adapter.transmitRate)}</dd>
          </div>
        </dl>
        <p class="ips">
          {#if adapter.addresses.length > 0}
            {adapter.addresses.join(' · ')}
          {:else}
            {t('vpn.net.noAddr')}
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
    gap: 16px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .card {
    padding: 16px;
    border-radius: var(--radius-lg);
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
    font-size: 18px;
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
    gap: 16px;
    margin: 16px 0 8px;
  }

  dt {
    font-size: var(--text-caption);
    font-weight: 800;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: var(--muted);
  }

  dd {
    margin: 2px 0 0;
    font-size: 16px;
    font-weight: 800;
    font-variant-numeric: tabular-nums;
    color: var(--ink);
  }

  .ips {
    margin: 0;
    overflow-wrap: anywhere;
  }

  .state {
    margin: 0;
    font-weight: 700;
    color: #000;
  }
</style>
