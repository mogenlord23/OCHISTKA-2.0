<script lang="ts">
  import {
    bypassSitesAdd,
    bypassSitesList,
    bypassSitesRemove,
    errorText,
    MAX_SITES,
  } from '../lib/bypassSites';

  let {
    open,
    onclose,
  }: {
    open: boolean;
    onclose: () => void;
  } = $props();

  let sites = $state<string[]>([]);
  let draft = $state('');
  let error = $state<string | null>(null);
  let busy = $state(false);

  async function load() {
    try {
      sites = (await bypassSitesList()) ?? [];
    } catch {
      sites = [];
    }
  }

  // Список подгружается при каждом открытии панели; закрытие его не стирает.
  $effect(() => {
    if (open) void load();
  });

  async function add() {
    if (busy || draft.trim() === '') return;
    busy = true;
    error = null;
    try {
      sites = (await bypassSitesAdd(draft)) ?? sites;
      draft = '';
    } catch (err) {
      error = errorText(err);
    } finally {
      busy = false;
    }
  }

  async function remove(domain: string) {
    error = null;
    try {
      sites = (await bypassSitesRemove(domain)) ?? sites;
    } catch (err) {
      error = errorText(err);
    }
  }
</script>

<div class="sites" class:open aria-hidden={!open} inert={!open}>
  <div class="inner">
    <section class="card" aria-label="Список сайтов">
      <div class="head">
        <span class="title">Мои сайты</span>
        <button type="button" class="icon-btn" aria-label="Закрыть" onclick={onclose}>✕</button>
      </div>
      <p class="note">Эти сайты открываются через VPN</p>
      <textarea
        bind:value={draft}
        rows="3"
        aria-label="Адреса сайтов"
        placeholder="example.com"
        spellcheck="false"
      ></textarea>
      <button type="button" class="add" disabled={busy || draft.trim() === ''} onclick={add}
        >Добавить</button
      >
      {#if error}<p class="error" role="alert">{error}</p>{/if}
      {#if sites.length > 0}
        <ul aria-label="Добавленные сайты">
          {#each sites as domain (domain)}
            <li>
              <span class="domain">{domain}</span>
              <button
                type="button"
                class="icon-btn remove"
                aria-label={`Удалить ${domain}`}
                onclick={() => remove(domain)}>✕</button
              >
            </li>
          {/each}
        </ul>
        <p class="note count">{sites.length} из {MAX_SITES}</p>
      {/if}
    </section>
  </div>
</div>

<style>
  /* Выезжает высотой строки сетки: быстро (220 мс), без лишних эффектов. */
  .sites {
    display: grid;
    grid-template-rows: 0fr;
    opacity: 0;
    transition:
      grid-template-rows 220ms ease,
      opacity 220ms ease;
  }

  .sites.open {
    grid-template-rows: 1fr;
    opacity: 1;
  }

  .inner {
    min-height: 0;
    overflow: hidden;
  }

  .card {
    display: flex;
    flex-direction: column;
    gap: 10px;
    margin-top: 8px;
    padding: 16px 18px;
    border-radius: var(--radius-md);
    background: var(--surface-alt);
  }

  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .title {
    font-size: 1.1rem;
    font-weight: 800;
    color: var(--ink);
  }

  .note,
  .error {
    margin: 0;
    font-size: var(--text-caption);
    color: var(--muted);
  }

  .error {
    color: var(--ink);
    font-weight: 700;
  }

  textarea {
    width: 100%;
    box-sizing: border-box;
    padding: 8px 10px;
    border: 0;
    border-radius: var(--radius-md);
    background: var(--surface);
    color: var(--ink);
    font: inherit;
    resize: vertical;
  }

  .add {
    align-self: flex-start;
    padding: 6px 14px;
    border: 0;
    border-radius: var(--radius-md);
    background: var(--surface);
    color: var(--ink);
    font: inherit;
    font-weight: 700;
    cursor: pointer;
  }

  .add:disabled {
    opacity: 0.5;
    cursor: default;
  }

  ul {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  li {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 4px 8px;
    border-radius: var(--radius-md);
  }

  li:hover {
    background: var(--surface);
  }

  .domain {
    color: var(--ink);
    overflow-wrap: anywhere;
  }

  .icon-btn {
    border: 0;
    background: transparent;
    color: var(--muted);
    font: inherit;
    cursor: pointer;
  }

  /* Крестик удаления виден при наведении и при фокусе с клавиатуры. */
  .remove {
    opacity: 0;
  }

  li:hover .remove,
  li:focus-within .remove,
  .remove:focus-visible {
    opacity: 1;
  }

  @media (prefers-reduced-motion: reduce) {
    .sites {
      transition: none;
    }
  }
</style>
