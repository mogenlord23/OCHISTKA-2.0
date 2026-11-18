<script lang="ts">
  import { onMount } from 'svelte';
  import { formatBytes } from '../lib/format';
  import {
    analyzeCleanup,
    cleanSelected,
    largeFiles,
    listDrives,
    listQuarantine,
    privacyReport,
    purgeItem,
    restoreItem,
    spaceReport,
    stopAnalysis,
    toDisksError,
    type CleanupCategory,
    type DisksErrorInfo,
    type FolderSize,
    type LargeFile,
    type MoveOutcome,
    type PrivacyItem,
    type QuarantineEntry,
  } from '../lib/disksApi';

  let { subsection }: { subsection: string } = $props();

  type Running = 'cleanup' | 'files' | 'space' | 'privacy' | null;
  let running = $state<Running>(null);
  let error = $state<DisksErrorInfo | null>(null);
  let notice = $state('');

  // Глубокая очистка
  let categories = $state<CleanupCategory[]>([]);
  let selected = $state<Record<string, boolean>>({});
  let outcome = $state<MoveOutcome | null>(null);
  let quarantine = $state<QuarantineEntry[]>([]);
  let alternatives = $state<Record<string, string>>({});
  let occupiedId = $state<string | null>(null);
  let confirmId = $state<string | null>(null);

  // Очистка файлов
  let files = $state<LargeFile[]>([]);
  let filesScanned = $state(0);
  let filesComplete = $state(true);
  let filesLoaded = $state(false);

  // Анализ пространства
  let drives = $state<string[]>([]);
  let drive = $state('');
  let folders = $state<FolderSize[]>([]);
  let looseBytes = $state(0);
  let spaceComplete = $state(true);
  let spaceLoaded = $state(false);

  // Приватность
  let privacy = $state<PrivacyItem[]>([]);
  let privacyLoaded = $state(false);

  const chosenIds = $derived(
    categories
      .filter((item) => selected[item.id] && item.found && item.fileCount > 0)
      .map((item) => item.id),
  );
  const maxFolderBytes = $derived(Math.max(1, ...folders.map((item) => item.bytes)));

  function fail(err: unknown) {
    error = toDisksError(err);
  }

  async function refreshQuarantine() {
    try {
      quarantine = await listQuarantine();
    } catch (err) {
      fail(err);
    }
  }

  onMount(async () => {
    await refreshQuarantine();
    try {
      drives = await listDrives();
      drive = drives[0] ?? '';
    } catch (err) {
      fail(err);
    }
  });

  async function analyze() {
    running = 'cleanup';
    error = null;
    notice = '';
    outcome = null;
    try {
      categories = await analyzeCleanup();
      selected = {};
    } catch (err) {
      fail(err);
    } finally {
      running = null;
    }
  }

  async function clean() {
    running = 'cleanup';
    error = null;
    try {
      outcome = await cleanSelected(chosenIds);
      // План расходуется при переносе: прежние размеры уже неверны.
      categories = [];
      selected = {};
      notice = 'Выбранное перенесено в карантин. Анализ нужно запустить снова.';
      await refreshQuarantine();
    } catch (err) {
      fail(err);
    } finally {
      running = null;
    }
  }

  async function restore(entry: QuarantineEntry) {
    error = null;
    const alt = (alternatives[entry.id] ?? '').trim();
    try {
      await restoreItem(entry.id, alt === '' ? null : alt);
      occupiedId = null;
      notice = 'Файл возвращён на место.';
      await refreshQuarantine();
    } catch (err) {
      const info = toDisksError(err);
      if (info.kind === 'occupied') {
        occupiedId = entry.id;
      }
      error = info;
    }
  }

  async function purge(entry: QuarantineEntry) {
    error = null;
    try {
      await purgeItem(entry.id, entry.purgeToken);
      confirmId = null;
      notice = 'Файл удалён окончательно.';
      await refreshQuarantine();
    } catch (err) {
      fail(err);
    }
  }

  async function stop() {
    try {
      await stopAnalysis();
    } catch (err) {
      fail(err);
    }
  }

  async function loadFiles() {
    running = 'files';
    error = null;
    try {
      const report = await largeFiles();
      files = report.files;
      filesScanned = report.scanned;
      filesComplete = report.complete;
      filesLoaded = true;
    } catch (err) {
      fail(err);
    } finally {
      running = null;
    }
  }

  async function loadSpace() {
    running = 'space';
    error = null;
    try {
      const report = await spaceReport(drive);
      folders = report.folders;
      looseBytes = report.looseBytes;
      spaceComplete = report.complete;
      spaceLoaded = true;
    } catch (err) {
      fail(err);
    } finally {
      running = null;
    }
  }

  async function loadPrivacy() {
    running = 'privacy';
    error = null;
    try {
      privacy = await privacyReport();
      privacyLoaded = true;
    } catch (err) {
      fail(err);
    } finally {
      running = null;
    }
  }

  function formatDate(unix: number): string {
    return unix > 0 ? new Date(unix * 1000).toLocaleDateString('ru-RU') : '—';
  }
