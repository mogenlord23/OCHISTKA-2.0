<script lang="ts">
  import { isTauri } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';
  import SectionIcon from './SectionIcon.svelte';
  import { listenDesktop, trayAction, trayHover, trayStatus, type TrayAction, type TrayStatus } from '../lib/tray';
  import { toCommandError } from '../lib/api';
  import { SECTIONS } from '../lib/sections';
  import { t } from '../lib/i18n.svelte';
  import { BODY_FONT, NEUTRAL, RADIUS, SECTION_COLORS, SHELL, textOn } from '../lib/tokens';

  let present = $state(!isTauri());
  let status = $state<TrayStatus | null>(null);
  let pending = $state(false);
  let notice = $state('');

  async function refresh() {
    try {
      status = await trayStatus();
    } catch {
      // Предварительный просмотр в браузере не подключён к приложению.
    }
  }

  onMount(() => {
    void refresh();
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
  style:--window-radius={RADIUS.lg}
  style:--tile-radius={RADIUS.sm}
  style:--vpn={SECTION_COLORS.vpn}
  style:--on-vpn={textOn(SECTION_COLORS.vpn)}
  style:--font={BODY_FONT}
>
  <button
    type="button"
    class="power"
    class:connected={status?.vpnConnected}
    disabled={pending || status === null}
    aria-pressed={status?.vpnConnected ?? false}
    onclick={() => void run('toggleVpn')}
  >
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" aria-hidden="true">
      <path d="M12 3v9m-5.5-6a8 8 0 1 0 11 0" />
    </svg>
    {status?.vpnConnected ? 'Выключить VPN' : 'Включить VPN'}
  </button>

  {#if notice}<p role="status">{notice}</p>{/if}

  <div class="sections">
    {#each SECTIONS as section (section.id)}
      <button type="button" style:--marker={section.accent} disabled={pending} onclick={() => void run(section.id)}>
        <SectionIcon id={section.id} />
        <span>{t(section.titleKey)}</span>
      </button>
    {/each}
  </div>
</section>

<style>
  :global(html), :global(body) {
    margin: 0;
    background: transparent;
    overflow: hidden;
  }

  .tray {
    display: grid;
    gap: 10px;
    align-content: start;
    box-sizing: border-box;
    height: 100dvh;
    padding: 12px;
    overflow: hidden auto;
    scrollbar-width: none;
    border: 1px solid var(--border);
    border-radius: var(--window-radius);
    background: var(--surface);
    color: var(--ink);
    opacity: 0;
    font-family: var(--font);
    transform-origin: bottom center;
  }

  .tray.present {
    opacity: 1;
    animation: enter 260ms cubic-bezier(.16, 1, .3, 1) both;
  }

  button {
    display: grid;
    place-content: center;
    justify-items: center;
    gap: 5px;
    min-width: 0;
    padding: 4px;
    border: 1px solid var(--border);
    border-radius: var(--tile-radius);
    background: var(--tile);
    color: var(--ink);
    font: 700 11px/1.15 var(--font);
    text-align: center;
    cursor: pointer;
    transition: border-color 150ms ease, color 150ms ease, background-color 150ms ease;
  }

  button:focus-visible { outline: 2px solid var(--marker, var(--vpn)); outline-offset: 2px; }
  button:disabled { opacity: .65; cursor: default; }

  .power {
    --marker: var(--vpn);
    grid-auto-flow: column;
    align-items: center;
    gap: 8px;
    min-height: 44px;
    border-color: var(--vpn);
    color: var(--vpn);
    font-size: 13px;
  }

  .power svg { width: 20px; height: 20px; }

  .power.connected {
    background-color: var(--vpn);
    color: var(--on-vpn);
  }

  .sections {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    gap: 8px;
  }

  .sections button {
    flex: 0 0 calc((100% - 24px) / 4);
    aspect-ratio: 1;
  }

  .sections button:hover:not(:disabled), .sections button:focus-visible {
    border-color: var(--marker);
    color: var(--marker);
    background-color: color-mix(in srgb, var(--tile), var(--marker) 9%);
  }

  p { margin: 0; font-size: 12px; overflow-wrap: anywhere; }

  @keyframes enter {
    from { opacity: 0; transform: translateY(12px) scale(.985); }
    to { opacity: 1; transform: none; }
  }

  @media (prefers-reduced-motion: reduce) {
    .tray.present { animation: none; }
    button { transition: none; }
  }

  :global(:root[data-reduce-motion='true']) .tray.present { animation: none; }
</style>
