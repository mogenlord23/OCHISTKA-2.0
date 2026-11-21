<!--
  Кнопка-иконка с раскрытием при наведении. Принцип движения (ширина, смещение иконки вниз,
  появление подписи сверху) и тайминги .3s взяты из «smart-emu-83» (Uiverse.io, vinodjangid07,
  лицензия MIT; текст лицензии — в THIRD-PARTY.md). Размеры и цвета заменены токенами OCHISTKA.
  Раскрытие — только ширина кнопки внутри слота фиксированной ширины: соседи не двигаются.
-->
<script lang="ts">
  import type { HTMLButtonAttributes } from 'svelte/elements';

  type Props = HTMLButtonAttributes & {
    /** Контур иконки (одним path). */
    icon: string;
    viewBox?: string;
    /** Короткая подпись, видимая при наведении и фокусе. */
    label: string;
    /** Полное имя действия для чтения с экрана. */
    name: string;
    /** Ширина раскрытой кнопки, px. */
    width?: number;
    /** Заливка вместо контура (корзина Uiverse). */
    filled?: boolean;
    danger?: boolean;
  };

  let {
    icon,
    viewBox = '0 0 24 24',
    label,
    name,
    width = 124,
    filled = false,
    danger = false,
    ...rest
  }: Props = $props();
</script>

<button
  type="button"
  class="expand"
  class:danger
  class:filled
  style:--w="{width}px"
  aria-label={name}
  {...rest}
>
  <svg {viewBox} class="icon" aria-hidden="true" focusable="false"><path d={icon}></path></svg>
  <span class="label" aria-hidden="true">{label}</span>
</button>

<style>
  .expand {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
    box-sizing: border-box;
    flex: none;
    width: 40px;
    height: 40px;
    padding: 0;
    overflow: hidden;
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: var(--radius-sm);
    background: var(--black);
    color: var(--ink);
    cursor: pointer;
    transition:
      width 0.3s,
      background-color 0.3s,
      border-color 0.3s;
  }

  .icon {
    width: 18px;
    height: 18px;
    flex: none;
    transition: transform 0.3s;
  }

  .icon path {
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .filled .icon path {
    fill: currentColor;
    stroke: none;
  }

  .label {
    position: absolute;
    top: 4px;
    left: 0;
    right: 0;
    text-align: center;
    white-space: nowrap;
    font-size: 2px;
    font-weight: 600;
    transform: translateY(-8px);
    line-height: 1.2;
    opacity: 0;
    transition:
      font-size 0.3s,
      opacity 0.3s,
      transform 0.3s;
    pointer-events: none;
  }

  .expand:hover:not(:disabled),
  .expand:focus-visible:not(:disabled) {
    width: var(--w);
    background: var(--ink);
    border-color: transparent;
    color: var(--black);
  }

  .expand.danger:hover:not(:disabled),
  .expand.danger:focus-visible:not(:disabled) {
    background: #ff4545;
    color: #fff;
  }

  .expand:hover:not(:disabled) .icon,
  .expand:focus-visible:not(:disabled) .icon {
    transform: translateY(8px);
  }

  .expand:hover:not(:disabled) .label,
  .expand:focus-visible:not(:disabled) .label {
    font-size: var(--text-small);
    opacity: 1;
    transform: translateY(0);
  }

  .expand:focus-visible {
    outline: 2px solid var(--ink);
    outline-offset: 2px;
  }

  .expand:disabled {
    color: var(--muted);
    cursor: not-allowed;
  }

  @media (prefers-reduced-motion: reduce) {
    .expand,
    .icon,
    .label {
      transition-duration: 0.01ms;
    }
  }
</style>
