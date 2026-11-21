<script lang="ts">
  /** Слово OCHISTKA: статичный текст, без эффектов и обработчиков мыши. Нужен переменный шрифт:
      в проект положен Roboto Flex (SIL OFL 1.1), урезанный до букв слова (оси wght и opsz).
      Начертание — плотное и жирное: wght 900, opsz 28. */
  const WORD = 'OCHISTKA';
  const REST = "'wght' 900, 'opsz' 28";
</script>

<!-- Клики по слову попадают прямо в строку заголовка, поэтому окно перетаскивается за него. -->
<strong
  class="brand-word"
  data-tauri-drag-region
  aria-label={WORD}
  style:font-variation-settings={REST}>{WORD}</strong
>

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
</style>
