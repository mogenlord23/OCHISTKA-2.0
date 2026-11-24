<script lang="ts">
  import { onMount } from 'svelte';
  import BrandMark from './BrandMark.svelte';
  import { listenDesktop, trayAction, trayHover, trayStatus, type TrayAction, type TrayStatus } from '../lib/tray';
  import { toCommandError } from '../lib/api';
  import { HEADING_FONT, NEUTRAL, RADIUS, SECTION_COLORS } from '../lib/tokens';

  const sections: { action: TrayAction; label: string; color: string }[] = [
    { action: 'findings', label: 'Находки', color: SECTION_COLORS.viruses },
    { action: 'processes', label: 'Процессы', color: SECTION_COLORS.processes },
    { action: 'programs', label: 'Программы', color: SECTION_COLORS.programs },
    { action: 'disks', label: 'Диски', color: SECTION_COLORS.disks },
    { action: 'computer', label: 'Компьютер', color: SECTION_COLORS.computer },
    { action: 'design', label: 'Оформление', color: SECTION_COLORS.design },
  ];

  let revision = $state(0);
  let status = $state<TrayStatus>({ vpnConnected: false, scanning: false });
  let pending = $state<TrayAction | null>(null);
  let notice = $state('');

  async function refresh() {
    try { status = await trayStatus(); } catch { /* browser preview */ }
  }

  onMount(() => {
    void refresh();
    const stopOpen = listenDesktop('tray-open', () => {
      revision += 1;
      notice = '';
      void refresh();
    });
    const stopScan = listenDesktop<boolean>('tray-scan-running', ({ payload }) => {
      status = { ...status, scanning: payload };
    });
    return () => { stopOpen(); stopScan(); };
  });

  async function run(action: TrayAction) {
    if (pending) return;
    pending = action;
    notice = '';
    try {
      await trayAction(action);
      await refresh();
    } catch (error) {
      notice = toCommandError(error).message;
    } finally {
      pending = null;
    }
  }
</script>

<section class="tray" aria-label="Быстрые действия OCHISTKA"
  onpointerenter={() => trayHover(true)} onpointerleave={() => trayHover(false)}
  style:--surface={NEUTRAL.surface} style:--panel={NEUTRAL.surfaceAlt}
  style:--ink={NEUTRAL.ink} style:--muted={NEUTRAL.muted}
  style:--border={NEUTRAL.border} style:--radius={RADIUS.sm}
  style:--accent={SECTION_COLORS.viruses} style:--font-heading={HEADING_FONT}>
  {#key revision}
    <div class="contents">
      <header><BrandMark /></header>
      <div class="primary">
        <button type="button" class:connected={status.vpnConnected} disabled={pending !== null} onclick={() => void run('vpn')}>
          {status.vpnConnected ? 'Выключить VPN' : 'Включить VPN'}
        </button>
        <button type="button" class="scan" disabled={pending !== null || status.scanning} onclick={() => void run('scan')}>
          {status.scanning ? 'Идёт проверка' : 'Проверить ПК'}
        </button>
      </div>
      <div class="sections">
        {#each sections as section (section.action)}
          <button type="button" style:--marker={section.color} onclick={() => void run(section.action)}>{section.label}</button>
        {/each}
      </div>
      {#if notice}<p role="status">{notice}</p>{/if}
      <footer>
        <button type="button" onclick={() => void run('open')}>Открыть окно</button>
        <button type="button" onclick={() => void run('exit')}>Выйти</button>
      </footer>
    </div>
  {/key}
</section>

<style>
  .tray {
    height: 100vh;
    overflow-y: auto;
    padding: 12px;
    background: var(--surface);
    color: var(--ink);
  }
  .contents { display: grid; gap: 9px; animation: enter 220ms cubic-bezier(.15,.7,.3,1) both; }
  header, footer { display: flex; justify-content: space-between; align-items: center; gap: 8px; }
  header { min-height: 36px; }
  .primary, .sections { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 6px; }
  button {
    min-height: 35px;
    padding: 5px 8px;
    border-radius: var(--radius);
    border: 1px solid var(--border);
    background: var(--panel);
    color: var(--ink);
    font: 700 12px var(--font-heading);
    cursor: pointer;
    transition: border-color 140ms ease, background 140ms ease;
  }
  button:hover { border-color: var(--accent); }
  button:focus-visible { outline: 2px solid var(--accent); outline-offset: 1px; }
  button:disabled { opacity: .65; cursor: default; }
  .primary button { min-height: 42px; }
  .primary .scan, .primary .connected { background: var(--accent); color: #181a06; }
  .sections button { border-inline-start: 3px solid var(--marker); text-align: left; }
  footer { border-top: 1px solid var(--border); padding-top: 8px; }
  footer button { background: transparent; min-height: 30px; }
  p { margin: 0; color: var(--ink); font-size: 12px; overflow-wrap: anywhere; }
  @keyframes enter { from { opacity: 0; transform: translateY(7px); } to { opacity: 1; transform: translateY(0); } }
  @media (prefers-reduced-motion: reduce) { .contents { animation: none; } button { transition: none; } }
  :global(:root[data-reduce-motion='true']) .contents { animation: none; }
</style>
