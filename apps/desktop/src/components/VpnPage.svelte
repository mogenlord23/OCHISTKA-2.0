<script lang="ts">
  import { t } from '../lib/i18n.svelte';
  import VpnConnectionsTab from './VpnConnectionsTab.svelte';
  import VpnNetworkTab from './VpnNetworkTab.svelte';
  import {
    VPN_TUN_MODE_REASON,
    formatElapsed,
    parseSubscription,
    validateSubscriptionUrl,
    type ParsedSubscription,
  } from '../lib/vpn';

  /** Заголовок, описание и вкладки рисует PageShell; здесь только содержимое подраздела. */
  let { subsection }: { subsection: string } = $props();

  type Tab = 'vpn' | 'advanced' | 'network' | 'connections';
  const TAB_IDS: readonly Tab[] = ['vpn', 'advanced', 'network', 'connections'];
  const tab = $derived<Tab>(TAB_IDS.find((id) => id === subsection) ?? 'vpn');

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

  function summary(value: ParsedSubscription): string {
    return t('vpn.imported')
      .replace('{n}', String(value.servers.length))
      .replace('{m}', String(value.unknownCount));
  }

  async function onFile(event: Event) {
    const file = (event.currentTarget as HTMLInputElement).files?.[0];
    if (!file) return;
    parsed = parseSubscription(await file.text());
    selected = 0;
    importMessage = summary(parsed);
  }

  /** Zapret и Xbox DNS: карточки остаются, переключатели выключены до подключения сетевого модуля. */
  const ADVANCED = [
    { id: 'zapret', label: 'Zapret', icon: '↗' },
    { id: 'xbox-dns', label: 'Xbox DNS', icon: 'X' },
  ] as const;

  let urlInput = $state('');
  let importMessage = $state<string | null>(null);
  let parsed = $state<ParsedSubscription | null>(null);

  /** Подключение не устанавливается, поэтому таймер стоит на нуле. */
  const elapsedSeconds = 0;

  function onImport() {
    // Вставленный текст подписки (vless://… и т. п.) разбираем сразу, ссылку — проверяем.
    const pasted = parseSubscription(
      urlInput.replace(/\s+(?=(vless|vmess|trojan|ss):\/\/)/g, '\n'),
    );
    if (pasted.servers.length > 0) {
      parsed = pasted;
      selected = 0;
      importMessage = summary(pasted);
      return;
    }
    const result = validateSubscriptionUrl(urlInput);
    importMessage = result.ok ? t('vpn.linkOk') : result.reason;
  }
</script>

