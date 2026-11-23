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
          <strong>{finding.signature}</strong>
          <span class="path selectable">{finding.path}</span>
        </span>
      </Check>
      <small>Причина: сигнатура {finding.signature}</small>
      {#if score}
        <div
          class="meter"
          role="meter"
          aria-label="Подсказка о подозрительности"
          aria-valuemin="0"
          aria-valuemax="100"
          aria-valuenow={score.score}
        >
          <span style:width="{score.score}%"></span>
        </div>
        <small>
          Подсказка: {score.score}/100 — {score.reasons.join('; ') || 'признаков не найдено'}
        </small>
      {/if}
    </li>
  {/each}
</ul>

<style>
  .findings {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .finding {
    margin: 0 0 8px;
    padding: 16px;
    border-radius: var(--radius-lg);
    background: var(--surface);
    box-shadow: 0 0 0 2px #000;
    font-size: var(--text-caption);
  }

  .head {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .path {
    color: var(--muted);
    word-break: break-all;
  }

  .meter {
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
