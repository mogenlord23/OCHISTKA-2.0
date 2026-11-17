<script lang="ts">
  import { getCurrentWindow } from '@tauri-apps/api/window';

  const win = getCurrentWindow();
</script>

<header class="titlebar" data-tauri-drag-region>
  <div class="brand" data-tauri-drag-region>
    <strong data-tauri-drag-region>OCHISTKA</strong>
    <small data-tauri-drag-region>Create: Slava Lavs</small>
  </div>
  <div class="controls">
    <button type="button" aria-label="Свернуть" onclick={() => win.minimize()}>—</button>
    <button type="button" aria-label="Развернуть" onclick={() => win.toggleMaximize()}>▢</button>
    <button type="button" class="close" aria-label="Закрыть" onclick={() => win.close()}>✕</button>
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
    font-size: 26px;
    font-weight: 900;
    letter-spacing: 0.02em;
    color: #080808;
  }

  small {
    font-size: 11px;
    color: #080808;
  }

  .controls {
    display: flex;
    gap: 8px;
  }

  button {
    width: 44px;
    height: 40px;
    border: 0;
    border-radius: 10px;
    background: #080808;
    color: #fafafa;
    font: inherit;
    font-size: 16px;
    cursor: pointer;
  }

  button:hover {
    background: #1f1f1f;
  }

  button.close:hover {
    background: #ff009d;
    color: #080808;
  }
</style>
