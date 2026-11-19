<script lang="ts">
  import { onDestroy } from 'svelte';
  import {
    CANCELLED_MESSAGE,
    IDLE,
    PHASE_LABEL,
    SPEED_NOTE,
    START,
    easeToward,
    failureMessage,
    formatMbps,
    formatMs,
    gaugeFraction,
    markNoteSeen,
    noteSeen,
    reduce,
    summarize,
    type SpeedUi,
  } from '../lib/speedtest';
  import { cancelSpeedtest, startSpeedtest } from '../lib/speedtestApi';

  let ui = $state<SpeedUi>(IDLE);
  let showNote = $state(!noteSeen());
  let shown = $state(0);

  const running = $derived(ui.status === 'running');
  const target = $derived(ui.status === 'done' ? (ui.downloadMbps ?? 0) : ui.liveMbps);
  const fraction = $derived(gaugeFraction(shown));
  const summary = $derived(
    ui.status === 'done' &&
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
      ? PHASE_LABEL[ui.phase]
      : ui.status === 'done'
        ? 'Скачивание, Мбит/с'
        : 'Мбит/с',
  );

  // Плавное число и дуга: приближаем показанное значение к целевому по кадрам.
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
    void target;
    schedule();
  });

  onDestroy(() => {
    if (frame !== 0 && typeof cancelAnimationFrame === 'function') cancelAnimationFrame(frame);
    if (ui.status === 'running') void cancelSpeedtest().catch(() => {});
  });

  async function start() {
    if (running) return;
    showNote = false;
    markNoteSeen();
    shown = 0;
    ui = START;
    try {
      await startSpeedtest((event) => {
        ui = reduce(ui, event);
      });
      // Если Rust завершился, не прислав итога, не оставляем кнопку в ожидании.
      if (ui.status === 'running') ui = reduce(ui, { type: 'failed', kind: 'server' });
    } catch {
      ui = reduce(ui, { type: 'failed', kind: 'server' });
    }
  }

  function stop() {
    void cancelSpeedtest().catch(() => {});
  }
</script>

<section class="card speed" aria-label="Твой интернет">
  <div class="head">
    <h2>Твой интернет</h2>
    {#if running}
      <button type="button" class="btn" onclick={stop}>Остановить</button>
    {:else}
      <button type="button" class="btn go" onclick={start}>
        {ui.status === 'idle' ? 'Проверить скорость' : 'Повторить'}
      </button>
    {/if}
  </div>

  <div class="body">
    <div class="gauge" role="img" aria-label={`Скорость ${formatMbps(shown)} Мбит/с`}>
      <svg viewBox="0 0 200 124" aria-hidden="true">
        <defs>
          <linearGradient id="speed-grad" x1="0" y1="0" x2="1" y2="0">
            <stop offset="0%" stop-color="var(--accent)" />
            <stop offset="100%" stop-color="#b4ff00" />
          </linearGradient>
        </defs>
        <path class="track" d="M20 104 A80 80 0 0 1 180 104" pathLength="100" />
        <path
          class="fill"
          d="M20 104 A80 80 0 0 1 180 104"
          pathLength="100"
          stroke-dasharray="100"
          stroke-dashoffset={100 - fraction * 100}
        />
        <text x="14" y="120" class="tick" text-anchor="start">0</text>
        <text x="186" y="120" class="tick" text-anchor="end">1000</text>
      </svg>
      <div class="readout">
        <span class="value" data-testid="speed-value">{formatMbps(shown)}</span>
        <span class="caption">{caption}</span>
      </div>
    </div>

    <dl class="cards">
      <div class="stat">
        <dt>Скачивание</dt>
        <dd data-testid="speed-down">
          {formatMbps(ui.downloadMbps)}<small>Мбит/с</small>
        </dd>
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
        <dt>Под нагрузкой</dt>
        <dd data-testid="speed-loaded">{formatMs(ui.loadedMs)}<small>мс</small></dd>
      </div>
      <div class="stat">
        <dt>Стабильность</dt>
        <dd data-testid="speed-jitter">{formatMs(ui.jitterMs)}<small>мс</small></dd>
      </div>
    </dl>
  </div>

  {#if running}
    <div
      class="progress"
      role="progressbar"
      aria-label="Ход теста"
      aria-valuemin="0"
      aria-valuemax="100"
      aria-valuenow={Math.round(ui.fraction * 100)}
    >
      <span style:width={`${Math.round(ui.fraction * 100)}%`}></span>
    </div>
  {/if}

  {#if ui.status === 'failed'}
    <p class="msg error" role="alert">{failureMessage(ui.failure)}</p>
  {:else if ui.status === 'cancelled'}
    <p class="msg" role="status">{CANCELLED_MESSAGE}</p>
  {:else if summary}
    <p class="msg" role="status"><b>{summary.title}.</b> {summary.text}</p>
  {:else if showNote}
    <p class="msg note" role="note">{SPEED_NOTE}</p>
  {/if}
</section>

<style>
  .card {
    padding: 16px;
    border-radius: var(--radius-md);
    background: var(--surface-alt);
    box-shadow: 0 0 0 2px #000;
  }

  .head {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 8px 16px;
    margin-bottom: 8px;
  }

  h2 {
    margin: 0;
    font-size: var(--text-caption);
    font-weight: 800;
    text-transform: uppercase;
    color: var(--ink);
  }

  .btn {
    min-height: 36px;
    padding: 6px 18px;
    border: 0;
    border-radius: var(--radius-sm);
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
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
  }

  .gauge {
    position: relative;
    width: min(100%, 250px);
  }

  svg {
    display: block;
    width: 100%;
    height: auto;
  }

  .track,
  .fill {
    fill: none;
    stroke-width: 14;
    stroke-linecap: round;
  }

  .track {
    stroke: #262626;
  }

  .fill {
    stroke: url(#speed-grad);
  }

  .tick {
    fill: var(--muted);
    font-size: 9px;
    font-weight: 800;
  }

  .readout {
    position: absolute;
    inset: 38% 0 auto;
    display: flex;
    flex-direction: column;
    align-items: center;
    pointer-events: none;
  }

  .value {
    font-size: 2.1rem;
    font-weight: 800;
    line-height: 1;
    font-variant-numeric: tabular-nums;
    color: var(--ink);
  }

  .caption {
    margin-top: 4px;
    font-size: var(--text-caption);
    color: var(--muted);
  }

  .cards {
    display: grid;
    width: 100%;
    grid-template-columns: repeat(auto-fit, minmax(88px, 1fr));
    gap: 8px;
    margin: 0;
  }

  .stat {
    min-width: 0;
    padding: 8px 10px;
    border-radius: var(--radius-sm);
    background: #1a1a1a;
  }

  dt {
    font-size: 0.6rem;
    font-weight: 800;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--muted);
  }

  dd {
    margin: 2px 0 0;
    font-size: 1.1rem;
    font-weight: 800;
    font-variant-numeric: tabular-nums;
    color: var(--ink);
  }

  dd small {
    display: block;
    margin-left: 0;
    font-size: var(--text-caption);
    font-weight: 600;
    color: var(--muted);
  }

  .progress {
    height: 4px;
    margin-top: 12px;
    border-radius: var(--radius-xs);
    background: #262626;
    overflow: hidden;
  }

  .progress span {
    display: block;
    height: 100%;
    background: var(--accent);
    transition: width 0.12s linear;
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

  @media (prefers-reduced-motion: reduce) {
    .progress span {
      transition: none;
    }
  }
</style>
