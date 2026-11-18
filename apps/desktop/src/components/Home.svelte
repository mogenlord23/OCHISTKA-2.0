<script lang="ts">
  import { getHardware, getSystemOverview, POLL_INTERVAL_MS, toCommandError } from '../lib/api';
  import {
    boardName,
    gpuMemoryLine,
    gpuName,
    gpuUsageLine,
    HARDWARE_LOADING,
  } from '../lib/computer';
  import { formatBytes, formatPercent } from '../lib/format';
  import { HISTORY_LIMIT, pushSample } from '../lib/history';
  import { SECTION_COLORS } from '../lib/tokens';
  import type { CommandError, DiskKind, HardwareOverview, SystemOverview } from '../lib/types';
  import LineGraph from './LineGraph.svelte';

  /** Через сколько запрос считается зависшим. */
  const REQUEST_TIMEOUT_MS = 10_000;

  type TileId = 'cpu' | 'gpu' | 'ram' | 'board' | 'disks' | 'system';

  /** Цвета плиток — из палитры токенов: lime, pink, blue, yellow. */
  const TILES: readonly { id: TileId; title: string; color: string }[] = [
    { id: 'cpu', title: 'CPU / Процессор', color: SECTION_COLORS.viruses },
    { id: 'gpu', title: 'GPU / Видеокарта', color: SECTION_COLORS.vpn },
    { id: 'ram', title: 'RAM / Оперативная память', color: SECTION_COLORS.processes },
    { id: 'board', title: 'Материнская плата', color: SECTION_COLORS.disks },
    { id: 'disks', title: 'SSD / HDD', color: SECTION_COLORS.viruses },
    { id: 'system', title: 'Система', color: SECTION_COLORS.vpn },
  ];

  const KIND_LABELS: Record<DiskKind, string> = {
    ssd: 'SSD',
    hdd: 'HDD',
    unknown: 'тип не определён',
  };

  let overview = $state<SystemOverview | null>(null);
  let error = $state<CommandError | null>(null);
  let hardware = $state<HardwareOverview | null>(null);
  let hardwareError = $state<CommandError | null>(null);
  let cpuHistory = $state<number[]>([]);
  let ramHistory = $state<number[]>([]);
  /** Открытая плитка: карточка с графиком появляется только после нажатия. */
  let selected = $state<TileId | null>(null);
  /** Последняя открытая плитка: её содержимое остаётся видимым, пока карточка закрывается. */
  let shownId = $state<TileId>('cpu');

  function toggle(id: TileId) {
    if (selected === id) {
      selected = null;
    } else {
      selected = id;
      shownId = id;
    }
  }

  $effect(() => {
    // Ответ, пришедший после размонтирования, игнорируется.
    let active = true;
    // Пока предыдущий запрос не завершился, новый не запускается: зависший опрос не накапливается.
    let pending = false;

    const load = async () => {
      if (pending) {
        return;
      }
      pending = true;
      let timeoutId: ReturnType<typeof setTimeout> | undefined;
      try {
        // Запрос, который не завершился за REQUEST_TIMEOUT_MS, превращается в ошибку,
        // чтобы `pending` сбросился и опрос продолжился. Бэкенд отменить нельзя:
        // отменённый вызов остаётся в очереди на мьютекс, его длительность ограничена
        // узкой блокировкой на стороне Rust.
        const timeout = new Promise<never>((_, reject) => {
          timeoutId = setTimeout(
            () =>
              reject({
                kind: 'probe',
                message: 'Превышено время ожидания ответа (10 с). Повторим попытку автоматически.',
              } satisfies CommandError),
            REQUEST_TIMEOUT_MS,
          );
        });
        const [next, hw] = await Promise.race([
          Promise.all([getSystemOverview(), readHardware()]),
          timeout,
        ]);
        if (active) {
          overview = next;
          error = null;
          hardware = hw.data;
          hardwareError = hw.error;
          // История за 90 секунд: по одному замеру CPU и RAM на каждый опрос.
          cpuHistory = pushSample(cpuHistory, next.cpu.usagePercent, HISTORY_LIMIT);
          ramHistory = pushSample(ramHistory, next.memory.usagePercent, HISTORY_LIMIT);
        }
      } catch (err) {
        if (active) {
          error = toCommandError(err);
          hardware = null;
          hardwareError = null;
        }
      } finally {
        clearTimeout(timeoutId);
        pending = false;
      }
    };

    load();
    const timer = setInterval(load, POLL_INTERVAL_MS);
    return () => {
      active = false;
      clearInterval(timer);
    };
  });

  /** Комплектующие читаются вместе с опросом; их ошибка не скрывает загрузку и диски. */
  async function readHardware(): Promise<{
    data: HardwareOverview | null;
    error: CommandError | null;
  }> {
    try {
      return { data: await getHardware(), error: null };
    } catch (err) {
      return { data: null, error: toCommandError(err) };
    }
  }

  function title(id: TileId): string {
    return TILES.find((tile) => tile.id === id)?.title ?? '';
  }
