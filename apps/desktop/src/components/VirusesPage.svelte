<script lang="ts">
  import Check from './Check.svelte';
  import ContextMenu, { type MenuItem } from './ContextMenu.svelte';
  import FindingsList from './FindingsList.svelte';
  import RemoveSelected from './RemoveSelected.svelte';
  import VirusBin from './VirusBin.svelte';
  import { onDestroy, onMount, tick } from 'svelte';
  import { clamavSignatures, clamavStatus, defaultScanTarget, toCommandError } from '../lib/api';
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

  const startBlockReason = $derived(
    busy
      ? ''
      : !engine?.available
        ? 'Проверка пока недоступна'
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
    // Поле не затираем, если проверка уже идёт или путь введён раньше. Значение по умолчанию
    // подставляем, только если владелец ещё ничего не ввёл, пока шёл запрос.
    if (scan.target === '') {
      try {
        const fallback = await defaultScanTarget();
        if (scan.target === '') scan.target = fallback;
      } catch {
        // Пустое поле — пользователь укажет папку сам.
      }
    }
  });

  /** Открывает папку баз в Проводнике: владелец кладёт туда файлы базы сам. */
  async function openSignaturesDir() {
    const dir = signatures?.dir ?? '';
    if (dir === '') return;
    actionNotice = '';
    try {
      await revealInExplorer(dir);
    } catch (err) {
      actionNotice = toProgramsError(err).message;
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
    {#if scan.report.incomplete || scan.report.errors > 0 || scan.report.skippedFiles > 0}
      <p class="alert" role="status">
        Проверено не всё: часть папок недоступна или слишком много файлов.
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

        {#if engine && !engine.available}
          {#if signatures?.enginePresent}
            <p class="empty" data-testid="signatures-missing">
              Базы проверки не загружены. Положите файлы базы в папку:
              <span class="path selectable">{signatures.dir}</span>
            </p>
            <div class="actions">
              <button type="button" onclick={() => void openSignaturesDir()}>
                Открыть папку баз
              </button>
            </div>
          {:else}
            <p class="empty" data-testid="engine-unavailable">
              Проверка пока недоступна: встроенная защита ещё не готова. Подробности — в «Журнале».
            </p>
          {/if}
        {/if}

        {#if engine?.available && signatures?.signatures}
          <p class="hint" data-testid="signatures-loaded">
            База загружена: сигнатур — {signatures.signatures}.
          </p>
        {/if}

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
        <RemoveSelected disabled={!canRemove} reason={removeReason} onremove={removeSelected} />
      </div>
    </div>

    <section class="findings-panel" aria-label="Находки проверки">
      <div class="panel-head">
        <h3>Находки</h3>
        <span class="reason">Выбрано: {pickedCount}</span>
      </div>
      {#if actionNotice}
        <p class="empty" role="status" data-testid="finding-action">{actionNotice}</p>
      {/if}
      <div class="list-scroll">
        {#if findings.length === 0}
          {@render findingsEmpty()}
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
      <RemoveSelected disabled={!canRemove} reason={removeReason} onremove={removeSelected} />
    </div>
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
  {/if}
{/if}

{#if subsection === 'log'}
  {#if engine && !engine.available}
    <p class="empty panel selectable" data-testid="engine-log">
      Почему проверка недоступна: {engine.reason}. Технические подробности:
      docs/status/clamav-build.md.
    </p>
  {/if}

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
    flex-wrap: wrap;
    align-items: center;
    justify-content: center;
    gap: 16px;
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

  .hint {
    margin: 0;
    color: var(--muted);
    font-size: var(--text-small);
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
