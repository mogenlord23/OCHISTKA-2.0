<script lang="ts">
  import { flagUrl } from '../lib/countryFlags';

  /** Флаг страны сервера: локальный SVG одного размера; без флага — заглушка в цветах приложения. */
  let { code = null }: { code?: string | null } = $props();
  const url = $derived(flagUrl(code));
</script>

{#if url}
  <img class="flag" src={url} alt="" width="24" height="18" draggable="false" />
{:else}
  <!-- Нет флага для кода страны (например, «EU»): нейтральная заглушка в цветах приложения. -->
  <svg class="flag placeholder" viewBox="0 0 24 18" width="24" height="18" aria-hidden="true">
    <path d="M0 0h24v18H0z" class="half-pink" />
    <path d="M0 18V0h24z" class="half-accent" />
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

  .placeholder {
    box-shadow: 0 0 0 1px rgba(255, 255, 255, 0.14);
  }

  .half-pink {
    fill: #ff4fd8;
  }

  .half-accent {
    fill: var(--accent);
  }
</style>
