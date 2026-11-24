<script lang="ts">
  import AcidScan from './AcidScan.svelte';
  import Check from './Check.svelte';
  import ContextMenu, { type MenuItem } from './ContextMenu.svelte';
  import FindingsList from './FindingsList.svelte';
  import RemoveSelected from './RemoveSelected.svelte';
  import VirusBin from './VirusBin.svelte';
  import { onDestroy, onMount, tick } from 'svelte';
  import { clamavSignatures, clamavStatus, defaultScanTarget, pickScanFolder, toCommandError } from '../lib/api';
  import { FINDINGS_TRANSFER_READY, removeAllowed } from '../lib/scanCapabilities';
  import { revealInExplorer, toProgramsError } from '../lib/programsApi';
  import {
    listQuarantine,
    purgeItem,
    restoreItem,
    toDisksError,
    type QuarantineEntry,
  } from '../lib/disksApi';
  import { cancelScan, quarantineSelected, scan, startScan } from '../lib/scanState.svelte';
  import type { CommandError, EngineStatus, SignaturesInfo } from '../lib/types';

  let { subsection }: { subsection: string } = $props();

  /** Причина, по которой карантин помечает находки этой страницы (см. quarantineSelected). */
  const SCAN_REASON_PREFIX = 'проверка «Вирусы»';

  let engine = $state<EngineStatus | null>(null);
  let engineError = $state<CommandError | null>(null);
  let signatures = $state<SignaturesInfo | null>(null);
  let folderOpen = $state(false);
  let choosingFolder = $state(false);
  let folderNotice = $state('');
  let picked = $state<Record<number, boolean>>({});
  let journalEntries = $state<QuarantineEntry[]>([]);
  let journalNotice = $state('');
  let confirmPurgeId = $state<string | null>(null);

  /** Полёт находок в корзину: сколько фигур летит и открыта ли крышка. */
  const FLIGHT_MS = 520;
  let flight = $state<{ id: number; count: number } | null>(null);
  let flightSeq = 0;
  let binState = $state<'idle' | 'filling' | 'closed' | 'throwing'>('idle');

  /** Меню находки: где открылось и куда вернуть фокус. */
  let menu = $state<{ index: number; x: number; y: number } | null>(null);
  let menuReturnFocus: HTMLElement | null = null;
  let actionNotice = $state('');

  // Идёт либо сама проверка, либо оценка находок: новую начинать нельзя.
  const busy = $derived(scan.phase !== 'idle');
  const error = $derived(scan.error ?? engineError);
  const findings = $derived(scan.report?.findings ?? []);
  const pickedCount = $derived(findings.filter((_, index) => picked[index]).length);

  /** Вид станции: идёт проверка, есть находки, чисто или проверить не удалось. */
  const stationState = $derived(
    scan.phase !== 'idle'
      ? 'scanning'
      : error
        ? 'failed'
        : scan.report
          ? scan.report.findings.length > 0
            ? 'found'
            : scan.report.incomplete || scan.report.errors > 0 || scan.report.skippedFiles > 0
              ? 'failed'
              : 'clean'
          : 'idle',
  );

  const sweepWidth = $derived(
    scan.progress && scan.progress.totalFiles > 0
      ? Math.min(100, (scan.progress.processedFiles / scan.progress.totalFiles) * 100)
      : 0,
  );
  const scanPercent = $derived(Math.round(sweepWidth));
  // Для всего компьютера объём заранее неизвестен. Уровень реагирует на обработку,
  // а числом рядом остаётся количество файлов. Полный бак требует отчёта.
  const acidLevel = $derived.by(() => {
    if (scan.report) return 1;
    if (scan.phase !== 'scanning') return 0;
    if (scan.progress?.totalFiles) return Math.min(0.97, sweepWidth / 100);
    const processed = scan.progress?.processedFiles ?? 0;
    return 0.06 + (0.86 * processed) / (processed + 4096);
  });
  const statScanned = $derived(scan.progress?.scannedFiles ?? scan.report?.scannedFiles ?? 0);
  const statSkipped = $derived(scan.report?.skippedFiles ?? scan.progress?.skippedFiles ?? 0);
  const statErrors = $derived(scan.report?.errors ?? scan.progress?.errors ?? 0);
  const gaugeValue = $derived(busy ? scan.progress?.totalFiles ? `${scanPercent}%` : `${statScanned}` : `${findings.length}`);
  const gaugeCaption = $derived(busy ? scan.progress?.totalFiles ? 'проверено' : 'файлов проверено' : 'находок');
  const gaugeLabel = $derived(
    scan.phase === 'scanning' || scan.phase === 'assessing'
      ? scan.progress?.totalFiles
        ? `Проверка ${scanPercent}%`
        : `Проверено файлов: ${statScanned}`
      : 'Запустить проверку в квадрате',
  );

  /** Движение отключают и системная настройка, и наш переключатель в «Оформлении». */
  function reducedMotion(): boolean {
    if (typeof window === 'undefined') return false;
    if (document.documentElement.dataset.reduceMotion === 'true') return true;
    return window.matchMedia?.('(prefers-reduced-motion: reduce)').matches ?? false;
  }

  // Кнопка «Убрать выбранные»: перенос пока запрещён бэкендом, поэтому причина простая и понятная.
  const canRemove = $derived(removeAllowed(FINDINGS_TRANSFER_READY, pickedCount, busy));
  const removeReason = $derived(
    FINDINGS_TRANSFER_READY
      ? pickedCount === 0
        ? 'Выберите находки'
        : ''
      : 'Пока нельзя убрать: перенос файлов ещё не готов',
  );
  let throwTimer: number | undefined;

  onDestroy(() => {
    if (throwTimer) window.clearTimeout(throwTimer);
  });

  /** «Корзина выбрасывает»: крышка открыта, фигуры улетают, счётчик обнуляется отчётом. */
  function playThrow() {
    if (reducedMotion()) {
      binState = 'closed';
      return;
    }
    if (throwTimer) window.clearTimeout(throwTimer);
    binState = 'throwing';
    throwTimer = window.setTimeout(() => {
      binState = 'idle';
      throwTimer = undefined;
    }, FLIGHT_MS);
  }

  async function removeSelected() {
    if (!canRemove) return;
    const selected = findings.filter((_, index) => picked[index]);
    actionNotice = '';
    try {
      const moved = await quarantineSelected(selected);
      if (moved > 0) playThrow();
    } catch (err) {
      actionNotice = toDisksError(err).message;
    }
  }

  // Выбор относится к конкретному отчёту: новый отчёт или перенос сбрасывают отметки.
  $effect(() => {
    void scan.report;
    picked = {};
    menu = null;
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
      // Пока корзина «выбрасывает», её вид не перебиваем.
      if (binState !== 'throwing') binState = 'closed';
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

  /** Папка, в которой лежит находка: без последнего куска пути и без хвостовых слэшей. */
  function parentFolder(path: string): string {
    const trimmed = path.replace(/[\\/]+$/, '');
    const cut = Math.max(trimmed.lastIndexOf('\\'), trimmed.lastIndexOf('/'));
    return cut > 0 ? trimmed.slice(0, cut) : trimmed;
  }

  function openFindingMenu(event: MouseEvent | KeyboardEvent, index: number) {
    if (event instanceof MouseEvent) {
      event.preventDefault();
    }
    const row = (event.target as HTMLElement | null)?.closest('li');
    menuReturnFocus = row?.querySelector<HTMLElement>('input[type="checkbox"]') ?? null;
    let x = 0;
    let y = 0;
    if (event instanceof MouseEvent) {
      x = event.clientX;
      y = event.clientY;
    }
    if (x === 0 && y === 0) {
      const rect = (menuReturnFocus ?? (event.target as HTMLElement)).getBoundingClientRect();
      x = rect.left + 24;
      y = rect.bottom;
    }
    menu = { index, x, y };
  }

  function findingMenuItems(): MenuItem[] {
    return [
      { id: 'folder', label: 'Открыть папку' },
      { id: 'copy', label: 'Скопировать путь' },
    ];
  }

  async function closeFindingMenu() {
    menu = null;
    await tick();
    menuReturnFocus?.focus();
  }

  async function openFolder(path: string) {
    actionNotice = '';
    try {
      await revealInExplorer(parentFolder(path));
    } catch (err) {
      actionNotice = toProgramsError(err).message;
    }
  }

  async function copyPath(path: string) {
    try {
      await navigator.clipboard.writeText(path);
      actionNotice = 'Путь скопирован.';
    } catch {
      actionNotice = 'Не удалось скопировать путь.';
    }
  }

  async function onFindingMenuSelect(id: string) {
    const index = menu?.index;
    menu = null;
    if (index === undefined) return;
    const finding = findings[index];
    if (!finding) return;
    if (id === 'folder') {
      await openFolder(finding.path);
    } else if (id === 'copy') {
      await copyPath(finding.path);
    }
    await tick();
    menuReturnFocus?.focus();
  }

  onMount(async () => {
    if (scan.target === '') {
      void defaultScanTarget()
        .then((fallback) => {
          if (scan.target === '') scan.target = fallback;
        })
        .catch(() => {
          // Поле остаётся пустым, основной запуск не зависит от этой подсказки.
        });
    }
    try {
      engine = await clamavStatus();
    } catch (err) {
      engineError = toCommandError(err);
    }
    try {
      signatures = await clamavSignatures();
    } catch {
      // Без сведений о базах экран работает как раньше: показывает причину от движка.
    }
  });

  async function chooseFolder() {
    if (busy || choosingFolder) return;
    choosingFolder = true;
    folderNotice = '';
    try {
      const path = await pickScanFolder();
      if (path !== null) scan.target = path;
    } catch (err) {
      folderNotice = toCommandError(err).message;
    } finally {
      choosingFolder = false;
    }
  }

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
    if (busy) return;
    void startScan(null);
  }

  function startFolder() {
    if (busy || scan.target.trim() === '') return;
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
    const skipped = run.skipped > 0 ? `, пропущено ${run.skipped}` : '';
    return `проверено ${run.scanned}, находок ${run.infected}${skipped}`;
  }
</script>

{#snippet findingsEmpty()}
  {#if !scan.report}
    <p class="empty">Здесь появятся находки после проверки.</p>
  {:else if findings.length === 0}
    {#if scan.report.scannedFiles === 0 && scan.report.errors === 0 && !scan.report.incomplete}
      <p class="empty">Проверять нечего.</p>
    {:else if scan.report.errors > 0}
      <p class="empty">Проверить не удалось: файлов без проверки — {scan.report.errors}.</p>
    {:else if scan.report.incomplete || scan.report.skippedFiles > 0}
      <p class="empty">Проверено не всё: часть файлов или папок осталась без проверки.</p>
    {:else if scan.report.mode === 'signs'}
      <p class="empty">Подозрительных признаков не найдено.</p>
    {:else}
      <p class="empty">Угроз не найдено.</p>
    {/if}
  {/if}
{/snippet}

{#snippet panelTitle(title: string, kind: 'findings' | 'log' | 'quarantine')}
  <div class="panel-title">
    <span class="panel-icon" data-kind={kind} aria-hidden="true">
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
        {#if kind === 'findings'}
          <path d="M12 3 5 6v5c0 4 2.7 7.5 7 10 4.3-2.5 7-6 7-10V6Z M9 12l2 2 4-4" />
        {:else if kind === 'log'}
          <path d="M7 3h10a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2Z M9 8h6m-6 4h6m-6 4h4" />
        {:else}
          <path d="M4 8h16v11a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1Z M3 5h18v3H3Z M9 12h6" />
        {/if}
      </svg>
    </span>
    <h3>{title}</h3>
  </div>
{/snippet}

{#snippet stationStatus()}
  {#if scan.phase === 'scanning'}
    {#if scan.progress}
      <p>
        Обработано файлов: {scan.progress.processedFiles}{scan.progress.totalFiles > 0
          ? ` из ${scan.progress.totalFiles}`
          : ''}. Проверено: {scan.progress.scannedFiles}.
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
    {#if scan.report.mode === 'signs'}
      <p>Результат проверки по признакам, без базы сигнатур.</p>
    {/if}
    {#if scan.report.incomplete || scan.report.errors > 0 || scan.report.skippedFiles > 0}
      <p class="alert" role="status">
        Проверено не всё: некоторые файлы или папки недоступны, пропущены либо превышают лимиты проверки.
      </p>
    {/if}
  {:else}
    <p>Готово к проверке.</p>
  {/if}
{/snippet}

{#if subsection === 'scan'}
  {#if error}
    <div class="alert" role="alert"><p>{error.message}</p></div>
  {/if}

  <div class="scan-screen">
    <section
      class="card scanner"
      data-testid="scan-station"
      data-state={stationState}
      aria-label="Проверка компьютера"
    >
      <div class="scanner-body">
        <button
          type="button"
          class="gauge"
          style:--fill={`${acidLevel * 100}%`}
          style:--number-scale={Math.min(29, 135 / gaugeValue.length)}
          onclick={start}
          disabled={busy || engine === null}
          aria-label={gaugeLabel}
        >
          <AcidScan level={acidLevel} running={scan.phase === 'scanning'} />
          <span class="readout">
            <span class="value" data-testid={!busy || scan.progress?.totalFiles ? 'scan-percent' : undefined}>{gaugeValue}</span>
            <span class="caption">{gaugeCaption}</span>
          </span>
          <span class="readout readout-ink" aria-hidden="true">
            <span class="value" data-ink={gaugeValue}></span>
            <span class="caption" data-ink={gaugeCaption}></span>
          </span>
        </button>

        <div class="scanner-details">
          <dl class="stats">
            <div class="stat">
              <dt>Проверено</dt>
              <dd data-testid="scan-scanned">{statScanned}</dd>
            </div>
            <div class="stat">
              <dt>Находки</dt>
              <dd data-testid="scan-findings">{findings.length}</dd>
            </div>
            <div class="stat">
              <dt>Пропущено</dt>
              <dd data-testid="scan-skipped">{statSkipped}</dd>
            </div>
            <div class="stat">
              <dt>Ошибки</dt>
              <dd data-testid="scan-errors">{statErrors}</dd>
            </div>
          </dl>

        </div>
      </div>

      <div class="track" aria-hidden="true">
        {#if scan.phase === 'scanning'}
          <div
            class="sweep"
            class:indeterminate={scan.progress?.totalFiles === 0}
            data-testid="scan-sweep"
            style:width={`${scan.progress?.totalFiles === 0 ? 100 : sweepWidth}%`}
          ></div>
        {/if}
      </div>

      <div class="actions">
        {#if scan.phase === 'scanning'}
          <button type="button" class="btn" onclick={() => void cancelScan()}>Остановить</button>
        {:else}
          <button type="button" class="btn go" onclick={start} disabled={busy || engine === null}>
            Проверить компьютер
          </button>
        {/if}
      </div>

      <div class="folder">
        <button type="button" class="folder-toggle" aria-expanded={folderOpen}
          onclick={() => (folderOpen = !folderOpen)}>
          <svg viewBox="0 0 24 24" fill="none" aria-hidden="true"><path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2Z" stroke="currentColor" stroke-width="1.7" stroke-linejoin="round"/></svg>
          <span>Проверить отдельную папку</span>
          <span class="chevron" class:expanded={folderOpen} aria-hidden="true"></span>
        </button>
        <div class="folder-reveal" class:open={folderOpen} inert={!folderOpen}>
          <div class="folder-clip"><div class="folder-content">
            <label>
              Папка для проверки
              <input bind:value={scan.target} placeholder="Укажите путь или выберите папку" disabled={busy || choosingFolder} />
            </label>
            <div class="folder-actions">
              <button type="button" class="btn" onclick={() => void chooseFolder()} disabled={busy || choosingFolder}>
                Выбрать папку
              </button>
              <button type="button" class="btn go" onclick={startFolder} disabled={busy || choosingFolder || engine === null || !scan.target.trim()}>
                Проверить папку
              </button>
            </div>
            {#if folderNotice}<p class="empty" role="status">{folderNotice}</p>{/if}
          </div></div>
        </div>
      </div>

      {#if !busy && !scan.report}
        <p class="hint">Проверка доступных файлов на локальных дисках компьютера.</p>
      {/if}

      <div class="msg" aria-live="polite">
        {@render stationStatus()}
      </div>

      {#if engine && !engine.available}
        {#if signatures?.enginePresent}
          <p class="empty" data-testid="signatures-missing">
            Базы проверки не загружены. Доступна проверка по признакам.
          </p>
        {:else if signatures?.enginePresent === false}
          <p class="empty" data-testid="engine-unavailable">
            Встроенный движок вирусов в этой сборке не собран, поэтому проверка идёт по признакам:
            программа ищет подозрительные файлы. Подробности — в «Журнале».
          </p>
        {:else}
          <p class="empty" data-testid="signatures-unknown">
            Проверка идёт по признакам: базы вирусов пока не загружены, а сведения о движке не
            получены. Если это не так — перезапустите программу.
          </p>
        {/if}
      {/if}

      {#if engine?.available && signatures?.signatures}
        <p class="hint" data-testid="signatures-loaded">
          База загружена: сигнатур — {signatures.signatures}.
        </p>
      {/if}
    </section>

    <div class="bottom">
      <section class="findings-panel" aria-label="Находки проверки">
        <div class="panel-head">
          {@render panelTitle('Находки', 'findings')}
          <span class="selection-count">Выбрано: {pickedCount}</span>
        </div>
        {#if actionNotice}
          <p class="empty" role="status" data-testid="finding-action">{actionNotice}</p>
        {/if}
        <div class="list-scroll">
          {#if findings.length === 0}
            <div class="empty-state">{@render findingsEmpty()}</div>
          {:else}
            <FindingsList
              {findings}
              suspicions={scan.suspicions}
              selected={picked}
              onToggle={toggleFinding}
              onOpenMenu={openFindingMenu}
            />
          {/if}
        </div>
      </section>

      <aside class="bin-card" aria-label="Корзина находок">
        <VirusBin count={findings.length} state={binState} />
        <RemoveSelected disabled={!canRemove} reason={removeReason} onremove={removeSelected} />
      </aside>
    </div>

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
  <section class="findings-panel workspace" aria-label="Находки проверки">
    <div class="panel-head">
      {@render panelTitle('Находки', 'findings')}
      {#if findings.length > 0}
        <span class="selection-count">Выбрано: {pickedCount}</span>
      {/if}
    </div>
    {#if findings.length === 0}
      <div class="empty-state">{@render findingsEmpty()}</div>
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
    {#if actionNotice}
      <p class="empty" role="status" data-testid="finding-action">{actionNotice}</p>
    {/if}
    <FindingsList
      {findings}
      suspicions={scan.suspicions}
      selected={picked}
      onToggle={toggleFinding}
      onOpenMenu={openFindingMenu}
    />
    <div class="findings-tools">
      <RemoveSelected disabled={!canRemove} reason={removeReason} onremove={removeSelected} />
    </div>
    {/if}
  </section>
{/if}

{#if subsection === 'log'}
  <div class="journal">
  <section class="findings-panel workspace" aria-label="Журнал проверки">
    <div class="panel-head">{@render panelTitle('Журнал проверки', 'log')}</div>
    {#if scan.runs.length === 0}
      <div class="empty-state"><p class="empty">Проверок в этой сессии пока не было.</p></div>
    {:else}
      <ul class="log selectable">
        {#each scan.runs as run (run.id)}
          <li class="run-row" data-status={run.status}>
            <time>{run.startedAt}</time>
            <div class="run-detail">
              <strong>{run.target}</strong>
              <p>{runSummary(run)}</p>
            </div>
          </li>
        {/each}
      </ul>
    {/if}
  </section>

  <section class="quarantine-block findings-panel" aria-label="Карантин от проверок">
    <div class="panel-head">{@render panelTitle('Карантин от проверок', 'quarantine')}</div>
    {#if journalNotice}
      <p class="empty" role="status">{journalNotice}</p>
    {/if}
    {#if journalEntries.length === 0}
      <div class="empty-state"><p class="empty">В карантине от проверок пусто.</p></div>
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
  {#if engine && !engine.available}
    <details class="engine-details">
      <summary>Сведения о базах проверки</summary>
      <p class="empty selectable" data-testid="engine-log">
        Почему проверка по базе недоступна: {engine.reason}. Технические подробности:
        docs/status/clamav-build.md.
      </p>
      {#if signatures?.enginePresent && signatures.dir}
        <p class="path selectable">{signatures.dir}</p>
      {/if}
    </details>
  {/if}
  </div>
{/if}

{#if menu}
  <ContextMenu
    label="Действия с находкой"
    items={findingMenuItems()}
    x={menu.x}
    y={menu.y}
    onselect={(id) => void onFindingMenuSelect(id)}
    onclose={() => void closeFindingMenu()}
  />
{/if}

<style>
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

  .quarantine-list li {
    padding: 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--surface-alt);
    color: var(--ink-soft);
  }

  .quarantine-list {
    display: grid;
    gap: 8px;
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

  .scan-screen {
    position: relative;
    display: flex;
    flex: 1 1 auto;
    flex-direction: column;
    gap: 16px;
    min-height: 0;
  }

  /* Карточка проверки в духе карточки скорости из «Сети»: большая шкала, цифры, одна кнопка. */
  .card.scanner {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 16px;
    border-radius: var(--radius-md);
    background: var(--surface-alt);
    box-shadow: 0 0 0 2px #000;
  }

  .scanner-body {
    display: grid;
    grid-template-columns: minmax(150px, 1fr) minmax(220px, 1.4fr);
    align-items: stretch;
    gap: 12px;
  }

  .gauge {
    position: relative;
    container-type: inline-size;
    display: grid;
    place-items: center;
    width: 100%;
    aspect-ratio: 1;
    min-width: 0;
    overflow: hidden;
    border: 0;
    border-radius: var(--radius-sm);
    background: radial-gradient(ellipse at 50% 110%, #243719, #191c19 62%, #141614);
    padding: 0;
    font: inherit;
  }

  .gauge:not(:disabled) { cursor: pointer; }
  .gauge:disabled { opacity: 1; }
  .gauge:hover:not(:disabled), .gauge:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }

  .readout {
    position: absolute;
    inset: 0;
    z-index: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    max-width: 100%;
    padding: 8px;
    text-align: center;
    text-shadow: none;
    pointer-events: none;
    color: #fff;
  }

  .readout-ink {
    color: #182000;
    clip-path: inset(calc(100% - var(--fill)) 0 0);
    transition: clip-path 620ms cubic-bezier(0.23, 0.78, 0.32, 1);
  }

  .readout-ink [data-ink]::before {
    content: attr(data-ink);
  }

  .value {
    max-width: 100%;
    overflow-wrap: anywhere;
    font-size: clamp(1.7rem, calc(var(--number-scale) * 1cqi), 6.7rem);
    font-weight: 800;
    line-height: 1;
    font-variant-numeric: tabular-nums;
  }

  .caption {
    margin-top: 4px;
    font-size: clamp(0.82rem, 6cqi, 1.32rem);
    font-weight: 750;
  }

  .scanner-details {
    display: flex;
    flex-direction: column;
    min-width: 0;
    gap: 8px;
  }

  .stats {
    display: grid;
    flex: 1;
    min-width: 0;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 8px;
    margin: 0;
  }

  .stat {
    min-width: 0;
    padding: 8px 10px;
    border-radius: var(--radius-sm);
    background: #202020;
    text-align: center;
  }

  .stat dt {
    font-size: var(--text-caption);
    font-weight: 800;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--muted);
  }

  .stat dd {
    margin: 2px 0 0;
    font-size: clamp(1.6rem, 4cqi, 2.6rem);
    font-weight: 800;
    line-height: 1.05;
    font-variant-numeric: tabular-nums;
    color: var(--ink);
  }

  .folder {
    overflow: hidden;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--surface);
  }

  .folder-toggle {
    display: grid;
    grid-template-columns: 20px minmax(0, 1fr) 12px;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 10px 14px;
    background: transparent;
    box-shadow: none;
    text-align: left;
  }

  .folder-toggle:hover { background: var(--surface-alt); }
  .folder-toggle svg { width: 18px; color: var(--accent); }
  .folder-toggle:focus-visible { outline: 2px solid var(--accent); outline-offset: -2px; }

  .chevron {
    width: 8px;
    height: 8px;
    border: solid var(--muted);
    border-width: 0 2px 2px 0;
    transform: translateY(-2px) rotate(45deg);
    transition: transform 230ms ease;
  }

  .chevron.expanded { transform: translateY(2px) rotate(225deg); }

  .folder-reveal {
    display: grid;
    grid-template-rows: 0fr;
    visibility: hidden;
    transition: grid-template-rows 230ms ease, visibility 0s linear 230ms;
  }

  .folder-reveal.open {
    grid-template-rows: 1fr;
    visibility: visible;
    transition-delay: 0s;
  }

  .folder-clip { min-height: 0; overflow: hidden; }

  .folder-content {
    display: grid;
    gap: 8px;
    padding: 12px 14px;
    border-top: 1px solid var(--border);
  }

  .folder-content label { margin: 0; }
  .folder-content input { min-width: 0; }
  .folder-actions { display: flex; flex-wrap: wrap; gap: 8px; }
  .folder-actions .btn { width: auto; flex: 1 0 auto; }

  .btn {
    width: 100%;
    min-height: 38px;
    margin-top: 4px;
    padding: 6px 18px;
    border: 0;
    border-radius: var(--radius-sm);
    background: var(--btn-bg);
    color: var(--ink);
    box-shadow: inset 0 0 0 1px var(--btn-border);
    font: inherit;
    font-size: var(--text-caption);
    font-weight: 800;
    cursor: pointer;
  }

  .btn.go {
    background: var(--accent);
    color: var(--black);
    box-shadow: none;
  }

  .btn:disabled {
    opacity: 0.55;
    cursor: not-allowed;
  }

  .btn:focus-visible {
    outline: 2px solid var(--ink);
    outline-offset: 2px;
  }

  .msg {
    margin: 0;
    font-size: var(--text-caption);
    color: var(--ink-soft);
  }

  .msg p {
    margin: 0 0 4px;
  }

  .bottom {
    display: grid;
    flex: 1 1 auto;
    grid-template-columns: minmax(0, 2fr) minmax(220px, 1fr);
    gap: 16px;
    min-height: 0;
  }

  .bin-card {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    padding: 16px;
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface-alt);
  }

  .scanner {
    container-type: inline-size;
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

  .findings-panel {
    display: flex;
    flex-direction: column;
    min-height: 0;
    padding: 16px;
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface);
    font-size: var(--text-caption);
  }

  .panel-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    margin-bottom: 12px;
  }

  .panel-head h3 {
    margin: 0;
  }

  .panel-title { display: flex; align-items: center; gap: 10px; min-width: 0; }
  .panel-icon {
    display: grid;
    flex: 0 0 34px;
    place-items: center;
    height: 34px;
    border-radius: var(--radius-sm);
    background: var(--accent);
    color: var(--black);
  }
  .panel-icon svg { width: 20px; }
  .panel-icon[data-kind='quarantine'] { background: var(--surface-alt); color: var(--accent); }

  .selection-count {
    flex: none;
    padding: 5px 9px;
    border-radius: var(--radius-pill);
    background: var(--surface-alt);
    color: var(--ink-soft);
    font-variant-numeric: tabular-nums;
  }

  .empty-state {
    display: grid;
    align-items: center;
    min-height: 60px;
    padding: 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--surface-alt);
  }

  .workspace { min-width: 0; }
  .findings-tools { padding-top: 8px; }
  .journal { display: grid; gap: 12px; }

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

  .sweep.indeterminate {
    opacity: 0.45;
  }

  .empty {
    margin: 0;
    color: var(--ink-soft);
  }

  .hint {
    margin: 0;
    color: var(--muted);
    font-size: var(--text-small);
  }

  @media (prefers-reduced-motion: reduce) {
    .sweep, .readout-ink, .chevron, .folder-reveal {
      transition: none;
    }

    .drop {
      animation: none;
    }
  }

  /* Наш переключатель «меньше движения» в «Оформлении» делает то же самое. */
  :global(:root[data-reduce-motion='true']) .sweep,
  :global(:root[data-reduce-motion='true']) .readout-ink,
  :global(:root[data-reduce-motion='true']) .folder-reveal,
  :global(:root[data-reduce-motion='true']) .chevron {
    transition: none;
  }

  :global(:root[data-reduce-motion='true']) .drop {
    animation: none;
  }

  .log {
    display: grid;
    gap: 7px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .run-row {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    gap: 12px;
    padding: 11px 12px;
    border: 1px solid var(--border);
    border-inline-start: 3px solid var(--accent);
    border-radius: var(--radius-sm);
    background: var(--surface-alt);
  }

  .run-row[data-status='cancelled'] { border-inline-start-color: var(--muted); }
  .run-row[data-status='failed'] { border-inline-start-color: var(--danger, #ff758d); }
  .run-row time { color: var(--muted); font-variant-numeric: tabular-nums; }
  .run-detail { overflow-wrap: anywhere; color: var(--ink); }
  .run-detail p { margin: 3px 0 0; color: var(--ink-soft); }
  .engine-details {
    padding: 12px 16px;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
    color: var(--muted);
    font-size: var(--text-small);
  }
  .engine-details summary { cursor: pointer; }
  .engine-details p { margin: 8px 0 0; }

  .path {
    color: var(--muted);
    word-break: break-all;
  }

  small {
    color: var(--muted);
  }

  @media (max-width: 980px) {
    .scan-screen {
      flex: none;
    }

    .bottom {
      grid-template-columns: minmax(0, 1fr);
    }

    .list-scroll {
      overflow: visible;
    }
  }

  @media (max-width: 720px) {
    .scan-screen {
      gap: 8px;
    }

    .scanner-body {
      grid-template-columns: minmax(0, 1fr);
    }

    .gauge {
      width: min(100%, 240px);
      justify-self: center;
    }

    .findings-panel {
      padding: 12px;
    }
  }
</style>
