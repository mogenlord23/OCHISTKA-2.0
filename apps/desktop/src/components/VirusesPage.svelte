<script lang="ts">
  import Check from './Check.svelte';
  import { onMount } from 'svelte';
  import { clamavStatus, defaultScanTarget, toCommandError } from '../lib/api';
  import {
    listQuarantine,
    purgeItem,
    restoreItem,
    toDisksError,
    type QuarantineEntry,
  } from '../lib/disksApi';
  import { cancelScan, quarantineSelected, scan, startScan } from '../lib/scanState.svelte';
  import type { CommandError, EngineStatus } from '../lib/types';

  let { subsection }: { subsection: string } = $props();

  /** Причина, по которой карантин помечает находки этой страницы (см. quarantineSelected). */
  const SCAN_REASON_PREFIX = 'проверка «Вирусы»';

  let engine = $state<EngineStatus | null>(null);
  let engineError = $state<CommandError | null>(null);
  let picked = $state<Record<number, boolean>>({});
  let moving = $state(false);
  let journalEntries = $state<QuarantineEntry[]>([]);
  let journalNotice = $state('');
  let confirmPurgeId = $state<string | null>(null);

  // Идёт либо сама проверка, либо оценка находок: новую начинать нельзя.
  const busy = $derived(scan.phase !== 'idle');
  const error = $derived(scan.error ?? engineError);
  const findings = $derived(scan.report?.findings ?? []);
  const pickedCount = $derived(findings.filter((_, index) => picked[index]).length);

  const startBlockReason = $derived(
    busy
      ? ''
      : !engine?.available
        ? 'Сначала проверьте движок'
        : scan.target.trim() === ''
          ? 'Укажите папку для проверки'
          : '',
  );

  // Выбор относится к конкретному отчёту: новый отчёт или перенос сбрасывают отметки.
  $effect(() => {
    void scan.report;
    picked = {};
  });

  onMount(async () => {
    try {
      engine = await clamavStatus();
    } catch (err) {
      engineError = toCommandError(err);
    }
    // Поле не затираем, если проверка уже идёт или путь введён раньше.
    if (scan.target === '') {
      try {
        scan.target = await defaultScanTarget();
      } catch {
        // Пустое поле — пользователь укажет папку сам.
      }
    }
  });

  async function refreshJournal(): Promise<void> {
    try {
      journalEntries = (await listQuarantine()).filter((entry) =>
        entry.reason.startsWith(SCAN_REASON_PREFIX),
      );
    } catch (err) {
      journalNotice = toDisksError(err).message;
    }
  }

  $effect(() => {
    if (subsection === 'log') {
      void refreshJournal();
    }
  });

  function start() {
    void startScan(scan.target.trim());
  }

  async function moveSelected(): Promise<void> {
    const chosen = findings.filter((_, index) => picked[index]);
    moving = true;
    try {
      await quarantineSelected(chosen);
    } catch (err) {
      scan.notice = toDisksError(err).message;
    } finally {
      moving = false;
    }
  }

  async function restore(entry: QuarantineEntry): Promise<void> {
    try {
      await restoreItem(entry.id, null);
      journalNotice = `Возвращено: ${entry.originalPath}`;
    } catch (err) {
      const info = toDisksError(err);
      journalNotice =
        info.kind === 'occupied'
          ? `Место занято, файл остаётся в карантине: ${entry.originalPath}`
          : info.message;
    }
    await refreshJournal();
  }

  async function purge(entry: QuarantineEntry): Promise<void> {
    confirmPurgeId = null;
    try {
      await purgeItem(entry.id, entry.purgeToken);
      journalNotice = `Удалено окончательно: ${entry.originalPath}`;
    } catch (err) {
      journalNotice = toDisksError(err).message;
    }
    await refreshJournal();
  }

  function runSummary(run: (typeof scan.runs)[number]): string {
    if (run.status === 'failed') {
      return `ошибка: ${run.message}`;
    }
    if (run.status === 'cancelled') {
      return 'отменена';
    }
    const skipped = run.skipped > 0 ? `, пропущено ${run.skipped} (больше 200 МБ)` : '';
    return `проверено ${run.scanned}, находок ${run.infected}${skipped}`;
  }
