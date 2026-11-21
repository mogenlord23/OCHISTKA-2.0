<script lang="ts">
  import DecryptedText from './DecryptedText.svelte';
  import FloatingField from './FloatingField.svelte';
  import { segments, visualLine } from '../lib/secretMask';

  /** Поле для секретной ссылки: плавающая подпись внутри поля. Пока поле не редактируется, вместо
      ссылки показана «приманка»: адрес сервера и случайные символы вместо токена, по которым спокойно
      пробегает эффект Decrypted Text (порт React Bits): участки строки перемешиваются по очереди.
      В фокусе поле обычное: можно править, выделять и вставлять настоящую ссылку. Настоящее значение
      `value` не меняется и уходит в импорт. Приманка не выводится из символов токена. */
  let {
    value = $bindable(''),
    label = 'Ссылка подписки',
    onenter,
  }: { value?: string; label?: string; onenter?: () => void } = $props();

  const PARTS = 3;
  const PERIOD = 4200; // один круг: участок 1, участок 2, участок 3 и пауза
  const GAP = 1100;

  let focused = $state(false);
  let parts = $state<({ play: () => void } | undefined)[]>([]);

  const hasValue = $derived(value !== '');
  const showOverlay = $derived(hasValue && !focused);
  // Новая приманка только при смене ссылки, а не на каждый кадр анимации.
  const line = $derived(visualLine(value));
  const chunks = $derived(segments(line, PARTS));

  $effect(() => {
    if (!showOverlay || chunks.length === 0) return;
    if (window.matchMedia?.('(prefers-reduced-motion: reduce)').matches) return;
    const timers: ReturnType<typeof setTimeout>[] = [];
    const circle = () => {
      chunks.forEach((_, i) => timers.push(setTimeout(() => parts[i]?.play(), i * GAP)));
    };
    timers.push(setTimeout(circle, 500));
    const loop = setInterval(circle, PERIOD + chunks.length * GAP);
    return () => {
      timers.forEach(clearTimeout);
      clearInterval(loop);
    };
  });
</script>

<FloatingField bind:value bind:focused {label} {onenter} hideText={showOverlay}>
  {#snippet overlay()}
    {#each chunks as chunk, i (i)}
      <DecryptedText
        bind:this={parts[i]}
        text={chunk}
        animateOn="manual"
        speed={70}
        maxIterations={12}
        sequential={false}
      />
    {/each}
  {/snippet}
</FloatingField>
