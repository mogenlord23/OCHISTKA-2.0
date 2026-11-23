<script lang="ts">
  /** Корзина находок: простая векторная форма, без растровых картинок.
      Число на корзине равно числу находок отчёта. Крышка открывается и закрывается по состоянию. */
  let {
    count = 0,
    state = 'idle',
  }: {
    count?: number;
    /** idle — пустая, filling — находки летят, closed — крышка закрыта, throwing — выбрасывает. */
    state?: 'idle' | 'filling' | 'closed' | 'throwing';
  } = $props();
</script>

<div class="bin" data-testid="virus-bin" data-state={state}>
  <svg viewBox="0 0 64 72" aria-hidden="true" focusable="false">
    <g class="lid">
      <rect x="8" y="10" width="48" height="7" rx="3.5" fill="currentColor" />
      <rect x="25" y="4" width="14" height="6" rx="3" fill="currentColor" />
    </g>
    <path
      d="M12 22h40l-4 44a4 4 0 0 1-4 3.6H20a4 4 0 0 1-4-3.6Z"
      fill="none"
      stroke="currentColor"
      stroke-width="4"
      stroke-linejoin="round"
    />
    <g class="ribs" stroke="currentColor" stroke-width="3" stroke-linecap="round">
      <path d="M26 30v30" />
      <path d="M38 30v30" />
    </g>
  </svg>
  <span class="count" data-testid="bin-count">{count}</span>
  <span class="sr-only">Корзина находок: {count}</span>
</div>

<style>
  .bin {
    position: relative;
    display: grid;
    place-items: center;
    width: 96px;
    height: 96px;
    border-radius: var(--radius-lg);
    background: var(--surface);
    box-shadow: 0 0 0 2px #000;
    color: var(--accent);
    /* Точка приземления фигур: полёт считает смещение до центра корзины. */
    overflow: hidden;
  }

  svg {
    width: 64px;
    height: 72px;
    overflow: visible;
  }

  .lid {
    transform-origin: 32px 17px;
    transition: transform 180ms cubic-bezier(0.2, 0, 0, 1);
  }

  .bin[data-state='filling'] .lid,
  .bin[data-state='throwing'] .lid {
    transform: rotate(-24deg) translateY(-3px);
  }

  .count {
    position: absolute;
    right: 8px;
    bottom: 8px;
    min-width: 24px;
    padding: 2px 6px;
    border-radius: var(--radius-pill);
    background: var(--accent);
    color: var(--black);
    font-size: var(--text-small);
    font-weight: 800;
    font-variant-numeric: tabular-nums;
    text-align: center;
  }

  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
  }

  @media (prefers-reduced-motion: reduce) {
    .lid {
      transition: none;
    }
  }

  :global(:root[data-reduce-motion='true']) .lid {
    transition: none;
  }
</style>
