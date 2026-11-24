<script lang="ts">
  import Check from './Check.svelte';
  import type { Finding, Suspicion } from '../lib/types';

  /** Список находок проверки: один и тот же в подразделах «Проверка» и «Находки».
      Выбор хранит страница, флажки управляемые (`controlled`): состояние всегда одно. */
  let {
    findings,
    suspicions = {},
    selected = {},
    onToggle,
    onOpenMenu,
  }: {
    findings: Finding[];
    suspicions?: Record<string, Suspicion>;
    selected?: Record<number, boolean>;
    onToggle?: (index: number, next: boolean) => void;
    /** Правый клик или меню с клавиатуры на строке находки. */
    onOpenMenu?: (event: MouseEvent | KeyboardEvent, index: number) => void;
  } = $props();

  function onKey(event: KeyboardEvent, index: number) {
    // Меню с клавиатуры: клавиша меню или Shift+F10, как в «Процессах».
    if (event.key === 'ContextMenu' || (event.key === 'F10' && event.shiftKey)) {
      event.preventDefault();
      onOpenMenu?.(event, index);
    }
  }
</script>

<ul class="findings" data-testid="findings-list">
  {#each findings as finding, index (`${finding.path}#${index}`)}
    {@const score = suspicions[finding.path]}
    <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
    <!-- Строка — не кнопка: меню открывается по правому клику на самой строке, как в «Процессах».
         С клавиатуры фокусируется флажок внутри, а Shift+F10 всплывает сюда же. -->
    <li
      class="finding"
      data-testid="finding"
      oncontextmenu={(event) => onOpenMenu?.(event, index)}
      onkeydown={(event) => onKey(event, index)}
    >
      <Check
        controlled
        checked={selected[index] ?? false}
        onchange={(next) => onToggle?.(index, next)}
      >
        <span class="head">
          <strong>{finding.path.split(/[\\/]/).pop() || finding.path}</strong>
          <span class="path selectable">{finding.path}</span>
        </span>
        <span class="details">
          <small>Причина: сигнатура {finding.signature}</small>
          {#if score}
            <span
              class="meter"
              role="meter"
              aria-label="Подсказка о подозрительности"
              aria-valuemin="0"
              aria-valuemax="100"
              aria-valuenow={score.score}
            >
              <span style:width="{score.score}%"></span>
            </span>
            <small>
              Подсказка: {score.score}/100 — {score.reasons.join('; ') || 'признаков не найдено'}
            </small>
          {/if}
        </span>
      </Check>
    </li>
  {/each}
</ul>

<style>
  .findings {
    display: grid;
    gap: 7px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .finding {
    min-width: 0;
    border: 1px solid var(--border);
    border-inline-start: 3px solid var(--accent);
    border-radius: var(--radius-sm);
    background: var(--surface-alt);
    font-size: var(--text-caption);
  }

  .finding:focus-within { border-color: var(--accent); }

  .finding > :global(label) {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    gap: 4px 10px;
    padding: 12px 14px;
  }

  .finding :global(.box) {
    transition-property: transform;
  }

  .details {
    grid-column: 2;
    min-width: 0;
    overflow-wrap: anywhere;
  }

  .head {
    display: flex;
    flex-direction: column;
    gap: 4px;
    overflow-wrap: anywhere;
  }

  .path {
    color: var(--muted);
    word-break: break-all;
  }

  .meter {
    display: block;
    height: 8px;
    margin: 8px 0 6px;
    border-radius: var(--radius-xs);
    background: #21262d;
    overflow: hidden;
  }

  .meter span {
    display: block;
    height: 100%;
    background: var(--accent);
  }

  small {
    color: var(--muted);
  }
</style>
