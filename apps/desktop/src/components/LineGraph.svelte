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
      {#each [1, 2, 3, 4, 5] as col (col)}
        <line x1={(WIDTH / 6) * col} x2={(WIDTH / 6) * col} y1="0" y2={HEIGHT} class="grid" />
      {/each}
      {#each [1, 2, 3] as row (row)}
        <line x1="0" x2={WIDTH} y1={(HEIGHT / 4) * row} y2={(HEIGHT / 4) * row} class="grid" />
      {/each}
      {#if values.length > 1}
        <polyline
          {points}
          fill="none"
          stroke={color}
          stroke-width="1.5"
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
    position: relative;
    margin: 12px 0;
  }

  .frame {
    position: relative;
    height: 170px;
    border-radius: 8px;
    background: #0e0e0e;
    overflow: hidden;
  }

  svg {
    display: block;
    width: 100%;
    height: 100%;
  }

  .grid {
    stroke: #262626;
    stroke-width: 1;
    vector-effect: non-scaling-stroke;
  }

  .top,
  .bottom {
    position: absolute;
    left: 10px;
    font-size: 10px;
    color: #d4d4d4;
  }

  .top {
    top: 8px;
  }

  .bottom {
    bottom: 8px;
    margin: 0;
  }
</style>
