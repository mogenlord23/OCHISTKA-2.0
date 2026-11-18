<script lang="ts">
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

  /** Один экран, как на эталоне: «главный» или «Дополнительно» (Zapret, Xbox DNS). */
  // svelte-ignore state_referenced_locally
  let view = $state<'main' | 'advanced'>(subsection === 'routes' ? 'advanced' : 'main');
  $effect(() => {
    view = subsection === 'routes' ? 'advanced' : 'main';
  });
  let selected = $state(0);
  let fileInput: HTMLInputElement | undefined = $state();

  /** Код страны из флага-эмодзи в начале имени (🇱🇻 → LV); иначе пусто. */
  function countryCode(name: string): string {
    const points = [...name].slice(0, 2).map((c) => c.codePointAt(0) ?? 0);
    if (points.length === 2 && points.every((p) => p >= 0x1f1e6 && p <= 0x1f1ff)) {
      return points.map((p) => String.fromCharCode(p - 0x1f1e6 + 65)).join('');
    }
    return '';
  }

  function displayName(name: string): string {
    return countryCode(name) ? [...name].slice(2).join('').trim() : name;
  }

  async function onFile(event: Event) {
    const file = (event.currentTarget as HTMLInputElement).files?.[0];
    if (!file) return;
    parsed = parseSubscription(await file.text());
    selected = 0;
    importMessage = `Серверов: ${parsed.servers.length}. Не распознано строк: ${parsed.unknownCount}.`;
  }

  const ADVANCED = [
    { id: 'zapret', label: 'Zapret', description: 'Обход блокировок DPI' },
    { id: 'xbox-dns', label: 'Xbox DNS', description: 'DNS-серверы Xbox' },
  ] as const;

  let urlInput = $state('');
  let importMessage = $state<string | null>(null);
  let parsed = $state<ParsedSubscription | null>(null);

  let connectMessage = $state<string | null>(null);
  /** Подключение пока не устанавливается, поэтому таймер стоит на нуле. */
  let elapsedSeconds = $state(0);

  let networkMessages = $state<Record<string, string | null>>({});
  /** Переключатель, для которого ждём подтверждения (id из ADVANCED). */
  let pendingNetwork = $state<(typeof ADVANCED)[number] | null>(null);

  function askNetworkToggle(item: (typeof ADVANCED)[number]) {
    pendingNetwork = item;
  }

  function cancelNetworkToggle() {
    pendingNetwork = null;
  }

  function confirmNetworkToggle() {
    const target = pendingNetwork;
    if (!target) return;
    pendingNetwork = null;
    networkMessages = { ...networkMessages, [target.id]: requestNetworkChange().reason };
  }

  function onDialogKey(event: KeyboardEvent) {
    if (event.key === 'Escape') cancelNetworkToggle();
  }

  function onImport() {
    // Вставленный текст подписки (vless://… и т. п.) разбираем сразу, ссылку — проверяем.
    const pasted = parseSubscription(
      urlInput.replace(/\s+(?=(vless|vmess|trojan|ss):\/\/)/g, '\n'),
    );
    if (pasted.servers.length > 0) {
      parsed = pasted;
      selected = 0;
      importMessage = `Серверов: ${pasted.servers.length}. Не распознано строк: ${pasted.unknownCount}.`;
      return;
    }
    const result = validateSubscriptionUrl(urlInput);
    importMessage = result.ok ? IMPORT_REASON : result.reason;
  }

  function onConnect() {
    connectMessage = requestVpnConnect().reason;
  }
</script>

