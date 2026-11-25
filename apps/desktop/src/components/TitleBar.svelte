<script lang="ts">
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { t } from '../lib/i18n.svelte';
  import type { Snippet } from 'svelte';

  let { maximized = false, children }: { maximized?: boolean; children?: Snippet } = $props();

  const win = getCurrentWindow();
</script>

<header class="titlebar" data-tauri-drag-region>
  <div class="tabs" data-tauri-drag-region>
    {@render children?.()}
  </div>
  <div class="controls">
    <button type="button" aria-label={t('minimize')} onclick={() => win.minimize()}>
      <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M6.5 12h11" /></svg>
    </button>
    <button
      type="button"
      class="maximize"
      aria-label={t('maximize')}
      onclick={() => win.toggleMaximize()}
    >
      <svg viewBox="0 0 24 24" aria-hidden="true">
        {#if maximized}
          <path d="M9.5 8V6.5a1 1 0 0 1 1-1h7a1 1 0 0 1 1 1v7a1 1 0 0 1-1 1H16" />
          <rect x="5" y="9.5" width="10.5" height="9.5" rx="1.5" />
        {:else}
          <rect x="6" y="6" width="12" height="12" rx="2" />
        {/if}
      </svg>
    </button>
    <button type="button" class="close" aria-label={t('close')} onclick={() => win.close()}>
      <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M7 7l10 10M17 7 7 17" /></svg>
    </button>
  </div>
</header>

<style>
  .titlebar {
    --subtabs-height: 40px;
    --subtabs-padding: 3px;
    display: flex;
    align-items: stretch;
    flex: none;
    min-width: 0;
    gap: 8px;
    min-height: var(--subtabs-height);
    user-select: none;
  }

  .tabs {
    display: flex;
    flex: 1;
    min-width: 0;
  }

  .tabs :global(.subtabs) {
    flex: 1;
    margin: 0;
  }

  .tabs :global(.subtabs button) {
    padding-inline: clamp(6px, 1vw, 16px);
  }

  .controls {
    display: flex;
    gap: 8px;
  }

  button {
    display: grid;
    place-items: center;
    box-sizing: border-box;
    width: var(--subtabs-height);
    height: var(--subtabs-height);
    padding: 0;
    border: 1px solid var(--btn-border);
    border-radius: var(--radius-sm);
    background: var(--black);
    color: #fff;
    cursor: pointer;
  }

  svg {
    width: 20px;
    height: 20px;
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
    shape-rendering: geometricPrecision;
  }

  button:hover {
    background: var(--btn-hover);
  }

  button.close:hover {
    background: #ff009d;
    color: var(--black);
  }
</style>
