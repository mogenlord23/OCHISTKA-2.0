<script lang="ts">
  import { isTauri } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';
  import SectionIcon from './SectionIcon.svelte';
  import VpnPowerIcon from './VpnPowerIcon.svelte';
  import { listenDesktop, trayAction, trayHover, trayStatus, type TrayAction, type TrayStatus } from '../lib/tray';
  import { getHardware, getSystemOverview, POLL_INTERVAL_MS, toCommandError } from '../lib/api';
  import { shortPercent } from '../lib/computer';
  import { SECTIONS } from '../lib/sections';
  import { t } from '../lib/i18n.svelte';
  import { BODY_FONT, NEUTRAL, SECTION_COLORS, SHELL, textOn } from '../lib/tokens';

  const metrics = [
    { id: 'cpu', label: 'CPU' },
    { id: 'gpu', label: 'GPU' },
    { id: 'ram', label: 'RAM' },
  ] as const;
  const loads = $state<{ cpu: number | null; gpu: number | null; ram: number | null }>({
    cpu: null, gpu: null, ram: null,
  });

  let present = $state(!isTauri());
  let status = $state<TrayStatus | null>(null);
  let pending = $state(false);
  let notice = $state('');
  let systemPending = false;
  let gpuPending = false;

  async function refresh() {
    try {
      status = await trayStatus();
    } catch {
      // Предварительный просмотр в браузере не подключён к приложению.
    }
  }

  async function sampleSystem() {
    if (systemPending) return;
    systemPending = true;
    try {
      const overview = await getSystemOverview();
      loads.cpu = overview.cpu.usagePercent;
      loads.ram = overview.memory.usagePercent;
    } catch {
      // При сбое оставляем последний успешный замер.
    } finally {
      systemPending = false;
    }
  }

  async function sampleGpu() {
    if (gpuPending) return;
    gpuPending = true;
    try {
      loads.gpu = (await getHardware()).gpus[0]?.usagePercent ?? null;
    } catch {
      // Медленный ответ видеокарты не задерживает процессор и память.
    } finally {
      gpuPending = false;
    }
  }

  function sample() {
    void sampleSystem();
    void sampleGpu();
  }

  onMount(() => {
    void refresh();
    // Один стартовый замер готовит значения к первому наведению.
    if (!present) sample();
    const stopOpen = listenDesktop('tray-open', () => {
      present = true;
      notice = '';
      void refresh();
    });
    const stopClose = listenDesktop('tray-closed', () => (present = false));
    return () => {
      stopOpen();
      stopClose();
    };
  });

  $effect(() => {
    if (!present) return;
    sample();
    const timer = window.setInterval(sample, POLL_INTERVAL_MS);
    return () => window.clearInterval(timer);
  });

  async function run(action: TrayAction) {
    if (pending) return;
    pending = true;
    notice = '';
    try {
      await trayAction(action);
      if (action === 'toggleVpn') await refresh();
    } catch (error) {
      notice = toCommandError(error).message;
    } finally {
      pending = false;
    }
  }
</script>

<section
  class="tray"
  class:present
  aria-label="Быстрые действия OCHISTKA"
  inert={!present}
  onpointerenter={() => trayHover(true)}
  onpointerleave={() => trayHover(false)}
  style:--surface={SHELL.window}
  style:--tile={NEUTRAL.surfaceAlt}
  style:--ink={NEUTRAL.ink}
  style:--border={NEUTRAL.border}
  style:--computer={SECTION_COLORS.computer}
  style:--vpn={SECTION_COLORS.vpn}
  style:--on-vpn={textOn(SECTION_COLORS.vpn)}
  style:--font={BODY_FONT}
