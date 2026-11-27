<script lang="ts">
  import { invoke, isTauri } from '@tauri-apps/api/core';
  import { errorMessage } from '../lib/vpnNetwork';
  import './vpnPower.css';

  let { active, vpnState, onchanged }: {
    active: boolean;
    vpnState: string | undefined;
    onchanged: () => void;
  } = $props();

  type ZapretStatus = { running: boolean; configured: boolean };
  const onWindows =
    typeof navigator !== 'undefined' && /Windows/i.test(navigator.userAgent) && isTauri();
  let status = $state<ZapretStatus>();
  let pending = $state<boolean | null>(null);
  let message = $state('');
  const filled = $derived(pending ?? status?.running ?? false);

  function refresh(_vpnState: string | undefined) {
    let current = true;
    void invoke<ZapretStatus>('zapret_status').then(
      (result) => { if (current) status = result; },
      (error) => { if (current) message = errorMessage(error); },
    );
    return () => { current = false; };
  }

  $effect(() => {
    if (onWindows && active && pending === null) return refresh(vpnState);
  });

  async function toggle() {
    if (pending !== null || !status?.configured) return;
    const enabled = !status.running;
    pending = enabled;
    message = '';
    try {
      status = await invoke<ZapretStatus>('zapret_set', { enabled });
    } catch (error) {
      message = errorMessage(error);
    } finally {
      pending = null;
      onchanged();
    }
  }
</script>

{#if onWindows}
  <section class="card" aria-labelledby="zapret-title">
    <button
      type="button"
      class="vpn-power-tile zapret-tile"
      class:filled
      disabled={!status?.configured}
      aria-pressed={status?.running ?? false}
      aria-busy={pending !== null}
      aria-disabled={pending !== null || !status?.configured}
      aria-labelledby="zapret-title"
      onclick={toggle}
    >
      <span class="track" aria-hidden="true"><span class="thumb"></span></span>
    </button>
    <div class="copy">
      <h2 id="zapret-title" class="vpn-power-heading">Обход блокировок</h2>
      {#if message || status?.configured === false}
        <p role="status">{message || 'Функция недоступна в этой сборке'}</p>
      {/if}
    </div>
  </section>
{/if}

<style>
  .card {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: var(--card-inset);
    border-radius: var(--radius-adaptive);
    background: var(--surface-alt);
    box-shadow: inset 0 0 0 2px #000;
  }

  .zapret-tile {
    overflow: hidden;
    transition: color 220ms ease;
  }

  .zapret-tile::before {
    content: '';
    position: absolute;
    inset: 0;
    background: var(--accent);
    transform-origin: left;
    transform: scaleX(0);
    transition: transform 220ms ease;
  }

  .zapret-tile.filled { color: #000; }
  .zapret-tile.filled::before { transform: scaleX(1); }
  .zapret-tile[aria-busy='true'] { opacity: 1; cursor: wait; }

  .track {
    position: relative;
    display: block;
    width: 38px;
    height: 20px;
    border: 2px solid currentColor;
    border-radius: 10px;
  }

  .thumb {
    position: absolute;
    left: 3px;
    top: 3px;
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: currentColor;
    transition: transform 220ms ease;
  }

  .filled .thumb { transform: translateX(18px); }
  h2 { margin: 0; }
  p { margin: 6px 0 0; color: var(--muted); font-size: var(--text-caption); }

  @media (prefers-reduced-motion: reduce) {
    .zapret-tile, .zapret-tile::before, .thumb { transition: none; }
  }
</style>
