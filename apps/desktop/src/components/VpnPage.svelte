<script lang="ts">
  import { onMount } from 'svelte';
  import poster from '../assets/pop-art.png';
  import chePink from '../assets/che-pink.png';
  import cheLime from '../assets/che-lime.png';
  import { playHeehee } from '../lib/sound';
  import VpnConnectionsTab from './VpnConnectionsTab.svelte';
  import VpnNetworkTab from './VpnNetworkTab.svelte';
  import VpnRoutesTab from './VpnRoutesTab.svelte';
  import {
    IMPORTING_LABEL,
    PREPARE_FAILED_PREFIX,
    NETWORK_STATUS_OFF,
    TUN_MODE_REASON,
    connectUi,
    displayName,
    elapsedSince,
    formatElapsed,
    formatLatency,
    importMessage,
    normalizePasted,
    prepareImport,
    requestNetworkChange,
    statusLine,
  } from '../lib/vpn';
  import {
    vpnConnect,
    vpnDisconnect,
    vpnImport,
    vpnMeasureLatency,
    vpnSelect,
    vpnSnapshot,
    vpnStatus,
    type RoutingRules,
    type VpnSnapshot,
  } from '../lib/vpnApi';
  import { errorMessage } from '../lib/vpnNetwork';

  let { subsection }: { subsection: string } = $props();

  type Tab = 'vpn' | 'routes' | 'advanced' | 'network' | 'connections';
  const TABS: { id: Tab; label: string }[] = [
    { id: 'vpn', label: 'VPN' },
    { id: 'routes', label: 'Маршруты и исключения' },
    { id: 'advanced', label: 'Дополнительно' },
    { id: 'network', label: 'Сеть' },
    { id: 'connections', label: 'Соединения' },
  ];

  function tabOf(id: string): Tab {
    return TABS.find((item) => item.id === id)?.id ?? 'vpn';
  }

  // svelte-ignore state_referenced_locally
  let tab = $state<Tab>(tabOf(subsection));
  $effect(() => {
    tab = tabOf(subsection);
  });
  const tabLabel = $derived(TABS.find((item) => item.id === tab)?.label ?? 'VPN');
  let fileInput: HTMLInputElement | undefined = $state();

  const ADVANCED = [
    { id: 'zapret', label: 'Zapret', description: 'Обход блокировок DPI', icon: '↗' },
    { id: 'xbox-dns', label: 'Xbox DNS', description: 'DNS-серверы Xbox', icon: 'X' },
  ] as const;

  /** Размер файла подписки не больше 5 МБ (так же ограничивает Rust). */
  const MAX_FILE_BYTES = 5 * 1024 * 1024;

  let snapshot = $state<VpnSnapshot | null>(null);
  let urlInput = $state('');
  let importNote = $state<string | null>(null);
  let importing = $state(false);
  let measuring = $state(false);
  let requesting = $state(false);
  let actionNote = $state<string | null>(null);
  let nowMs = $state(Date.now());
  /** Проценты подготовки VPN при первом подключении; `null`, когда подготовки нет. */
  let preparing = $state<number | null>(null);
  let retry = $state(false);

  const servers = $derived(snapshot?.servers ?? []);
  const ui = $derived(connectUi(snapshot, requesting || importing, { preparing, retry }));
  const elapsedSeconds = $derived(
    snapshot?.connection.state === 'connected' ? elapsedSince(snapshot.connection.since, nowMs) : 0,
  );
  const status = $derived(
    actionNote ??
      (preparing !== null
        ? 'Скачиваем нужные файлы: это нужно один раз'
        : importing
          ? IMPORTING_LABEL
          : statusLine(snapshot?.connection ?? null, ui.reason)),
  );

  async function refresh() {
    try {
      const next = await vpnSnapshot();
      snapshot = next;
      if (urlInput === '' && next.subscriptionUrl) urlInput = next.subscriptionUrl;
    } catch (err) {
      actionNote = `Не удалось получить состояние VPN: ${errorMessage(err)}`;
    }
  }

  onMount(() => {
    void refresh();
    const clock = setInterval(() => (nowMs = Date.now()), 1000);
    // Пока подключено, раз в 3 секунды проверяем, что VPN жив.
    const poll = setInterval(async () => {
      if (snapshot?.connection.state !== 'connected') return;
      try {
        const connection = await vpnStatus();
        if (snapshot) snapshot = { ...snapshot, connection };
      } catch {
        // Следующий опрос повторит запрос.
      }
    }, 3000);
    return () => {
      clearInterval(clock);
      clearInterval(poll);
    };
  });

  async function runImport(raw: string) {
    const prepared = prepareImport(normalizePasted(raw));
    if (!prepared.ok) {
      importNote = prepared.reason;
      return;
    }
    importing = true;
    actionNote = null;
    importNote = IMPORTING_LABEL;
    try {
      const summary = await vpnImport(prepared.input);
      importNote = importMessage(summary.count, summary.skipped, summary.issues);
      await refresh();
    } catch (err) {
      importNote = errorMessage(err);
    } finally {
      importing = false;
    }
  }

  function onImport() {
    void runImport(urlInput);
  }

  async function onFile(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    const file = input.files?.[0];
    input.value = '';
    if (!file) return;
    if (file.size > MAX_FILE_BYTES) {
      importNote = 'Файл слишком большой: подписка не больше 5 МБ';
      return;
    }
    await runImport(await file.text());
  }

  async function onSelect(id: string) {
    if (snapshot) snapshot = { ...snapshot, selected: id };
    try {
      await vpnSelect(id);
    } catch (err) {
      actionNote = errorMessage(err);
      await refresh();
    }
  }

  async function onMeasure() {
    measuring = true;
    try {
      snapshot = await vpnMeasureLatency();
    } catch (err) {
      actionNote = errorMessage(err);
    } finally {
      measuring = false;
    }
  }

  async function onConnect() {
    if (ui.disabled || requesting) return;
    requesting = true;
    actionNote = null;
    retry = false;
    const connecting = snapshot?.connection.state !== 'connected';
    // Подготовка начинается только если файлов VPN ещё нет: до первого ответа показываем её.
    if (connecting && snapshot && !snapshot.core.available) preparing = 0;
    try {
      const connection = connecting
        ? await vpnConnect((percent) => {
            if (preparing !== null) preparing = percent;
          })
        : await vpnDisconnect();
      if (snapshot) {
        snapshot = {
          ...snapshot,
          connection,
          core: connecting ? { available: true } : snapshot.core,
        };
      }
    } catch (err) {
      actionNote = errorMessage(err);
      retry = actionNote.startsWith(PREPARE_FAILED_PREFIX);
      await refresh();
    } finally {
      preparing = null;
      requesting = false;
    }
  }

  function onRulesSaved(rules: RoutingRules) {
    if (snapshot) snapshot = { ...snapshot, rules };
  }

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

  /** Пасхалка: звук (если включён) и анимация нажатия. */
  let pressed = $state(false);
  function onPoster() {
    pressed = false;
    requestAnimationFrame(() => (pressed = true));
    playHeehee();
  }
