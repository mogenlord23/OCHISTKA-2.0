<script lang="ts">
  import { onDestroy } from 'svelte';

  /** Слово OCHISTKA с мягким откликом на курсор: ближайшие к мыши буквы чуть жирнее и выше.
      Шрифт заголовков (Arial Black) не переменный, поэтому «толщина» делается тонким обводом
      того же цвета (text-stroke), а форма — лёгким растяжением по вертикали (transform). Ни то ни другое
      не меняет ширину букв и положение соседей. Слушатель движения живёт только пока курсор над словом. */
  const WORD = 'OCHISTKA';
  const RADIUS = 72;
  const letters = WORD.split('');

  let host = $state<HTMLElement>();
  let spans: HTMLElement[] = [];
  let centres: number[] = [];
  let frame = 0;
  let lastX = 0;
  let active = false;

  const reduced = () => window.matchMedia?.('(prefers-reduced-motion: reduce)').matches ?? false;

  function paint() {
    frame = 0;
    spans.forEach((el, i) => {
      const d = Math.abs(lastX - centres[i]);
      const p = Math.max(0, 1 - d / RADIUS) ** 2;
      el.style.setProperty('--p', p.toFixed(3));
    });
  }

  function onMove(event: PointerEvent) {
    lastX = event.clientX;
    if (!frame) frame = requestAnimationFrame(paint);
  }

  function enter() {
    if (reduced() || active || !host) return;
    // Центры букв считаются один раз при входе курсора.
    centres = spans.map((el) => {
      const r = el.getBoundingClientRect();
      return (r.left + r.right) / 2;
    });
    active = true;
    host.addEventListener('pointermove', onMove);
  }

  function leave() {
    if (!active || !host) return;
    active = false;
    host.removeEventListener('pointermove', onMove);
    if (frame) cancelAnimationFrame(frame);
    frame = 0;
    spans.forEach((el) => el.style.setProperty('--p', '0'));
  }

  onDestroy(leave);
</script>

<!-- Область отклика чуть шире слова (отрицательные поля не двигают соседей). -->
<strong
  class="brand-word"
  data-tauri-drag-region
  aria-label={WORD}
  bind:this={host}
  onpointerenter={enter}
  onpointerleave={leave}
>
  {#each letters as ch, i (i)}
    <span class="ch" data-tauri-drag-region aria-hidden="true" bind:this={spans[i]}>{ch}</span>
  {/each}
</strong>

<style>
  strong {
    display: inline-block;
    padding: 12px 18px;
    margin: -12px -18px;
    font-family: var(--font-heading);
    font-size: 28px;
    font-weight: 900;
    line-height: 1;
    letter-spacing: 0.01em;
    color: var(--black);
    white-space: nowrap;
  }

  .ch {
    display: inline-block;
    --p: 0;
    transform: scaleY(calc(1 + var(--p) * 0.07));
    transform-origin: 50% 70%;
    -webkit-text-stroke: calc(var(--p) * 1.1px) currentColor;
    transition:
      transform 140ms ease-out,
      -webkit-text-stroke-width 140ms ease-out;
  }

  @media (prefers-reduced-motion: reduce) {
    .ch {
      transition: none;
      transform: none;
      -webkit-text-stroke: 0;
    }
  }
</style>
