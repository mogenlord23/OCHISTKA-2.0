<script lang="ts">
  import { onMount, tick } from 'svelte';

  export interface MenuItem {
    id: string;
    label: string;
    disabled?: boolean;
    /** Почему пункт выключен: показывается подсказкой. */
    reason?: string | null;
  }

  let {
    items,
    x,
    y,
    label,
    onselect,
    onclose,
  }: {
    items: MenuItem[];
    x: number;
    y: number;
    label: string;
    onselect: (id: string) => void;
    onclose: () => void;
  } = $props();

  let menu = $state<HTMLDivElement | null>(null);
  let left = $state(0);
  let top = $state(0);

  const enabled = $derived(items.filter((item) => !item.disabled));

  function buttons(): HTMLButtonElement[] {
    return menu ? Array.from(menu.querySelectorAll<HTMLButtonElement>('button')) : [];
  }

  onMount(() => {
    left = x;
    top = y;
    void tick().then(() => {
      if (!menu) return;
      // Не даём меню уйти за край окна.
      const rect = menu.getBoundingClientRect();
      left = Math.max(8, Math.min(x, window.innerWidth - rect.width - 8));
      top = Math.max(8, Math.min(y, window.innerHeight - rect.height - 8));
      (buttons().find((button) => !button.disabled) ?? menu).focus();
    });
    const onPointer = (event: PointerEvent) => {
      if (menu && !menu.contains(event.target as Node)) onclose();
    };
    window.addEventListener('pointerdown', onPointer, true);
    window.addEventListener('blur', onclose);
    window.addEventListener('resize', onclose);
    return () => {
      window.removeEventListener('pointerdown', onPointer, true);
      window.removeEventListener('blur', onclose);
      window.removeEventListener('resize', onclose);
    };
  });

  function onKey(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      event.preventDefault();
      event.stopPropagation();
      onclose();
      return;
    }
    if (event.key === 'Tab') {
      event.preventDefault();
      onclose();
      return;
    }
    const list = buttons().filter((button) => !button.disabled);
    if (list.length === 0) return;
    const index = list.indexOf(document.activeElement as HTMLButtonElement);
    if (event.key === 'ArrowDown') {
      event.preventDefault();
      list[(index + 1) % list.length].focus();
    } else if (event.key === 'ArrowUp') {
      event.preventDefault();
      list[(index - 1 + list.length) % list.length].focus();
    } else if (event.key === 'Home') {
      event.preventDefault();
      list[0].focus();
    } else if (event.key === 'End') {
      event.preventDefault();
      list[list.length - 1].focus();
    }
  }
</script>

<div
  class="menu"
  role="menu"
  aria-label={label}
  tabindex="-1"
  bind:this={menu}
  style:left="{left}px"
  style:top="{top}px"
  onkeydown={onKey}
>
  {#each items as item (item.id)}
    <button
      type="button"
      role="menuitem"
      class="item"
      disabled={item.disabled}
      title={item.disabled ? (item.reason ?? undefined) : undefined}
      onclick={() => onselect(item.id)}
    >
      {item.label}
    </button>
  {/each}
  {#if enabled.length === 0}
    <span class="sr-only">Нет доступных действий</span>
  {/if}
</div>

<style>
  .menu {
    position: fixed;
    z-index: 20;
    display: flex;
    flex-direction: column;
    min-width: 200px;
    padding: 6px;
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: var(--radius-md);
    background: #141414;
    box-shadow: 0 12px 32px rgba(0, 0, 0, 0.55);
    animation: menu-in 140ms cubic-bezier(0.2, 0, 0, 1);
    transform-origin: top left;
  }
  .menu:focus {
    outline: none;
  }
  .item {
    padding: 9px 12px;
    border: 0;
    border-radius: var(--radius-sm);
    background: none;
    color: var(--ink);
    font: inherit;
    font-size: var(--text-small);
    font-weight: 600;
    text-align: left;
    cursor: pointer;
  }
  .item:hover:not(:disabled),
  .item:focus-visible {
    background: color-mix(in srgb, var(--accent) 22%, transparent);
    outline: none;
  }
  .item:focus-visible {
    box-shadow: inset 0 0 0 2px var(--accent);
  }
  .item:disabled {
    color: var(--muted);
    cursor: not-allowed;
  }
  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
  }
  @keyframes menu-in {
    from {
      opacity: 0;
      transform: scale(0.96) translateY(-4px);
    }
    to {
      opacity: 1;
      transform: none;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .menu {
      animation: none;
    }
  }
</style>
