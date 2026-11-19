<script lang="ts">
  import { HISTORY_LIMIT } from '../lib/history';

  let {
    values,
    label,
    color,
    height = 170,
  }: {
    /** Замеры в процентах, от старого к новому. */
    values: readonly number[];
    /** Название графика для программ чтения с экрана. */
    label: string;
    color: string;
    /** Высота рамки графика, px. */
    height?: number;
  } = $props();

  const WIDTH = 300;
  const HEIGHT = 100;

  /** Точки линии: последний замер справа, шкала 0–100 % снизу вверх. */
  /** Сдвиг по оси X на один замер. */
  const STEP = WIDTH / (HISTORY_LIMIT - 1);
  let layer = $state<SVGGElement | null>(null);

  /** Плавная прокрутка: новый кадр стартует со сдвига на один замер и за секунду приходит на место. */
  $effect(() => {
    void values.length;
    void values[values.length - 1];
    if (!layer || typeof layer.animate !== 'function') {
      return;
    }
    if (document.documentElement.dataset.reduceMotion === 'true') {
      return;
    }
    if (window.matchMedia?.('(prefers-reduced-motion: reduce)').matches) {
      return;
    }
    layer.animate([{ transform: `translateX(${STEP}px)` }, { transform: 'translateX(0)' }], {
      duration: 950,
      easing: 'linear',
    });
  });

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

  /** Заливка под линией. */
  const area = $derived(
    values.length > 1
      ? `${points} ${WIDTH},${HEIGHT} ${WIDTH - (values.length - 1) * STEP},${HEIGHT}`
      : '',
  );
</script>

<figure class="graph">
  <div class="frame" style:height="{height}px">
    <span class="top">100%</span>
    <svg viewBox="0 0 {WIDTH} {HEIGHT}" preserveAspectRatio="none" role="img" aria-label={label}>
      {#each [1, 2, 3, 4, 5] as col (col)}
        <line x1={(WIDTH / 6) * col} x2={(WIDTH / 6) * col} y1="0" y2={HEIGHT} class="grid" />
      {/each}
      {#each [1, 2, 3] as row (row)}
        <line x1="0" x2={WIDTH} y1={(HEIGHT / 4) * row} y2={(HEIGHT / 4) * row} class="grid" />
      {/each}
      <g bind:this={layer}>
        {#if values.length > 1}
          <polygon points={area} fill={color} opacity="0.12" />
          <polyline
            {points}
            fill="none"
            stroke={color}
            stroke-width="1.8"
            vector-effect="non-scaling-stroke"
            stroke-linejoin="round"
            stroke-linecap="round"
          />
        {/if}
      </g>
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
    border-radius: var(--radius-sm);
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
