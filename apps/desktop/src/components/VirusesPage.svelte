<script lang="ts">
  import Check from './Check.svelte';
  import FindingsList from './FindingsList.svelte';
  import VirusBin from './VirusBin.svelte';
  import { onMount } from 'svelte';
  import { clamavStatus, defaultScanTarget, toCommandError } from '../lib/api';
  import {
    listQuarantine,
    purgeItem,
    restoreItem,
    toDisksError,
    type QuarantineEntry,
  } from '../lib/disksApi';
  import { cancelScan, scan, startScan } from '../lib/scanState.svelte';
  import type { CommandError, EngineStatus } from '../lib/types';

  let { subsection }: { subsection: string } = $props();

  /** Причина, по которой карантин помечает находки этой страницы (см. quarantineSelected). */
  const SCAN_REASON_PREFIX = 'проверка «Вирусы»';

  let engine = $state<EngineStatus | null>(null);
  let engineError = $state<CommandError | null>(null);
  let picked = $state<Record<number, boolean>>({});
  let journalEntries = $state<QuarantineEntry[]>([]);
  let journalNotice = $state('');
  let confirmPurgeId = $state<string | null>(null);

  /** Полёт находок в корзину: сколько фигур летит и открыта ли крышка. */
  const FLIGHT_MS = 520;
  let flight = $state<{ id: number; count: number } | null>(null);
  let flightSeq = 0;
  let binState = $state<'idle' | 'filling' | 'closed' | 'throwing'>('idle');

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

  /** Вид станции: идёт проверка, есть находки, чисто или проверить не удалось. */
  const stationState = $derived(
    scan.phase !== 'idle'
      ? 'scanning'
      : error || (engine && !engine.available)
        ? 'failed'
        : scan.report
          ? scan.report.findings.length > 0
            ? 'found'
            : scan.report.incomplete || scan.report.errors > 0 || scan.report.skippedFiles > 0
              ? 'failed'
              : 'clean'
          : 'idle',
  );

  /** Движение отключают и системная настройка, и наш переключатель в «Оформлении». */
  function reducedMotion(): boolean {
    if (typeof window === 'undefined') return false;
    if (document.documentElement.dataset.reduceMotion === 'true') return true;
    return window.matchMedia?.('(prefers-reduced-motion: reduce)').matches ?? false;
  }

  // Выбор относится к конкретному отчёту: новый отчёт или перенос сбрасывают отметки.
  $effect(() => {
    void scan.report;
    picked = {};
  });

  // Находки слетаются в корзину после отчёта, крышка закрывается в конце.
  // Без движения состояние меняется сразу: ни полёта, ни ожидания.
  $effect(() => {
    const report = scan.report;
    if (!report) {
      flight = null;
      binState = 'idle';
      return;
    }
    if (report.findings.length === 0 || reducedMotion()) {
      flight = null;
      binState = 'closed';
      return;
    }
    flight = { id: ++flightSeq, count: Math.min(3, report.findings.length) };
    binState = 'filling';
    const timer = window.setTimeout(() => {
      binState = 'closed';
      flight = null;
    }, FLIGHT_MS);
    return () => window.clearTimeout(timer);
  });

  function toggleFinding(index: number, next: boolean) {
    picked = { ...picked, [index]: next };
  }

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

