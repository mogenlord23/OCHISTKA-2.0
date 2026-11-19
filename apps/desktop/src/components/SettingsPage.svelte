<script lang="ts">
  import { onMount } from 'svelte';
  import { tweakSet, tweaksRevert } from '../lib/tweaksApi';
  import { TweakRunner, tweakErrorText } from '../lib/tweakRunner.svelte';
  import { SETTINGS_TWEAKS } from '../lib/tweaksText';
  import TweakCard from './TweakCard.svelte';

  const runner = new TweakRunner();

  onMount(() => {
    void runner.refresh();
  });

  const unsupported = $derived(runner.error?.kind === 'unsupported');

  function applied(id: string): boolean {
    return runner.data?.tweaks.find((tweak) => tweak.id === id)?.applied ?? false;
  }

  const hasBackup = $derived(
    SETTINGS_TWEAKS.some((tweak) => runner.data?.tweaks.find((t) => t.id === tweak.id)?.hasBackup),
  );

  const toggle = (id: string, enable: boolean) =>
    void runner.apply({ [id]: enable }, () => tweakSet(id, enable));
</script>

<section class="page">
  <h2>Настройки Windows</h2>
  <p class="lead">
    Изменения интерфейса, рекламы и приватности. Выключение возвращает точные исходные значения.
  </p>
  <div class="toolbar">
    <button type="button" onclick={() => void runner.refresh()}>Обновить состояние</button>
    <button
      type="button"
      disabled={unsupported || !hasBackup}
      onclick={() => void runner.apply({}, () => tweaksRevert('settings'))}
      >Вернуть изменения раздела</button
    >
  </div>

  {#if runner.error}
    <p class="error" role="alert">
      {tweakErrorText(runner.error, 'Настройки Windows недоступны на этой системе.')}
    </p>
  {/if}

  <div class="list">
    {#each SETTINGS_TWEAKS as tweak (tweak.id)}
      <TweakCard
        title={tweak.title}
        description={tweak.description}
        checked={runner.shown(tweak.id, applied(tweak.id))}
        disabled={runner.data === null || unsupported}
        onchange={(next) => toggle(tweak.id, next)}
      />
    {/each}
  </div>
</section>

<style>
  .page h2 {
    margin: 0 0 8px;
    font-size: var(--text-title);
    text-transform: uppercase;
  }

  .lead {
    margin: 0 0 8px;
    font-size: var(--text-small);
    color: var(--page-text);
  }

  .toolbar {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin-bottom: 8px;
  }

  .toolbar button {
    min-height: 44px;
    padding: 8px 18px;
    border: 0;
    border-radius: var(--radius-sm);
    background: var(--black, #080808);
    color: var(--ink, #fafafa);
    font: inherit;
    font-size: var(--text-small);
    cursor: pointer;
    transition: background-color 160ms ease;
  }

  .toolbar button:hover:not(:disabled) {
    background: #222;
  }

  .toolbar button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .list {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin-bottom: 16px;
  }

  .error {
    margin: 0 0 8px;
    padding: 10px 14px;
    border: 1px solid #da3633;
    border-radius: var(--radius-sm);
    background: var(--black, #080808);
    color: var(--ink, #fafafa);
  }
</style>
