<script lang="ts">
  import { tick, untrack } from 'svelte';
  import AppIcon from './AppIcon.svelte';
  import ContextMenu, { type MenuItem } from './ContextMenu.svelte';
  import {
    commandExePath,
    filterPrograms,
    kindLabel,
    measureBlockReason,
    NUMERIC_PROGRAM_KEYS,
    sortPrograms,
    startupActionLabel,
    type ProgramSortKey,
  } from '../lib/programs';
  import { ariaSort, nextSort, type SortState } from '../lib/sortState';
  import { formatSize } from '../lib/units';
  import {
    getIcon,
    getProgramIcon,
    prefetchPathIcons,
    prefetchProgramIcons,
    programIconKey,
    listInstalled,
    listStartup,
    measureFolder,
    setStartupEnabled,
    stopMeasure,
    toProgramsError,
    uninstallProgram,
    type FolderMeasure,
    type InstalledProgram,
    type ProgramsErrorInfo,
    type StartupEntry,
  } from '../lib/programsApi';

  let { subsection }: { subsection: string } = $props();

  const NONE_SELECTED = 'Сначала выбери программу';

  let loading = $state(false);
  let error = $state<ProgramsErrorInfo | null>(null);
  let notice = $state('');

  // Установленные
  let programs = $state<InstalledProgram[]>([]);
  let loadedInstalled = $state(false);
  let query = $state('');
  let selectedId = $state<string | null>(null);
  let measuring = $state(false);
  let measures = $state<Record<string, FolderMeasure>>({});
  /** При открытии: по размеру от большего к меньшему; дальше сортирует пользователь. */
  let sort = $state<SortState<ProgramSortKey> | null>({ key: 'size', dir: 'desc' });
  /** Программа, ожидающая подтверждения удаления. Без подтверждения ядро не вызывается. */
  let pendingUninstall = $state<InstalledProgram | null>(null);
  let menu = $state<{ program: InstalledProgram; x: number; y: number } | null>(null);
  let menuReturnFocus: HTMLElement | null = null;
  let cancelButton = $state<HTMLButtonElement | null>(null);

  // Автозагрузка
  let startup = $state<StartupEntry[]>([]);
  let loadedStartup = $state(false);
  let startupKey = $state<string | null>(null);
  let pendingStartup = $state<StartupEntry | null>(null);

  const COLUMNS: Array<{ key: ProgramSortKey; label: string; cls: string }> = [
    { key: 'name', label: 'Программа', cls: 'c-name' },
    { key: 'size', label: 'Размер', cls: 'c-size' },
    { key: 'disk', label: 'Диск', cls: 'c-disk' },
    { key: 'version', label: 'Версия', cls: 'c-version' },
    { key: 'kind', label: 'Тип', cls: 'c-kind' },
    { key: 'publisher', label: 'Издатель', cls: 'c-publisher' },
  ];

  const visible = $derived(sortPrograms(filterPrograms(programs, query), sort));
  const selected = $derived(programs.find((program) => program.id === selectedId) ?? null);
  const selectedMeasure = $derived(
    selected?.installLocation ? (measures[selected.installLocation] ?? null) : null,
  );
  const selectedBlock = $derived(selected ? measureBlockReason(selected) : null);
  const selectedStartup = $derived(
    startup.find((entry) => entry.source + '\u0000' + entry.key === startupKey) ?? null,
  );

  function uninstallBlock(program: InstalledProgram | null): string | null {
    if (!program) return NONE_SELECTED;
    return program.canUninstall ? null : (program.uninstallReason ?? 'Удаление недоступно');
  }

  function fail(err: unknown) {
    error = toProgramsError(err);
  }

  async function loadInstalled() {
    loading = true;
    error = null;
    try {
      programs = await listInstalled();
      prefetchProgramIcons(programs);
      loadedInstalled = true;
      if (!programs.some((program) => program.id === selectedId)) selectedId = null;
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
      prefetchPathIcons(
        startup.flatMap((entry) => {
          const path = commandExePath(entry.command);
          return path ? [path] : [];
        }),
      );
      loadedStartup = true;
      if (!startup.some((entry) => entry.source + '\u0000' + entry.key === startupKey)) {
        startupKey = null;
      }
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

  // Фокус на «Отмена» в диалогах: случайный Enter ничего не удаляет и не отключает.
  $effect(() => {
    if ((pendingUninstall || pendingStartup) && cancelButton) cancelButton.focus();
  });

  function select(program: InstalledProgram) {
    selectedId = selectedId === program.id ? null : program.id;
    notice = '';
  }

  function sortBy(column: (typeof COLUMNS)[number]) {
    sort = nextSort(sort, column.key, NUMERIC_PROGRAM_KEYS.includes(column.key) ? 'desc' : 'asc');
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

  function showPath(program: InstalledProgram) {
    error = null;
    notice = program.installLocation
      ? `Путь: ${program.installLocation}`
      : 'Папка установки в реестре не указана.';
  }

  function askUninstall(program: InstalledProgram) {
    if (uninstallBlock(program)) return;
    error = null;
    notice = '';
    pendingUninstall = program;
  }

  async function confirmUninstall() {
    const target = pendingUninstall;
    if (!target) return;
    pendingUninstall = null;
    try {
      await uninstallProgram(target.id, true);
      notice = `Запущено удаление «${target.name}». Когда деинсталлятор закончит, нажми «Обновить».`;
    } catch (err) {
      fail(err);
    }
  }

  function openMenu(event: MouseEvent, program: InstalledProgram) {
    event.preventDefault();
    selectedId = program.id;
    const target = event.currentTarget as HTMLElement;
    menuReturnFocus = target.querySelector<HTMLElement>('button.name-btn') ?? target;
    let { clientX: x, clientY: y } = event;
    if (x === 0 && y === 0) {
      // Клавиатурный вызов (Shift+F10, клавиша меню): рядом с названием строки.
      const rect = menuReturnFocus.getBoundingClientRect();
      x = rect.left + 24;
      y = rect.bottom;
    }
    menu = { program, x, y };
  }

  async function closeMenu() {
    menu = null;
    await tick();
    menuReturnFocus?.focus();
  }

  function menuItems(program: InstalledProgram): MenuItem[] {
    const block = measureBlockReason(program);
    const removal = uninstallBlock(program);
    return [
      {
        id: 'measure',
        label: 'Размер папки',
        disabled: measuring || block !== null,
        reason: block,
      },
      { id: 'path', label: 'Показать путь' },
      { id: 'uninstall', label: 'Удалить…', disabled: removal !== null, reason: removal },
    ];
  }

  async function onMenuSelect(id: string) {
    const program = menu?.program;
    if (!program) return;
    menu = null;
    if (id === 'measure') {
      menuReturnFocus?.focus();
      await measure(program);
    } else if (id === 'path') {
      menuReturnFocus?.focus();
      showPath(program);
    } else if (id === 'uninstall') {
      askUninstall(program);
    }
  }

  function selectStartup(entry: StartupEntry) {
    const key = entry.source + '\u0000' + entry.key;
    startupKey = startupKey === key ? null : key;
    notice = '';
  }

  function askStartup(entry: StartupEntry) {
    error = null;
    notice = '';
    pendingStartup = entry;
  }

  async function confirmStartup() {
    const target = pendingStartup;
    if (!target) return;
    pendingStartup = null;
    const enable = !target.enabled;
    try {
      const result = await setStartupEnabled(target, enable, true);
      notice =
        `«${target.name}»: было ${result.previousEnabled ? 'включено' : 'отключено'}, ` +
        `стало ${result.enabled ? 'включено' : 'отключено'}. ` +
        `Вернуть можно кнопкой «${startupActionLabel(!result.enabled)}».`;
      await loadStartup();
    } catch (err) {
      fail(err);
    }
  }

  function onDialogKey(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      pendingUninstall = null;
      pendingStartup = null;
    }
  }

  function startupIconSource(entry: StartupEntry) {
    const path = commandExePath(entry.command);
    return path ? () => getIcon(path) : null;
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

{#snippet sortHead(column: (typeof COLUMNS)[number])}
  <span class={column.cls} role="columnheader" aria-sort={ariaSort(sort, column.key)}>
    <button
      type="button"
      class="sort"
      class:is-active={sort?.key === column.key}
      onclick={() => sortBy(column)}
      title="Сортировать: {column.label}"
    >
      {column.label}
      <svg
        class="arrow"
        class:is-asc={sort?.key === column.key && sort.dir === 'asc'}
        viewBox="0 0 10 10"
        aria-hidden="true"><path d="M5 8.5 1.5 3h7z" fill="currentColor" /></svg
      >
    </button>
  </span>
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
        title={!selected ? NONE_SELECTED : (selectedBlock ?? 'Посчитать размер папки')}
        aria-describedby={selectedBlock ? 'measure-reason' : undefined}
      >
        Размер папки
      </button>
      {#if measuring}
        <button type="button" onclick={() => void stop()}>Стоп</button>
      {/if}
      <button
        type="button"
        class="danger"
        disabled={uninstallBlock(selected) !== null}
        title={uninstallBlock(selected) ?? 'Запустить штатное удаление программы'}
        aria-describedby="delete-reason"
        onclick={() => selected && askUninstall(selected)}
      >
        Удалить
      </button>
    </div>

    <p class="status-line" aria-live="polite">
      {#if loadedInstalled}Приложений: {visible.length} из {programs.length}.{/if}
      Размер — оценка установщика; «Нет данных» значит, что размер не указан. Правый клик по строке —
      меню.
    </p>

    {@render messages()}

    {#if loading && !loadedInstalled}
      <p class="status-line" role="status">Читаю список программ…</p>
    {:else if loadedInstalled && visible.length === 0}
      <p class="status-line">Ничего не найдено.</p>
    {:else}
      <div class="grid" role="table" aria-label="Программы" aria-rowcount={visible.length + 1}>
        <div class="grid-head" role="row">
          <span class="c-icon" role="columnheader"><span class="sr-only">Значок</span></span>
          {#each COLUMNS as column (column.key)}
            {@render sortHead(column)}
          {/each}
        </div>
        <div class="grid-body" role="rowgroup">
          {#each visible as program (program.id)}
            <!-- svelte-ignore a11y_click_events_have_key_events, a11y_interactive_supports_focus -->
            <!-- Клик по строке — удобство для мыши; с клавиатуры строку выбирает кнопка с названием. -->
            <div
              class="row"
              role="row"
              class:is-selected={program.id === selectedId}
              onclick={() => select(program)}
              oncontextmenu={(event) => openMenu(event, program)}
            >
              <span class="c-icon" role="cell">
                <AppIcon
                  name={program.name}
                  cacheKey={programIconKey(program.id)}
                  resolve={() => getProgramIcon(program.id)}
                  size={30}
                />
              </span>
              <span class="c-name cut" role="cell">
                <button
                  type="button"
                  class="name-btn"
                  aria-pressed={program.id === selectedId}
                  onclick={(event) => {
                    event.stopPropagation();
                    select(program);
                  }}>{program.name}</button
                >
              </span>
              <span class="c-size" role="cell" class:is-empty={program.sizeBytes === null}
                >{program.sizeBytes === null ? 'Нет данных' : formatSize(program.sizeBytes)}</span
              >
              <span class="c-disk" role="cell" class:is-empty={!program.diskLetter}
                >{program.diskLetter ? `${program.diskLetter}\\` : 'Нет данных'}</span
              >
              <span class="c-version cut" role="cell">{program.version ?? '—'}</span>
              <span class="c-kind" role="cell">{kindLabel(program.kind)}</span>
              <span class="c-publisher cut" role="cell">{program.publisher ?? '—'}</span>
            </div>
          {/each}
        </div>
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
            Размер: {formatSize(selectedMeasure.bytes)}, файлов: {selectedMeasure.files}
          </p>
          {#if !selectedMeasure.complete}
            <p class="reason">Подсчёт неполный: достигнут лимит записей. Цифра — нижняя оценка.</p>
          {/if}
        {/if}
      {:else}
        Выбери программу
      {/if}
      <p id="delete-reason" class="reason">
        {#if selected && !uninstallBlock(selected)}
          Удалить: откроется штатная программа удаления, перед этим спросим подтверждение.
        {:else}
          Удалить недоступно: {uninstallBlock(selected)}
        {/if}
      </p>
    </div>
  </section>

  {#if menu}
    <ContextMenu
      label="Действия с программой «{menu.program.name}»"
      items={menuItems(menu.program)}
      x={menu.x}
      y={menu.y}
      onselect={(id) => void onMenuSelect(id)}
      onclose={() => void closeMenu()}
    />
  {/if}

  {#if pendingUninstall}
    <div class="backdrop">
      <div
        class="dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="uninstall-title"
        aria-describedby="uninstall-text"
        tabindex="-1"
        onkeydown={onDialogKey}
      >
        <h3 id="uninstall-title">Удалить программу?</h3>
        <p id="uninstall-text">
          Откроется штатная программа удаления «{pendingUninstall.name}». Отменить удаление после её
          запуска нельзя: вернуть программу можно только повторной установкой.
        </p>
        <div class="dialog-actions">
          <button type="button" bind:this={cancelButton} onclick={() => (pendingUninstall = null)}
            >Отмена</button
          >
          <button type="button" class="danger" onclick={() => void confirmUninstall()}
            >Удалить</button
          >
        </div>
      </div>
    </div>
  {/if}
{/if}

{#if subsection === 'startup'}
  <section class="page" aria-label="Автозагрузка">
    <p class="status-line">
      Реестр «Run» (HKCU и HKLM) и папки «Автозагрузка». Отключение меняет флаг, который читает
      «Диспетчер задач»: прежнее состояние запоминается, запись можно включить обратно.
    </p>
    <div class="buttons">
      <button type="button" onclick={() => void loadStartup()} disabled={loading}>Обновить</button>
      <button
        type="button"
        disabled={!selectedStartup}
        title={selectedStartup
          ? startupActionLabel(selectedStartup.enabled)
          : 'Сначала выбери запись'}
        onclick={() => selectedStartup && askStartup(selectedStartup)}
      >
        {startupActionLabel(selectedStartup?.enabled ?? true)}
      </button>
    </div>

    {@render messages()}

    {#if loading && !loadedStartup}
      <p class="status-line" role="status">Читаю автозагрузку…</p>
    {:else if loadedStartup && startup.length === 0}
      <p class="status-line">Автозагрузка пуста.</p>
    {:else}
      <div class="grid" role="table" aria-label="Записи автозагрузки">
        <div class="grid-head startup" role="row">
          <span class="c-icon" role="columnheader"><span class="sr-only">Значок</span></span>
          <span class="c-name" role="columnheader"><span class="head-text">Название</span></span>
          <span class="c-state" role="columnheader"><span class="head-text">Состояние</span></span>
          <span class="c-where" role="columnheader"><span class="head-text">Где</span></span>
          <span class="c-command" role="columnheader"><span class="head-text">Команда</span></span>
        </div>
        <div class="grid-body" role="rowgroup">
          {#each startup as entry (entry.source + entry.key)}
            <!-- svelte-ignore a11y_click_events_have_key_events, a11y_interactive_supports_focus -->
            <!-- Клик по строке — удобство для мыши; с клавиатуры строку выбирает кнопка с названием. -->
            <div
              class="row startup"
              role="row"
              class:is-selected={entry.source + '\u0000' + entry.key === startupKey}
              onclick={() => selectStartup(entry)}
            >
              <span class="c-icon" role="cell">
                <AppIcon
                  name={entry.name}
                  cacheKey={commandExePath(entry.command)}
                  resolve={startupIconSource(entry)}
                  size={30}
                />
              </span>
              <span class="c-name cut" role="cell">
                <button
                  type="button"
                  class="name-btn"
                  aria-pressed={entry.source + '\u0000' + entry.key === startupKey}
                  onclick={(event) => {
                    event.stopPropagation();
                    selectStartup(entry);
                  }}>{entry.name}</button
                >
              </span>
              <span class="c-state" role="cell">
                <span class="pill" class:is-off={!entry.enabled}
                  >{entry.enabled ? 'Включена' : 'Отключена'}</span
                >
                {#if entry.previousEnabled !== null}
                  <span class="was">раньше: {entry.previousEnabled ? 'включена' : 'отключена'}</span
                  >
                {/if}
              </span>
              <span class="c-where cut" role="cell">{entry.location}</span>
              <span class="c-command cut path" role="cell">{entry.command ?? '—'}</span>
            </div>
          {/each}
        </div>
      </div>
    {/if}
  </section>

  {#if pendingStartup}
    <div class="backdrop">
      <div
        class="dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="startup-title"
        aria-describedby="startup-text"
        tabindex="-1"
        onkeydown={onDialogKey}
      >
        <h3 id="startup-title">{startupActionLabel(pendingStartup.enabled)}?</h3>
        <p id="startup-text">
          {#if pendingStartup.enabled}
            «{pendingStartup.name}» перестанет запускаться вместе с Windows. Прежнее состояние
            (включено) запомним, включить обратно можно в любой момент.
          {:else}
            «{pendingStartup.name}» снова будет запускаться вместе с Windows.
          {/if}
          {#if pendingStartup.source === 'hklmRun' || pendingStartup.source === 'commonFolder'}
            Для записей «всех пользователей» могут понадобиться права администратора.
          {/if}
        </p>
        <div class="dialog-actions">
          <button type="button" bind:this={cancelButton} onclick={() => (pendingStartup = null)}
            >Отмена</button
          >
          <button type="button" class="danger" onclick={() => void confirmStartup()}
            >{pendingStartup.enabled ? 'Отключить' : 'Включить'}</button
          >
        </div>
      </div>
    </div>
  {/if}
{/if}

<style>
  .page {
    display: flex;
    flex-direction: column;
    gap: 8px;
    color: var(--page-text);
  }
  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
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
  button.danger {
    border-color: transparent;
  }
  .dialog button.danger {
    background: #9b1c1c;
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
    overflow-wrap: anywhere;
  }
  .alert {
    color: #ff8a8a;
  }

  /* Список: сетка из div с ролями таблицы. Выделение — скруглённая рамка цвета раздела. */
  .grid {
    max-height: calc(100vh - 440px);
    min-height: 180px;
    overflow: auto;
    border-radius: 16px;
    background: #111;
    color: var(--ink);
    font-size: var(--text-small);
  }
  .grid-head,
  .row {
    display: grid;
    grid-template-columns:
      56px minmax(170px, 2.4fr) 96px 92px minmax(90px, 1fr) 72px
      minmax(130px, 2fr);
    align-items: center;
    column-gap: 10px;
  }
  .grid-head.startup,
  .row.startup {
    grid-template-columns: 56px minmax(140px, 1.4fr) 190px minmax(150px, 1.4fr) minmax(
        160px,
        2.6fr
      );
  }
  .grid-head {
    position: sticky;
    top: 0;
    z-index: 1;
    height: 44px;
    padding: 0 18px 0 8px;
    background: #050505;
  }
  .c-icon {
    display: flex;
    justify-content: center;
  }
  .sort,
  .head-text {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 6px 4px;
    border: 0;
    border-radius: 6px;
    background: none;
    color: var(--accent);
    font-size: 10px;
    font-weight: 800;
    letter-spacing: 0.18em;
    text-transform: uppercase;
  }
  .sort {
    transition: background 120ms ease;
  }
  .sort:hover {
    background: color-mix(in srgb, var(--accent) 16%, transparent);
    border-color: transparent;
  }
  .sort:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }
  .arrow {
    width: 9px;
    height: 9px;
    opacity: 0;
    transform: translateY(2px) rotate(0deg);
    transition:
      opacity 140ms ease,
      transform 180ms cubic-bezier(0.2, 0, 0, 1);
  }
  .is-active .arrow {
    opacity: 1;
    transform: none;
  }
  .is-active .arrow.is-asc {
    transform: rotate(180deg);
  }
  .grid-body {
    padding: 4px 6px 6px;
  }
  .row {
    position: relative;
    height: 52px;
    padding: 0 12px 0 2px;
    border-radius: 12px;
    font-weight: 600;
    cursor: default;
    transition: background 120ms ease;
  }
  .row::after {
    content: '';
    position: absolute;
    right: 12px;
    bottom: 0;
    left: 12px;
    height: 1px;
    background: #262626;
  }
  .row:hover {
    background: #181818;
  }
  .row :global(.icon) {
    transition: transform 160ms cubic-bezier(0.2, 0, 0, 1);
  }
  .row:hover :global(.icon) {
    transform: scale(1.07);
  }
  .row.is-selected {
    background: color-mix(in srgb, var(--accent) 13%, #111);
    box-shadow: inset 0 0 0 2px var(--accent);
  }
  .row.is-selected::after {
    display: none;
  }
  .name-btn {
    max-width: 100%;
    padding: 0;
    border: 0;
    background: none;
    color: var(--ink);
    font-weight: 700;
    text-align: left;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .name-btn:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 3px;
    border-radius: 4px;
  }
  .cut {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .c-disk {
    white-space: nowrap;
  }
  .c-size,
  .c-disk {
    color: var(--accent);
    font-variant-numeric: tabular-nums;
  }
  .is-empty {
    color: var(--muted);
    font-weight: 600;
  }
  .c-version,
  .c-kind,
  .c-publisher,
  .c-where {
    color: var(--ink-soft);
  }
  .pill {
    display: inline-block;
    padding: 4px 10px;
    border-radius: 999px;
    background: color-mix(in srgb, #b4ff00 16%, #111);
    color: #b4ff00;
    font-size: 11px;
    font-weight: 800;
  }
  .pill.is-off {
    background: #2a0f1e;
    color: #ff2e93;
  }
  .c-state {
    white-space: nowrap;
  }
  .was {
    margin-left: 6px;
    color: var(--muted);
    font-size: var(--text-caption);
  }
  .path {
    font-family: ui-monospace, Consolas, monospace;
    font-size: var(--text-caption);
    color: var(--ink-soft);
    overflow-wrap: anywhere;
  }
  .row .path {
    overflow-wrap: normal;
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
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 10;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(0, 0, 0, 0.6);
  }
  .dialog {
    width: min(440px, calc(100vw - 32px));
    padding: 20px;
    border-radius: var(--radius-lg);
    background: var(--surface);
    color: var(--ink);
    animation: dialog-in 160ms cubic-bezier(0.2, 0, 0, 1);
  }
  .dialog h3 {
    margin: 0 0 8px;
  }
  .dialog p {
    margin: 0;
  }
  .dialog-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 16px;
  }
  @keyframes dialog-in {
    from {
      opacity: 0;
      transform: translateY(6px) scale(0.98);
    }
    to {
      opacity: 1;
      transform: none;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .row,
    .row :global(.icon),
    .sort,
    .arrow {
      transition: none;
    }
    .dialog {
      animation: none;
    }
  }
</style>
