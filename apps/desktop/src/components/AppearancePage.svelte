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
          onclick={() => pick(tile.id)}
        >
          <span class="preview" style:background={tile.preview} aria-hidden="true">
            {#if kind === 'cursor'}<span class="pointer">↖</span>{/if}
          </span>
          <span class="label">{tile.label}</span>
          {#if isSelected}<span class="state">✓ Выбрано</span>{/if}
        </button>
      </li>
    {/each}
  </ul>

  <div class="apply">
    <button type="button" disabled aria-describedby="apply-reason">Применить к Windows</button>
    <p id="apply-reason" class="reason">{APPLY_REASON}</p>
  </div>
</section>

<style>
  .appearance {
    color: var(--page-text);
  }

  .current {
    margin: 0 0 12px;
    font-size: var(--text-lead);
    font-weight: 700;
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
    gap: 10px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .tile {
    display: flex;
    flex-direction: column;
    gap: 8px;
    width: 100%;
    min-height: 44px;
    padding: 6px;
    border: 2px solid transparent;
    border-radius: var(--radius-md);
    background: var(--surface);
    color: var(--ink);
    font: inherit;
    text-align: left;
    cursor: pointer;
  }

  .tile.selected {
    border-color: var(--ink);
  }

  .preview {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 96px;
    border-radius: var(--radius-sm);
  }

  .pointer {
    font-size: 28px;
    color: #fafafa;
  }

  .label {
    font-size: var(--text-caption);
    font-weight: 800;
    text-transform: uppercase;
  }

  .state {
    font-size: var(--text-caption);
    font-weight: 700;
    color: var(--muted);
  }

  .tile.selected .state {
    color: var(--ink);
  }

  .apply {
    margin-top: 24px;
  }

  .apply button {
    min-height: 44px;
    padding: 10px 16px;
    border: 0;
    border-radius: var(--radius-sm);
    background: var(--black);
    color: var(--ink);
    font: inherit;
    font-weight: 800;
    text-transform: uppercase;
    font-size: var(--text-body);
    opacity: 0.4;
    cursor: not-allowed;
  }

  .reason {
    margin: 8px 0 0;
    font-size: var(--text-small);
    font-weight: 700;
  }
</style>
