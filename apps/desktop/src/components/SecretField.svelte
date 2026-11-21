<script lang="ts">
  import { tick } from 'svelte';
  import DecryptedText from './DecryptedText.svelte';
  import { maskSubscriptionUrl } from '../lib/secretMask';

  /** Поле для секретной ссылки: плавающая подпись, маска (видны схема и сервер) и показ по кнопке.
      Значение `value` всегда настоящее: именно оно уходит в импорт. Эффект Decrypted Text (порт React
      Bits) играет только на маске: при вставке и при повторном скрытии. Настоящая ссылка в
      анимацию не попадает ни одним кадром. */
  let {
    value = $bindable(''),
    label = 'Ссылка подписки',
    onenter,
  }: { value?: string; label?: string; onenter?: () => void } = $props();

  let revealed = $state(false);
  let focused = $state(false);
  let hovered = $state(false);
  let decrypt = $state<{ play: () => void }>();

  const hasValue = $derived(value !== '');
  const lifted = $derived(focused || hovered || hasValue);
  const masked = $derived(!revealed);
  const overlay = $derived(maskSubscriptionUrl(value));
  const showOverlay = $derived(hasValue && masked && !focused);

  /** Маска «собирается» из случайных символов; запускаем после обновления DOM. */
  let playWhenShown = false;

  async function replay() {
    await tick();
    if (decrypt) decrypt.play();
    else playWhenShown = true;
  }

  // Вставка делается при фокусе, когда маски на экране ещё нет: эффект сыграет, как только она появится.
  $effect(() => {
    if (showOverlay && decrypt && playWhenShown) {
      playWhenShown = false;
      decrypt.play();
    }
  });

  async function toggle() {
    revealed = !revealed;
    if (!revealed) await replay();
  }

  /** Вставка в пустое поле: после ухода фокуса маска проявится эффектом. */
  function onInput(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    const pasted = (event as InputEvent).inputType === 'insertFromPaste';
    if (pasted && input.value !== '') void replay();
  }
</script>

<div
  class="field"
  class:lifted
  class:focused
  role="group"
  onmouseenter={() => (hovered = true)}
  onmouseleave={() => (hovered = false)}
>
  <input
    bind:value
    class:masked
    class:hide-text={showOverlay}
    type="text"
    autocomplete="off"
    spellcheck="false"
    aria-label={label}
    onfocus={() => (focused = true)}
    onblur={() => (focused = false)}
    oninput={onInput}
    onkeydown={(e) => e.key === 'Enter' && onenter?.()}
  />
  <span class="lbl" aria-hidden="true">{label}</span>
  {#if showOverlay}
    <span class="overlay" aria-hidden="true">
      <DecryptedText
        bind:this={decrypt}
        text={overlay}
        animateOn="manual"
        speed={40}
        maxIterations={9}
      />
    </span>
  {/if}
  <button
    type="button"
    class="eye"
    aria-pressed={revealed}
    aria-label={revealed ? 'Скрыть ссылку' : 'Показать ссылку'}
    title={revealed ? 'Скрыть ссылку' : 'Показать ссылку'}
    onclick={toggle}
  >
    <svg viewBox="0 0 24 24" aria-hidden="true">
      <path d="M2 12s3.6-6.5 10-6.5S22 12 22 12s-3.6 6.5-10 6.5S2 12 2 12z" />
      <circle cx="12" cy="12" r="3" />
      {#if !revealed}<path d="M4 4l16 16" />{/if}
    </svg>
  </button>
</div>

<style>
  .field {
    position: relative;
    flex: 1;
    min-width: 0;
    height: 52px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--black);
    transition: border-color 160ms ease;
  }

  .field:hover {
    border-color: #5a5a5a;
  }

  .field.focused {
    border-color: var(--accent);
  }

  input {
    box-sizing: border-box;
    width: 100%;
    height: 100%;
    padding: 20px 46px 6px 16px;
    border: 0;
    border-radius: inherit;
    background: transparent;
    color: var(--ink);
    font: inherit;
    outline: none;
  }

  /* Скрыто и поле в фокусе: ввод идёт точками, счёт символов настоящий, каретка на месте. */
  input.masked {
    -webkit-text-security: disc;
  }

  /* Скрыто и вне фокуса: вместо текста показана маска поверх (схема и сервер видны). */
  input.hide-text {
    color: transparent;
    -webkit-text-security: none;
  }

  .lbl,
  .overlay {
    position: absolute;
    left: 16px;
    pointer-events: none;
    white-space: nowrap;
  }

  .lbl {
    top: 15px;
    color: var(--muted, #8b8b8b);
    transform-origin: left top;
    transition:
      transform 160ms cubic-bezier(0.2, 0, 0, 1),
      color 160ms ease;
  }

  .field.lifted .lbl {
    transform: translateY(-9px) scale(0.76);
  }

  .field.focused .lbl {
    color: var(--accent);
  }

  .overlay {
    top: 20px;
    right: 46px;
    overflow: hidden;
    color: var(--ink);
    text-overflow: clip;
  }

  .eye {
    position: absolute;
    top: 50%;
    right: 6px;
    display: grid;
    place-items: center;
    width: 36px;
    height: 36px;
    padding: 0;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--ink);
    transform: translateY(-50%);
    cursor: pointer;
  }

  .eye:hover {
    background: rgba(255, 255, 255, 0.09);
  }

  .eye:focus-visible {
    outline: 2px solid var(--ink);
    outline-offset: -2px;
  }

  .eye svg {
    width: 20px;
    height: 20px;
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  @media (prefers-reduced-motion: reduce) {
    .field,
    .lbl {
      transition: none;
    }
  }
</style>
