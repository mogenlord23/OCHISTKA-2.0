<script lang="ts">
  import { onMount } from 'svelte';
  import { clamavStatus, defaultScanTarget, toCommandError } from '../lib/api';
  import { cancelScan, scan, startScan } from '../lib/scanState.svelte';
  import type { CommandError, EngineStatus } from '../lib/types';

  let { subsection }: { subsection: string } = $props();

  let engine = $state<EngineStatus | null>(null);
  let engineError = $state<CommandError | null>(null);

  // Идёт либо сама проверка, либо оценка находок: новую начинать нельзя.
  const busy = $derived(scan.phase !== 'idle');
  const error = $derived(scan.error ?? engineError);

  const startBlockReason = $derived(
    busy
      ? ''
      : !engine?.available
        ? 'Сначала проверьте движок'
        : scan.target.trim() === ''
          ? 'Укажите папку для проверки'
          : '',
  );

  onMount(async () => {
    try {
      engine = await clamavStatus();
    } catch (err) {
      engineError = toCommandError(err);
    }
    // Поле не затираем, если проверка уже идёт или путь введён раньше.
    if (scan.target === '') {
      try {
        scan.target = await defaultScanTarget();
      } catch {
        // Пустое поле — пользователь укажет папку сам.
      }
    }
  });

  function start() {
    void startScan(scan.target.trim());
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
      <input bind:value={scan.target} placeholder="C:\Users\Имя\Downloads" disabled={busy} />
    </label>
    <div class="actions">
      <button
        type="button"
        class="primary"
        onclick={start}
        disabled={!engine?.available || busy || scan.target.trim() === ''}
      >
        Начать проверку
      </button>
      {#if startBlockReason}
        <span class="reason">{startBlockReason}</span>
      {/if}
      {#if scan.phase === 'scanning'}
        <button type="button" onclick={() => void cancelScan()}>Остановить</button>
      {/if}
    </div>
  </section>

  <section class="scanner" class:active={busy} aria-live="polite">
    {#if scan.phase === 'scanning'}
      <div class="sweep" data-testid="scan-sweep"></div>
      <p>Проверяем файлы…</p>
    {:else if scan.phase === 'assessing'}
      <p>Оцениваем находки…</p>
    {:else if scan.notice}
      <p>{scan.notice}</p>
    {:else if scan.report}
      <p>Проверено файлов: {scan.report.scannedFiles}. Находок: {scan.report.infectedFiles}.</p>
    {:else}
      <p>Готово к проверке.</p>
    {/if}
  </section>
{/if}

{#if subsection === 'findings'}
  {#if !scan.report}
    <p class="empty">Здесь появятся находки после проверки.</p>
  {:else if scan.report.findings.length === 0}
    <p class="empty">Угроз не найдено.</p>
  {:else}
    {#if scan.assessFailures.count > 0}
      <p class="empty" role="status">
        Не удалось оценить файлов: {scan.assessFailures.count}. Причина: {scan.assessFailures
          .firstMessage}
      </p>
    {/if}
    <ul class="findings">
      {#each scan.report.findings as finding, index (`${finding.path}#${index}`)}
        {@const score = scan.suspicions[finding.path]}
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
  {#if scan.runs.length === 0}
    <p class="empty">Проверок в этой сессии пока не было.</p>
  {:else}
    <ul class="log">
      {#each scan.runs as run (run.id)}
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
