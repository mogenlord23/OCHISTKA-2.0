<script lang="ts">
  import { getHardware } from '../lib/api';
  import { UNSUPPORTED_OS_REASON } from '../lib/computer';
  import { CATALOG, detectVendors, VENDORS } from '../lib/driversCatalog';
  import type { HardwareOverview } from '../lib/types';

  type OsId = 'windows' | 'macos' | 'linux';

  /** Включена только Windows; остальные ОС выключены с причиной. */
  const OS_OPTIONS: readonly { id: OsId; label: string; reason: string | null }[] = [
    { id: 'windows', label: 'Windows', reason: null },
    { id: 'macos', label: 'macOS', reason: UNSUPPORTED_OS_REASON },
    { id: 'linux', label: 'Linux', reason: UNSUPPORTED_OS_REASON },
  ];

  let selected = $state<OsId>('windows');
  let hardware = $state<HardwareOverview | null>(null);
  let copiedId = $state<string | null>(null);

  $effect(() => {
    let active = true;
    getHardware()
      .then((value) => {
        if (active) {
          hardware = value;
        }
      })
      .catch(() => {
        // Без сведений о железе показываем основных производителей.
      });
    return () => {
      active = false;
    };
  });

  const vendors = $derived(detectVendors(hardware));

  async function copy(id: string, url: string) {
    try {
      await navigator.clipboard.writeText(url);
      copiedId = id;
      setTimeout(() => {
        if (copiedId === id) {
          copiedId = null;
        }
      }, 1800);
    } catch {
      copiedId = null;
    }
  }
</script>

<section class="drivers">
  <div class="os-switch" role="group" aria-label="Операционная система">
    {#each OS_OPTIONS as option (option.id)}
      <button
        type="button"
        aria-pressed={selected === option.id}
        disabled={option.reason !== null}
        aria-describedby={option.reason ? `os-reason-${option.id}` : undefined}
        onclick={() => (selected = option.id)}
      >
        {option.label}
      </button>
    {/each}
  </div>

  {#each OS_OPTIONS as option (option.id)}
    {#if option.reason}
      <p class="reason" id="os-reason-{option.id}">{option.label}: {option.reason}</p>
    {/if}
  {/each}

  {#if selected === 'windows'}
    <h2>Драйверы / твоё оборудование</h2>
    <p class="lead">
      Windows Update подбирает совместимые драйверы по идентификаторам устройств. Универсального
      драйвера для всех компьютеров нет. Приложение ничего не скачивает и не устанавливает: открой
      официальную страницу сам.
    </p>
    <ul class="hardware" aria-label="Твоё оборудование">
      {#each vendors as id (id)}
        {@const vendor = VENDORS[id]}
        <li class="hw">
          <svg width="58" height="58" viewBox="0 0 58 58" aria-hidden="true" focusable="false">
            <rect width="58" height="58" rx="10" fill={vendor.color} />
            <text
              x="29"
              y="36"
              text-anchor="middle"
              fill="#fff"
              font-family="Arial Black, Segoe UI, sans-serif"
              font-weight="900"
              font-size={vendor.mark.length > 3 ? 11 : vendor.id === 'bluetooth' ? 30 : 17}
              >{vendor.mark}</text
            >
          </svg>
          <span>{vendor.label}</span>
        </li>
      {/each}
    </ul>

    <h2>Готовый каталог</h2>
    <div class="catalog">
      {#each CATALOG as item (item.id)}
        <article class="item">
          <h3>{item.title}</h3>
          <p>{item.description}</p>
          <p class="url">{item.url}</p>
          <button type="button" onclick={() => void copy(item.id, item.url)}>
            {copiedId === item.id ? 'Ссылка скопирована' : 'Копировать ссылку'}
          </button>
        </article>
      {/each}
    </div>
  {/if}
</section>

<style>
  .os-switch {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin-bottom: 8px;
  }

  .os-switch button {
    min-height: 44px;
    padding: 10px 16px;
    border: 0;
    border-radius: var(--radius-sm);
    background: var(--black);
    color: var(--ink);
    font: inherit;
    font-weight: 800;
    text-transform: uppercase;
    font-size: var(--text-body);
    cursor: pointer;
  }

  .os-switch button[aria-pressed='true'] {
    background: var(--btn-bg);
    color: var(--ink);
    box-shadow: inset 0 0 0 1px var(--btn-border);
  }

  .os-switch button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .reason {
    margin: 4px 0;
    font-size: var(--text-small);
    color: var(--page-text);
  }

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

  .hardware {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .hw {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    width: 150px;
    padding: 14px 8px;
    border-radius: var(--radius-md);
    background: var(--black, #080808);
    color: var(--ink, #fafafa);
    font-size: var(--text-caption);
    text-align: center;
    transition: transform 160ms ease;
  }

  .hw:hover {
    transform: translateY(-2px);
  }

  .catalog {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(230px, 1fr));
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

  .item .url {
    margin-top: auto;
    font-size: 12px;
    color: #00e5ff;
    overflow-wrap: anywhere;
  }

  .item button {
    align-self: flex-start;
    min-height: 44px;
    padding: 8px 14px;
    border: 1px solid #444;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--ink, #fafafa);
    font: inherit;
    font-size: var(--text-small);
    cursor: pointer;
    transition: background-color 160ms ease;
  }

  .item button:hover {
    background: #222;
  }

  @media (prefers-reduced-motion: reduce) {
    .hw,
    .item button {
      transition: none;
    }

    .hw:hover {
      transform: none;
    }
  }
</style>
