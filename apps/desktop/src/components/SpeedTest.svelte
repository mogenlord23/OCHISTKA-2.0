<script lang="ts">
  import { onDestroy, untrack } from 'svelte';
  import { speedStore } from '../lib/speedtestStore.svelte';
  import { SECTION_COLORS } from '../lib/tokens';
  import {
    CANCELLED_MESSAGE,
    PHASE_LABEL,
    SPEED_NOTE,
    easeToward,
    failureMessage,
    formatMbps,
    formatMs,
    gaugeFraction,
    noteSeen,
    summarize,
  } from '../lib/speedtest';

  let { active = true }: { active?: boolean } = $props();

  /** Состояние теста лежит в хранилище: уход из раздела его не прерывает. */
  const ui = $derived(speedStore.view);
  const status = $derived(speedStore.ui.status);
  let showNote = $state(!noteSeen());
  const running = $derived(speedStore.running);
  const target = $derived(running ? speedStore.ui.liveMbps : (ui.downloadMbps ?? 0));
  // При возврате в раздел показываем прежнее значение сразу, без разгона с нуля.
  let shown = $state(
    untrack(() =>
      speedStore.running ? speedStore.ui.liveMbps : (speedStore.view.downloadMbps ?? 0),
    ),
  );
  const fraction = $derived(gaugeFraction(shown));
  const summary = $derived(
    status === 'done' &&
      ui.downloadMbps !== null &&
      ui.uploadMbps !== null &&
      ui.pingMs !== null &&
      ui.jitterMs !== null
      ? summarize({
          downloadMbps: ui.downloadMbps,
          uploadMbps: ui.uploadMbps,
          pingMs: ui.pingMs,
          jitterMs: ui.jitterMs,
          loadedPingMs: ui.loadedMs,
        })
      : null,
  );
  const caption = $derived(
    running && ui.phase
      ? `${PHASE_LABEL[ui.phase]} · Мбит/с`
      : ui.status === 'done'
        ? 'Скачивание, Мбит/с'
        : 'Мбит/с',
  );

  // Плавное число и заливка: приближаем показанное значение к целевому по кадрам.
  let frame = 0;
  let last = 0;
  const reduceMotion =
    typeof matchMedia === 'function' && matchMedia('(prefers-reduced-motion: reduce)').matches;

  function tick(now: number) {
    frame = 0;
    const next = easeToward(shown, target, last === 0 ? 16 : now - last);
    last = now;
    shown = next;
    if (next !== target) schedule();
    else last = 0;
  }

  function schedule() {
    if (frame !== 0) return;
    if (reduceMotion || typeof requestAnimationFrame !== 'function') {
      shown = target;
      return;
    }
    frame = requestAnimationFrame(tick);
  }

  $effect(() => {
    if (!active || !running) {
      if (frame !== 0 && typeof cancelAnimationFrame === 'function') cancelAnimationFrame(frame);
      frame = 0;
      last = 0;
      if (active) shown = target;
    } else {
      void target;
      schedule();
    }
  });

  onDestroy(() => {
    if (frame !== 0 && typeof cancelAnimationFrame === 'function') cancelAnimationFrame(frame);
    // Тест не отменяется: он продолжается в хранилище, пока открыт другой раздел.
  });

  function start() {
    showNote = false;
    shown = 0;
    void speedStore.start();
  }

  function stop() {
    speedStore.stop();
  }
</script>

<section
  class="card speed"
  aria-label="Тест скорости интернета"
  style:--speed-green={SECTION_COLORS.viruses}
