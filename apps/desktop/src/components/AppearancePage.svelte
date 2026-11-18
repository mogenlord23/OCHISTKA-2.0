<script lang="ts">
  import { tilesFor, type AppearanceKind } from '../lib/appearance';
  import { loadChoice, saveChoice, type AppearanceChoice } from '../lib/appearanceStore';

  let { subsection }: { subsection: string } = $props();

  const APPLY_REASON = 'Применение к Windows пока не подключено';

  const KIND_BY_SUBSECTION: Record<string, AppearanceKind> = {
    wallpapers: 'wallpaper',
    cursors: 'cursor',
    colors: 'color',
  };

  const HEADINGS: Record<AppearanceKind, { title: string; hint: string }> = {
    wallpaper: {
      title: 'Обои',
      hint: 'Нажми на картинку, чтобы выбрать фон. Выбор запоминается; установка в Windows появится позже.',
    },
    cursor: {
      title: 'Курсоры',
      hint: 'Нажми на схему, чтобы выбрать курсор. Выбор запоминается; применение появится позже.',
    },
    color: {
      title: 'Цвета',
      hint: 'Нажми на цвет, чтобы выбрать акцент. Выбор запоминается; применение появится позже.',
    },
  };

  let choice = $state<AppearanceChoice>(loadChoice());

  const kind = $derived<AppearanceKind>(KIND_BY_SUBSECTION[subsection] ?? 'wallpaper');
  const tiles = $derived(tilesFor(kind));
  const selectedId = $derived(choice[kind]);
  const selectedLabel = $derived(tiles.find((tile) => tile.id === selectedId)?.label ?? '');

  function pick(id: string) {
    saveChoice(kind, id);
    choice = { ...choice, [kind]: id };
  }
</script>

<section class="appearance" aria-label="Оформление">
  <h2>{HEADINGS[kind].title}</h2>
  <p class="hint">{HEADINGS[kind].hint}</p>

  <div class="actions">
    {#if kind === 'wallpaper'}
      <button type="button" disabled title={APPLY_REASON}>Добавить свои обои…</button>
      <button type="button" disabled title={APPLY_REASON}>Вернуть прежний фон</button>
    {/if}
    <button type="button" disabled aria-describedby="apply-reason">Применить к Windows</button>
  </div>
  <p id="apply-reason" class="reason">{APPLY_REASON}</p>

  <p class="current">Выбрано: <strong>{selectedLabel}</strong></p>

  <ul class="grid">
    {#each tiles as tile (tile.id)}
      {@const isSelected = tile.id === selectedId}
      <li>
        <button
          type="button"
          class="tile"
          class:selected={isSelected}
          aria-pressed={isSelected}
          title={tile.description}
          style:background={tile.preview}
          onclick={() => pick(tile.id)}
        >
          {#if kind === 'cursor'}<span class="pointer" aria-hidden="true">↖</span>{/if}
          <span class="label">{tile.label}</span>
          {#if isSelected}<span class="state">✓<span class="sr">Выбрано</span></span>{/if}
        </button>
      </li>
    {/each}
  </ul>
</section>

<style>
  .appearance {
    color: var(--page-text);
  }

  h2 {
    margin: 0 0 14px;
    font-family: var(--font-heading);
    font-size: var(--text-section);
    font-weight: 900;
    text-transform: uppercase;
  }

  .hint {
    margin: 0 0 14px;
    font-size: var(--text-small);
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  .actions button {
    min-height: 44px;
    padding: 10px 14px;
    border: 1px solid var(--btn-border, #343434);
    border-radius: var(--radius-sm);
    background: var(--black);
    color: var(--ink);
    font: inherit;
    font-size: var(--text-small);
    font-weight: 600;
    cursor: not-allowed;
  }

  .actions button:disabled {
    color: var(--ink-soft);
  }

  .reason {
    margin: 6px 0 0;
    font-size: var(--text-caption);
    font-weight: 700;
  }

  .current {
    margin: 10px 0 12px;
    font-size: var(--text-small);
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(6, minmax(0, 1fr));
    gap: 10px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .tile {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 100%;
    aspect-ratio: 140 / 78;
    min-height: 44px;
    padding: 0;
    border: 2px solid transparent;
    border-radius: 12px;
    color: var(--ink);
    font: inherit;
    cursor: pointer;
    box-shadow: inset 0 0 0 1px rgb(0 0 0 / 12%);
  }

  .tile.selected {
    border-color: var(--black);
    box-shadow: 0 0 0 2px var(--ink);
  }

  .label {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }

  .sr {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
  }

  .pointer {
    font-size: 28px;
    color: #fafafa;
  }

  .state {
    position: absolute;
    right: 6px;
    bottom: 6px;
    display: grid;
    place-items: center;
    width: 22px;
    height: 22px;
    border-radius: 50%;
    background: var(--black);
    color: var(--ink);
    font-size: 13px;
    font-weight: 800;
  }
</style>
