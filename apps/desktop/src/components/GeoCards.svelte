<script lang="ts">
  import { onMount } from 'svelte';
  import {
    GEO_UNDETERMINED,
    GEO_VPN_NOTE,
    determineGeo,
    geo,
    initGeo,
    revokeGeo,
  } from '../lib/geoStore.svelte';

  onMount(() => {
    void initGeo();
  });

  let changingConsent = $state(false);

  async function changeConsent() {
    if (changingConsent || geo.loading) return;
    changingConsent = true;
    try {
      if (geo.consent) await revokeGeo();
      else await determineGeo();
    } finally {
      changingConsent = false;
    }
  }

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
  <div class="actions">
    <button
      type="button"
      class="btn"
      class:go={!geo.consent}
      disabled={geo.consent === null || geo.loading || changingConsent}
      onclick={() => void changeConsent()}
      >{geo.loading ? 'Определяем…' : geo.consent ? 'Отозвать разрешение' : 'Определить'}</button
    >
  </div>
  {#if geo.error}<p class="note error" role="alert">{geo.error}</p>{/if}
  <p class="note">{GEO_VPN_NOTE}</p>
</section>

<style>
  .geo {
    display: flex;
    flex-direction: column;
    gap: 8px;
    min-width: 0;
    margin: 0;
    padding: 16px;
    border-radius: var(--radius-md);
    background: var(--surface-alt);
    box-shadow: 0 0 0 2px #000;
    container-type: inline-size;
  }

  .cards {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 8px;
  }

  .card {
    display: flex;
    align-items: center;
    gap: 12px;
    min-width: 0;
    padding: 12px 14px;
    border-radius: var(--radius-sm);
    background: #1a1a1a;
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
    overflow-wrap: anywhere;
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
    width: 100%;
    min-width: 0;
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

  .actions {
    display: flex;
    flex-direction: column;
    gap: 8px;
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

  @container (max-width: 400px) {
    .cards {
      grid-template-columns: 1fr;
    }
  }
</style>
