<script lang="ts">
  import { onMount } from 'svelte';
  import {
    GEO_CONSENT_TEXT,
    GEO_UNDETERMINED,
    GEO_VPN_NOTE,
    determineGeo,
    geo,
    initGeo,
    lookupGeo,
    revokeGeo,
  } from '../lib/geoStore.svelte';

  onMount(() => {
    void initGeo();
  });

  const provider = $derived(geo.info?.provider ?? GEO_UNDETERMINED);
  const place = $derived(
    geo.info && (geo.info.country || geo.info.city)
      ? [geo.info.city, geo.info.country].filter(Boolean).join(', ')
      : GEO_UNDETERMINED,
  );
</script>

<section class="geo" aria-label="Провайдер и расположение">
  <div class="cards">
    <div class="card" data-testid="geo-provider">
      <span class="icon" aria-hidden="true">
        <svg
          viewBox="0 0 24 24"
          width="26"
          height="26"
          fill="none"
          stroke="currentColor"
          stroke-width="2.2"
          stroke-linecap="round"
          stroke-linejoin="round"
          ><rect x="3" y="4" width="18" height="6" rx="2" /><rect
            x="3"
            y="14"
            width="18"
            height="6"
            rx="2"
          /><path d="M7 7h.01M7 17h.01" /></svg
        >
      </span>
      <span class="text">
        <span class="label">Провайдер</span>
        <span class="value selectable" class:none={provider === GEO_UNDETERMINED}>{provider}</span>
      </span>
    </div>
    <div class="card" data-testid="geo-place">
      <span class="icon" aria-hidden="true">
        <svg
          viewBox="0 0 24 24"
          width="26"
          height="26"
          fill="none"
          stroke="currentColor"
          stroke-width="2.2"
          stroke-linecap="round"
          stroke-linejoin="round"
          ><path d="M12 21s-7-6.2-7-11.5A7 7 0 0 1 19 9.5C19 14.800 12 21 12 21Z" /><circle
            cx="12"
            cy="9.5"
            r="2.5"
          /></svg
        >
      </span>
      <span class="text">
        <span class="label">Расположение</span>
        <span class="value selectable" class:none={place === GEO_UNDETERMINED}>{place}</span>
      </span>
    </div>
  </div>
  <div class="panel">
    <div class="actions">
      {#if geo.consent}
        <button type="button" class="btn" disabled={geo.loading} onclick={() => void lookupGeo()}
          >{geo.loading ? 'Определяем…' : 'Обновить'}</button
        >
        <button type="button" class="btn" onclick={() => void revokeGeo()}
          >Отозвать разрешение</button
        >
      {:else}
        <button
          type="button"
          class="btn go"
          disabled={geo.consent === null}
          onclick={() => void determineGeo()}>Определить</button
        >
        <span class="note">{GEO_CONSENT_TEXT}</span>
      {/if}
    </div>
    {#if geo.error}<p class="note error" role="alert">{geo.error}</p>{/if}
    <p class="note">{GEO_VPN_NOTE} Данные примерные.</p>
  </div>
</section>

<style>
  .geo {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin: 0;
  }

  .cards {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
  }

  .card {
    display: flex;
    align-items: center;
    gap: 12px;
    min-width: 0;
    padding: 12px 14px;
    border-radius: var(--radius-md);
    background: var(--surface-alt);
    box-shadow: 0 0 0 2px #000;
  }

  .icon {
    display: grid;
    flex-shrink: 0;
    place-items: center;
    width: 44px;
    height: 44px;
    border-radius: var(--radius-md);
    background: var(--accent);
    color: #000;
  }

  .text {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .label {
    font-size: 0.7rem;
    font-weight: 800;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: var(--muted);
  }

  .value {
    font-size: 1.05rem;
    font-weight: 800;
    color: var(--ink);
    overflow-wrap: anywhere;
  }

  .value.none {
    color: var(--muted);
  }

  .btn {
    flex-shrink: 0;
    min-height: 36px;
    padding: 6px 18px;
    border: 0;
    border-radius: var(--radius-sm);
    background: var(--btn-bg);
    color: var(--ink);
    box-shadow: inset 0 0 0 1px var(--btn-border);
    font: inherit;
    font-size: var(--text-caption);
    font-weight: 800;
    cursor: pointer;
  }

  .btn.go {
    background: var(--accent);
    color: #000;
    box-shadow: none;
  }

  .btn:disabled {
    opacity: 0.6;
    cursor: default;
  }

  .btn:focus-visible {
    outline: 2px solid #fff;
    outline-offset: 2px;
  }

  .panel {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 12px 14px;
    border-radius: var(--radius-md);
    background: var(--surface-alt);
    box-shadow: 0 0 0 2px #000;
  }

  .actions {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .actions .note {
    flex: 1;
  }

  .note {
    margin: 0;
    font-size: var(--text-caption);
    color: var(--muted);
  }

  .note.error {
    opacity: 1;
    font-weight: 700;
  }

  @media (max-width: 640px) {
    .cards {
      grid-template-columns: 1fr;
    }
  }
</style>
