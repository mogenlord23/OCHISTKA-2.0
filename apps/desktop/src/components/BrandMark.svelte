<script lang="ts">
  import VariableProximity from './VariableProximity.svelte';

  /** Слово OCHISTKA с эффектом Variable Proximity (React Bits, порт `VariableProximity.svelte`).
      Нужен переменный шрифт: в проект положен Roboto Flex (SIL OFL 1.1), урезанный до букв слова
      (5 КБ, оси wght 100–1000 и opsz). Ширина слова зарезервирована по самому жирному состоянию,
      поэтому положение и размер логотипа не меняются, соседи не двигаются. */
  const WORD = 'OCHISTKA';
  // Покой: плотный и жирный, как логотип до внедрения эффекта (проверено рядом с прежним набором).
  // У курсора: ещё жирнее и уже по форме (оптический размер растёт). Место под слово резервируется
  // по самому широкому состоянию, wght 1000 при opsz 28.
  const REST = "'wght' 900, 'opsz' 28";
  const NEAR = "'wght' 1000, 'opsz' 144";
  const WIDEST = "'wght' 1000, 'opsz' 28";

  let box = $state<HTMLElement | null>(null);
  const reduced = window.matchMedia?.('(prefers-reduced-motion: reduce)').matches ?? false;
</script>

<strong class="brand-word" data-tauri-drag-region bind:this={box} aria-label={WORD}>
  <!-- Невидимая копия в самом жирном состоянии: задаёт ширину и высоту места под слово. -->
  <span class="sizer" aria-hidden="true" style:font-variation-settings={WIDEST}>{WORD}</span>
  <span class="live" aria-hidden="true" style:font-variation-settings={REST}>
    {#if reduced}
      {WORD}
    {:else}
      <VariableProximity
        label={WORD}
        fromFontVariationSettings={REST}
        toFontVariationSettings={NEAR}
        containerRef={box}
        radius={80}
        falloff="linear"
        smoothing={0.2}
      />
    {/if}
  </span>
</strong>

<style>
  @font-face {
    font-family: 'Roboto Flex OCHISTKA';
    src: url('../assets/fonts/RobotoFlex-OCHISTKA.woff2') format('woff2');
    font-weight: 100 1000;
    font-style: normal;
    font-display: block;
  }

  strong {
    position: relative;
    display: inline-block;
    font-family: 'Roboto Flex OCHISTKA', var(--font-heading);
    font-size: 28px;
    font-weight: 900;
    line-height: 1;
    letter-spacing: 0.01em;
    color: var(--black);
    white-space: nowrap;
  }

  .sizer {
    visibility: hidden;
  }

  /* Живое слово поверх зарезервированного места; клики проходят сквозь него на строку заголовка,
     поэтому окно по-прежнему перетаскивается за слово. */
  .live {
    position: absolute;
    top: 0;
    left: 0;
    pointer-events: none;
  }
</style>
