<script lang="ts">
  import SecretField from './SecretField.svelte';
  import { createLabelTypewriter, LABEL } from '../lib/labelTypewriter';
  import ServerFlag from './ServerFlag.svelte';
  import ExpandButton from './ExpandButton.svelte';
  import VpnPowerIcon from './VpnPowerIcon.svelte';
  import { groupByLabel, numberedLabels, serverLabel } from '../lib/countryFlags';
  import { i18n } from '../lib/i18n.svelte';
  import Subtabs from './Subtabs.svelte';
  import { SECTIONS } from '../lib/sections';
  import { onDestroy, onMount, untrack, type Snippet } from 'svelte';
  import { listenDesktop } from '../lib/tray';
  import { slideInOnChange } from '../lib/slideInOnChange';
  import poster from '../assets/pop-art.png';
  import { playHeehee } from '../lib/sound';
  import { coverGeometry, EYE, pageAllowsWink, startWinking } from '../lib/wink';
  import VpnConnectionsTab from './VpnConnectionsTab.svelte';
  import VpnNetworkTab from './VpnNetworkTab.svelte';
  import VpnRoutesTab from './VpnRoutesTab.svelte';
  import {
    PREPARE_FAILED_PREFIX,
    connectUi,
    elapsedSince,
    formatElapsed,
    formatLatency,
    importMessage,
    isSubscriptionUrl,
    normalizePasted,
    prepareImport,
    statusLine,
    NO_SERVERS_REASON,
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

  let { subsection, header, active = true }: { subsection: string; header?: Snippet; active?: boolean } = $props();

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
  let networkVisited = $state(false);
  $effect(() => {
    if (tab === 'network') networkVisited = true;
  });
  let fileInput: HTMLInputElement | undefined = $state();
  /** Размер файла подписки не больше 5 МБ (так же ограничивает Rust). */
  const MAX_FILE_BYTES = 5 * 1024 * 1024;

  let snapshot = $state<VpnSnapshot | null>(null);
  let urlInput = $state('');
  let importNote = $state<string | null>(null);
  /** Подпись поля ссылки: меняется печатным эффектом после успешной вставки. */
  let fieldLabel = $state(LABEL);
  const labelEffect = createLabelTypewriter((text) => (fieldLabel = text));
  onDestroy(() => labelEffect.reset());
  let importing = $state(false);
  let measuring = $state(false);
  /** Растёт после каждого успешного импорта: запускает новую первую проверку. */
  let importTick = $state(0);
  let requesting = $state(false);
  let actionNote = $state<string | null>(null);
  let nowMs = $state(Date.now());
  /** Проценты подготовки VPN при первом подключении; `null`, когда подготовки нет. */
  let preparing = $state<number | null>(null);
  let retry = $state(false);

  const servers = $derived(snapshot?.servers ?? []);
  /** Число, а не сам список: замер задержек не перезапускает таймер проверок. */
  const serverCount = $derived(servers.length);
  const ui = $derived(
    connectUi(snapshot, requesting || importing, {
      preparing,
      retry,
      disconnecting: requesting && snapshot?.connection.state === 'connected',
    }),
  );
  const elapsedSeconds = $derived(
    snapshot?.connection.state === 'connected' || snapshot?.connection.state === 'switching'
      ? elapsedSince(snapshot.connection.since, nowMs)
      : 0,
  );
  const status = $derived(
    actionNote ??
      (preparing !== null
        ? 'Скачиваем нужные файлы: это нужно один раз'
        : statusLine(
            snapshot?.connection ?? null,
            ui.reason === NO_SERVERS_REASON ? null : ui.reason,
          )),
  );

  async function refresh() {
    try {
      const next = await vpnSnapshot();
      snapshot = next;
      // Поле подписки не заполняется сохранённой ссылкой: личная ссылка не показывается при открытии.
    } catch (err) {
      actionNote = `Не удалось получить состояние VPN: ${errorMessage(err)}`;
    }
  }

  /** Сохранённая ссылка подписки кладётся в поле. Поле прячет её анимацией, пока не в фокусе.
      Ссылку уже импортировали, поэтому автоимпорт и повторный импорт не запускаем: она помечается
      как обработанная. */
  function restoreSavedLink() {
    const saved = snapshot?.subscriptionUrl?.trim() ?? '';
    if (!saved || urlInput !== '') return;
    previousLength = saved.length;
    lastAutoLink = saved;
    urlInput = saved;
  }

  $effect(() => {
    if (!active) return;
    // Подписка уже сохранена (серверы есть до вставки ссылки): подпись сразу «Подписка сохранена».
    void refresh().then(() => {
      restoreSavedLink();
      if ((snapshot?.servers?.length ?? 0) > 0 && fieldLabel === LABEL && !importing) {
        labelEffect.showSaved();
      }
    });
  });

  /** Только пока VPN включён: секундомер и проверка раз в 3 секунды, что он жив. Когда VPN выключен,
      ни таймеров, ни запросов к ядру нет: следующий запрос — по нажатию. */
  const isConnected = $derived(
    snapshot?.connection?.state === 'connected' || snapshot?.connection?.state === 'switching',
  );
  $effect(() => {
    if (!active || !isConnected) return;
    nowMs = Date.now();
    const clock = setInterval(() => (nowMs = Date.now()), 1000);
    const poll = setInterval(async () => {
      try {
        const connection = await vpnStatus();
        if (snapshot && connection) snapshot = { ...snapshot, connection };
      } catch {
        // Следующая проверка повторит запрос.
      }
    }, 3000);
    return () => {
      clearInterval(clock);
      clearInterval(poll);
    };
  });

  /** Задержки серверов видны всё время, пока открыт раздел VPN (включён он или нет): замеры идут
      непрерывно. Уход из раздела останавливает их. */
  $effect(() => {
    if (!active || tab !== 'vpn') return;
    // Замеры идут один за другим, без паузы в несколько секунд: цифры обновляются постоянно.
    let alive = true;
    const loop = async () => {
      while (alive) {
        await onMeasure();
        if (!alive) break;
        await new Promise((resolve) => setTimeout(resolve, 1000));
      }
    };
    untrack(() => void loop());
    return () => {
      alive = false;
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
    importNote = null;
    try {
      const summary = await vpnImport(prepared.input);
      importNote = importMessage(summary.count, summary.skipped, summary.issues) || null;
      labelEffect.play();
      await refresh();
      measuredAt = 0;
      importTick += 1;
    } catch (err) {
      importNote = errorMessage(err);
    } finally {
      importing = false;
    }
  }

  /** Автоимпорт: как только вставлена корректная ссылка https:// (длина выросла сразу на много знаков),
      подписка импортируется сама. Побуквенный набор не импортирует недописанный адрес. */
  let lastAutoLink = '';
  let previousLength = 0;
  $effect(() => {
    const raw = urlInput;
    if (raw === '') lastAutoLink = '';
    const pasted = raw.length - previousLength >= 8;
    previousLength = raw.length;
    if (pasted && !importing && /^http:\/\/\S+$/i.test(raw.trim())) {
      untrack(() => (importNote = 'Разрешены только ссылки https://'));
      return;
    }
    if (!pasted || importing || !isSubscriptionUrl(normalizePasted(raw))) return;
    const link = raw.trim();
    if (link === lastAutoLink) return;
    lastAutoLink = link;
    untrack(() => void runImport(raw));
  });

  function onImport() {
    // Ссылка уже импортирована автоматически: повторно не грузим.
    if (urlInput.trim() !== '' && urlInput.trim() === lastAutoLink) return;
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

  /** Список по странам: серверы одной страны идут подряд. Подписи: страна, а повторы получают номер
      (Европа, Европа 2, США, США 2). */
  const groupedServers = $derived(
    groupByLabel(servers, (server) => serverLabel(server.name, server.country, i18n.lang)),
  );
  const selectedServerIndex = $derived(
    groupedServers.findIndex((server) => server.id === snapshot?.selected),
  );
  const serverLabels = $derived(
    numberedLabels(
      groupedServers.map((server) => serverLabel(server.name, server.country, i18n.lang)),
    ),
  );

  /** Имя для скринридера: подпись со страной и исходное имя сервера. */
  function serverAria(name: string, country: string | null, lang: string): string {
    const label = serverLabel(name, country, lang);
    return label === name ? name : `${label}, ${name}`;
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

  let measuredAt = 0;
  let measuredTick = 0;

  /** Один замер за раз. Меняет только задержки серверов: выбор области и состояние VPN, которые
      пользователь мог поменять за это время, не затираются. */
  async function onMeasure() {
    if (measuring) return;
    measuring = true;
    try {
      const fresh = await vpnMeasureLatency();
      if (snapshot && fresh) snapshot = { ...snapshot, servers: fresh.servers };
    } catch {
      // Тихо: показанные значения сохраняются.
    } finally {
      measuredAt = Date.now();
      measuring = false;
    }
  }

  /** Задержки меряются один раз после импорта подписки (по действию пользователя), без таймеров. */
  $effect(() => {
    const tick = importTick;
    if (subsection !== 'vpn' || serverCount === 0) return;
    if (tick === 0 || tick === untrack(() => measuredTick)) return;
    measuredTick = tick;
    untrack(() => void onMeasure());
  });

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
      if (snapshot && connection) {
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

  onMount(() => listenDesktop('tray-vpn-changed', () => { void refresh(); }));

  function onRulesSaved(rules: RoutingRules) {
    if (snapshot) snapshot = { ...snapshot, rules };
  }

  /** Пасхалка: звук (если включён) и анимация нажатия. */
  let pressed = $state(false);
  let posterEl = $state<HTMLButtonElement | null>(null);
  let portraitWrap = $state<HTMLElement | null>(null);
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

  onMount(() => startWinking({ canWink: () => active && !pressed && pageAllowsWink(), wink }));

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
      <p class="tagline">Подписка, серверы и сайты · сеть и соединения</p>
    </header>

    {#if header && active}
      {@render header()}
    {:else if !header}
      <Subtabs items={vpnSubs} active={tab} onSelect={(id) => (tab = tabOf(id))} />
    {/if}

    <div class="pane-slot" use:slideInOnChange={active ? tab : null}>
      {#if networkVisited || tab === 'network'}
        <div class="pane section-scroll" role="tabpanel" aria-label="Сеть" hidden={tab !== 'network'}>
          <VpnNetworkTab active={active && tab === 'network'} />
        </div>
      {/if}

      {#if tab !== 'network'}
        <div class="pane section-scroll" role="tabpanel" aria-label={tabLabel}>
          {#if tab === 'vpn'}
            <section class="card">
              <h2 class="sr-only">Ссылка подписки / импорта</h2>
              <div class="row">
                <SecretField bind:value={urlInput} label={fieldLabel} onenter={onImport} />
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
              <div class="connect-grid">
                <div class="power-row">
                  <button
                    type="button"
                    class="power"
                    class:on={ui.pressed}
                    aria-pressed={ui.pressed}
                    aria-busy={ui.busy}
                    aria-label={ui.label}
                    title={ui.reason ?? undefined}
                    aria-disabled={ui.disabled}
                    onclick={onConnect}
                  >
                    <VpnPowerIcon />
                  </button>
                  <div class="power-text">
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
              </div>
            </section>

            <h2 class="sr-only">Серверы / {servers.length}</h2>
            <section class="card servers-card">
              {#if servers.length > 0}
                <ul
                  class="servers"
                  class:has-selection={selectedServerIndex >= 0}
                  style:--selected-server-index={selectedServerIndex}
                  aria-label="Серверы"
                >
                  {#each groupedServers as server, i (server.id)}
                    <li>
                      <button
                        type="button"
                        class="server"
                        aria-pressed={snapshot?.selected === server.id}
                        aria-label={serverAria(server.name, server.country, i18n.lang)}
                        onclick={() => onSelect(server.id)}
                      >
                        <ServerFlag code={server.country} />
                        <span class="srv-name"
                          >{serverLabels[i]}</span
                        >
                        <span class="scheme">{server.protocol.toUpperCase()}</span>
                        <span
                          class="latency"
                          class:dead={server.measured && server.latencyMs === null}
                          >{server.measured ? formatLatency(server) : ''}</span
                        >
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
            <VpnRoutesTab {snapshot} onsaved={onRulesSaved} />
          {:else}
            <VpnConnectionsTab />
          {/if}
        </div>
      {/if}
    </div>
  </div>
</div>

<style>
  .vpn {
    display: grid;
    flex: 1;
    grid-template-columns: clamp(154px, 27%, 320px) minmax(0, 1fr);
    gap: var(--frame-inset, 14px);
    min-height: 0;
  }

  .poster-cell {
    display: flex;
    min-width: 0;
    min-height: 0;
  }

  .poster {
    display: flex;
    flex-direction: column;
    width: 100%;
    height: 100%;
    padding: 0;
    border: 0;
    border-radius: var(--radius-adaptive);
    background: none;
    box-shadow: none;
    cursor: pointer;
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
    width: 100%;
    border-radius: var(--radius-adaptive);
    user-select: none;
    pointer-events: none;
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

  .column {
    display: flex;
    flex-direction: column;
    gap: var(--section-gap, 14px);
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

  .pane-slot,
  .pane {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-height: 0;
  }

  .pane {
    gap: var(--section-gap, 14px);
  }

  .pane[hidden] { display: none; }

  @media (prefers-reduced-motion: reduce) {
    .eye {
      display: none;
    }
  }

  .tagline {
    margin: 8px 0 0;
    font-size: var(--text-caption);
    font-weight: 800;
    color: #000;
  }

  .card {
    padding: var(--card-inset);
    border-radius: var(--radius-adaptive);
    background: var(--surface-alt);
    box-shadow: inset 0 0 0 2px #000;
  }

  h2 {
    margin: 0 0 8px;
    font-size: var(--text-caption);
    font-weight: 800;
    text-transform: uppercase;
    color: var(--ink);
  }

  button:disabled {
    opacity: 0.6;
    cursor: default;
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

  input {
    flex: 1;
    min-width: 0;
    padding: 8px 16px;
    border: 1px solid var(--border);
    border-radius: var(--radius-adaptive);
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
    border-radius: var(--radius-adaptive);
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

  .power-row {
    display: flex;
    align-items: center;
    gap: 16px;
    margin-bottom: 8px;
  }

  /* Квадратная плитка с тем же скруглением и размером, что у боковой панели. Выключено — тёмная с
     обводкой акцентом; включено — акцент. */
  .power {
    position: relative;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 88px;
    height: 88px;
    min-height: 0;
    padding: 0;
    border: 0;
    border-radius: var(--radius-adaptive);
    background: #2a2a2a;
    color: var(--accent);
    box-shadow: 0 0 0 2px var(--accent);
    cursor: pointer;
  }

  .power:disabled,
  .power[aria-disabled='true'] {
    opacity: 0.55;
    cursor: not-allowed;
  }

  .power.on {
    background: var(--accent);
    color: #000;
  }

  .power-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .prepare-bar {
    display: block;
    width: 100%;
    height: 6px;
    border-radius: var(--radius-adaptive);
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
    font-size: 2.1rem;
    font-weight: 650;
    line-height: 1;
    letter-spacing: 0.01em;
    color: var(--ink);
  }

  .status-line {
    font-size: 0.75rem;
    color: var(--ink);
  }

  .servers-card {
    flex: 1;
    min-height: 120px;
    /* Внутренний отступ от обводки до первой строки одинаковый с трёх сторон. */
    padding: var(--card-inset);
    overflow-y: auto;
    /* Прокрутка работает (колесо, тачпад, клавиши), полоса не показывается. */
    scrollbar-width: none;
  }

  .servers-card::-webkit-scrollbar {
    display: none;
  }

  .servers {
    --server-height: 44px;
    position: relative;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .servers.has-selection::before {
    --corner-size: var(--server-height);
    content: '';
    position: absolute;
    inset: 0 0 auto;
    height: var(--server-height);
    border-radius: var(--radius-adaptive);
    background: #222;
    box-shadow: inset 0 0 0 1px var(--accent);
    transform: translateY(calc(var(--selected-server-index) * var(--server-height)));
    transition: transform var(--selection-duration, 190ms) var(--selection-ease, ease-out);
    pointer-events: none;
  }

  .server {
    --corner-size: var(--server-height);
    position: relative;
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    height: var(--server-height);
    padding: 10px 12px;
    border-radius: var(--radius-adaptive);
    background: transparent;
    box-shadow: none;
    color: var(--ink);
    font-weight: 600;
    text-align: left;
    transition: background-color 120ms ease;
  }

  .server:hover:not([aria-pressed='true']) {
    background: #1a1a1a;
  }

  .server:active:not(:disabled) {
    transform: none;
  }

  .srv-name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .scheme {
    flex: none;
    font-size: 0.7rem;
    font-weight: 800;
    letter-spacing: 0.06em;
    color: var(--muted);
  }

  .server .latency {
    flex: none;
    min-width: 58px;
    text-align: right;
  }

  .note {
    margin: 8px 0 0;
    font-size: var(--text-caption);
    color: var(--muted);
  }

  .empty {
    margin: 8px;
  }







  .connect-grid {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 16px;
  }

  .connect-grid .power-row {
    margin-bottom: 0;
  }










  @media (max-width: 760px) {
    .vpn {
      height: auto;
    }
  }

  /* Невысокое окно (минимум 712): блок серверов и «Соединения» заканчиваются на нижней линии постера.
     Отступ карточек остаётся одинаковым со всех сторон (16 px), как в остальных разделах. */
  @media (max-height: 780px) {
    .card {
      padding: 16px;
    }

    .servers-card {
      min-height: 96px;
    }

    .note {
      margin-top: 6px;
    }
  }
</style>
