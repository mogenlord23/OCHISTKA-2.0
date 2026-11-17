<script lang="ts">
  import PlaceholderPage from './PlaceholderPage.svelte';

  let { subsection }: { subsection: string } = $props();
</script>

{#if subsection === 'subscription'}
  <section class="card">
    <h2>Ссылка подписки / импорта</h2>
    <div class="row">
      <input disabled placeholder="https://…" aria-label="Ссылка подписки" />
      <button type="button" disabled title="Импорт появится в следующей версии">Импорт</button>
    </div>
    <p class="note">Подключение пока не реализовано.</p>
  </section>
{:else}
  <PlaceholderPage />
{/if}

<style>
  .card {
    padding: 18px;
    border-radius: 14px;
    background: #111111;
  }

  h2 {
    margin: 0 0 12px;
    font-size: 13px;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: #fafafa;
  }

  .row {
    display: flex;
    gap: 8px;
  }

  input {
    flex: 1;
    padding: 10px 12px;
    border: 1px solid #343434;
    border-radius: 8px;
    background: #080808;
    color: #8b8b8b;
    font: inherit;
  }

  button {
    padding: 10px 18px;
    border: 0;
    border-radius: 8px;
    background: #fafafa;
    color: #080808;
    font: inherit;
    font-weight: 800;
    opacity: 0.5;
    cursor: not-allowed;
  }

  .note {
    margin: 12px 0 0;
    color: #8b8b8b;
  }
</style>
