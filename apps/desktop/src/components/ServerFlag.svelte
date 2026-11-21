<script lang="ts">
  import { flagUrl } from '../lib/countryFlags';

  /** Флаг страны сервера: локальный SVG одного размера; неизвестная страна — нейтральный глобус. */
  let { code = null }: { code?: string | null } = $props();
  const url = $derived(flagUrl(code));
</script>

{#if url}
  <img class="flag" src={url} alt="" width="24" height="18" draggable="false" />
{:else}
  <svg class="flag globe" viewBox="0 0 24 18" width="24" height="18" aria-hidden="true">
    <circle cx="12" cy="9" r="6.2" />
    <path d="M5.8 9h12.4M12 2.8c-2.2 1.9-2.2 10.5 0 12.4M12 2.8c2.2 1.9 2.2 10.5 0 12.4" />
  </svg>
{/if}

<style>
  .flag {
    display: block;
    flex: none;
    width: 24px;
    height: 18px;
    border-radius: var(--radius-xs);
    object-fit: cover;
    box-shadow: 0 0 0 1px rgba(255, 255, 255, 0.14);
  }

  .globe {
    box-shadow: none;
    fill: none;
    stroke: var(--muted);
    stroke-width: 1.3;
    stroke-linecap: round;
  }
</style>
