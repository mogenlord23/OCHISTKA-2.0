<script lang="ts">
  import SecretField from './SecretField.svelte';
  import Subtabs from './Subtabs.svelte';
  import { SECTIONS } from '../lib/sections';
  import { onMount } from 'svelte';
  import poster from '../assets/pop-art.png';
  import chePink from '../assets/che-pink.png';
  import cheLime from '../assets/che-lime.png';
  import { playHeehee } from '../lib/sound';
  import { coverGeometry, EYE, pageAllowsWink, startWinking } from '../lib/wink';
  import VpnConnectionsTab from './VpnConnectionsTab.svelte';
  import VpnNetworkTab from './VpnNetworkTab.svelte';
  import VpnRoutesTab from './VpnRoutesTab.svelte';
  import {
    IMPORTING_LABEL,
    PREPARE_FAILED_PREFIX,
    TUN_MODE_REASON,
    connectUi,
    displayName,
    elapsedSince,
    formatElapsed,
    formatLatency,
    importMessage,
    normalizePasted,
    prepareImport,
    TOOL_UNAVAILABLE,
    toolStateText,
    type ToolModel,
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

  let { subsection, onSubsection }: { subsection: string; onSubsection?: (id: string) => void } =
    $props();

  const vpnSubs = SECTIONS.find((section) => section.id === 'vpn')?.subsections ?? [];

  type Tab = 'vpn' | 'advanced' | 'network' | 'connections';
  const TABS: { id: Tab; label: string }[] = [
    { id: 'vpn', label: 'VPN' },
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
    {
      id: 'zapret',
      label: 'Zapret',
      description: 'Помогает открывать заблокированные сайты',
    },
    {
      id: 'xbox-dns',
      label: 'Xbox DNS',
      description: 'Свои DNS-серверы для Xbox и игр',
    },
  ] as const;

  /** Состояние инструментов. Пока оба не реализованы: кнопка выключена и недоступна, без имитации. */
  const TOOLS: Record<(typeof ADVANCED)[number]['id'], ToolModel> = {
    zapret: TOOL_UNAVAILABLE,
    'xbox-dns': TOOL_UNAVAILABLE,
  };

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

  /** Пасхалка: звук (если включён) и анимация нажатия. */
  let pressed = $state(false);
  let posterEl: HTMLButtonElement | null = null;
  let portraitWrap: HTMLElement | null = null;
  let winking = $state(false);
  let eyeStyle = $state('');

  /** Подмигивание: раз в 20-40 с, только когда окно видно и в фокусе и нет нажатия. */
  function wink() {
    if (!portraitWrap) return;
    const g = coverGeometry(portraitWrap.clientWidth, portraitWrap.clientHeight);
    eyeStyle =
      `left:${g.offsetX + EYE.x * g.scale}px;top:${g.offsetY + EYE.y * g.scale}px;` +
      `width:${EYE.w * g.scale}px;height:${EYE.h * g.scale}px`;
    winking = true;
  }

  onMount(() => startWinking({ canWink: () => !pressed && pageAllowsWink(), wink }));

  /** Один запуск анимации и звука на одно нажатие. Повторное нажатие перезапускает цикл. */
  function playPoster() {
    winking = false;
    pressed = false;
    // Чтение размера заставляет браузер применить снятый класс: анимация начнётся заново.
    void posterEl?.offsetWidth;
    pressed = true;
    playHeehee();
  }

  /** Мышь и касание срабатывают при нажатии (мгновенно). */
  function onPosterDown(event: PointerEvent) {
    if (event.button === 0) playPoster();
  }

  /** Клик с мыши уже обработан при нажатии; сюда доходит только клавиатура (detail === 0). */
  function onPosterClick(event: MouseEvent) {
    if (event.detail === 0) playPoster();
  }

  /** Колесо над постером не прокручивает страницу. Слушатель не пассивный, иначе
      preventDefault игнорируется. */
  function noWheelScroll(node: HTMLElement) {
    const stop = (event: WheelEvent) => event.preventDefault();
    node.addEventListener('wheel', stop, { passive: false });
    return { destroy: () => node.removeEventListener('wheel', stop) };
  }
</script>

<div class="vpn">
  <div class="poster-cell">
    <button
      type="button"
      class="poster"
      class:pressed
      aria-label="Постер"
      bind:this={posterEl}
      use:noWheelScroll
      onpointerdown={onPosterDown}
      onclick={onPosterClick}
      onanimationend={() => (pressed = false)}
    >
      <span class="band">
        <img src={chePink} alt="" draggable="false" />
        <img src={cheLime} alt="" draggable="false" />
      </span>
      <span class="portrait-wrap" bind:this={portraitWrap}>
        <img class="portrait" src={poster} alt="" draggable="false" />
        {#if winking}
          <span class="eye" style={eyeStyle} aria-hidden="true">
            <span class="lid" onanimationend={() => (winking = false)}></span>
          </span>
        {/if}
      </span>
    </button>
  </div>
  <div class="column">
    <header class="head">
      <div class="title" data-testid="vpn-title">{tabLabel}</div>
      <p class="tagline">Подписка, серверы, маршруты · сеть и соединения</p>
    </header>

    <Subtabs
      items={vpnSubs}
      active={tab}
      onSelect={(id) => {
        tab = tabOf(id);
        onSubsection?.(tab);
      }}
    />

    {#key tab}
      <div class="pane" role="tabpanel" aria-label={tabLabel}>
        {#if tab === 'vpn'}
          <section class="card">
            <h2 class="sr-only">Ссылка подписки / импорта</h2>
            <div class="row">
              <SecretField bind:value={urlInput} onenter={onImport} />
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

          <h2 class="sr-only">Серверы / {servers.length}</h2>
          <section class="card servers-card">
            {#if servers.length > 0}
              <div class="list-head">
                <button type="button" class="small" disabled={measuring} onclick={onMeasure}
                  >{measuring ? 'Проверяем…' : 'Проверить задержку'}</button
                >
              </div>
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
        {:else if tab === 'advanced'}
          {#each ADVANCED as item (item.id)}
            {@const tool = TOOLS[item.id]}
            <section class="card tool switch-row">
              <button
                type="button"
                role="switch"
                class="power tool-power"
                class:on={tool.available && tool.status === 'on'}
                aria-checked={tool.available && tool.status === 'on'}
                aria-busy={tool.available && tool.status === 'pending'}
                aria-label={item.label}
                disabled={!tool.available || tool.status === 'pending'}
              >
                <span class="power-knob" aria-hidden="true"></span>
              </button>
              <div class="tool-text">
                <span class="switch-title">{item.label}</span>
                <span class="tool-state">{toolStateText(tool)}</span>
                <span class="note">{item.description}.{tool.reason ? ` ${tool.reason}` : ''}</span>
              </div>
            </section>
          {/each}
          <VpnRoutesTab {snapshot} onsaved={onRulesSaved} />
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

  .portrait-wrap {
    position: relative;
    flex: 1;
    min-height: 0;
  }

  .poster .portrait {
    position: absolute;
    inset: 0;
    height: 100%;
    border-radius: 0 0 var(--radius-lg) var(--radius-lg);
    object-fit: cover;
    object-position: 50% 30%;
  }

  /* Нажатие уже играет собственную анимацию: общее сжатие кнопки на постере
     не добавляется, иначе получался двойной цикл. */
  /* Подмигивание: веко цвета кожи с чёрной линией ресниц, только transform. */
  .eye {
    position: absolute;
    pointer-events: none;
  }

  .lid {
    display: block;
    width: 100%;
    height: 100%;
    box-sizing: border-box;
    border-bottom: 3px solid #000;
    border-radius: 50%;
    background: #fbbebd;
    transform: scaleY(0);
    transform-origin: 50% 0;
    animation: poster-wink 340ms ease-in-out forwards;
  }

  @keyframes poster-wink {
    0% {
      transform: scaleY(0);
    }
    35%,
    55% {
      transform: scaleY(1);
    }
    100% {
      transform: scaleY(0);
    }
  }

  .poster:active:not(:disabled) {
    transform: none;
  }

  .poster {
    overscroll-behavior: contain;
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

  .head {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }

  /* Три уровня промежутков: внутри карточки 8, между карточками и вкладками 12, между крупными
     секциями 20. Верхняя панель вкладок стоит вплотную к верху колонки, как постер слева. */
  .column :global(.subtabs) {
    margin: 0;
    min-height: 0;
  }

  .column {
    display: flex;
    flex-direction: column;
    gap: 12px;
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

  .pane {
    display: flex;
    flex-direction: column;
    flex: 1;
    gap: 12px;
    min-height: 0;
    /* Обводка карточек (тень 2px) раньше обрезалась прокруткой: поле 2px внутри, минус снаружи. */
    margin: -2px;
    padding: 2px;
    /* Обрезание прокручиваемого содержимого идёт по скруглённой границе, а не по прямой линии. */
    border-radius: var(--radius-lg);
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
    .eye {
      display: none;
    }

    .pane {
      animation: none;
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

  .list-head {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 8px;
    margin: 0 0 8px;
    /* Кнопка остаётся наверху карточки при прокрутке списка серверов. */
    position: sticky;
    top: 0;
    z-index: 1;
    background: var(--surface-alt);
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
    border-radius: var(--radius-pill);
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

  .tool-power {
    align-self: center;
  }

  .tool-state {
    font-size: 0.7rem;
    font-weight: 800;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    color: var(--accent);
  }

  .tool-text {
    display: flex;
    flex: 1;
    flex-direction: column;
    gap: 3px;
    min-width: 0;
  }

  .switch-title {
    font-size: 1.15rem;
    font-weight: 800;
    color: var(--ink);
  }

  .tool-text .note {
    margin: 0;
  }

  @media (max-width: 760px) {
    .vpn {
      height: auto;
    }
  }

  /* Невысокое окно (минимум 712): карточки компактнее, чтобы блок серверов и «Соединения»
     заканчивались на нижней линии постера, а не уходили под неё. Размеры вкладок те же. */
  @media (max-height: 780px) {
    .card {
      padding: 12px 16px;
    }

    .servers-card {
      min-height: 96px;
    }

    .note {
      margin-top: 6px;
    }
  }
</style>
