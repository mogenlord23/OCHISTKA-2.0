<script lang="ts">
  import { untrack } from 'svelte';
  import { formatBytes } from '../lib/format';
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

{#if error}
  <div class="alert" role="alert"><p>{error.message}</p></div>
{/if}
{#if notice}
  <p class="notice" role="status">{notice}</p>
{/if}

{#if subsection === 'installed'}
  <section class="panel">
    <div class="actions">
      <label class="search">
        <span>Поиск по названию или издателю</span>
        <input type="search" bind:value={query} placeholder="Например, Chrome" />
      </label>
      <button type="button" onclick={() => void loadInstalled()} disabled={loading}>
        Обновить список
      </button>
    </div>
    {#if loadedInstalled}
      <p class="hint" aria-live="polite">Найдено: {visible.length} из {programs.length}</p>
    {/if}
  </section>

  {#if loading && !loadedInstalled}
    <p class="notice" role="status">Читаю список программ…</p>
  {:else if loadedInstalled && visible.length === 0}
    <p class="notice">Ничего не найдено.</p>
  {:else}
    <ul class="rows">
      {#each visible as program (program)}
        <li>
          <button
            type="button"
            class="row"
            aria-pressed={selected === program}
            onclick={() => select(program)}
          >
            <span class="name">{program.name}</span>
            <span class="meta">{program.publisher ?? '—'}</span>
            <span class="meta">{program.version ?? '—'}</span>
            <span class="meta">{program.diskLetter ?? '—'}</span>
            <span class="meta">{formatSize(program.sizeBytes)}</span>
            <span class="meta">{kindLabel(program.kind)}</span>
          </button>
        </li>
      {/each}
    </ul>
  {/if}

  {#if selected}
    <section class="panel details" aria-label="Выбранная программа">
      <h2>{selected.name}</h2>
      <dl>
        <dt>Издатель</dt>
        <dd>{selected.publisher ?? '—'}</dd>
        <dt>Версия</dt>
        <dd>{selected.version ?? '—'}</dd>
        <dt>Диск</dt>
        <dd>{selected.diskLetter ?? '—'}</dd>
        <dt>Папка установки</dt>
        <dd class="path">{selected.installLocation ?? '—'}</dd>
        <dt>Размер по реестру</dt>
        <dd>{formatSize(selected.sizeBytes)}</dd>
      </dl>

      <div class="actions">
        <button
          type="button"
          class="primary"
          onclick={() => void measure(selected!)}
          disabled={measuring || selectedBlock !== null}
          aria-describedby={selectedBlock ? 'measure-reason' : undefined}
        >
          Размер папки
        </button>
        {#if measuring}
          <button type="button" onclick={() => void stop()}>Стоп</button>
          <span class="reason" aria-live="polite">Считаю…</span>
        {/if}
        <button type="button" disabled aria-describedby="delete-reason">Удалить</button>
        <span id="delete-reason" class="reason">{DELETE_REASON}</span>
      </div>
      {#if selectedBlock}
        <p id="measure-reason" class="hint">{selectedBlock}</p>
      {/if}

      {#if selectedMeasure}
        <p class="result" aria-live="polite">
          Размер: {formatBytes(selectedMeasure.bytes)}, файлов: {selectedMeasure.files}
        </p>
        {#if !selectedMeasure.complete}
          <p class="hint">Подсчёт неполный: достигнут лимит записей. Цифра — нижняя оценка.</p>
        {/if}
      {/if}
    </section>
  {/if}
{/if}

{#if subsection === 'startup'}
  <section class="panel">
    <p class="hint">
      Только чтение: реестр «Run» (HKCU и HKLM) и папки «Автозагрузка». Изменение записей пока не
      подключено.
    </p>
    <div class="actions">
      <button type="button" onclick={() => void loadStartup()} disabled={loading}>
        Обновить список
      </button>
    </div>
  </section>

  {#if loading && !loadedStartup}
    <p class="notice" role="status">Читаю автозагрузку…</p>
  {:else if loadedStartup && startup.length === 0}
    <p class="notice">Автозагрузка пуста.</p>
  {:else}
    <ul class="rows" aria-label="Записи автозагрузки">
      {#each startup as entry (entry.location + entry.name)}
        <li class="startup-row">
          <span class="name">{entry.name}</span>
          <span class="meta">{entry.location}</span>
          {#if entry.command}<span class="path">{entry.command}</span>{/if}
        </li>
      {/each}
    </ul>
  {/if}
{/if}

<style>
  .panel,
  .rows,
  .alert,
  .notice,
  .hint {
    margin: 0 0 12px;
  }

  .panel {
    padding: 16px;
    border-radius: var(--radius-md);
    background: var(--black);
    color: var(--ink);
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    align-items: flex-end;
  }

  .search {
    display: flex;
    flex-direction: column;
    gap: 4px;
    flex: 1;
    min-width: 220px;
    font-size: var(--text-caption);
    color: var(--muted);
  }

  input {
    min-height: 40px;
    padding: 8px 10px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--surface);
    color: var(--ink);
    font: inherit;
  }

  button {
    min-height: 40px;
    padding: 8px 14px;
    border: 0;
    border-radius: var(--radius-sm);
    background: var(--surface);
    color: var(--ink);
    font: inherit;
    font-weight: 800;
    cursor: pointer;
  }

  button.primary {
    background: var(--btn-bg);
    color: var(--ink);
    box-shadow: inset 0 0 0 1px var(--btn-border);
  }

  button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .hint,
  .reason {
    color: var(--muted);
    font-size: var(--text-caption);
  }

  .alert {
    padding: 12px;
    border-radius: var(--radius-sm);
    background: var(--black);
    color: var(--page-text);
  }

  .notice {
    color: var(--page-text);
  }

  .rows {
    list-style: none;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .row {
    display: grid;
    grid-template-columns: minmax(180px, 2fr) minmax(120px, 1.5fr) 90px 60px 100px 80px;
    gap: 8px;
    width: 100%;
    text-align: left;
    background: var(--black);
    color: var(--ink);
  }

  .row[aria-pressed='true'] {
    outline: 2px solid var(--ink);
  }

  .startup-row {
    display: grid;
    gap: 4px;
    padding: 10px 12px;
    border-radius: var(--radius-sm);
    background: var(--black);
    color: var(--ink);
  }

  .name {
    font-weight: 800;
    overflow-wrap: anywhere;
  }

  .meta,
  .path {
    font-size: var(--text-caption);
    color: var(--muted);
    overflow-wrap: anywhere;
  }

  .details h2 {
    margin: 0 0 8px;
    font-family: var(--font-heading);
    font-size: var(--text-section);
    text-transform: uppercase;
  }

  dl {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: 4px 12px;
    margin: 0 0 12px;
  }

  dt {
    color: var(--muted);
    font-size: var(--text-caption);
  }

  dd {
    margin: 0;
  }

  .result {
    font-weight: 800;
    margin: 12px 0 4px;
  }

  @media (max-width: 720px) {
    .row {
      grid-template-columns: 1fr;
    }
  }
</style>
