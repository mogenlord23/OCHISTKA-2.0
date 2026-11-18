<script lang="ts">
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { t } from '../lib/i18n.svelte';

  const win = getCurrentWindow();
</script>

<header class="titlebar" data-tauri-drag-region>
  <div class="brand" data-tauri-drag-region>
    <strong data-tauri-drag-region>OCHISTKA</strong>
    <small data-tauri-drag-region>Create: Slava Lavs</small>
  </div>
  <div class="controls">
    <button type="button" aria-label={t('minimize')} onclick={() => win.minimize()}>—</button>
    <button type="button" aria-label={t('maximize')} onclick={() => win.toggleMaximize()}>▢</button>
    <button type="button" class="close" aria-label={t('close')} onclick={() => win.close()}
      >✕</button
    >
  </div>
</header>

<style>
  .titlebar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 16px;
    user-select: none;
  }

  .brand {
    display: flex;
    flex-direction: column;
    line-height: 1.1;
  }

  strong {
    font-size: var(--text-section);
    font-weight: 900;
    letter-spacing: 0.02em;
    color: var(--black);
  }

  small {
    font-size: var(--text-caption);
    color: var(--black);
  }

  .controls {
    display: flex;
    gap: 8px;
  }

  button {
    width: 44px;
    height: 44px;
    border: 0;
    border-radius: 10px;
    background: var(--black);
    color: var(--ink);
    font: inherit;
    font-size: var(--text-title);
    cursor: pointer;
  }

  button:hover {
    background: #1f1f1f;
  }

  button.close:hover {
    background: #ff009d;
    color: var(--black);
  }
</style>
