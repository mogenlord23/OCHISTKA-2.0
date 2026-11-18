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
    { id: 'cpu', title: 'Процессор (CPU)', color: SECTION_COLORS.viruses },
    { id: 'gpu', title: 'Видеокарта (GPU)', color: SECTION_COLORS.vpn },
    { id: 'ram', title: 'Оперативная память', color: SECTION_COLORS.processes },
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
  let selected = $state<TileId>('cpu');

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
  <div class="tiles" role="group" aria-label="Комплектующие">
    {#each TILES as tile (tile.id)}
      <button
        type="button"
        class="tile"
        class:selected={selected === tile.id}
        aria-pressed={selected === tile.id}
        onclick={() => (selected = tile.id)}
      >
        <span class="swatch" style:background={tile.color} aria-hidden="true"></span>
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
              {#if gpu}{gpuUsageLine(gpu) ?? 'Загрузка не отдаётся ОС'}{:else}Видеокарты не найдены.{/if}
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
            <span class="tile-sub">{hardware.motherboard.vendor ?? 'Производитель не указан'}</span>
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
      </button>
    {/each}
  </div>

  <section class="card detail" aria-label={title(selected)}>
    {#if selected === 'cpu'}
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
    {:else if selected === 'ram'}
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
    {:else if selected === 'gpu'}
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
    {:else if selected === 'board'}
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
    {:else if selected === 'disks'}
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
    {:else if selected === 'system'}
      <h2>Система</h2>
      <p>Название: {overview.osName || '—'}</p>
      <p>Версия: {overview.osVersion || '—'}</p>
    {/if}
  </section>
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

  .tiles {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
    gap: 12px;
    margin-bottom: 16px;
  }

  .tile {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 4px;
    min-height: 44px;
    padding: 16px;
    border: 1px solid #343434;
    border-radius: var(--radius-sm);
    background: #161b22;
    color: var(--ink, #fafafa);
    font: inherit;
    text-align: left;
    cursor: pointer;
  }

  .tile.selected {
    border-color: var(--ink, #fafafa);
  }

  .swatch {
    width: 16px;
    height: 16px;
    border-radius: 3px;
  }

  .tile-title {
    font-weight: 800;
    text-transform: uppercase;
    font-size: var(--text-body);
  }

  .tile-value {
    font-size: 24px;
    font-weight: 800;
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
    border-radius: var(--radius-sm);
    background: #161b22;
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