</script>

{#if error}
  <div class="alert" role="alert"><p>{error.message}</p></div>
{/if}
{#if notice}
  <p class="notice" role="status">{notice}</p>
{/if}

{#if subsection === 'cleanup'}
  <section class="panel">
    <div class="actions">
      <button
        type="button"
        class="primary"
        onclick={() => void analyze()}
        disabled={running !== null}
      >
        Анализировать
      </button>
      {#if running === 'cleanup'}
        <button type="button" onclick={() => void stop()}>Стоп</button>
        <span class="reason" aria-live="polite">Анализ идёт…</span>
      {/if}
      <button
        type="button"
        class="primary"
        onclick={() => void clean()}
        disabled={running !== null || chosenIds.length === 0}
      >
        Очистить выбранное
      </button>
    </div>
    <p class="hint">
      Файлы переносятся в карантин. Окончательно удаляется только то, что вы подтвердите.
    </p>
  </section>

  {#if categories.length > 0}
    <ul class="rows">
      {#each categories as item (item.id)}
        <li>
          <label>
            <input
              type="checkbox"
              bind:checked={selected[item.id]}
              disabled={!item.found || item.fileCount === 0}
            />
            <span class="title">{item.title}</span>
          </label>
          <span class="size">
            {#if !item.found}
              не найдено
            {:else}
              {formatBytes(item.bytes)} · файлов: {item.fileCount}{item.complete
                ? ''
                : ' (неполно)'}
            {/if}
          </span>
        </li>
      {/each}
    </ul>
  {/if}

  {#if outcome}
    <p class="notice" role="status">
      Перенесено: {outcome.moved}, освобождено: {formatBytes(outcome.bytes)}{outcome.failed > 0
        ? `, не удалось: ${outcome.failed}`
        : ''}.
    </p>
    {#if outcome.failures.length > 0}
      <ul class="failures">
        {#each outcome.failures as failure (failure.path)}
          <li><span class="path">{failure.path}</span> — {failure.message}</li>
        {/each}
      </ul>
    {/if}
  {/if}

  <h2>Карантин</h2>
  {#if quarantine.length === 0}
    <p class="empty">Карантин пуст.</p>
  {:else}
    <ul class="rows">
      {#each quarantine as entry (entry.id)}
        <li class="quarantine">
          <div>
            <span class="title">{entry.originalPath}</span>
            <small
              >{formatBytes(entry.size)} · {formatDate(entry.movedAtUnix)} · {entry.reason}</small
            >
          </div>
          <div class="actions">
            <button type="button" onclick={() => void restore(entry)}>Восстановить</button>
            <button type="button" onclick={() => (confirmId = entry.id)}>Удалить навсегда</button>
          </div>
          {#if occupiedId === entry.id}
            <label class="alt">
              Место занято. Другой путь:
              <input bind:value={alternatives[entry.id]} placeholder="C:\Папка\файл.tmp" />
            </label>
          {/if}
          {#if confirmId === entry.id}
            <div class="confirm" role="alertdialog" aria-label="Подтверждение удаления">
              <p>Удалить файл навсегда? Это нельзя отменить.</p>
              <div class="actions">
                <button type="button" class="danger" onclick={() => void purge(entry)}
                  >Удалить навсегда</button
                >
                <button type="button" onclick={() => (confirmId = null)}>Отмена</button>
              </div>
            </div>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
{/if}

{#if subsection === 'files'}
  <section class="panel">
    <div class="actions">
      <button
        type="button"
        class="primary"
        onclick={() => void loadFiles()}
        disabled={running !== null}
      >
        Найти большие старые файлы
      </button>
      {#if running === 'files'}
        <button type="button" onclick={() => void stop()}>Стоп</button>
      {/if}
    </div>
    <p class="hint">
      Только список: старше 30 дней и не меньше 10 МБ, папка «Загрузки». Ничего не переносится.
    </p>
  </section>
  {#if filesLoaded}
    {#if files.length === 0}
      <p class="empty">Подходящих файлов нет.</p>
    {:else}
      <ul class="rows">
        {#each files as file (file.path)}
          <li>
            <span class="path">{file.path}</span>
            <span class="size">{formatBytes(file.size)} · {formatDate(file.modifiedUnix)}</span>
          </li>
        {/each}
      </ul>
    {/if}
    <p class="hint">Проверено записей: {filesScanned}{filesComplete ? '' : ' (неполно)'}.</p>
  {/if}
{/if}

{#if subsection === 'space'}
  <section class="panel">
    <div class="actions">
      <label>
        Диск
        <select bind:value={drive} disabled={running !== null}>
          {#each drives as item (item)}
            <option value={item}>{item}</option>
          {/each}
        </select>
      </label>
      <button
        type="button"
        class="primary"
        onclick={() => void loadSpace()}
        disabled={running !== null || drive === ''}
      >
        Анализировать диск
      </button>
      {#if running === 'space'}
        <button type="button" onclick={() => void stop()}>Стоп</button>
      {/if}
    </div>
  </section>
  {#if spaceLoaded}
    <ul class="rows">
      {#each folders as folder (folder.path)}
        <li>
          <span class="title">{folder.name}</span>
          <span class="size"
            >{formatBytes(folder.bytes)} · файлов: {folder.files}{folder.complete
              ? ''
              : ' (неполно)'}</span
          >
          <span class="meter" aria-hidden="true"
            ><span style:width="{(folder.bytes / maxFolderBytes) * 100}%"></span></span
          >
        </li>
      {/each}
      <li>
        <span class="title">Файлы в корне диска</span>
        <span class="size">{formatBytes(looseBytes)}</span>
      </li>
    </ul>
    <p class="hint">
      {spaceComplete ? 'Подсчёт полный.' : 'Подсчёт неполный: достигнут лимит записей.'}
    </p>
  {/if}
{/if}

{#if subsection === 'privacy'}
  <section class="panel">
    <div class="actions">
      <button
        type="button"
        class="primary"
        onclick={() => void loadPrivacy()}
        disabled={running !== null}
      >
        Проверить места хранения
      </button>
      {#if running === 'privacy'}
        <button type="button" onclick={() => void stop()}>Стоп</button>
      {/if}
    </div>
    <p class="hint">Только наличие и размер. Содержимое не открывается, ничего не удаляется.</p>
  </section>
  {#if privacyLoaded}
    <ul class="rows">
      {#each privacy as item (item.id)}
        <li>
          <span class="title">{item.title}</span>
          <span class="size">
            {#if item.found}
              {formatBytes(item.bytes)} · записей: {item.files}{item.complete ? '' : ' (неполно)'}
            {:else}
              не найдено
            {/if}
          </span>
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
  .empty,
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
    align-items: center;
  }

  .hint,
  .reason,
  small {
    color: var(--muted);
    font-size: var(--text-caption);
  }

  .alert {
    padding: 12px;
    border-radius: var(--radius-sm);
    background: var(--black);
    color: var(--page-text);
  }

  .notice,
  .empty {
    color: var(--page-text);
  }

  h2 {
    margin: 20px 0 8px;
    font-family: var(--font-heading);
    font-size: var(--text-section);
    color: var(--page-text);
    text-transform: uppercase;
  }

  .rows {
    list-style: none;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .rows li {
    display: flex;
    flex-wrap: wrap;
    gap: 8px 16px;
    align-items: center;
    justify-content: space-between;
    padding: 10px 12px;
    border-radius: var(--radius-sm);
    background: var(--black);
    color: var(--ink);
  }

  .rows li.quarantine {
    flex-direction: column;
    align-items: stretch;
  }

  .title {
    font-weight: 800;
  }

  .path {
    word-break: break-all;
  }

  .size {
    color: var(--ink-soft);
    font-size: var(--text-small);
  }

  .meter {
    flex-basis: 100%;
    height: 6px;
    border-radius: 3px;
    background: var(--surface);
    overflow: hidden;
  }

  .meter span {
    display: block;
    height: 100%;
    background: var(--ink);
  }

  .confirm {
    padding: 8px;
    border-radius: var(--radius-sm);
    background: var(--surface);
    color: var(--ink);
  }

  .confirm p {
    margin: 0 0 8px;
  }

  .alt {
    display: flex;
    flex-direction: column;
    gap: 4px;
    color: var(--ink-soft);
  }

  .failures {
    color: var(--page-text);
    font-size: var(--text-small);
  }

  label {
    display: inline-flex;
    gap: 8px;
    align-items: center;
    color: var(--ink);
  }

  input,
  select {
    min-height: 36px;
    padding: 6px 10px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border);
    background: var(--surface);
    color: var(--ink);
    font: inherit;
  }

  button {
    min-height: 44px;
    padding: 10px 16px;
    border: 0;
    border-radius: var(--radius-sm);
    background: var(--surface);
    color: var(--ink);
    font: inherit;
    font-weight: 800;
    cursor: pointer;
  }

  button.primary {
    background: var(--ink);
    color: var(--black);
  }

  button.danger {
    background: var(--page-text);
    color: var(--black);
  }

  button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
