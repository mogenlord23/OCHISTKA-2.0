<script lang="ts">
  import PlaceholderPage from './PlaceholderPage.svelte';
  import {
    IMPORT_REASON,
    NETWORK_STATUS_OFF,
    VPN_TUN_MODE_REASON,
    formatElapsed,
    parseSubscription,
    requestNetworkChange,
    requestVpnConnect,
    validateSubscriptionUrl,
    type ParsedSubscription,
  } from '../lib/vpn';

  let { subsection }: { subsection: string } = $props();

  const ADVANCED = [
    { id: 'zapret', label: 'Zapret', description: 'Обход блокировок DPI' },
    { id: 'xbox-dns', label: 'Xbox DNS', description: 'DNS-серверы Xbox' },
  ] as const;

  let urlInput = $state('');
  let importMessage = $state<string | null>(null);
  let pasteText = $state('');
  let parsed = $state<ParsedSubscription | null>(null);

  let connectMessage = $state<string | null>(null);
  /** Подключение пока не устанавливается, поэтому таймер стоит на нуле. */
  let elapsedSeconds = $state(0);

  let networkMessages = $state<Record<string, string | null>>({});

  function onImport() {
    const result = validateSubscriptionUrl(urlInput);
    importMessage = result.ok ? IMPORT_REASON : result.reason;
  }

  function onParse() {
    parsed = parseSubscription(pasteText);
  }

  function onConnect() {
    connectMessage = requestVpnConnect().reason;
  }

  function onNetworkToggle(id: string) {
    networkMessages = { ...networkMessages, [id]: requestNetworkChange().reason };
  }
</script>

{#if subsection === 'subscription'}
  <section class="card">
    <h2>Ссылка подписки / импорта</h2>
    <div class="row">
      <input bind:value={urlInput} placeholder="https://…" aria-label="Ссылка подписки" />
      <button type="button" onclick={onImport}>Импорт</button>
    </div>
    {#if importMessage}
      <p class="note" role="status">{importMessage}</p>
    {/if}

    <label class="field-label" for="vpn-paste">Текст подписки (вставь вручную)</label>
    <textarea
      id="vpn-paste"
      bind:value={pasteText}
      rows="5"
      aria-label="Текст подписки"
      placeholder="vless://…&#10;trojan://…"
    ></textarea>
    <div class="row">
      <button type="button" onclick={onParse}>Разобрать</button>
    </div>
    {#if parsed}
      <p class="note" role="status">
        Серверов: {parsed.servers.length}. Не распознано строк: {parsed.unknownCount}.
      </p>
    {/if}
  </section>
{:else if subsection === 'servers'}
  <section class="card">
    <div class="power-area">
      <button type="button" class="power" aria-pressed="false" onclick={onConnect}>
        Подключить
      </button>
      <p class="timer" aria-label="Время подключения">{formatElapsed(elapsedSeconds)}</p>
      {#if connectMessage}
        <p class="note" role="status">{connectMessage}</p>
      {/if}
    </div>

    <fieldset class="modes">
      <legend>Режим</legend>
      <label class="mode">
        <input
          type="radio"
          name="vpn-mode"
          value="tun"
          disabled
          aria-describedby="vpn-tun-reason"
        />
        TUN
      </label>
      <p class="note" id="vpn-tun-reason">{VPN_TUN_MODE_REASON}</p>
    </fieldset>

    <h2>Серверы</h2>
    {#if parsed && parsed.servers.length > 0}
      <ul class="servers">
        {#each parsed.servers as server, index (index)}
          <li>
            <span class="scheme">{server.scheme}</span>
            <span>{server.name}</span>
          </li>
        {/each}
      </ul>
    {:else}
      <p class="note">Серверов пока нет. Вставь текст подписки на странице «Подписка и импорт».</p>
    {/if}
  </section>
{:else if subsection === 'routes'}
  <section class="card">
    <h2>Дополнительно</h2>
    {#each ADVANCED as item (item.id)}
      <div class="switch-row">
        <div>
          <span class="switch-title">{item.label}</span>
          <span class="note">{item.description}</span>
          <span class="status">{NETWORK_STATUS_OFF}</span>
        </div>
        <button
          type="button"
          role="switch"
          aria-checked="false"
          aria-label={item.label}
          onclick={() => onNetworkToggle(item.id)}
        >
          <span class="knob"></span>
        </button>
        {#if networkMessages[item.id]}
          <p class="note" role="status">{networkMessages[item.id]}</p>
        {/if}
      </div>
    {/each}
  </section>
  <PlaceholderPage />
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
    margin-bottom: 12px;
  }

  input,
  textarea {
    flex: 1;
    padding: 10px 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--black);
    color: var(--ink);
    font: inherit;
  }

  textarea {
    width: 100%;
    box-sizing: border-box;
    resize: vertical;
  }

  button {
    min-height: 44px;
    padding: 10px 18px;
    border: 0;
    border-radius: var(--radius-sm);
    background: var(--ink);
    color: var(--black);
    font: inherit;
    font-weight: 800;
    cursor: pointer;
  }

  button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .field-label {
    display: block;
    margin: 4px 0 6px;
    color: var(--muted);
  }

  .power-area {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    margin-bottom: 16px;
  }

  .power {
    min-width: 220px;
    min-height: 88px;
    font-size: var(--text-body);
  }

  .timer {
    margin: 0;
    font-variant-numeric: tabular-nums;
    font-size: 1.5rem;
    color: var(--ink);
  }

  .modes {
    margin: 0 0 16px;
    padding: 0;
    border: 0;
  }

  .modes legend {
    margin-bottom: 6px;
    color: var(--muted);
  }

  .mode {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    color: var(--ink);
  }

  .servers {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .servers li {
    display: flex;
    gap: 10px;
    align-items: center;
    padding: 8px 0;
    border-bottom: 1px solid var(--border);
  }

  .scheme {
    padding: 2px 8px;
    border-radius: var(--radius-sm);
    background: var(--black);
    color: var(--ink);
    font-size: 0.8rem;
    text-transform: uppercase;
  }

  .switch-row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 12px 0;
    border-bottom: 1px solid var(--border);
  }

  .switch-row > div {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .switch-title {
    color: var(--ink);
    font-weight: 800;
  }

  .status {
    color: var(--muted);
  }

  [role='switch'] {
    position: relative;
    width: 52px;
    min-height: 30px;
    height: 30px;
    padding: 0;
    border-radius: 15px;
    background: var(--black);
    border: 1px solid var(--border);
  }

  .knob {
    position: absolute;
    top: 3px;
    left: 3px;
    width: 22px;
    height: 22px;
    border-radius: 50%;
    background: var(--muted);
  }

  .note {
    margin: 8px 0 0;
    color: var(--muted);
  }
</style>
