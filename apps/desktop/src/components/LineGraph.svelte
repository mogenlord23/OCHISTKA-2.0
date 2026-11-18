<script lang="ts">
  import { HISTORY_LIMIT } from '../lib/history';

  let {
    values,
    label,
    color,
  }: {
    /** Замеры в процентах, от старого к новому. */
    values: readonly number[];
    /** Название графика для программ чтения с экрана. */
    label: string;
    color: string;
  } = $props();

  const WIDTH = 300;
  const HEIGHT = 100;

  /** Точки линии: последний замер справа, шкала 0–100 % снизу вверх. */
  const points = $derived.by(() => {
    const count = values.length;
    return values
      .map((value, index) => {
        const x = WIDTH - ((count - 1 - index) / (HISTORY_LIMIT - 1)) * WIDTH;
        const clamped = Math.min(100, Math.max(0, value));
        const y = HEIGHT - (clamped / 100) * HEIGHT;
        return `${x.toFixed(2)},${y.toFixed(2)}`;
      })
      .join(' ');
  });
</script>

<figure class="graph">
  <div class="frame">
    <span class="top">100%</span>
    <svg viewBox="0 0 {WIDTH} {HEIGHT}" preserveAspectRatio="none" role="img" aria-label={label}>
      <line x1="0" x2={WIDTH} y1={HEIGHT / 2} y2={HEIGHT / 2} class="midline" />
      {#if values.length > 1}
        <polyline
          {points}
          fill="none"
          stroke={color}
          stroke-width="2"
          vector-effect="non-scaling-stroke"
          stroke-linejoin="round"
        />
      {/if}
    </svg>
  </div>
  <figcaption class="bottom">90 секунд</figcaption>
</figure>

<style>
  .graph {
    margin: 16px 0 0;
  }

  .frame {
    position: relative;
    height: 120px;
    padding: 4px 0 0 40px;
    border: 1px solid #343434;
    border-radius: 8px;
    background: #111111;
  }

  svg {
    display: block;
    width: 100%;
    height: 100%;
  }

  .midline {
    stroke: #343434;
    stroke-dasharray: 4 4;
    vector-effect: non-scaling-stroke;
  }

  .top {
    position: absolute;
    left: 8px;
    top: 4px;
    font-size: 12px;
    color: #8b8b8b;
  }

  .bottom {
    margin-top: 6px;
    font-size: 12px;
    color: #8b8b8b;
  }
</style>