</script>

<div class="vpn">
  <div class="poster-cell">
    <button
      type="button"
      class="poster"
      class:pressed
      aria-label="Постер"
      onclick={onPoster}
      onanimationend={() => (pressed = false)}
    >
      <span class="band">
        <img src={chePink} alt="" draggable="false" />
        <img src={cheLime} alt="" draggable="false" />
      </span>
      <img class="portrait" src={poster} alt="" draggable="false" />
    </button>
  </div>
  <div class="column">
    <header class="head">
      <div class="title" data-testid="vpn-title">{tabLabel}</div>
      <p class="tagline">Подписка, серверы, маршруты · сеть и соединения</p>
    </header>

    <div class="tabbar" role="tablist" aria-label="Вкладки VPN">
      {#each TABS as item (item.id)}
        <button
          type="button"
          role="tab"
          class="tab"
          class:on={tab === item.id}
          aria-selected={tab === item.id}
          onclick={() => (tab = item.id)}>{item.label}</button
        >
      {/each}
    </div>

    {#key tab}
      <div class="pane" role="tabpanel" aria-label={tabLabel}>
        {#if tab === 'vpn'}
          <section class="card">
            <h2>Ссылка подписки / импорта</h2>
            <div class="row">
              <input
                bind:value={urlInput}
                placeholder="https://…"
                aria-label="Ссылка подписки"
                onkeydown={(e) => e.key === 'Enter' && onImport()}
              />
              <button type="button" disabled={importing} onclick={onImport}
                >{importing ? 'Загрузка…' : 'Импорт'}</button
              >
            </div>
            <div class="row tight">
              <button type="button" disabled={importing} onclick={() => fileInput?.click()}
                >Из файла…</button
              >
              <input
                bind:this={fileInput}
                class="hidden"
                type="file"
                accept=".txt,.json,text/plain,application/json"
                aria-label="Файл подписки"
                onchange={onFile}
              />
            </div>
            {#if importNote}
              <p class="note" role="status">{importNote}</p>
            {/if}
          </section>

          <section class="card connect">
            <div class="power-row">
              <button
                type="button"
                class="power"
                class:on={ui.pressed}
                aria-pressed={ui.pressed}
                aria-busy={ui.busy}
                aria-label={ui.label}
                aria-disabled={ui.disabled}
                onclick={onConnect}
              >
                <span class="power-knob" aria-hidden="true"></span>
              </button>
              <div class="power-text">
                <span class="power-label">{ui.label}</span>
                {#if preparing !== null}
                  <span
                    class="prepare-bar"
                    role="progressbar"
                    aria-label="Подготовка VPN"
                    aria-valuemin="0"
                    aria-valuemax="100"
                    aria-valuenow={preparing}
                  >
                    <span class="prepare-fill" style:width={`${preparing}%`}></span>
                  </span>
                {/if}
                <span class="timer" aria-label="Время подключения"
                  >{formatElapsed(elapsedSeconds)}</span
                >
                <span class="status-line" role="status">{status}</span>
              </div>
            </div>
            <label class="select">
              <span class="hidden">Режим</span>
              <select aria-describedby="vpn-tun-reason" aria-label="Режим">
                <option value="proxy">Системный прокси · совместимые приложения</option>
                <option value="tun" disabled>TUN · весь компьютер (скоро)</option>
              </select>
              <span class="chev" aria-hidden="true"></span>
            </label>
            <p class="note" id="vpn-tun-reason">
              {TUN_MODE_REASON}. Прежние настройки прокси Windows возвращаются при отключении.
            </p>
          </section>

          <div class="list-head">
            <h2 class="list-title">Серверы / {servers.length}</h2>
            {#if servers.length > 0}
              <button type="button" class="small" disabled={measuring} onclick={onMeasure}
                >{measuring ? 'Проверяем…' : 'Проверить задержку'}</button
              >
            {/if}
          </div>
          <section class="card servers-card">
            {#if servers.length > 0}
              <ul class="servers" aria-label="Серверы">
                {#each servers as server (server.id)}
                  <li>
                    <button
                      type="button"
                      class="server"
                      class:selected={snapshot?.selected === server.id}
                      aria-pressed={snapshot?.selected === server.id}
                      onclick={() => onSelect(server.id)}
                    >
                      <span class="srv-name">
                        {#if server.country}<b class="cc">{server.country}</b>{/if}{displayName(
                          server.name,
                        )}
                      </span>
                      <span class="meta">
                        <span class="scheme">{server.protocol.toUpperCase()}</span>
                        {#if server.measured}
                          <span class="latency" class:dead={server.latencyMs === null}
                            >{formatLatency(server)}</span
                          >
                        {/if}
                      </span>
                    </button>
                  </li>
                {/each}
              </ul>
            {:else}
              <p class="note empty">
                Серверов пока нет. Вставь ссылку https:// или список ссылок серверов в поле выше,
                либо выбери файл.
              </p>
            {/if}
          </section>
        {:else if tab === 'routes'}
          <VpnRoutesTab {snapshot} onsaved={onRulesSaved} />
        {:else if tab === 'advanced'}
          {#each ADVANCED as item (item.id)}
            <section class="card tool switch-row">
              <span class="tool-icon" class:glyph={item.icon === 'X'} aria-hidden="true"
                >{item.icon}</span
              >
              <div class="tool-text">
                <span class="tool-label">Дополнительный инструмент</span>
                <span class="switch-title">{item.label}</span>
                <span class="note"
                  >Запуск и изменение сетевых настроек отключены.<span class="hidden">
                    Состояние: {NETWORK_STATUS_OFF}</span
                  ></span
                >
                {#if networkMessages[item.id]}
                  <p class="note" role="status">{networkMessages[item.id]}</p>
                {/if}
              </div>
              <button
                type="button"
                role="switch"
                aria-checked="false"
                aria-label={item.label}
                onclick={() => askNetworkToggle(item)}
              >
                <span class="knob"></span>
              </button>
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
                  {pendingNetwork.description}. Включение меняет сетевые настройки компьютера. Пока
                  это заглушка: ничего не изменится.
                </p>
                <div class="dialog-actions">
                  <button type="button" onclick={cancelNetworkToggle}>Отмена</button>
                  <button type="button" onclick={confirmNetworkToggle}>Включить</button>
                </div>
              </div>
            </div>
          {/if}
        {:else if tab === 'network'}
          <VpnNetworkTab />
        {:else}
          <VpnConnectionsTab />
        {/if}
      </div>
    {/key}
  </div>
</div>

<style>
  /* Экран VPN рисует свой заголовок и вкладку «Дополнительно», как на эталоне old-01. */
  :global(main:has(> .vpn) > h1),
  :global(main:has(> .vpn) > .lead),
  :global(main:has(> .vpn) > .subtabs) {
    display: none;
  }

  /* Тот же верхний отступ, что у заголовка h1 на остальных страницах (12px). */
  :global(main:has(> .vpn.vpn)) {
    padding-top: 12px;
  }

  /* Слева плакат владельца (как на эталоне old-01), справа колонка карточек. */
  .vpn {
    display: grid;
    flex: 1;
    grid-template-columns: clamp(200px, 26%, 320px) minmax(0, 1fr);
    gap: 20px;
    min-height: 0;
    padding-bottom: 4px;
  }

  /* Постер во всю высоту колонки, как на эталоне old-01: сверху полоса с двумя
     квадратами (Че Гевара на розовом и лаймовом), ниже портрет на остаток высоты.
     Лицо портрета не обрезается: cover с фокусом на верхней трети. */
  .poster-cell {
    display: flex;
    min-width: 0;
    min-height: 0;
  }

  /* Без чёрного фона и без overflow: каждая картинка скруглена сама, поэтому в углах
     нет чёрной каймы от фона (раньше она просвечивала по краю скругления). Чёрные
     разделители между картинками нарисованы рамками самих картинок. */
  .poster {
    display: flex;
    flex-direction: column;
    width: 100%;
    height: 100%;
    padding: 0;
    border: 0;
    border-radius: var(--radius-lg);
    background: none;
    box-shadow: none;
    cursor: pointer;
  }

  .band {
    display: grid;
    flex: none;
    grid-template-columns: 1fr 1fr;
    gap: 0;
  }

  .poster img {
    display: block;
    width: 100%;
    user-select: none;
    pointer-events: none;
  }

  .band img {
    box-sizing: border-box;
    aspect-ratio: 1;
    object-fit: cover;
    border-bottom: 8px solid #000;
  }

  .band img:first-child {
    border-top-left-radius: var(--radius-lg);
    border-right: 4px solid #000;
  }

  .band img:last-child {
    border-top-right-radius: var(--radius-lg);
    border-left: 4px solid #000;
  }

  .poster .portrait {
    flex: 1;
    min-height: 0;
    border-radius: 0 0 var(--radius-lg) var(--radius-lg);
    object-fit: cover;
    object-position: 50% 30%;
  }

  .poster.pressed {
    animation: poster-press 260ms cubic-bezier(0.2, 0, 0, 1);
  }

  @keyframes poster-press {
    0% {
      transform: scale(1);
    }
    35% {
      transform: scale(0.94);
    }
    100% {
      transform: scale(1);
    }
  }

  .column {
    display: flex;
    flex-direction: column;
    gap: 8px;
    min-width: 0;
    min-height: 0;
  }

  .title {
    margin: 0;
    font-family: 'Arial Black', 'Segoe UI', sans-serif;
    font-size: 2.6rem;
    font-weight: 900;
    line-height: 1;
    text-transform: uppercase;
    color: #000;
  }

  /* Чёрная сегментированная панель вкладок; активная — «таблетка» цвета раздела. */
  .tabbar {
    display: flex;
    gap: 4px;
    padding: 6px;
    border-radius: var(--radius-lg);
    background: #111;
  }

  .tab {
    flex: 1;
    min-height: 44px;
    line-height: 1.15;
    padding: 8px 12px;
    border-radius: var(--radius-md);
    background: transparent;
    box-shadow: none;
    color: var(--ink);
    font-size: var(--text-caption);
    font-weight: 800;
    transition:
      background-color 0.18s ease,
      color 0.18s ease;
  }

  .tab:hover:not(.on) {
    background: #1d1d1d;
  }

  .tab.on {
    background: var(--accent);
    color: #000;
    box-shadow: none;
  }

  .tab:focus-visible {
    outline: 2px solid #fff;
    outline-offset: 2px;
  }

  .pane {
    display: flex;
    flex-direction: column;
    flex: 1;
    gap: 8px;
    min-height: 0;
    /* Обводка карточек (тень 2px) раньше обрезалась прокруткой: поле 2px внутри, минус снаружи. */
    margin: -2px;
    padding: 2px;
    overflow-y: auto;
    animation: pane-in 0.22s ease-out;
  }

  @keyframes pane-in {
    from {
      opacity: 0;
      transform: translateY(8px);
    }
    to {
      opacity: 1;
      transform: none;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .pane {
      animation: none;
    }

    .tab {
      transition: none;
    }
  }

  .tagline {
    margin: 8px 0 0;
    font-size: var(--text-caption);
    font-weight: 800;
    color: #000;
  }

  .card {
    padding: 16px 16px;
    border-radius: var(--radius-md);
    background: var(--surface-alt);
    box-shadow: 0 0 0 2px #000;
  }

  h2 {
    margin: 0 0 8px;
    font-size: var(--text-caption);
    font-weight: 800;
    text-transform: uppercase;
    color: var(--ink);
  }

  .list-title {
    margin: 0;
    color: #000;
  }

  .list-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    margin: 8px 0 -2px;
  }

  button.small {
    min-height: 30px;
    padding: 4px 12px;
  }

  button:disabled {
    opacity: 0.6;
    cursor: default;
  }

  .meta {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    width: 100%;
  }

  .latency {
    font-variant-numeric: tabular-nums;
    font-size: 0.75rem;
    font-weight: 800;
    color: var(--accent);
  }

  .latency.dead {
    color: var(--muted);
  }

  .row {
    display: flex;
    gap: 8px;
  }

  .row.tight {
    margin-top: 8px;
  }

  input {
    flex: 1;
    min-width: 0;
    padding: 8px 16px;
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
    margin-bottom: 8px;
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

  .power:disabled,
  .power[aria-disabled='true'] {
    opacity: 0.55;
    cursor: not-allowed;
  }

  .power.on {
    background: var(--accent);
  }

  .power.on .power-knob {
    left: auto;
    right: 6px;
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

  .prepare-bar {
    display: block;
    width: 100%;
    height: 6px;
    border-radius: var(--radius-sm);
    background: #2a2a2a;
    box-shadow: 0 0 0 2px #000;
    overflow: hidden;
  }

  .prepare-fill {
    display: block;
    height: 100%;
    background: var(--accent);
    transition: width 0.2s ease;
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
    border-radius: var(--radius-sm);
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
    margin: 8px;
  }

  .switch-row {
    display: flex;
    flex-shrink: 0;
    align-items: center;
    gap: 16px;
    padding: 22px 20px;
  }

  .tool-icon {
    display: grid;
    flex-shrink: 0;
    place-items: center;
    width: 58px;
    height: 58px;
    border-radius: var(--radius-md);
    background: var(--accent);
    color: #000;
    font-size: 1.5rem;
    font-weight: 900;
  }

  .tool-icon.glyph {
    font-family: 'Arial Black', sans-serif;
    font-size: 1.6rem;
  }

  .tool-text {
    display: flex;
    flex: 1;
    flex-direction: column;
    gap: 3px;
    min-width: 0;
  }

  .tool-label {
    font-size: 0.7rem;
    font-weight: 800;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    color: var(--ink);
  }

  .switch-title {
    font-size: 1.15rem;
    font-weight: 800;
    color: var(--ink);
  }

  .tool-text .note {
    margin: 0;
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
    border-radius: var(--radius-md);
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
      height: auto;
    }

    .tab {
      padding: 8px 4px;
    }
  }
</style>
