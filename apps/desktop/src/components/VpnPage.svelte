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
    border-radius: var(--radius-lg);
    background: var(--surface-alt);
  }

  h2 {
    margin: 0 0 12px;
    font-size: var(--text-body);
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--ink);
  }

  .row {
    display: flex;
    gap: 8px;
  }

  input {
    flex: 1;
    padding: 10px 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--black);
    color: var(--muted);
    font: inherit;
  }

  button {
    padding: 10px 18px;
    border: 0;
    border-radius: var(--radius-sm);
    background: var(--ink);
    color: var(--black);
    font: inherit;
    font-weight: 800;
    opacity: 0.5;
    cursor: not-allowed;
  }

  .note {
    margin: 12px 0 0;
    color: var(--muted);
  }
</style>
