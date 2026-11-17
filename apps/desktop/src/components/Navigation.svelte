<script lang="ts">
  import { SECTIONS, type SectionId } from '../lib/sections';

  let {
    active,
    onSelect,
  }: {
    active: SectionId;
    onSelect: (id: SectionId) => void;
  } = $props();

  const statusLabel = (id: SectionId) =>
    id === 'viruses' || id === 'computer' ? 'В РАБОТЕ' : 'В РАЗРАБОТКЕ';
</script>

<aside class="rail" aria-label="Разделы приложения">
  <div class="heading">РАЗДЕЛЫ</div>
  <nav aria-label="Навигация">
    {#each SECTIONS as section, index (section.id)}
      <button
        type="button"
        class="item"
        class:active={active === section.id}
        style:--accent={section.accent}
        aria-current={active === section.id ? 'page' : undefined}
        onclick={() => onSelect(section.id)}
      >
        <span class="index">{String(index + 1).padStart(2, '0')}</span>
        <span class="copy">
          <span class="title">{section.title}</span>
          <small>{statusLabel(section.id)}</small>
        </span>
      </button>
    {/each}
  </nav>
</aside>

<style>
  .rail {
    display: flex;
    flex-direction: column;
    gap: 12px;
    width: 230px;
    padding: 18px 12px;
    background: #111111;
    border-left: 2px solid #343434;
  }

  .heading {
    font-size: 12px;
    letter-spacing: 0.12em;
    color: #8b8b8b;
  }

  nav {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .item {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px;
    border: 2px solid transparent;
    border-radius: 12px;
    background: #141414;
    color: #fafafa;
    font: inherit;
    text-align: left;
    cursor: pointer;
  }

  .item:hover {
    border-color: var(--accent);
  }

  .item.active {
    background: var(--accent);
    color: #080808;
  }

  .index {
    font-weight: 800;
    color: var(--accent);
  }

  .item.active .index {
    color: #080808;
  }

  .copy {
    display: flex;
    flex-direction: column;
  }

  .title {
    font-weight: 800;
    text-transform: uppercase;
  }

  small {
    font-size: 11px;
    opacity: 0.8;
  }
</style>
