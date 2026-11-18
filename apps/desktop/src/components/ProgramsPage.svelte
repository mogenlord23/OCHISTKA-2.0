<script lang="ts">
  import { untrack } from 'svelte';
  import { formatBytes } from '../lib/format';
  import { iconColor } from '../lib/processes';
  import { filterPrograms, kindLabel, measureBlockReason } from '../lib/programs';
  import {
    listInstalled,
    listStartup,
    measureFolder,
    stopMeasure,
    toProgramsError,
    type FolderMeasure,
    type InstalledProgram,
    type ProgramsErrorInfo,
    type StartupEntry,
  } from '../lib/programsApi';

  let { subsection }: { subsection: string } = $props();

  const DELETE_REASON = 'Удаление программ пока не подключено';

  let loading = $state(false);
  let error = $state<ProgramsErrorInfo | null>(null);
  let notice = $state('');

  // Установленные
  let programs = $state<InstalledProgram[]>([]);
  let loadedInstalled = $state(false);
  let query = $state('');
  let selected = $state<InstalledProgram | null>(null);
  let measuring = $state(false);
  let measures = $state<Record<string, FolderMeasure>>({});

  // Автозагрузка
  let startup = $state<StartupEntry[]>([]);
  let loadedStartup = $state(false);

  const visible = $derived(filterPrograms(programs, query));
  const selectedMeasure = $derived(
    selected?.installLocation ? (measures[selected.installLocation] ?? null) : null,
  );
  const selectedBlock = $derived(selected ? measureBlockReason(selected) : null);

  function fail(err: unknown) {
    error = toProgramsError(err);
  }

  async function loadInstalled() {
    loading = true;
    error = null;
    try {
      programs = await listInstalled();
      loadedInstalled = true;
      selected = null;
    } catch (err) {
      fail(err);
    } finally {
      loading = false;
    }
  }

  async function loadStartup() {
    loading = true;
    error = null;
    try {
      startup = await listStartup();
      loadedStartup = true;
    } catch (err) {
      fail(err);
    } finally {
      loading = false;
    }
  }

  // Загрузка при открытии подраздела: один раз на подраздел, список обновляет кнопка.
  $effect(() => {
    const current = subsection;
    untrack(() => {
      if (current === 'installed' && !loadedInstalled) void loadInstalled();
      if (current === 'startup' && !loadedStartup) void loadStartup();
    });
  });

  function select(program: InstalledProgram) {
    selected = selected === program ? null : program;
    notice = '';
  }

  async function measure(program: InstalledProgram) {
    if (!program.installLocation || measuring) return;
    measuring = true;
    error = null;
    notice = '';
    try {
      const report = await measureFolder(program.installLocation);
      measures = { ...measures, [program.installLocation]: report };
    } catch (err) {
      const info = toProgramsError(err);
      if (info.kind === 'cancelled') {
        notice = 'Подсчёт остановлен.';
      } else {
        error = info;
      }
    } finally {
      measuring = false;
    }
  }

  async function stop() {
    try {
      await stopMeasure();
    } catch (err) {
      fail(err);
    }
  }

  function formatSize(bytes: number | null): string {
    return bytes === null ? '—' : formatBytes(bytes);
  }
</script>