>
  <div class="tray-columns">
    <div class="overview">
      <button
        type="button"
        class="power"
        class:connected={status?.vpnConnected}
        disabled={pending || status === null}
        aria-pressed={status?.vpnConnected ?? false}
        aria-label={status?.vpnConnected ? 'Выключить VPN' : 'Включить VPN'}
        title={status?.vpnConnected ? 'Выключить VPN' : 'Включить VPN'}
        onclick={() => void run('toggleVpn')}
      >
        <VpnPowerIcon />
      </button>
      <dl class="loads" aria-label="Загрузка компьютера" title="Последние показания. Обновляются, пока открыт трей.">
        {#each metrics as metric (metric.id)}
          {@const value = loads[metric.id]}
          <div class="metric">
            <dt>{metric.label}</dt>
            <dd>
              {value === null ? '—' : shortPercent(value)}
              <span class="load-track" aria-hidden="true">
                <span style:width={(value ?? 0) + '%'}></span>
              </span>
            </dd>
          </div>
        {/each}
      </dl>
    </div>

    <nav class="sections" aria-label={t('navLabel')}>
      {#each SECTIONS as section (section.id)}
        <button type="button" style:--marker={section.accent} disabled={pending} onclick={() => void run(section.id)}>
          <SectionIcon id={section.id} />
          <span>{t(section.titleKey)}</span>
        </button>
      {/each}
    </nav>
  </div>

  {#if notice}<p role="status">{notice}</p>{/if}
</section>

<style>
  :global(html), :global(body) {
    margin: 0;
    background: transparent;
    overflow: hidden;
  }

  .tray {
    --tray-gap: 8px;
    --small-tile: 64px;
    /* Четыре квадрата слева и семь справа заканчиваются на одной линии. */
    --large-tile: calc((7 * var(--small-tile) + 3 * var(--tray-gap)) / 4);
    box-sizing: border-box;
    height: 100dvh;
    padding: 12px;
    overflow: hidden auto;
    scrollbar-width: none;
    border: 1px solid var(--border);
    border-radius: var(--radius-adaptive);
    background: var(--surface);
    color: var(--ink);
    opacity: 0;
    font-family: var(--font);
    transform-origin: bottom center;
  }

  .tray.present {
    opacity: 1;
    animation: enter 180ms cubic-bezier(.16, 1, .3, 1) both;
  }

  .tray-columns {
    display: grid;
    grid-template-columns: var(--large-tile) var(--small-tile);
    gap: var(--tray-gap);
  }

  .overview, .loads, .sections {
    display: grid;
    align-content: start;
    gap: var(--tray-gap);
  }

  .sections {
    min-width: 0;
    grid-template-columns: minmax(0, 1fr);
  }

  button, .metric {
    display: grid;
    place-content: center;
    justify-items: center;
    box-sizing: border-box;
    min-width: 0;
    width: 100%;
    aspect-ratio: 1;
    padding: 3px;
    border: 1px solid var(--border);
    border-radius: var(--radius-adaptive);
    background: var(--tile);
    color: var(--ink);
    text-align: center;
  }

  button { cursor: pointer; }
  button:focus-visible { outline: 2px solid var(--marker, var(--vpn)); outline-offset: 2px; }
  button:disabled { opacity: .65; cursor: default; }

  .power {
    border: 0;
    background: #2a2a2a;
    color: var(--vpn);
    box-shadow: inset 0 0 0 2px var(--vpn);
  }

  .power.connected {
    background-color: var(--vpn);
    color: var(--on-vpn);
  }

  .sections button {
    --section-icon-size: 23px;
    grid-template-columns: minmax(0, 1fr);
    gap: 4px;
    font: 700 9.5px/1.15 var(--font);
  }

  .sections button span { overflow-wrap: anywhere; }

  .sections button:hover:not(:disabled), .sections button:focus-visible {
    border-color: var(--marker);
    color: var(--marker);
    background-color: color-mix(in srgb, var(--tile), var(--marker) 9%);
  }

  .loads { margin: 0; }
  .metric { gap: 4px; }
  .metric dt { color: var(--computer); font-size: 12px; font-weight: 800; }

  .metric dd {
    margin: 0;
    font-size: 27px;
    font-weight: 800;
    font-variant-numeric: tabular-nums;
  }

  .load-track {
    display: block;
    width: 44px;
    height: 3px;
    margin: 5px auto 0;
    overflow: hidden;
    border-radius: var(--radius-adaptive);
    background: var(--border);
  }

  .load-track > span {
    display: block;
    max-width: 100%;
    height: 100%;
    background: var(--computer);
  }

  p {
    margin: var(--tray-gap) 0 0;
    font-size: 12px;
    overflow-wrap: anywhere;
  }

  @keyframes enter {
    from { transform: translateY(9px); }
    to { transform: none; }
  }

  @media (prefers-reduced-motion: reduce) {
    .tray.present { animation: none; }
  }

  :global(:root[data-reduce-motion='true']) .tray.present { animation: none; }
</style>