</script>

{#if subsection === 'scan'}
  {#if engine && !engine.available}
    <div class="alert" role="alert">
      <strong>Движок ClamAV не найден</strong>
      <p>{engine.reason}</p>
      <p>Сборка libclamav не выполнена: статус и причина в docs/status/clamav-build.md.</p>
    </div>
  {/if}

  {#if error}
    <div class="alert" role="alert"><p>{error.message}</p></div>
  {/if}

  <section class="panel">
    <label>
      Папка для проверки
      <input bind:value={scan.target} placeholder="C:\Users\Имя\Downloads" disabled={busy} />
    </label>
    <div class="actions">
      <button
        type="button"
        class="primary"
        onclick={start}
        disabled={!engine?.available || busy || scan.target.trim() === ''}
      >
        Начать проверку
      </button>
      {#if startBlockReason}
        <span class="reason">{startBlockReason}</span>
      {/if}
      {#if scan.phase === 'scanning'}
        <button type="button" onclick={() => void cancelScan()}>Остановить</button>
      {/if}
    </div>
  </section>

  <section class="scanner" class:active={busy} aria-live="polite">
    <h3>Состояние</h3>
    <div class="track" aria-hidden="true">
      {#if scan.phase === 'scanning'}<div class="sweep" data-testid="scan-sweep"></div>{/if}
    </div>
    {#if scan.phase === 'scanning'}
      <p>Проверяем файлы…</p>
    {:else if scan.phase === 'assessing'}
      <p>Оцениваем находки…</p>
    {:else if scan.notice}
      <p>{scan.notice}</p>
    {:else if scan.report}
      <p>Проверено файлов: {scan.report.scannedFiles}. Находок: {scan.report.infectedFiles}.</p>
      {#if scan.report.incomplete}
        <p class="alert" role="status">Проверка неполная: достигнут лимит обхода.</p>
      {/if}
    {:else}
      <p>Готово к проверке.</p>
    {/if}
  </section>
{/if}

{#if subsection === 'findings'}
  {#if !scan.report}
    <p class="empty panel">Здесь появятся находки после проверки.</p>
  {:else if findings.length === 0}
    <p class="empty panel">Угроз не найдено.</p>
  {:else}
    {#if scan.notice}
      <p class="empty" role="status">{scan.notice}</p>
    {/if}
    {#if scan.assessFailures.count > 0}
      <p class="empty" role="status">
        Не удалось оценить файлов: {scan.assessFailures.count}. Причина: {scan.assessFailures
          .firstMessage}
      </p>
    {/if}
    <div class="actions bar">
      <span class="reason">Выбрано: {pickedCount}</span>
      <button
        type="button"
        disabled={pickedCount === 0 || moving || busy}
        title="Файлы переносятся в карантин; окончательное удаление — только в журнале"
        onclick={() => void moveSelected()}
      >
        Удалить
      </button>
    </div>
    <ul class="findings">
      {#each findings as finding, index (`${finding.path}#${index}`)}
        {@const score = scan.suspicions[finding.path]}
        <li>
          <Check bind:checked={picked[index]}>
            <span class="head">
              <strong>{finding.signature}</strong>
              <span class="path selectable">{finding.path}</span>
            </span>
          </Check>
          <small>Причина: сигнатура {finding.signature}</small>
          {#if score}
            <div
              class="meter"
              role="meter"
              aria-label="Подсказка о подозрительности"
              aria-valuemin="0"
              aria-valuemax="100"
              aria-valuenow={score.score}
            >
              <span style:width="{score.score}%"></span>
            </div>
            <small>
              Подсказка: {score.score}/100 — {score.reasons.join('; ') || 'признаков не найдено'}
            </small>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
{/if}

{#if subsection === 'log'}
  {#if scan.runs.length === 0}
    <p class="empty panel">Проверок в этой сессии пока не было.</p>
  {:else}
    <ul class="log panel selectable">
      {#each scan.runs as run (run.id)}
        <li>
          {run.startedAt} · {run.target} · {runSummary(run)}
        </li>
      {/each}
    </ul>
  {/if}

  <section class="quarantine-block panel">
    <h3>Карантин от проверок</h3>
    {#if journalNotice}
      <p class="empty" role="status">{journalNotice}</p>
    {/if}
    {#if journalEntries.length === 0}
      <p class="empty">В карантине от проверок пусто.</p>
    {:else}
      <ul class="quarantine-list">
        {#each journalEntries as entry (entry.id)}
          <li>
            <span class="path selectable">{entry.originalPath}</span>
            <small>{entry.reason}</small>
            <div class="actions">
              <button type="button" onclick={() => void restore(entry)}>Вернуть</button>
              {#if confirmPurgeId === entry.id}
                <span class="reason">Удалить навсегда? Отменить нельзя.</span>
                <button type="button" class="danger" onclick={() => void purge(entry)}>
                  Подтвердить удаление
                </button>
                <button type="button" onclick={() => (confirmPurgeId = null)}>Отмена</button>
              {:else}
                <button type="button" onclick={() => (confirmPurgeId = entry.id)}>
                  Удалить окончательно
                </button>
              {/if}
            </div>
          </li>
        {/each}
      </ul>
    {/if}
  </section>
{/if}

<style>
  .panel,
  .scanner,
  .alert,
  .findings li {
    margin: 0 0 8px;
    padding: 16px;
    border-radius: var(--radius-lg);
    background: var(--surface);
    box-shadow: 0 0 0 2px #000;
    font-size: var(--text-caption);
  }

  h3,
  label {
    margin: 0 0 8px;
    font-size: var(--text-caption);
    font-weight: 800;
    text-transform: uppercase;
    color: var(--ink);
  }

  .log li,
  .quarantine-list li {
    padding: 8px 0;
    border-bottom: 1px solid var(--border);
    color: var(--ink-soft);
  }

  .quarantine-list {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .track {
    position: relative;
    height: 10px;
    margin-bottom: 8px;
    overflow: hidden;
    border-radius: var(--radius-pill);
    background: #222;
    box-shadow: inset 0 0 0 1px var(--border);
  }

  .alert {
    border: 2px solid var(--accent);
    color: var(--ink);
  }

  .alert strong {
    text-transform: uppercase;
  }

  label {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  label input {
    text-transform: none;
    font-weight: 400;
  }

  input {
    padding: 12px 14px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--black);
    color: var(--ink);
    font: inherit;
  }

  .actions {
    display: flex;
    gap: 8px;
    margin-top: 8px;
  }

  button {
    min-height: 38px;
    padding: 8px 14px;
    border: 0;
    border-radius: var(--radius-sm);
    background: var(--btn-bg);
    box-shadow: inset 0 0 0 1px var(--btn-border);
    color: var(--ink);
    font: inherit;
    font-size: var(--text-caption);
    font-weight: 700;
    cursor: pointer;
  }

  button:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }

  .reason {
    align-self: center;
    color: var(--muted);
  }

  button.primary {
    border-color: var(--accent);
    background: var(--accent);
    color: var(--black);
    font-weight: 700;
  }

  .bar {
    margin: 0 0 8px;
    padding: 10px 14px;
    border-radius: var(--radius-lg);
    background: var(--surface);
    box-shadow: 0 0 0 2px #000;
  }

  .scanner {
    position: relative;
    min-height: 88px;
    overflow: hidden;
  }

  .scanner p,
  .empty {
    margin: 0;
    color: var(--ink-soft);
  }

  .sweep {
    position: absolute;
    inset: 0;
    background: linear-gradient(90deg, transparent, var(--accent), transparent);
    animation: sweep 1.6s linear infinite;
  }

  .scanner p {
    position: relative;
  }

  .empty.panel {
    margin-bottom: 8px;
  }

  @keyframes sweep {
    from {
      transform: translateX(-100%);
    }
    to {
      transform: translateX(100%);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .sweep {
      animation: none;
    }
  }

  .findings,
  .log {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .head {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .path {
    color: var(--muted);
    word-break: break-all;
  }

  .meter {
    height: 8px;
    margin: 8px 0 6px;
    border-radius: var(--radius-xs);
    background: #21262d;
    overflow: hidden;
  }

  .meter span {
    display: block;
    height: 100%;
    background: var(--accent);
  }

  small {
    color: var(--muted);
  }
</style>