</script>

{#if error}
  <section class="alert" role="alert">
    <h2>
      {error.kind === 'unsupported'
        ? 'Недоступно на этой ОС'
        : 'Не удалось прочитать системные данные'}
    </h2>
    <p>{error.message}</p>
  </section>
{:else if overview}
  <div class="reveal" class:open={selected !== null} inert={selected === null}>
    <div class="reveal-inner">
      <section class="card detail" aria-label={title(shownId)}>
        <button
          type="button"
          class="close"
          aria-label="Закрыть сведения"
          onclick={() => (selected = null)}>×</button
        >
        {#if shownId === 'cpu'}
          <h2>Процессор</h2>
          <p>Загрузка: {formatPercent(overview.cpu.usagePercent)}</p>
          <p>Логических ядер: {overview.cpu.logicalCores}</p>
          <div
            class="bar"
            role="progressbar"
            aria-label="Загрузка процессора"
            aria-valuemin="0"
            aria-valuemax="100"
            aria-valuenow={Math.round(overview.cpu.usagePercent)}
          >
            <div class="fill" style:width="{overview.cpu.usagePercent}%"></div>
          </div>
          <LineGraph
            values={cpuHistory}
            label="График загрузки процессора за 90 секунд"
            color={SECTION_COLORS.computer}
          />
        {:else if shownId === 'ram'}
          <h2>Оперативная память</h2>
          <p>
            {formatBytes(overview.memory.usedBytes)} из {formatBytes(overview.memory.totalBytes)}
          </p>
          <p>{formatPercent(overview.memory.usagePercent)}</p>
          <div
            class="bar"
            role="progressbar"
            aria-label="Использование памяти"
            aria-valuemin="0"
            aria-valuemax="100"
            aria-valuenow={Math.round(overview.memory.usagePercent)}
          >
            <div class="fill" style:width="{overview.memory.usagePercent}%"></div>
          </div>
          <LineGraph
            values={ramHistory}
            label="График использования памяти за 90 секунд"
            color={SECTION_COLORS.computer}
          />
        {:else if shownId === 'gpu'}
          <h2>Видеокарта (GPU)</h2>
          {#if hardwareError}
            <p>{hardwareError.message}</p>
          {:else if hardware}
            {#each hardware.gpus as gpu, index (index)}
              <div class="disk">
                <p>{gpuName(gpu)}{gpu.vendor && gpu.model ? ` · ${gpu.vendor}` : ''}</p>
                {#if gpuUsageLine(gpu)}<p>{gpuUsageLine(gpu)}</p>{/if}
                {#if gpuMemoryLine(gpu)}<p>{gpuMemoryLine(gpu)}</p>{/if}
              </div>
            {:else}
              <p>Видеокарты не найдены.</p>
            {/each}
          {:else}
            <p>{HARDWARE_LOADING}</p>
          {/if}
        {:else if shownId === 'board'}
          <h2>Материнская плата</h2>
          {#if hardwareError}
            <p>{hardwareError.message}</p>
          {:else if hardware}
            <p>Производитель: {hardware.motherboard.vendor ?? '—'}</p>
            <p>Модель: {hardware.motherboard.name ?? '—'}</p>
            <p>Версия: {hardware.motherboard.version ?? '—'}</p>
          {:else}
            <p>{HARDWARE_LOADING}</p>
          {/if}
        {:else if shownId === 'disks'}
          <h2>SSD / HDD</h2>
          {#each overview.disks as disk (disk.mountPoint)}
            <div class="disk">
              <p>
                <span>{disk.displayName}</span>
                <span class="muted">{KIND_LABELS[disk.kind]}</span>
              </p>
              <p>
                {formatBytes(disk.usedBytes)} из {formatBytes(disk.totalBytes)} · {formatPercent(
                  disk.usagePercent,
                )}
              </p>
              <div
                class="bar"
                role="progressbar"
                aria-label={`Заполнение диска ${disk.displayName}`}
                aria-valuemin="0"
                aria-valuemax="100"
                aria-valuenow={Math.round(disk.usagePercent)}
              >
                <div class="fill" style:width="{disk.usagePercent}%"></div>
              </div>
            </div>
          {:else}
            <p>Диски не найдены.</p>
          {/each}
        {:else if shownId === 'system'}
          <h2>Система</h2>
          <p>Название: {overview.osName || '—'}</p>
          <p>Версия: {overview.osVersion || '—'}</p>
        {/if}
      </section>
    </div>
  </div>
  <p class="hint">Нажми на комплектующую, чтобы открыть сведения и график</p>
  <div class="tiles" role="group" aria-label="Комплектующие">
    {#each TILES as tile (tile.id)}
      <button
        type="button"
        class="tile"
        class:selected={selected === tile.id}
        aria-pressed={selected === tile.id}
        onclick={() => toggle(tile.id)}
      >
        <span class="icon" style:background={tile.color} aria-hidden="true"
          ><span class="glyph glyph-{tile.id}"></span></span
        >
        <span class="tile-body">
          <span class="tile-title">{tile.title}</span>

          {#if tile.id === 'cpu'}
            <span class="tile-value">{formatPercent(overview.cpu.usagePercent)}</span>
            <span class="tile-sub">{overview.cpu.logicalCores} логических ядер</span>
          {:else if tile.id === 'ram'}
            <span class="tile-value">{formatPercent(overview.memory.usagePercent)}</span>
            <span class="tile-sub"
              >{formatBytes(overview.memory.usedBytes)} из {formatBytes(
                overview.memory.totalBytes,
              )}</span
            >
          {:else if tile.id === 'gpu'}
            {#if hardwareError}
              <span class="tile-value">—</span>
              <span class="tile-sub">{hardwareError.message}</span>
            {:else if hardware}
              {@const gpu = hardware.gpus[0]}
              <span class="tile-value">{gpu ? gpuName(gpu) : '—'}</span>
              <span class="tile-sub">
                {#if gpu}{gpuUsageLine(gpu) ?? 'Загрузка не отдаётся ОС'}{:else}Видеокарты не
                  найдены.{/if}
              </span>
            {:else}
              <span class="tile-value">—</span>
              <span class="tile-sub">{HARDWARE_LOADING}</span>
            {/if}
          {:else if tile.id === 'board'}
            {#if hardwareError}
              <span class="tile-value">—</span>
              <span class="tile-sub">{hardwareError.message}</span>
            {:else if hardware}
              <span class="tile-value">{boardName(hardware.motherboard)}</span>
              <span class="tile-sub"
                >{hardware.motherboard.vendor ?? 'Производитель не указан'}</span
              >
            {:else}
              <span class="tile-value">—</span>
              <span class="tile-sub">{HARDWARE_LOADING}</span>
            {/if}
          {:else if tile.id === 'disks'}
            <span class="tile-value">{overview.disks.length} шт.</span>
            {#each overview.disks as disk (disk.mountPoint)}
              <span class="tile-sub disk-row">
                <span>{disk.displayName}</span>
                {#if disk.displayName !== disk.mountPoint}
                  <span class="muted">{disk.mountPoint}</span>
                {/if}
                <span class="muted">{KIND_LABELS[disk.kind]}</span>
              </span>
            {:else}
              <span class="tile-sub">Диски не найдены.</span>
            {/each}
          {:else if tile.id === 'system'}
            <span class="tile-value">{overview.osName || '—'}</span>
            <span class="tile-sub">Версия: {overview.osVersion || '—'}</span>
          {/if}
        </span>
      </button>
    {/each}
  </div>
{:else}
  <p>Загрузка…</p>
{/if}

<style>
  h2 {
    margin: 0 0 8px;
    font-size: var(--text-title);
  }

  p {
    margin: 4px 0;
  }

  .hint {
    margin: 0 0 10px;
    font-size: var(--text-caption);
    font-weight: 800;
    text-transform: uppercase;
    color: var(--page-text);
  }

  .tiles {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 12px;
    margin-bottom: 16px;
  }

  .tile {
    display: flex;
    align-items: center;
    gap: 14px;
    min-height: 44px;
    padding: 20px 18px;
    border: 1px solid transparent;
    border-radius: var(--radius-md);
    background: var(--black, #080808);
    color: var(--ink, #fafafa);
    font: inherit;
    text-align: left;
    cursor: pointer;
  }

  .tile {
    transition:
      transform 160ms ease,
      border-color 160ms ease;
  }

  .tile:hover {
    transform: translateY(-2px);
  }

  .tile.selected {
    border-color: var(--ink, #fafafa);
  }

  @media (prefers-reduced-motion: reduce) {
    .tile:hover {
      transform: none;
    }
  }

  .icon {
    display: flex;
    flex: none;
    align-items: center;
    justify-content: center;
    width: 88px;
    height: 88px;
    border-radius: 12px;
  }

  .glyph {
    display: block;
    background: #080808;
  }

  .glyph-cpu {
    width: 48px;
    height: 48px;
    border-radius: 4px;
    box-shadow: inset 0 0 0 12px #080808;
    background: #f5f5f0;
    outline: 4px dashed #080808;
    outline-offset: 3px;
  }

  .glyph-gpu {
    width: 64px;
    height: 36px;
    border-radius: 4px;
    border-bottom: 5px solid #f5ff00;
  }

  .glyph-ram {
    width: 66px;
    height: 46px;
    background:
      repeating-linear-gradient(90deg, #b4ff00 0 9px, transparent 9px 15px) 8px 8px / 52px 10px
        no-repeat,
      repeating-linear-gradient(90deg, #b4ff00 0 9px, transparent 9px 15px) 8px 28px / 52px 10px
        no-repeat,
      #080808;
    border-radius: 4px;
  }

  .glyph-board {
    width: 56px;
    height: 60px;
    background:
      linear-gradient(#f5ff00, #f5ff00) 10px 10px / 20px 20px no-repeat,
      #080808;
    border-radius: 3px;
  }

  .glyph-disks {
    width: 46px;
    height: 62px;
    border-radius: 4px;
    background:
      linear-gradient(#f5f5f0, #f5f5f0) 9px 10px / 28px 18px no-repeat,
      radial-gradient(circle at 12px 50px, #ff009d 3px, transparent 4px),
      #080808;
  }

  .glyph-system {
    width: 62px;
    height: 44px;
    border: 5px solid #080808;
    background: #147cff;
    border-radius: 2px;
  }

  .tile-body {
    display: flex;
    flex-direction: column;
    gap: 6px;
    min-width: 0;
  }

  .tile-title {
    font-weight: 700;
    text-transform: uppercase;
    font-size: var(--text-caption);
    color: var(--ink-soft, #d4d4d4);
  }

  .tile-value {
    font-size: 17px;
    font-weight: 800;
    overflow-wrap: anywhere;
  }

  .tile-sub {
    font-size: 12px;
    color: #8b949e;
  }

  .disk-row {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .card,
  .alert {
    margin-bottom: 16px;
    padding: 16px;
    border-radius: var(--radius-md);
    background: var(--black, #080808);
    color: var(--ink, #fafafa);
  }

  .card {
    position: relative;
    padding: 18px 18px 14px;
  }

  .close {
    position: absolute;
    top: 8px;
    right: 8px;
    width: 44px;
    height: 44px;
    border: 0;
    border-radius: 50%;
    background: transparent;
    color: var(--ink, #fafafa);
    font: inherit;
    font-size: 26px;
    line-height: 1;
    cursor: pointer;
  }

  .close:hover {
    background: #222;
  }

  /* Открытие карточки: строка сетки 0fr -> 1fr, прозрачность и сдвиг. */
  .reveal {
    display: grid;
    grid-template-rows: 0fr;
    opacity: 0;
    transform: translateY(-8px);
    transition:
      grid-template-rows 260ms ease,
      opacity 200ms ease,
      transform 260ms ease;
  }

  .reveal.open {
    grid-template-rows: 1fr;
    opacity: 1;
    transform: none;
  }

  .reveal-inner {
    min-height: 0;
    overflow: hidden;
  }

  @media (prefers-reduced-motion: reduce) {
    .reveal,
    .tile {
      transition: none;
    }
  }

  :global(:root[data-reduce-motion='true']) .reveal {
    transition: none;
  }

  .card h2 {
    font-size: 19px;
    text-transform: uppercase;
  }

  .card p {
    font-size: var(--text-small);
  }

  .card .bar {
    height: 4px;
  }

  .alert {
    border: 1px solid #da3633;
  }

  .muted {
    color: #8b949e;
  }

  .bar {
    height: 8px;
    margin-top: 8px;
    border-radius: 4px;
    background: #21262d;
    overflow: hidden;
  }

  .fill {
    height: 100%;
    background: var(--accent);
  }

  .disk + .disk {
    margin-top: 12px;
  }
</style>
