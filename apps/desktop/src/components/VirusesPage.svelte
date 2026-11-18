<script lang="ts">
  import { onMount } from 'svelte';
  import {
    assessFile,
    clamavStatus,
    defaultScanTarget,
    scanStart,
    toCommandError,
  } from '../lib/api';
  import type { CommandError, EngineStatus, ScanReport, Suspicion } from '../lib/types';

  let { subsection }: { subsection: string } = $props();

  let nextRunId = 0;

  interface Run {
    id: number;
    startedAt: string;
    target: string;
    scanned: number;
    infected: number;
  }

  let engine = $state<EngineStatus | null>(null);
  let target = $state('');
  let scanning = $state(false);
  let report = $state<ScanReport | null>(null);
  let error = $state<CommandError | null>(null);
  let suspicions = $state<Record<string, Suspicion>>({});
  let runs = $state<Run[]>([]);
  let assessFailures = $state<{ count: number; firstMessage: string }>({
    count: 0,
    firstMessage: '',
  });

  const startBlockReason = $derived(
    scanning
      ? ''
      : !engine?.available
        ? 'Сначала проверьте движок'
        : target.trim() === ''
          ? 'Укажите папку для проверки'
          : '',
  );

  onMount(async () => {
    try {
      engine = await clamavStatus();
    } catch (err) {
      error = toCommandError(err);
    }
    try {
      target = await defaultScanTarget();
    } catch {
      // Пустое поле — пользователь укажет папку сам.
    }
  });

  async function start() {
    const path = target.trim();
    if (!engine?.available || scanning || path === '') {
      return;
    }
    scanning = true;
    error = null;
    report = null;
    suspicions = {};
    assessFailures = { count: 0, firstMessage: '' };
    try {
      const result = await scanStart(path);
      report = result;
      runs = [
        {
          id: nextRunId++,
          startedAt: new Date().toLocaleTimeString('ru-RU'),
          target: path,
          scanned: result.scannedFiles,
          infected: result.infectedFiles,
        },
        ...runs,
      ];
      let failureCount = 0;
      let firstFailure = '';
      const scores = await Promise.all(
        result.findings.map(async (finding) => {
          try {
            return [finding.path, await assessFile(finding.path)] as const;
          } catch (err) {
            failureCount += 1;
            if (firstFailure === '') {
              firstFailure = toCommandError(err).message;
            }
            return [finding.path, null] as const;
          }
        }),
      );
      assessFailures = { count: failureCount, firstMessage: firstFailure };
      const next: Record<string, Suspicion> = {};
      for (const [findingPath, score] of scores) {
        if (score) {
          next[findingPath] = score;
        }
      }
      suspicions = next;
    } catch (err) {
      error = toCommandError(err);
    } finally {
      scanning = false;
    }
  }
</script>

{#if subsection === 'scan'}
  {#if engine && !engine.available}
    <div class="alert" role="alert">
      <strong>Движок ClamAV не найден</strong>
      <p>{engine.reason}</p>
      <p>Положите ClamAV в папку «clamav» рядом с приложением.</p>
    </div>
  {/if}

  {#if error}
    <div class="alert" role="alert"><p>{error.message}</p></div>
  {/if}

  <section class="panel">
    <label>
      Папка для проверки
      <input bind:value={target} placeholder="C:\Users\Имя\Downloads" disabled={scanning} />
    </label>
    <div class="actions">
      <button
        type="button"
        class="primary"
        onclick={start}
        disabled={!engine?.available || scanning || target.trim() === ''}
      >
        Начать проверку
      </button>
      {#if startBlockReason}
        <span class="reason">{startBlockReason}</span>
      {/if}
      <button type="button" disabled title="Отмена появится в следующей версии">
        Остановить
      </button>
    </div>
  </section>

  <section class="scanner" class:active={scanning} aria-live="polite">
    {#if scanning}
      <div class="sweep" data-testid="scan-sweep"></div>
      <p>Проверяем файлы…</p>
    {:else if report}
      <p>Проверено файлов: {report.scannedFiles}. Находок: {report.infectedFiles}.</p>
    {:else}
      <p>Готово к проверке.</p>
    {/if}
  </section>
{/if}

{#if subsection === 'findings'}
  {#if !report}
    <p class="empty">Здесь появятся находки после проверки.</p>
  {:else if report.findings.length === 0}
    <p class="empty">Угроз не найдено.</p>
  {:else}
    {#if assessFailures.count > 0}
      <p class="empty" role="status">
        Не удалось оценить файлов: {assessFailures.count}. Причина: {assessFailures.firstMessage}
      </p>
    {/if}
    <ul class="findings">
      {#each report.findings as finding (finding.path)}
        {@const score = suspicions[finding.path]}
        <li>
          <div class="head">
            <strong>{finding.signature}</strong>
            <span class="path">{finding.path}</span>
          </div>
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
          <button type="button" disabled title="Карантин — следующий шаг">Удалить</button>
        </li>
      {/each}
    </ul>
  {/if}
{/if}

{#if subsection === 'log'}
  {#if runs.length === 0}
    <p class="empty">Проверок в этой сессии пока не было.</p>
  {:else}
    <ul class="log">
      {#each runs as run (run.id)}
        <li>
          {run.startedAt} · {run.target} · проверено {run.scanned}, находок {run.infected}
        </li>
      {/each}
    </ul>
  {/if}
{/if}

<style>
  .panel,
  .scanner,
  .alert,
  .findings li {
    margin-bottom: 16px;
    padding: 16px;
    border-radius: var(--radius-md);
    background: var(--surface);
  }

  .alert {
    border: 1px solid #ff009d;
  }

  label {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  input {
    padding: 10px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--black);
    color: var(--ink);
    font: inherit;
  }

  .actions {
    display: flex;
    gap: 8px;
    margin-top: 12px;
  }

  button {
    padding: 8px 14px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--surface);
    color: var(--ink-soft);
    font: inherit;
    cursor: pointer;
  }

  button:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }

  .reason {
    align-self: center;
    color: var(--muted);
  }

  button.primary {
    border-color: var(--accent);
    background: var(--accent);
    color: var(--black);
    font-weight: 700;
  }

  .scanner {
    position: relative;
    min-height: 88px;
    overflow: hidden;
  }

  .scanner p,
  .empty {
    margin: 0;
    color: var(--ink-soft);
  }

  .sweep {
    position: absolute;
    inset: 0;
    background: linear-gradient(90deg, transparent, var(--accent), transparent);
    opacity: 0.35;
    animation: sweep 1.6s linear infinite;
  }

  .scanner p {
    position: relative;
  }

  @keyframes sweep {
    from {
      transform: translateX(-100%);
    }
    to {
      transform: translateX(100%);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .sweep {
      animation: none;
    }
  }

  .findings,
  .log {
    margin: 0;
    padding: 0;
    list-style: none;
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
    margin: 10px 0 6px;
    border-radius: 4px;
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
