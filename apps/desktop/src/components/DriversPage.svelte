<script lang="ts">
  import { getHardware, getSystemOverview } from '../lib/api';
  import { loadSnapshot } from '../lib/computerCache';
  import {
    getDriversCatalog,
    installDriver,
    openDriverPage,
    openWindowsUpdate,
  } from '../lib/driversApi';
  import {
    buttonLabel,
    fillPercent,
    IDLE,
    isBusy,
    stateFromProgress,
    type ButtonState,
  } from '../lib/driverInstall';
  import {
    compatibilityNote,
    filterByVendor,
    layoutVendors,
    VENDORS,
    type DriverItem,
    type VendorId,
  } from '../lib/driversCatalog';
  import type { HardwareOverview } from '../lib/types';
  import VendorLogo from './VendorLogo.svelte';

  const cached = loadSnapshot();
  let hardware = $state<HardwareOverview | null>(cached?.hardware ?? null);
  let os = $state<{ osName: string; osVersion: string } | null>(
    cached?.overview
      ? { osName: cached.overview.osName, osVersion: cached.overview.osVersion }
      : null,
  );
  let items = $state<DriverItem[]>([]);
  let loadError = $state<string | null>(null);
  let selected = $state<VendorId | null>(null);
  let states = $state<Record<string, ButtonState>>({});

  $effect(() => {
    let active = true;
    getHardware()
      .then((value) => {
        if (active) {
          hardware = value;
        }
      })
      .catch(() => {
        // Без сведений о железе производители идут в обычном порядке, без отметок.
      });
    getSystemOverview()
      .then((value) => {
        if (active) {
          os = { osName: value.osName, osVersion: value.osVersion };
        }
      })
      .catch(() => {
        // Версия Windows неизвестна: заметка о совместимости покажет только то, что известно.
      });
    getDriversCatalog()
      .then((value) => {
        if (active) {
          items = value;
        }
      })
      .catch((err: unknown) => {
        if (active) {
          loadError = errorText(err, 'Не удалось загрузить каталог драйверов.');
        }
      });
    return () => {
      active = false;
    };
  });

  const layout = $derived(layoutVendors(hardware));
  const visible = $derived(filterByVendor(items, selected));
  const heading = $derived(selected === null ? 'Все драйверы' : VENDORS[selected].label);

  function errorText(err: unknown, fallback: string): string {
    if (typeof err === 'object' && err !== null && 'message' in err) {
      const message = (err as { message: unknown }).message;
      if (typeof message === 'string' && message.length > 0) {
        return message;
      }
    }
    return fallback;
  }

  function stateOf(id: string): ButtonState {
    return states[id] ?? IDLE;
  }

  function setState(id: string, state: ButtonState) {
    states = { ...states, [id]: state };
  }

  function anyBusy(): boolean {
    return Object.values(states).some(isBusy);
  }

  async function run(item: DriverItem) {
    if (isBusy(stateOf(item.id)) || anyBusy()) {
      return;
    }
    try {
      if (item.action === 'install') {
        setState(item.id, { kind: 'downloading', percent: 0 });
        await installDriver(item.id, (event) => {
          if (event.id === item.id) {
            setState(item.id, stateFromProgress(event));
          }
        });
        setState(item.id, { kind: 'done' });
      } else {
        await openDriverPage(item.id);
        setState(item.id, { kind: 'done' });
      }
    } catch (err) {
      setState(item.id, {
        kind: 'error',
        reason: errorText(err, 'Не удалось выполнить действие.'),
      });
    }
  }

  async function viaWindowsUpdate(item: DriverItem) {
    try {
      await openWindowsUpdate();
    } catch (err) {
      setState(item.id, {
        kind: 'error',
        reason: errorText(err, 'Не удалось открыть Windows Update.'),
      });
    }
  }
</script>

