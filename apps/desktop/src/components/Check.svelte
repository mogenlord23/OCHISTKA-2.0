<script lang="ts">
  import type { Snippet } from 'svelte';

  /** Флажок множественного выбора в списках (категории очистки, найденные файлы). Без галочки:
      выбран = заливка цветом раздела, не выбран = тёмный квадрат с обводкой. Для «включить / выключить»
      есть Switch.svelte (ADR 0012).
      Внутри настоящий input[type=checkbox] (клавиатура, скринридер, фокус).
      Размер задаётся от шрифта рядом стоящего текста (1.2em, от 18 до 22 px). */
  let {
    checked = $bindable(),
    disabled = false,
    label,
    children,
    bare = false,
    controlled = false,
    onchange,
  }: {
    checked?: boolean | undefined;
    disabled?: boolean;
    /** Подпись справа от квадрата. Вместо неё можно передать содержимое. */
    label?: string;
    children?: Snippet;
    /** Только квадрат, без оболочки label (когда подпись уже внутри чужого label). */
    bare?: boolean;
    /** Состояние показывает только родитель: после клика флажок возвращается к `checked`. */
    controlled?: boolean;
    onchange?: (next: boolean) => void;
  } = $props();

  function handle(event: Event & { currentTarget: HTMLInputElement }) {
    const box = event.currentTarget;
    const next = box.checked;
    if (controlled) {
      box.checked = !!checked;
    } else {
      checked = next;
    }
    onchange?.(next);
  }
</script>

{#snippet control()}
  <span class="control">
    <input type="checkbox" checked={!!checked} {disabled} onchange={handle} />
    <span class="box" class:on={!!checked}></span>
  </span>
{/snippet}

{#if bare}
  {@render control()}
{:else}
  <label class="check" class:disabled>
    {@render control()}
    {#if label}<span class="text">{label}</span>{/if}
    {@render children?.()}
  </label>
{/if}

<style>
  .check {
    display: inline-flex;
    align-items: center;
    gap: 10px;
    cursor: pointer;
  }

  .check.disabled {
    cursor: not-allowed;
  }

  .control {
    position: relative;
    display: inline-grid;
    flex: none;
    width: clamp(18px, 1.2em, 22px);
    height: clamp(18px, 1.2em, 22px);
  }

  /* Настоящий флажок лежит поверх квадрата невидимым: клик и фокус попадают в него. */
  input {
    position: absolute;
    inset: 0;
    z-index: 1;
    width: 100%;
    height: 100%;
    margin: 0;
    opacity: 0;
    cursor: inherit;
  }

  .box {
    box-sizing: border-box;
    display: grid;
    place-items: center;
    width: 100%;
    height: 100%;
    border: 2px solid #4a4a4a;
    border-radius: var(--radius-xs);
    background: #1c1c1c;
    transition:
      background-color 140ms ease,
      border-color 140ms ease,
      transform 120ms ease;
  }

  .box.on {
    border-color: var(--accent, #f83c5c);
    background: var(--accent, #f83c5c);
    box-shadow: none;
  }

  input:hover:not(:disabled) + .box {
    border-color: #8a8a8a;
  }

  input:hover:not(:disabled) + .box.on {
    border-color: var(--accent, #f83c5c);
    filter: brightness(1.12);
  }

  input:active:not(:disabled) + .box {
    transform: scale(0.86);
  }

  /* Видимый фокус с клавиатуры: двойное кольцо, как у остальных элементов. */
  input:focus-visible + .box {
    outline: 3px solid var(--page-text, #fff);
    outline-offset: 2px;
    box-shadow: 0 0 0 2px var(--ink, #fafafa);
  }

  input:disabled + .box {
    opacity: 0.45;
  }

  @media (prefers-reduced-motion: reduce) {
    .box {
      transition: none;
    }

    input:active:not(:disabled) + .box {
      transform: none;
    }
  }
</style>