{#key tab}
  <div class="vpn">
    {#if tab === 'vpn'}
      <div class="grid">
        <section class="card">
          <h2>{t('vpn.subscription.title')}</h2>
          <div class="row">
            <input
              bind:value={urlInput}
              placeholder="https://…"
              aria-label={t('vpn.subscription.aria')}
            />
            <button type="button" onclick={onImport}>{t('vpn.import')}</button>
          </div>
          <div class="row">
            <button type="button" onclick={() => fileInput?.click()}>{t('vpn.fromFile')}</button>
            <input
              bind:this={fileInput}
              class="hidden"
              type="file"
              accept=".txt,text/plain"
              aria-label={t('vpn.fileAria')}
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
              aria-label={t('vpn.connect')}
              disabled
            >
              <span class="power-knob" aria-hidden="true"></span>
            </button>
            <div class="power-text">
              <span class="power-label">{t('vpn.connect')}</span>
              <span class="timer" aria-label={t('vpn.timer')}>{formatElapsed(elapsedSeconds)}</span>
              <span class="status-line">{t('vpn.statusOff')}</span>
            </div>
          </div>
          <label class="select">
            <span class="hidden">{t('vpn.mode')}</span>
            <select disabled aria-label={t('vpn.mode')}>
              <option>{t('vpn.modeTun')}</option>
            </select>
            <span class="chev" aria-hidden="true"></span>
          </label>
          <p class="note">{VPN_TUN_MODE_REASON}</p>
        </section>
      </div>

      <h2 class="list-title">{t('vpn.servers')} / {parsed?.servers.length ?? 0}</h2>
      <section class="card servers-card">
        {#if parsed && parsed.servers.length > 0}
          <ul class="servers" aria-label={t('vpn.servers')}>
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
          <p class="note empty">{t('vpn.noServers')}</p>
        {/if}
      </section>
    {:else if tab === 'advanced'}
      <div class="grid">
        {#each ADVANCED as item (item.id)}
          <section class="card switch-row">
            <span class="tool-icon" class:glyph={item.icon === 'X'} aria-hidden="true"
              >{item.icon}</span
            >
            <div class="tool-text">
              <span class="tool-label">{t('vpn.tool')}</span>
              <span class="switch-title">{item.label}</span>
              <span class="note">{t('vpn.stateOff')}</span>
            </div>
            <button
              type="button"
              role="switch"
              aria-checked="false"
              aria-label={item.label}
              disabled
            >
              <span class="knob"></span>
            </button>
          </section>
        {/each}
      </div>
    {:else if tab === 'network'}
      <VpnNetworkTab />
    {:else}
      <VpnConnectionsTab />
    {/if}
  </div>
{/key}

<style>
  /* Фон страницы VPN: зелёный цвет первого экрана «Очистка» старой версии. Узор раздела
     (cover, фокус по центру) остаётся общим слоем оболочки; картинки в углу нет. */
  :global(.stage:has(.vpn)) {
    --accent: #b4ff00;
    --page-text: #000;
    background-color: #b4ff00;
  }

  /* Сетка 8 px: отступ между карточками 16, внутри карточки 16. */
  .vpn {
    display: flex;
    flex-direction: column;
    gap: 16px;
    animation: pane-in 0.22s ease-out;
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(min(100%, 440px), 1fr));
    gap: 16px;
    align-items: stretch;
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
    .vpn {
      animation: none;
    }
  }

  .card {
    margin: 0;
    padding: 16px;
    border-radius: var(--radius-lg);
    background: var(--surface-alt);
    box-shadow: 0 0 0 2px #000;
  }

  h2 {
    margin: 0 0 16px;
    font-size: var(--text-caption);
    font-weight: 800;
    text-transform: uppercase;
    color: var(--ink);
  }

  .list-title {
    margin: 8px 0 -8px;
    color: #000;
  }

  .row {
    display: flex;
    gap: 8px;
  }

  .row + .row {
    margin-top: 8px;
  }

  input {
    flex: 1;
    min-width: 0;
    min-height: 48px;
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
    min-height: 40px;
    padding: 8px 16px;
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

  button:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }

  .row > button {
    min-height: 48px;
  }

  .row + .row > button {
    min-height: 40px;
  }

  .power-row {
    display: flex;
    align-items: center;
    gap: 16px;
    margin-bottom: 16px;
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
    top: 4px;
    left: 8px;
    width: 40px;
    height: 40px;
    border-radius: 50%;
    background: #fff;
    box-shadow: 0 0 0 2px #000;
  }

  .power-text {
    display: flex;
    flex-direction: column;
    gap: 0;
  }

  .power-label {
    font-size: var(--text-caption);
    font-weight: 800;
    text-transform: uppercase;
    color: var(--ink);
  }

  .timer {
    font-variant-numeric: tabular-nums;
    font-size: 24px;
    font-weight: 800;
    line-height: 32px;
    color: var(--ink);
  }

  .status-line {
    font-size: var(--text-caption);
    color: var(--ink);
  }

  .select {
    position: relative;
    display: block;
  }

  select {
    width: 100%;
    min-height: 40px;
    padding: 8px 16px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: #141414;
    color: var(--ink);
    font: inherit;
    font-size: var(--text-caption);
    appearance: none;
  }

  select:disabled {
    opacity: 0.6;
  }

  .chev {
    position: absolute;
    right: 16px;
    top: 14px;
    width: 8px;
    height: 8px;
    border-right: 2px solid var(--accent);
    border-bottom: 2px solid var(--accent);
    transform: rotate(45deg);
  }

  .servers-card {
    min-height: 128px;
    padding: 8px;
  }

  .servers {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  /* Выбранный сервер отмечен одной обводкой в цвете раздела, без галочек и полос. */
  .server {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 0;
    width: 100%;
    min-height: 56px;
    padding: 8px 16px;
    background: transparent;
    box-shadow: none;
    color: var(--muted);
    font-weight: 400;
    text-align: left;
  }

  .server.selected {
    background: #222;
    box-shadow: inset 0 0 0 2px var(--accent);
  }

  .cc {
    margin-right: 8px;
    font-size: var(--text-caption);
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
    align-items: center;
    gap: 16px;
    min-height: 104px;
  }

  .tool-icon {
    display: grid;
    flex-shrink: 0;
    place-items: center;
    width: 56px;
    height: 56px;
    border-radius: 16px;
    background: var(--accent);
    color: #000;
    font-size: 24px;
    font-weight: 900;
  }

  .tool-icon.glyph {
    font-family: 'Arial Black', sans-serif;
  }

  .tool-text {
    display: flex;
    flex: 1;
    flex-direction: column;
    gap: 0;
    min-width: 0;
  }

  .tool-label {
    font-size: var(--text-caption);
    font-weight: 800;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    color: var(--ink);
  }

  .switch-title {
    font-size: 18px;
    line-height: 24px;
    font-weight: 800;
    color: var(--ink);
  }

  .tool-text .note {
    margin: 0;
  }

  [role='switch'] {
    position: relative;
    flex-shrink: 0;
    width: 96px;
    height: 48px;
    padding: 0;
    border-radius: 999px;
    background: #2a2a2a;
    box-shadow: 0 0 0 2px #fff;
  }

  .knob {
    position: absolute;
    top: 8px;
    left: 8px;
    width: 32px;
    height: 32px;
    border-radius: 50%;
    background: #fff;
  }
</style>