<section class="drivers">
  <h2>Драйверы / твоё оборудование</h2>
  <p class="lead">
    Выбери производителя и установи драйвер официальным установщиком. Приложение скачивает файл
    только с сайта производителя, проверяет цифровую подпись и запускает установщик; права
    администратора Windows запросит у него самого. Где прямой загрузки нет, откроется официальный
    сайт.
  </p>

  <ul class="vendors" aria-label="Производители">
    {#each layout.ordered as id (id)}
      <li>
        <button
          type="button"
          class="vendor"
          aria-pressed={selected === id}
          onclick={() => (selected = selected === id ? null : id)}
        >
          <VendorLogo vendor={id} />
          <span>{VENDORS[id].label}</span>
          {#if layout.detected.has(id)}
            <small class="mine">ваше устройство</small>
          {/if}
        </button>
      </li>
    {/each}
  </ul>

  <h2>{heading}</h2>
  {#if loadError}
    <p class="reason" role="alert">{loadError}</p>
  {/if}
  <div class="catalog">
    {#each visible as item (item.id)}
      {@const state = stateOf(item.id)}
      <article class="item">
        <h3>{item.title}</h3>
        <p class="meta">
          <span>{item.maker}</span><span aria-hidden="true"> · </span><span>{item.device}</span>
        </p>
        <p>{item.description}</p>
        {#if compatibilityNote(item, hardware, os)}
          <p class="compat">{compatibilityNote(item, hardware, os)}</p>
        {/if}
        <div class="actions">
          <button
            type="button"
            class="primary"
            class:busy={isBusy(state)}
            disabled={isBusy(state)}
            aria-busy={isBusy(state)}
            style:--fill="{fillPercent(state)}%"
            onclick={() => void run(item)}
          >
            <span class="fill" aria-hidden="true"></span>
            <span class="label">{buttonLabel(item.action, state)}</span>
          </button>
          {#if item.windowsUpdate}
            <button type="button" class="secondary" onclick={() => void viaWindowsUpdate(item)}>
              Установить через Windows Update
            </button>
          {/if}
          {#if state.kind === 'error'}
            <p class="error" role="alert">{state.reason}</p>
          {/if}
        </div>
      </article>
    {/each}
  </div>
</section>

<style>
  h2 {
    margin: 16px 0 8px;
    font-size: var(--text-title);
    text-transform: uppercase;
  }

  .lead {
    margin: 0 0 8px;
    font-size: var(--text-small);
    color: var(--page-text);
  }

  .reason {
    margin: 4px 0;
    font-size: var(--text-small);
    color: var(--page-text);
  }

  /* Плитки производителей: одинаковые квадраты, ровная сетка. */
  .vendors {
    display: grid;
    grid-template-columns: repeat(auto-fill, 136px);
    justify-content: start;
    gap: 8px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .vendor {
    position: relative;
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: flex-start;
    gap: 8px;
    width: 136px;
    aspect-ratio: 1;
    padding: 18px 8px 8px;
    border: 0;
    border-radius: var(--radius-md);
    background: var(--black, #080808);
    color: var(--ink, #fafafa);
    font: inherit;
    font-size: var(--text-caption);
    text-align: center;
    cursor: pointer;
    transition: transform 160ms ease;
  }

  .vendor:hover {
    transform: scale(1.05);
  }

  .vendor:focus-visible {
    outline: 2px solid #fff;
    outline-offset: 2px;
  }

  /* Выбранная плитка: белая рамка внутри, как у обоев и курсоров. */
  .vendor[aria-pressed='true']::after {
    content: '';
    position: absolute;
    inset: 0;
    border: 3px solid #fff;
    border-radius: inherit;
    pointer-events: none;
  }

  .mine {
    margin-top: -4px;
    font-size: 11px;
    color: #b8b8b8;
  }

  .catalog {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));
    gap: 8px;
    margin-bottom: 16px;
  }

  .item {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 16px;
    border-radius: var(--radius-md);
    background: var(--black, #080808);
    color: var(--ink, #fafafa);
  }

  .item h3 {
    margin: 0;
    font-size: 16px;
  }

  .item p {
    margin: 0;
    font-size: var(--text-small);
    color: #d4d4d4;
  }

  .item .meta {
    font-size: var(--text-caption);
    font-weight: 700;
    text-transform: uppercase;
    color: var(--ink-soft, #d4d4d4);
  }

  .item .compat {
    font-size: var(--text-caption);
    color: #b8b8b8;
  }

  .actions {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin-top: auto;
    padding-top: 8px;
  }

  .actions button {
    position: relative;
    width: 100%;
    min-height: 44px;
    padding: 10px 16px;
    border: 0;
    border-radius: var(--radius-sm);
    background: var(--surface);
    color: var(--ink);
    font: inherit;
    font-weight: 800;
    text-align: center;
    cursor: pointer;
    overflow: hidden;
  }

  .actions .primary {
    background: var(--btn-bg);
    box-shadow: inset 0 0 0 1px var(--btn-border);
  }

  .actions .secondary {
    font-weight: 600;
    font-size: var(--text-small);
    background: transparent;
    box-shadow: inset 0 0 0 1px #444;
  }

  .actions button:disabled {
    opacity: 0.85;
    cursor: progress;
  }

  .fill {
    position: absolute;
    inset: 0 auto 0 0;
    width: var(--fill, 0%);
    background: var(--btn-border);
    opacity: 0.55;
    transition: width 160ms linear;
  }

  .label {
    position: relative;
  }

  .actions .error {
    color: #ffb4b4;
    background: #2a0f0f;
    padding: 8px 10px;
    border-radius: var(--radius-sm);
    overflow-wrap: anywhere;
  }

  @media (prefers-reduced-motion: reduce) {
    .vendor,
    .fill {
      transition: none;
    }

    .vendor:hover {
      transform: none;
    }
  }
</style>
