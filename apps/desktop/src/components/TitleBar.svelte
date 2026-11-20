<script lang="ts">
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { t } from '../lib/i18n.svelte';

  let { maximized = false }: { maximized?: boolean } = $props();

  const win = getCurrentWindow();
</script>

<header class="titlebar" data-tauri-drag-region>
  <div class="brand" data-tauri-drag-region>
    <strong data-tauri-drag-region>OCHISTKA</strong>
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
  /* Строка 64 px: центр Y = 32 общий для слова OCHISTKA, кнопок окна и первого пункта панели. */
  .titlebar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex: none;
    height: 64px;
    padding: 0 var(--content-gutter, 22px);
    user-select: none;
  }

  /* Слово по центру строки 64 px: его ось совпадает с осью кнопок окна (Y = 32). */
  .brand {
    display: flex;
    align-items: center;
    height: 100%;
  }

  strong {
    font-family: var(--font-heading);
    font-size: 28px;
    font-weight: 900;
    line-height: 1;
    letter-spacing: 0.01em;
    color: var(--black);
  }

  .controls {
    display: flex;
    gap: 8px;
  }

  /* Как в прежнем приложении: крупные чёрные скруглённые квадраты, белый толстый знак. */
  button {
    display: grid;
    place-items: center;
    width: 40px;
    height: 40px;
    padding: 0;
    border: 1px solid var(--btn-border);
    border-radius: var(--radius-sm);
    background: var(--black);
    color: #fff;
    cursor: pointer;
  }

  svg {
    width: 24px;
    height: 24px;
    fill: none;
    stroke: currentColor;
    stroke-width: 2.5;
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