<div class="vpn">
  <aside class="poster" aria-hidden="true">
    <div class="pop-strip">
      {#each [0, 1, 2] as tile (tile)}
        <span class="pop-tile"><span class="pop-dot"></span></span>
      {/each}
    </div>
    <div class="pop-main">
      <span class="pop-ring"></span>
      <span class="pop-disc"></span>
      <span class="pop-glyph">VPN</span>
    </div>
  </aside>

  <div class="column">
    <header class="head">
      <div class="titles">
        <button
          type="button"
          class="title-tab"
          class:on={view === 'main'}
          aria-pressed={view === 'main'}
          onclick={() => (view = 'main')}>VPN</button
        >
        <button
          type="button"
          class="title-tab"
          class:on={view === 'advanced'}
          aria-pressed={view === 'advanced'}
          onclick={() => (view = 'advanced')}>Дополнительно</button
        >
      </div>
      <p class="tagline">Твоё подключение. Твои правила.</p>
    </header>

    {#if view === 'main'}
      <section class="card">
        <h2>Ссылка подписки / импорта</h2>
        <div class="row">
          <input bind:value={urlInput} placeholder="https://…" aria-label="Ссылка подписки" />
          <button type="button" onclick={onImport}>Импорт</button>
        </div>
        <div class="row tight">
          <button type="button" onclick={() => fileInput?.click()}>Из файла…</button>
          <button type="button" onclick={() => (view = 'advanced')}>Маршруты и исключения</button>
          <input
            bind:this={fileInput}
            class="hidden"
            type="file"
            accept=".txt,text/plain"
            aria-label="Файл подписки"
            onchange={onFile}
          />
        </div>
        {#if importMessage}
          <p class="note" role="status">{importMessage}</p>
        {/if}
      </section>

      <section class="card connect">
        <div class="power-row">
          <button
            type="button"
            class="power"
            aria-pressed="false"
            aria-label="Подключить"
            onclick={onConnect}
          >
            <span class="power-knob" aria-hidden="true"></span>
          </button>
          <div class="power-text">
            <span class="power-label">Подключить</span>
            <span class="timer" aria-label="Время подключения">{formatElapsed(elapsedSeconds)}</span
            >
            <span class="status-line" role="status">{connectMessage ?? 'Не подключено'}</span>
          </div>
        </div>
        <label class="select">
          <span class="hidden">Режим</span>
          <select disabled aria-describedby="vpn-tun-reason" aria-label="Режим">
            <option>TUN · весь компьютер</option>
          </select>
          <span class="chev" aria-hidden="true"></span>
        </label>
        <p class="note" id="vpn-tun-reason">{VPN_TUN_MODE_REASON}</p>
      </section>

      <h2 class="list-title">Серверы / {parsed?.servers.length ?? 0}</h2>
      <section class="card servers-card">
        {#if parsed && parsed.servers.length > 0}
          <ul class="servers" aria-label="Серверы">
            {#each parsed.servers as server, index (index)}
              <li>
                <button
                  type="button"
                  class="server"
                  class:selected={selected === index}
                  aria-pressed={selected === index}
                  onclick={() => (selected = index)}
                >
                  <span class="srv-name">
                    {#if countryCode(server.name)}<b class="cc">{countryCode(server.name)}</b
                      >{/if}{displayName(server.name)}
                  </span>
                  <span class="scheme">{server.scheme.toUpperCase()}</span>
                </button>
              </li>
            {/each}
          </ul>
        {:else}
          <p class="note empty">
            Серверов пока нет. Вставь ссылку https:// или текст подписки в поле выше, либо выбери
            файл.
          </p>
        {/if}
      </section>
    {:else}
      <h2 class="big">Инструменты доступа</h2>
      {#each ADVANCED as item (item.id)}
        <section class="card switch-row">
          <div class="switch-head">
            <span class="switch-title">{item.label}</span>
            <button
              type="button"
              role="switch"
              aria-checked="false"
              aria-label={item.label}
              onclick={() => askNetworkToggle(item)}
            >
              <span class="knob"></span>
            </button>
          </div>
          <span class="note"
            >{item.description} · <span class="status">{NETWORK_STATUS_OFF}</span></span
          >
          {#if networkMessages[item.id]}
            <p class="note" role="status">{networkMessages[item.id]}</p>
          {/if}
        </section>
      {/each}
      {#if pendingNetwork}
        <div class="backdrop">
          <div
            class="dialog"
            role="dialog"
            aria-modal="true"
            aria-labelledby="network-title"
            aria-describedby="network-text"
            tabindex="-1"
            onkeydown={onDialogKey}
          >
            <h3 id="network-title">Включить {pendingNetwork.label}?</h3>
            <p id="network-text">
              {pendingNetwork.description}. Включение меняет сетевые настройки компьютера. Пока это
              заглушка: ничего не изменится.
            </p>
            <div class="dialog-actions">
              <button type="button" onclick={cancelNetworkToggle}>Отмена</button>
              <button type="button" onclick={confirmNetworkToggle}>Включить</button>
            </div>
          </div>
        </div>
      {/if}
    {/if}
  </div>
</div>

<style>
  /* Экран VPN рисует свой заголовок и вкладку «Дополнительно», как на эталоне old-01. */
  :global(main:has(> .vpn) > h1),
  :global(main:has(> .vpn) > .lead),
  :global(main:has(> .vpn) > .subtabs) {
    display: none;
  }

  .vpn {
    display: grid;
    grid-template-columns: 286px minmax(0, 1fr);
    gap: 22px;
    height: calc(100vh - 126px);
    min-height: 560px;
  }

  /* Нейтральная поп-арт композиция из CSS вместо фотографии. */
  .poster {
    display: flex;
    flex-direction: column;
    overflow: hidden;
    border-radius: var(--radius-lg);
    background: #000;
  }

  .pop-strip {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 6px;
    height: 112px;
    padding-bottom: 8px;
    border-bottom: 14px solid var(--accent);
  }

  .pop-tile {
    display: grid;
    place-items: center;
    background: #b4ff00;
  }

  .pop-tile:nth-child(2) {
    background: var(--accent);
  }

  .pop-dot {
    width: 46px;
    height: 46px;
    border-radius: 50%;
    background: #2b1b6e;
    box-shadow: 0 0 0 8px #000;
  }

  .pop-main {
    position: relative;
    flex: 1;
    margin-top: 10px;
    background:
      radial-gradient(circle, rgba(0, 0, 0, 0.28) 1.6px, transparent 2px) 0 0 / 10px 10px,
      linear-gradient(180deg, #ffe500 0%, #ffe500 45%, #1a1a1a 45%);
  }

  .pop-ring {
    position: absolute;
    left: 50%;
    top: 44%;
    width: 210px;
    height: 210px;
    border-radius: 50%;
    transform: translate(-50%, -50%);
    background: #000;
    box-shadow:
      0 0 0 14px #ffb3c7,
      0 0 0 26px #000;
  }

  .pop-disc {
    position: absolute;
    left: 50%;
    top: 44%;
    width: 150px;
    height: 150px;
    border-radius: 50%;
    transform: translate(-50%, -50%);
    background: radial-gradient(circle at 35% 35%, #5ad1ff 0 18%, #ff2a2a 19% 100%);
  }

  .pop-glyph {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 26px;
    text-align: center;
    font-family: 'Arial Black', sans-serif;
    font-size: 3.4rem;
    color: #fff;
    -webkit-text-stroke: 3px #000;
    letter-spacing: 0.04em;
  }

  .column {
    display: flex;
    flex-direction: column;
    gap: 12px;
    min-width: 0;
    min-height: 0;
  }

  .titles {
    display: flex;
    gap: 22px;
    align-items: baseline;
  }

  .title-tab {
    min-height: 0;
    padding: 0;
    background: none;
    box-shadow: none;
    font-family: 'Arial Black', 'Segoe UI', sans-serif;
    font-size: 1.95rem;
    font-weight: 900;
    line-height: 1;
    text-transform: uppercase;
    color: rgba(0, 0, 0, 0.55);
  }

  .title-tab.on {
    color: #000;
  }

  .tagline {
    margin: 8px 0 0;
    font-size: var(--text-caption);
    font-weight: 800;
    color: #000;
  }

  .card {
    padding: 16px 16px;
    border-radius: var(--radius-lg);
    background: var(--surface-alt);
    box-shadow: 0 0 0 2px #000;
  }

  h2 {
    margin: 0 0 12px;
    font-size: var(--text-caption);
    font-weight: 800;
    text-transform: uppercase;
    color: var(--ink);
  }

  .list-title {
    margin: 10px 0 -2px;
    color: #000;
  }

  .big {
    margin: 10px 0 6px;
    font-family: 'Arial Black', sans-serif;
    font-size: 1.4rem;
    color: #000;
  }

  .row {
    display: flex;
    gap: 8px;
  }

  .row.tight {
    margin-top: 10px;
  }

  input {
    flex: 1;
    min-width: 0;
    padding: 12px 18px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--black);
    color: var(--ink);
    font: inherit;
  }

  .hidden {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
  }

  button {
    min-height: 38px;
    padding: 8px 14px;
    border: 0;
    border-radius: var(--radius-sm);
    background: var(--btn-bg);
    color: var(--ink);
    box-shadow: inset 0 0 0 1px var(--btn-border);
    font: inherit;
    font-size: var(--text-caption);
    font-weight: 700;
    cursor: pointer;
  }

  .row > button {
    min-height: 48px;
  }

  .row.tight > button {
    min-height: 38px;
  }

  .power-row {
    display: flex;
    align-items: center;
    gap: 16px;
    margin-bottom: 12px;
  }

  /* Большой переключатель в цвете раздела; ручка слева = выключено. */
  .power {
    position: relative;
    flex-shrink: 0;
    width: 104px;
    height: 48px;
    min-height: 0;
    padding: 0;
    border-radius: 999px;
    background: #2a2a2a;
    box-shadow: 0 0 0 2px var(--accent);
  }

  .power-knob {
    position: absolute;
    top: 5px;
    left: 6px;
    width: 38px;
    height: 38px;
    border-radius: 50%;
    background: #fff;
    box-shadow: 0 0 0 2px #000;
  }

  .power-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .power-label {
    font-size: var(--text-caption);
    font-weight: 800;
    text-transform: uppercase;
    color: var(--ink);
  }

  .timer {
    font-variant-numeric: tabular-nums;
    font-size: 1.6rem;
    font-weight: 800;
    color: var(--ink);
  }

  .status-line {
    font-size: 0.75rem;
    color: var(--ink);
  }

  .select {
    position: relative;
    display: block;
  }

  select {
    width: 100%;
    padding: 10px 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: #141414;
    color: var(--ink);
    font: inherit;
    font-size: var(--text-caption);
    appearance: none;
  }

  .chev {
    position: absolute;
    right: 16px;
    top: 13px;
    width: 7px;
    height: 7px;
    border-right: 2px solid var(--accent);
    border-bottom: 2px solid var(--accent);
    transform: rotate(45deg);
  }

  .servers-card {
    flex: 1;
    min-height: 120px;
    padding: 6px 6px 6px 6px;
    overflow-y: auto;
    scrollbar-width: thin;
    scrollbar-color: var(--accent) transparent;
  }

  .servers-card::-webkit-scrollbar {
    width: 10px;
  }

  .servers-card::-webkit-scrollbar-thumb {
    border-radius: 6px;
    background: var(--accent);
  }

  .servers {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .server {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 2px;
    width: 100%;
    min-height: 54px;
    padding: 8px 12px;
    background: transparent;
    box-shadow: none;
    color: var(--muted);
    font-weight: 400;
    text-align: left;
  }

  .server.selected {
    background: #222;
    box-shadow: inset 0 0 0 1px var(--accent);
  }

  .cc {
    margin-right: 5px;
    font-size: 0.7rem;
    font-weight: 800;
  }

  .note {
    margin: 8px 0 0;
    font-size: var(--text-caption);
    color: var(--muted);
  }

  .empty {
    margin: 10px;
  }

  .switch-row {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .switch-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .switch-title {
    font-size: 1.15rem;
    font-weight: 800;
    color: var(--ink);
  }

  [role='switch'] {
    position: relative;
    width: 100px;
    height: 44px;
    padding: 0;
    border-radius: 999px;
    background: #2a2a2a;
    box-shadow: 0 0 0 2px #fff;
  }

  .knob {
    position: absolute;
    top: 6px;
    left: 7px;
    width: 32px;
    height: 32px;
    border-radius: 50%;
    background: #fff;
  }

  .backdrop {
    position: fixed;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(0, 0, 0, 0.6);
  }

  .dialog {
    width: min(420px, calc(100vw - 32px));
    padding: 20px;
    border-radius: var(--radius-lg);
    background: var(--surface);
    color: var(--ink);
  }

  .dialog h3 {
    margin: 0 0 8px;
  }

  .dialog-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 16px;
  }

  @media (max-width: 760px) {
    .vpn {
      grid-template-columns: minmax(0, 1fr);
      height: auto;
    }

    .poster {
      display: none;
    }
  }
</style>
