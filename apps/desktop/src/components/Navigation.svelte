<script lang="ts">
  import { SECTIONS, type SectionId } from '../lib/sections';
  import { t } from '../lib/i18n.svelte';

  let {
    active,
    onSelect,
  }: {
    active: SectionId;
    onSelect: (id: SectionId) => void;
  } = $props();

  /** Контурные иконки разделов: собственные, в стиле панели приложения. */
  const icons: Record<SectionId, string> = {
    vpn: 'M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18zM3 12h18M12 3c3 3 3 15 0 18M12 3c-3 3-3 15 0 18',
    viruses: 'M12 3 4 6v6c0 5 3.5 8 8 9 4.5-1 8-4 8-9V6z',
    processes: 'M6 4h12v16H6zM9 8h6M9 12h6M9 16h6',
    programs: 'M4 4h6v6H4zM14 4h6v6h-6zM4 14h6v6H4zM14 14h6v6h-6z',
    disks: 'M4 6h16v12H4zM4 14h16M16 17h2',
    computer: 'M3 4h18v12H3zM8 20h8M12 16v4',
    design: 'M12 3a9 9 0 0 0 0 18c1.5 0 2-1 2-2s-1-2 0-3 3 0 4-2a9 9 0 0 0-6-11z',
  };
</script>

<nav class="rail" aria-label={t('navLabel')}>
  {#each SECTIONS as section (section.id)}
    <button
      type="button"
      class="item"
      class:active={active === section.id}
      aria-label={t(section.titleKey)}
      aria-current={active === section.id ? 'page' : undefined}
      onclick={() => onSelect(section.id)}
    >
      <svg viewBox="0 0 24 24" aria-hidden="true">
        <path d={icons[section.id]} />
      </svg>
      <span class="label">{t(section.titleKey)}</span>
    </button>
  {/each}
</nav>

<style>
  .rail {
    display: flex;
    flex-direction: column;
    gap: 10px;
    min-height: 0;
    overflow-y: auto;
    padding: 18px 10px 0;
    background: var(--window);
  }

  .item {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    gap: 8px;
    height: 84px;
    padding: 8px 4px;
    border: 0;
    border-radius: var(--radius-md);
    background: transparent;
    color: var(--ink);
    font: inherit;
    font-size: var(--text-caption);
    font-weight: 700;
    cursor: pointer;
    transition:
      background-color 150ms ease,
      color 150ms ease,
      transform 90ms ease;
  }

  .item:hover {
    background: var(--rail-hover);
  }

  .item.active {
    background: var(--accent);
    color: var(--black);
  }

  svg {
    width: 30px;
    height: 30px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.6;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .label {
    text-align: center;
  }
</style>
