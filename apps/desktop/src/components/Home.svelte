<script lang="ts">
  import { getHardware, getSystemOverview, POLL_INTERVAL_MS, toCommandError } from '../lib/api';
  import {
    boardLine,
    clockTime,
    cpuName,
    detailPercent,
    drivesLine,
    gpuFullName,
    gpuMemoryLine,
    HARDWARE_LOADING,
    ramTotal,
    shortPercent,
    systemLine,
  } from '../lib/computer';
  import { loadSnapshot, saveSnapshot } from '../lib/computerCache';
  import { formatBytes } from '../lib/format';
  import { HISTORY_LIMIT, pushSample } from '../lib/history';
  import { SECTION_COLORS } from '../lib/tokens';
  import type { CommandError, DiskKind, HardwareOverview, SystemOverview } from '../lib/types';
  import ComputerIcon from './ComputerIcon.svelte';
  import LineGraph from './LineGraph.svelte';

  /** Через сколько запрос считается зависшим. */
  const REQUEST_TIMEOUT_MS = 10_000;

  type TileId = 'cpu' | 'gpu' | 'ram' | 'board' | 'disks' | 'system';
  /** Что раскрыто над плитками: сводка с тремя графиками, одна комплектующая или ничего. */
  type View = TileId | 'summary' | null;

  const TILES: readonly { id: TileId; title: string; detail: string }[] = [
    { id: 'cpu', title: 'CPU / Процессор', detail: 'Процессор' },
    { id: 'gpu', title: 'GPU / Видеокарта', detail: 'Видеокарта' },
    { id: 'ram', title: 'RAM / Память', detail: 'Оперативная память' },
    { id: 'board', title: 'Материнская плата', detail: 'Материнская плата' },
    { id: 'disks', title: 'SSD / HDD', detail: 'Диски' },
    { id: 'system', title: 'Система', detail: 'Система' },
  ];

  const KIND_LABELS: Record<DiskKind, string> = {
    ssd: 'SSD',
    hdd: 'HDD',
    unknown: 'тип не определён',
  };

  const GRAPH_COLOR = SECTION_COLORS.viruses;

  /** Структура видна сразу: пока данных нет, берутся последние значения из кэша или прочерки. */
  const cached = loadSnapshot();
  const EMPTY_OVERVIEW: SystemOverview = {
    cpu: { usagePercent: 0, logicalCores: 0 },
    memory: { totalBytes: 0, usedBytes: 0, usagePercent: 0 },
    disks: [],
    osName: '',
    osVersion: '',
  };

  let overview = $state<SystemOverview | null>(cached?.overview ?? null);
  let error = $state<CommandError | null>(null);
  let hardware = $state<HardwareOverview | null>(cached?.hardware ?? null);
  let hardwareError = $state<CommandError | null>(null);
  let cpuHistory = $state<number[]>(cached?.cpuHistory ?? []);
  let gpuHistory = $state<number[]>(cached?.gpuHistory ?? []);
  let ramHistory = $state<number[]>(cached?.ramHistory ?? []);
  let updatedAt = $state<Date | null>(cached?.updatedAt ? new Date(cached.updatedAt) : null);
  /** Пока нет ни одного ответа, значения плиток показываются прочерками. */
  const ov = $derived(overview ?? EMPTY_OVERVIEW);
  const ready = $derived(overview !== null);
  let view = $state<View>(null);
  /** Последнее раскрытое: содержимое остаётся видимым, пока карточка закрывается. */
  let shownView = $state<Exclude<View, null>>('summary');
  /** Запуск внеочередного опроса по кнопке «Обновить сведения». */
  let reload: () => void = () => {};

  function open(next: Exclude<View, null>) {
    if (view === next) {
      view = null;
    } else {
      view = next;
      shownView = next;
    }
  }

  $effect(() => {
    // Ответ, пришедший после размонтирования, игнорируется.
    let active = true;
    // Пока предыдущий запрос не завершился, новый не запускается: зависший опрос не накапливается.
    let pending = false;

    // Системные сведения и комплектующие опрашиваются независимо: медленный ответ
    // комплектующих не задерживает загрузку, память и диски.
    let hardwarePending = false;
    const loadHardware = async () => {
      if (hardwarePending) {
        return;
      }
      hardwarePending = true;
      try {
        const hw = await readHardware();
        if (active) {
          hardware = hw.data ?? hardware;
          hardwareError = hw.error;
          const gpuUsage = hw.data?.gpus[0]?.usagePercent;
          if (gpuUsage !== null && gpuUsage !== undefined) {
            gpuHistory = pushSample(gpuHistory, gpuUsage, HISTORY_LIMIT);
          }
          persist();
        }
      } finally {
        hardwarePending = false;
      }
    };

    const persist = () => {
      saveSnapshot({
        overview,
        hardware,
        cpuHistory,
        gpuHistory,
        ramHistory,
        updatedAt: updatedAt?.getTime() ?? null,
      });
    };

    const load = async () => {
      void loadHardware();
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
        const next = await Promise.race([getSystemOverview(), timeout]);
        if (active) {
          overview = next;
          error = null;
          updatedAt = new Date();
          // История за 90 секунд: по одному замеру на каждый опрос.
          cpuHistory = pushSample(cpuHistory, next.cpu.usagePercent, HISTORY_LIMIT);
          ramHistory = pushSample(ramHistory, next.memory.usagePercent, HISTORY_LIMIT);
          persist();
        }
      } catch (err) {
        if (active) {
          error = toCommandError(err);
        }
      } finally {
        clearTimeout(timeoutId);
        pending = false;
      }
    };

    reload = () => void load();
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

  const gpuNow = $derived(hardware?.gpus[0]?.usagePercent ?? null);

  function detailTitle(id: Exclude<View, null>): string {
    return id === 'summary' ? 'Нагрузка' : (TILES.find((tile) => tile.id === id)?.detail ?? '');
  }

  /** Крупная строка карточки: процент и время замера. */
  function headline(percent: number | null): string {
    const time = updatedAt ? clockTime(updatedAt) : '';
    return percent === null ? `нет данных · ${time}` : `${detailPercent(percent)} · ${time}`;
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
{:else}
  <div class="summary" role="group" aria-label="Текущая загрузка">
    <button
      type="button"
      class="summary-values"
      aria-pressed={view === 'summary'}
      aria-label="Показать графики загрузки"
      onclick={() => open('summary')}
    >
      <span>CPU {ready ? shortPercent(ov.cpu.usagePercent) : '—'}</span>
      <span>GPU {gpuNow === null ? '—' : shortPercent(gpuNow)}</span>
      <span>RAM {ready ? shortPercent(ov.memory.usagePercent) : '—'}</span>
    </button>
    <button type="button" class="refresh" onclick={() => reload()}>Обновить сведения</button>
  </div>

  <div class="reveal" class:open={view !== null} inert={view === null}>
    <div class="reveal-inner">
      {#if shownView === 'summary'}
        <section class="card graphs" aria-label="Нагрузка">
          <div class="graph-col">
            <h2>CPU</h2>
            <LineGraph
              values={cpuHistory}
              label="График загрузки процессора за 90 секунд"
              color={GRAPH_COLOR}
              height={140}
            />
            <p class="note">
              Общая загрузка процессора. Нажми плитку процессора ниже, чтобы увидеть модель и число
              потоков.
            </p>
          </div>
          <div class="graph-col">
            <h2>GPU</h2>
            <LineGraph
              values={gpuHistory}
              label="График загрузки видеокарты за 90 секунд"
              color={GRAPH_COLOR}
              height={140}
            />
            <p class="note">
              Самый занятый графический движок Windows. Модель и память — в плитке видеокарты.
            </p>
          </div>
          <div class="graph-col">
            <h2>RAM</h2>
            <LineGraph
              values={ramHistory}
              label="График использования памяти за 90 секунд"
              color={GRAPH_COLOR}
              height={140}
            />
            <p class="note">Доля занятой оперативной памяти. Объём — в плитке памяти.</p>
          </div>
          <button type="button" class="hide" onclick={() => (view = null)}>Скрыть</button>
        </section>
      {:else}
        <section class="card detail" aria-label={detailTitle(shownView)}>
          <button type="button" class="hide" onclick={() => (view = null)}>Скрыть</button>
          <h2>{detailTitle(shownView)}</h2>
          {#if shownView === 'cpu'}
            <p class="headline">{headline(ov.cpu.usagePercent)}</p>
            <LineGraph
              values={cpuHistory}
              label="График загрузки процессора за 90 секунд"
              color={GRAPH_COLOR}
            />
            <p>Модель: {cpuName(hardware)}</p>
            <p>Потоков: {ov.cpu.logicalCores}</p>
          {:else if shownView === 'ram'}
            <p class="headline">{headline(ov.memory.usagePercent)}</p>
            <LineGraph
              values={ramHistory}
              label="График использования памяти за 90 секунд"
              color={GRAPH_COLOR}
            />
            <p>
              Занято {formatBytes(ov.memory.usedBytes)} из {formatBytes(ov.memory.totalBytes)}
            </p>
          {:else if shownView === 'gpu'}
            <p class="headline">{headline(gpuNow)}</p>
            {#if hardware && gpuNow === null}
              <p class="note">
                Загрузка недоступна: Windows не отдала счётчики видеокарты (нужен драйвер WDDM 2.0 и
                новее).
              </p>
            {/if}
            <LineGraph
              values={gpuHistory}
              label="График загрузки видеокарты за 90 секунд"
              color={GRAPH_COLOR}
            />
            {#if hardwareError}
              <p>{hardwareError.message}</p>
            {:else if hardware}
              {#each hardware.gpus as gpu, index (index)}
                <p>Модель: {gpuFullName(gpu)}</p>
                {#if gpuMemoryLine(gpu)}<p>Память: {gpuMemoryLine(gpu)}</p>{/if}
              {:else}
                <p>Видеокарты не найдены.</p>
              {/each}
              <p class="note">
                Нагрузка показывает самый занятый движок видеокарты по счётчикам Windows.
                Температура и частота без программы производителя недоступны.
              </p>
            {:else}
              <p>{HARDWARE_LOADING}</p>
            {/if}
          {:else if shownView === 'board'}
            {#if hardwareError}
              <p>{hardwareError.message}</p>
            {:else if hardware}
              <p>Производитель: {hardware.motherboard.vendor ?? '—'}</p>
              <p>Модель: {hardware.motherboard.name ?? '—'}</p>
              <p>Версия: {hardware.motherboard.version ?? '—'}</p>
            {:else}
              <p>{HARDWARE_LOADING}</p>
            {/if}
          {:else if shownView === 'disks'}
            {#each ov.disks as disk (disk.mountPoint)}
              <div class="disk">
                <p>
                  <span>{disk.displayName}</span>
                  <span class="muted">{KIND_LABELS[disk.kind]}</span>
                </p>
                <p>
                  {formatBytes(disk.usedBytes)} из {formatBytes(disk.totalBytes)} · {shortPercent(
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
          {:else if shownView === 'system'}
            <p>Название: {ov.osName || '—'}</p>
            <p>Версия: {ov.osVersion || '—'}</p>
          {/if}
        </section>
      {/if}
    </div>
  </div>

  <p class="updated">
    Сведения обновлены {updatedAt ? clockTime(updatedAt) : '—'} · CPU/GPU/RAM обновляются каждую секунду
  </p>
  <p class="hint">Нажми на комплектующую, чтобы открыть сведения и график</p>
  <div class="tiles" role="group" aria-label="Комплектующие">
    {#each TILES as tile (tile.id)}
      <button
        type="button"
        class="tile"
        class:selected={view === tile.id}
        aria-pressed={view === tile.id}
        onclick={() => open(tile.id)}
      >
        <span class="icon" aria-hidden="true"><ComputerIcon kind={tile.id} /></span>
        <span class="tile-body">
          <span class="tile-title">{tile.title}</span>
          {#if tile.id === 'cpu'}
            <span class="tile-value">{cpuName(hardware)}</span>
          {:else if tile.id === 'ram'}
            <span class="tile-value">{ready ? ramTotal(ov.memory.totalBytes) : '—'}</span>
          {:else if tile.id === 'gpu'}
            {#if hardwareError}
              <span class="tile-value">—</span>
              <span class="tile-sub">{hardwareError.message}</span>
            {:else if hardware}
              {@const gpu = hardware.gpus[0]}
              <span class="tile-value">{gpu ? gpuFullName(gpu) : 'Видеокарты не найдены'}</span>
            {:else}
              <span class="tile-value">{HARDWARE_LOADING}</span>
            {/if}
          {:else if tile.id === 'board'}
            {#if hardwareError}
              <span class="tile-value">—</span>
              <span class="tile-sub">{hardwareError.message}</span>
            {:else if hardware}
              <span class="tile-value">{boardLine(hardware.motherboard)}</span>
            {:else}
              <span class="tile-value">{HARDWARE_LOADING}</span>
            {/if}
          {:else if tile.id === 'disks'}
            <span class="tile-value"
              >{ready || hardware ? drivesLine(hardware?.driveModels ?? [], ov.disks) : '—'}</span
            >
          {:else if tile.id === 'system'}
            <span class="tile-value">{ready ? systemLine(ov.osName, ov.osVersion) : '—'}</span>
          {/if}
        </span>
      </button>
    {/each}
  </div>
{/if}

<style>
  h2 {
    margin: 0 0 8px;
    font-size: var(--text-title);
  }

  p {
    margin: 4px 0;
  }

  .summary {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    margin-bottom: 8px;
    padding: 8px 16px;
    border-radius: var(--radius-md);
    background: var(--black, #080808);
    color: var(--ink, #fafafa);
  }

  .summary-values {
    display: flex;
    flex-wrap: wrap;
    gap: 8px 28px;
    min-height: 44px;
    padding: 4px 8px;
    border: 1px solid transparent;
    border-radius: var(--radius-sm);
    background: transparent;
    color: inherit;
    font: inherit;
    font-size: 26px;
    font-weight: 800;
    font-variant-numeric: tabular-nums;
    cursor: pointer;
    transition: border-color 160ms ease;
  }

  .summary-values span {
    min-width: 5.6em;
  }

  .summary-values:hover,
  .summary-values[aria-pressed='true'] {
    border-color: var(--ink, #fafafa);
  }

  .refresh,
  .hide {
    min-height: 44px;
    padding: 8px 16px;
    border: 1px solid var(--btn-border, #cf00ff);
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--ink, #fafafa);
    font: inherit;
    font-size: var(--text-small);
    cursor: pointer;
    transition: background-color 160ms ease;
  }

  .refresh:hover,
  .hide:hover {
    background: #222;
  }

  .updated {
    margin: 0 0 4px;
    font-size: var(--text-caption);
    color: var(--page-text);
  }

  .hint {
    margin: 0 0 8px;
    font-size: var(--text-caption);
    font-weight: 800;
    text-transform: uppercase;
    color: var(--page-text);
  }

  .tiles {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 8px;
    margin-bottom: 16px;
  }

  .tile {
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 44px;
    padding: 16px;
    border: 1px solid transparent;
    border-radius: var(--radius-md);
    background: var(--black, #080808);
    color: var(--ink, #fafafa);
    font: inherit;
    text-align: left;
    cursor: pointer;
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

  .icon {
    display: flex;
    flex: none;
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
    /* Место под строку резервируется, значение приходит без скачка вёрстки. */
    min-height: 1.3em;
    font-size: 17px;
    font-weight: 800;
    font-variant-numeric: tabular-nums;
    overflow-wrap: anywhere;
  }

  .tile-sub {
    font-size: 12px;
    color: #8b949e;
  }

  .card,
  .alert {
    margin-bottom: 8px;
    padding: 16px;
    border-radius: var(--radius-md);
    background: var(--black, #080808);
    color: var(--ink, #fafafa);
  }

  .card {
    position: relative;
    padding: 16px;
  }

  .card .hide {
    position: absolute;
    top: 14px;
    right: 14px;
  }

  .card h2 {
    font-size: 19px;
    text-transform: uppercase;
  }

  .headline {
    margin: 0 0 4px;
    font-size: 20px;
    font-weight: 800;
    font-variant-numeric: tabular-nums;
  }

  .card p {
    font-size: var(--text-small);
  }

  .note {
    color: #b8b8b8;
  }

  .graphs {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 8px 20px;
    padding-top: 18px;
  }

  .graphs .hide {
    position: static;
    grid-column: 1 / -1;
    justify-self: end;
  }

  .graphs h2 {
    margin: 0;
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
    .tile,
    .summary-values,
    .refresh,
    .hide {
      transition: none;
    }

    .tile:hover {
      transform: none;
    }
  }

  :global(:root[data-reduce-motion='true']) .reveal {
    transition: none;
  }

  @media (max-width: 760px) {
    .graphs {
      grid-template-columns: minmax(0, 1fr);
    }
  }

  .alert {
    border: 1px solid #da3633;
  }

  .muted {
    color: #8b949e;
  }

  .bar {
    height: 4px;
    margin-top: 8px;
    border-radius: var(--radius-xs);
    background: #21262d;
    overflow: hidden;
  }

  .fill {
    height: 100%;
    background: var(--accent);
  }

  .disk + .disk {
    margin-top: 8px;
  }
</style>