{#snippet messages()}
  {#if error}
    <p class="alert" role="alert">{error.message}</p>
  {/if}
  {#if notice}
    <p class="notice" role="status">{notice}</p>
  {/if}
{/snippet}

{#if subsection === 'installed'}
  <section class="page" aria-label="Установленные программы">
    <label class="field-label" for="programs-search">Поиск программ</label>
    <input
      id="programs-search"
      type="search"
      class="search"
      bind:value={query}
      placeholder="Название или издатель, например Chrome"
    />

    <div class="buttons">
      <button type="button" onclick={() => void loadInstalled()} disabled={loading}>Обновить</button
      >
      <button
        type="button"
        onclick={() => selected && void measure(selected)}
        disabled={!selected || measuring || selectedBlock !== null}
        title={!selected ? 'Сначала выбери программу' : (selectedBlock ?? 'Посчитать размер папки')}
        aria-describedby={selectedBlock ? 'measure-reason' : undefined}
      >
        Размер папки
      </button>
      {#if measuring}
        <button type="button" onclick={() => void stop()}>Стоп</button>
      {/if}
      <button type="button" disabled title={DELETE_REASON} aria-describedby="delete-reason">
        Удалить
      </button>
    </div>

    <p class="status-line" aria-live="polite">
      {#if loadedInstalled}Приложений: {visible.length} из {programs.length}.{/if}
      Размер — оценка установщика; «—» значит, что размер не указан. Папку выбранной программы можно измерить.
    </p>

    {@render messages()}

    {#if loading && !loadedInstalled}
      <p class="status-line" role="status">Читаю список программ…</p>
    {:else if loadedInstalled && visible.length === 0}
      <p class="status-line">Ничего не найдено.</p>
    {:else}
      <div class="table" role="presentation">
        <div class="row head" aria-hidden="true">
          <span></span><span></span><span>Программа</span><span>Размер</span><span>Диск</span><span
            >Версия</span
          ><span>Тип</span><span>Издатель</span>
        </div>
        <ul class="rows">
          {#each visible as program (program)}
            <li>
              <button
                type="button"
                class="row"
                aria-pressed={selected === program}
                onclick={() => select(program)}
              >
                <span class="check" aria-hidden="true"></span>
                <span class="icon" style:background={iconColor(program.name)} aria-hidden="true"
                  >{program.name.charAt(0).toUpperCase()}</span
                >
                <span class="name">{program.name}</span>
                <span>{formatSize(program.sizeBytes)}</span>
                <span>{program.diskLetter ? `${program.diskLetter}\\` : '—'}</span>
                <span class="cut">{program.version ?? '—'}</span>
                <span>{kindLabel(program.kind)}</span>
                <span class="cut">{program.publisher ?? '—'}</span>
              </button>
            </li>
          {/each}
        </ul>
      </div>
    {/if}

    <div class="details" aria-label="Выбранная программа" aria-live="polite">
      {#if selected}
        <p class="details-name">{selected.name}</p>
        <p class="path">{selected.installLocation ?? 'Папка установки не указана'}</p>
        {#if selectedBlock}
          <p id="measure-reason" class="reason">{selectedBlock}</p>
        {/if}
        {#if measuring}
          <p class="reason">Считаю…</p>
        {/if}
        {#if selectedMeasure}
          <p class="result">
            Размер: {formatBytes(selectedMeasure.bytes)}, файлов: {selectedMeasure.files}
          </p>
          {#if !selectedMeasure.complete}
            <p class="reason">Подсчёт неполный: достигнут лимит записей. Цифра — нижняя оценка.</p>
          {/if}
        {/if}
      {:else}
        Выбери программу
      {/if}
      <p id="delete-reason" class="reason">{DELETE_REASON}</p>
    </div>
  </section>
{/if}

{#if subsection === 'startup'}
  <section class="page" aria-label="Автозагрузка">
    <p class="status-line">
      Только чтение: реестр «Run» (HKCU и HKLM) и папки «Автозагрузка». Изменение записей пока не
      подключено.
    </p>
    <div class="buttons">
      <button type="button" onclick={() => void loadStartup()} disabled={loading}>Обновить</button>
    </div>

    {@render messages()}

    {#if loading && !loadedStartup}
      <p class="status-line" role="status">Читаю автозагрузку…</p>
    {:else if loadedStartup && startup.length === 0}
      <p class="status-line">Автозагрузка пуста.</p>
    {:else}
      <div class="table">
        <div class="srow head" aria-hidden="true">
          <span></span><span>Название</span><span>Где</span><span>Команда</span>
        </div>
        <ul class="rows" aria-label="Записи автозагрузки">
          {#each startup as entry (entry.location + entry.name)}
            <li class="srow">
              <span class="icon" style:background={iconColor(entry.name)} aria-hidden="true"
                >{entry.name.charAt(0).toUpperCase()}</span
              >
              <span class="name">{entry.name}</span>
              <span>{entry.location}</span>
              <span class="path">{entry.command ?? '—'}</span>
            </li>
          {/each}
        </ul>
      </div>
    {/if}
  </section>
{/if}

<style>
  .page {
    display: flex;
    flex-direction: column;
    gap: 8px;
    color: var(--page-text);
  }
  .field-label {
    margin-top: 4px;
    font-size: var(--text-small);
    font-weight: 800;
    text-transform: uppercase;
  }
  .status-line {
    margin: 0;
    font-size: var(--text-small);
    font-weight: 600;
  }
  .search {
    height: 48px;
    padding: 0 14px;
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: var(--radius-md);
    background: var(--surface);
    color: var(--ink);
    font: inherit;
  }
  .search::placeholder {
    color: transparent;
  }
  .search:focus::placeholder {
    color: var(--muted);
  }
  .buttons {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  button {
    padding: 10px 14px;
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: var(--radius-md);
    background: var(--black);
    color: var(--ink);
    font: inherit;
    font-size: var(--text-small);
    font-weight: 600;
    cursor: pointer;
  }
  button:hover:not(:disabled) {
    border-color: var(--ink-soft);
  }
  button:disabled {
    opacity: 1;
    color: var(--muted);
    cursor: not-allowed;
  }
  .alert,
  .notice {
    margin: 0;
    padding: 8px 12px;
    border-radius: var(--radius-sm);
    background: var(--black);
    color: var(--ink);
    font-size: var(--text-small);
    font-weight: 700;
  }
  .alert {
    color: #ff8a8a;
  }
  .table {
    max-height: calc(100vh - 480px);
    min-height: 180px;
    overflow: auto;
    border-radius: var(--radius-md);
    background: var(--surface);
    color: var(--ink);
    font-size: var(--text-small);
  }
  .rows {
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .row,
  .srow {
    display: grid;
    grid-template-columns: 18px 22px minmax(160px, 2.4fr) 80px 50px 100px 60px minmax(120px, 2fr);
    gap: 12px;
    align-items: center;
    width: 100%;
    min-height: 46px;
    padding: 0 14px;
    border: 0;
    border-bottom: 1px solid #262626;
    border-radius: 0;
    background: none;
    color: var(--ink);
    font-weight: 500;
    text-align: left;
  }
  .srow {
    grid-template-columns: 22px minmax(140px, 1.5fr) 120px minmax(160px, 3fr);
  }
  .row:hover {
    border-color: #262626;
    background: #181818;
  }
  .row[aria-pressed='true'] {
    background: #222;
  }
  .head {
    position: sticky;
    top: 0;
    z-index: 1;
    min-height: 36px;
    border-bottom: 0;
    background: var(--black);
    color: var(--accent);
    font-size: var(--text-caption);
    font-weight: 700;
    text-transform: uppercase;
  }
  .check {
    width: 16px;
    height: 16px;
    border: 1.5px solid var(--muted);
    border-radius: 4px;
  }
  .row[aria-pressed='true'] .check {
    border-color: var(--accent);
    background: var(--accent);
  }
  .icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 22px;
    height: 22px;
    border-radius: 5px;
    color: #080808;
    font-size: 11px;
    font-weight: 800;
  }
  .name,
  .cut {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .path {
    font-family: ui-monospace, Consolas, monospace;
    font-size: var(--text-caption);
    color: var(--ink-soft);
    overflow-wrap: anywhere;
  }
  .details {
    padding: 12px 14px;
    border-radius: var(--radius-md);
    background: rgba(8, 8, 8, 0.35);
    color: var(--ink);
    font-size: var(--text-small);
  }
  .details p {
    margin: 0 0 2px;
  }
  .details-name,
  .result {
    font-weight: 800;
  }
  .reason {
    color: var(--ink-soft);
    font-size: var(--text-caption);
  }
</style>