?{#snippet findingsEmpty()}
  {#if !scan.report}
    <p class="empty">Здесь появятся находки после проверки.</p>
  {:else if findings.length === 0}
    {#if scan.report.incomplete || scan.report.errors > 0 || scan.report.skippedFiles > 0}
      <p class="empty">Находок нет. Часть файлов или папок не проверена.</p>
    {:else}
      <p class="empty">Угроз не найдено.</p>
    {/if}
  {/if}
{/snippet}

{#snippet stationStatus()}
  {#if scan.phase === 'scanning'}
    {#if scan.progress}
      <p>
        Обработано файлов: {scan.progress.processedFiles} из {scan.progress.totalFiles}. Проверено:
        {scan.progress.scannedFiles}.
      </p>
    {:else}
      <p>Подготавливаем список файлов…</p>
    {/if}
  {:else if scan.phase === 'assessing'}
    <p>Оцениваем находки…</p>
  {:else if scan.notice}
    <p>{scan.notice}</p>
  {:else if scan.report}
    <p>Проверено файлов: {scan.report.scannedFiles}. Находок: {scan.report.infectedFiles}.</p>
    {#if scan.report.incomplete}
      <p class="alert" role="status">
        Проверка неполная: часть папок недоступна или достигнут лимит обхода.
      </p>
    {/if}
  {:else}
    <p>Готово к проверке.</p>
  {/if}
{/snippet}

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

  <div class="scan-layout">
    <div class="station-col">
      <section
        class="station"
        data-testid="scan-station"
        data-state={stationState}
        aria-live="polite"
      >
        {#if stationState === 'scanning'}
          <span class="beam" data-testid="scan-beam" aria-hidden="true"></span>
        {/if}
        <div class="station-head">
          <span class="shield" aria-hidden="true">
            <svg viewBox="0 0 48 56" focusable="false">
              <path
                d="M24 3 5 10.5V28c0 11.6 7.7 21.2 19 24.9C35.3 49.2 43 39.6 43 28V10.5Z"
                fill="none"
                stroke="currentColor"
                stroke-width="4"
                stroke-linejoin="round"
              />
              <path
                d="m16 27.5 6.2 6.4L32 20.5"
                fill="none"
                stroke="currentColor"
                stroke-width="4"
                stroke-linecap="round"
                stroke-linejoin="round"
              />
            </svg>
          </span>
          <h3>Состояние</h3>
        </div>

        <label>
          Папка для проверки
          <input bind:value={scan.target} placeholder="C:\Users\Имя\Downloads" disabled={busy} />
        </label>

        <div class="actions">
          <button
            type="button"
            class="primary big"
            class:scanning={stationState === 'scanning'}
            data-state={stationState}
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

        <div class="track" aria-hidden="true">
          {#if scan.phase === 'scanning'}
            <div
              class="sweep"
              data-testid="scan-sweep"
              style:width={`${scan.progress && scan.progress.totalFiles > 0 ? (scan.progress.processedFiles / scan.progress.totalFiles) * 100 : 0}%`}
            ></div>
          {/if}
        </div>

        <div class="state">
          {@render stationStatus()}
        </div>
      </section>

      <div class="bin-slot">
        <VirusBin count={findings.length} state={binState} />
      </div>
    </div>

    <section class="findings-panel" aria-label="Находки проверки">
      <div class="panel-head">
        <h3>Находки</h3>
        <span class="reason">Выбрано: {pickedCount}</span>
      </div>
      <div class="list-scroll">
        {#if findings.length === 0}
          {@render findingsEmpty()}
        {:else}
          <FindingsList
            {findings}
            suspicions={scan.suspicions}
            selected={picked}
            onToggle={toggleFinding}
          />
        {/if}
      </div>
    </section>

    {#if flight}
      {@const current = flight}
      <div class="flight" data-testid="bin-flight" aria-hidden="true">
        {#each Array.from({ length: current.count }, (_, index) => index) as drop (drop)}
          <span class="drop" style:--drop-index={drop}></span>
        {/each}
      </div>
    {/if}
  </div>
{/if}

{#if subsection === 'findings'}
  {#if findings.length === 0}
    {@render findingsEmpty()}
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
        disabled
        title="Перенос временно недоступен: безопасная проверка файла ещё не реализована"
      >
        Удалить
      </button>
    </div>
    <p class="reason" role="status">Перенос находок в карантин временно недоступен.</p>
    <FindingsList
      {findings}
      suspicions={scan.suspicions}
      selected={picked}
      onToggle={toggleFinding}
    />
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
  .alert {
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

  button.big {
    min-height: 56px;
    padding: 8px 20px;
    font-size: var(--text-title);
  }

  .bar {
    margin: 0 0 8px;
    padding: 10px 14px;
    border-radius: var(--radius-lg);
    background: var(--surface);
    box-shadow: 0 0 0 2px #000;
  }

  .scan-layout {
    position: relative;
    display: grid;
    flex: 1 1 auto;
    grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
    gap: 16px;
    min-height: 0;
  }

  .station-col {
    display: flex;
    flex-direction: column;
    gap: 16px;
    min-height: 0;
  }

  .station {
    position: relative;
    display: flex;
    flex-direction: column;
    gap: 8px;
    overflow: hidden;
    padding: 16px;
    border-radius: var(--radius-lg);
    background: var(--surface);
    box-shadow: 0 0 0 2px #000;
    font-size: var(--text-caption);
  }

  /* Луч по станции: только пока идёт проверка. */
  .beam {
    position: absolute;
    inset: 0 auto 0 -40%;
    width: 40%;
    background: linear-gradient(
      90deg,
      transparent,
      color-mix(in srgb, var(--accent) 30%, transparent),
      transparent
    );
    animation: beam-sweep 1.6s linear infinite;
    pointer-events: none;
  }

  .big.scanning {
    animation: press-pulse 1.3s ease-in-out infinite;
  }

  .station[data-state='found'] {
    box-shadow: 0 0 0 2px var(--accent);
  }

  .station[data-state='clean'] {
    box-shadow: 0 0 0 2px color-mix(in srgb, var(--accent) 55%, #000);
  }

  .station[data-state='failed'] {
    box-shadow: 0 0 0 2px var(--muted);
  }

  .flight {
    position: absolute;
    inset: 0;
    z-index: 3;
    pointer-events: none;
  }

  .drop {
    position: absolute;
    width: 16px;
    height: 16px;
    background: var(--accent);
    animation: fly 480ms cubic-bezier(0.4, 0, 0.7, 1) both;
    animation-delay: calc(var(--drop-index) * 60ms);
  }

  .drop:nth-child(1) {
    border-radius: 50%;
  }

  .drop:nth-child(2) {
    border-radius: var(--radius-xs);
  }

  .drop:nth-child(3) {
    border-radius: 0;
    clip-path: polygon(50% 0, 100% 100%, 0 100%);
  }

  @keyframes beam-sweep {
    from {
      transform: translateX(0);
    }
    to {
      transform: translateX(360%);
    }
  }

  @keyframes press-pulse {
    0%,
    100% {
      box-shadow: 0 0 0 1px var(--accent);
    }
    50% {
      box-shadow: 0 0 0 9px color-mix(in srgb, var(--accent) 25%, transparent);
    }
  }

  @keyframes fly {
    0% {
      left: 70%;
      top: 20%;
      opacity: 0;
      transform: scale(0.6);
    }
    16% {
      opacity: 1;
    }
    86% {
      opacity: 1;
    }
    100% {
      left: 26%;
      top: 82%;
      opacity: 0;
      transform: scale(0.8);
    }
  }

  .station-head {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .station-head h3 {
    margin: 0;
  }

  .shield {
    display: inline-flex;
    color: var(--accent);
  }

  .shield svg {
    width: 28px;
    height: 32px;
  }

  .state {
    position: relative;
    min-height: 44px;
  }

  .state p {
    margin: 0 0 4px;
    color: var(--ink-soft);
  }

  .bin-slot {
    display: flex;
    justify-content: center;
    align-items: flex-end;
    margin-top: auto;
    padding-bottom: 8px;
  }

  .findings-panel {
    display: flex;
    flex-direction: column;
    min-height: 0;
    padding: 16px;
    border-radius: var(--radius-lg);
    background: var(--surface);
    box-shadow: 0 0 0 2px #000;
    font-size: var(--text-caption);
  }

  .panel-head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 8px;
    margin-bottom: 8px;
  }

  .panel-head h3 {
    margin: 0;
  }

  .list-scroll {
    min-height: 0;
    overflow: hidden auto;
    overscroll-behavior: contain;
  }

  .sweep {
    height: 100%;
    background: var(--accent);
    transition: width 0.2s ease-out;
  }

  .empty {
    margin: 0;
    color: var(--ink-soft);
  }

  .empty.panel {
    margin-bottom: 8px;
  }

  @media (prefers-reduced-motion: reduce) {
    .sweep {
      transition: none;
    }

    .beam {
      display: none;
    }

    .big.scanning {
      animation: none;
    }

    .drop {
      animation: none;
    }
  }

  /* Наш переключатель «меньше движения» в «Оформлении» делает то же самое. */
  :global(:root[data-reduce-motion='true']) .sweep {
    transition: none;
  }

  :global(:root[data-reduce-motion='true']) .beam {
    display: none;
  }

  :global(:root[data-reduce-motion='true']) .big.scanning {
    animation: none;
  }

  :global(:root[data-reduce-motion='true']) .drop {
    animation: none;
  }

  .log {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .path {
    color: var(--muted);
    word-break: break-all;
  }

  small {
    color: var(--muted);
  }

  @media (max-width: 980px) {
    .scan-layout {
      flex: none;
      grid-template-columns: minmax(0, 1fr);
    }

    .list-scroll {
      overflow: visible;
    }
  }

  @media (max-width: 720px) {
    .scan-layout {
      gap: 8px;
    }

    .station,
    .findings-panel {
      padding: 12px;
    }
  }
</style>