>
  <div class="body">
    <div
      class="gauge"
      class:running
      class:filled={fraction >= 0.5}
      role="img"
      aria-label={running && ui.phase
        ? `Скорость ${formatMbps(shown)} Мбит/с. ${PHASE_LABEL[ui.phase]} ${Math.round(speedStore.ui.fraction * 100)}%`
        : `Скорость ${formatMbps(shown)} Мбит/с`}
    >
      <div class="fill" aria-hidden="true" style:height={`${fraction * 100}%`}></div>
      <div class="readout">
        <span class="value" data-testid="speed-value">{formatMbps(shown)}</span>
        <span class="caption">{caption}</span>
      </div>
    </div>

    <div class="details">
      <dl class="cards">
        <div class="stat">
          <dt>Скачивание</dt>
          <dd data-testid="speed-down">{formatMbps(ui.downloadMbps)}<small>Мбит/с</small></dd>
        </div>
        <div class="stat">
          <dt>Отдача</dt>
          <dd data-testid="speed-up">{formatMbps(ui.uploadMbps)}<small>Мбит/с</small></dd>
        </div>
        <div class="stat">
          <dt>Пинг</dt>
          <dd data-testid="speed-ping">{formatMs(ui.pingMs)}<small>мс</small></dd>
        </div>
        <div class="stat">
          <dt>Стабильность</dt>
          <dd data-testid="speed-jitter">{formatMs(ui.jitterMs)}<small>мс</small></dd>
        </div>
      </dl>
    </div>
  </div>

  {#if running}
    <button type="button" class="btn" onclick={stop}>Остановить</button>
  {:else}
    <button type="button" class="btn go" onclick={start}>
      {ui.status === 'idle' && !speedStore.last ? 'Проверить скорость' : 'Проверить скорость снова'}
    </button>
  {/if}

  {#if status === 'failed'}
    <p class="msg error" role="alert">{failureMessage(speedStore.ui.failure)}</p>
  {:else if status === 'cancelled'}
    <p class="msg" role="status">{CANCELLED_MESSAGE}</p>
  {:else if summary}
    <p class="msg" role="status"><b>{summary.title}.</b> {summary.text}</p>
  {:else if showNote}
    <p class="msg note" role="note">{SPEED_NOTE}</p>
  {/if}
</section>

<style>
  .card {
    padding: var(--card-inset);
    border-radius: var(--radius-adaptive);
    background: var(--surface-alt);
    box-shadow: inset 0 0 0 2px #000;
  }

  .btn {
    --corner-size: var(--control-height);
    width: 100%;
    min-height: var(--control-height);
    margin-top: 8px;
    padding: 6px 18px;
    border: 0;
    border-radius: var(--radius-adaptive);
    background: var(--btn-bg);
    color: var(--ink);
    box-shadow: inset 0 0 0 1px var(--btn-border);
    font: inherit;
    font-size: var(--text-caption);
    font-weight: 800;
    cursor: pointer;
  }

  .btn.go {
    background: var(--accent);
    color: #000;
    box-shadow: none;
  }

  .btn:focus-visible {
    outline: 2px solid #fff;
    outline-offset: 2px;
  }

  .body {
    display: grid;
    grid-template-columns: minmax(150px, 1fr) minmax(200px, 1fr);
    align-items: stretch;
    gap: 12px;
  }

  .gauge {
    position: relative;
    display: grid;
    place-items: center;
    width: 100%;
    aspect-ratio: 1;
    min-width: 0;
    overflow: hidden;
    border-radius: var(--radius-adaptive);
    background: #202020;
  }

  .fill {
    position: absolute;
    inset: auto 0 0;
    background: linear-gradient(to top, var(--accent), var(--speed-green));
    transition: height 0.18s linear;
  }

  .gauge:not(.running) .fill {
    transition: none;
  }

  .readout {
    position: relative;
    z-index: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    max-width: 100%;
    padding: 8px;
    text-align: center;
  }

  .value {
    max-width: 100%;
    overflow-wrap: anywhere;
    font-size: clamp(3.5rem, 12cqi, 8rem);
    font-weight: 800;
    line-height: 1;
    font-variant-numeric: tabular-nums;
    color: #fff;
  }

  .caption {
    margin-top: 4px;
    font-size: clamp(1rem, 2.5cqi, 1.8rem);
    color: #fff;
  }

  .gauge.filled .value,
  .gauge.filled .caption {
    color: #000;
  }

  .details {
    display: flex;
    flex-direction: column;
    min-width: 0;
    gap: 8px;
  }

  .cards {
    display: grid;
    flex: 1;
    min-width: 0;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 8px;
    margin: 0;
  }

  .stat {
    min-width: 0;
    padding: 8px 10px;
    border-radius: var(--radius-adaptive);
    background: #202020;
    text-align: center;
  }

  dt {
    overflow-wrap: anywhere;
    font-size: clamp(0.75rem, 1.8cqi, 1.25rem);
    font-weight: 800;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--muted);
  }

  dd {
    margin: 2px 0 0;
    font-size: clamp(2rem, 6cqi, 5rem);
    font-weight: 800;
    overflow-wrap: anywhere;
    line-height: 1.05;
    font-variant-numeric: tabular-nums;
    color: var(--ink);
  }

  dd small {
    display: block;
    margin-left: 0;
    font-size: clamp(0.75rem, 1.5cqi, 1rem);
    font-weight: 600;
    color: var(--muted);
  }

  .msg {
    margin: 12px 0 0;
    font-size: var(--text-caption);
    color: var(--ink-soft, var(--ink));
  }

  .msg.note {
    color: var(--muted);
  }

  .msg.error {
    color: #ff8aa0;
    font-weight: 800;
  }

  .speed {
    container-type: inline-size;
  }

  @container (max-width: 420px) {
    .body {
      grid-template-columns: minmax(0, 1fr);
    }

    .gauge {
      width: min(100%, 240px);
      justify-self: center;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .fill {
      transition: none;
    }
  }
</style>
