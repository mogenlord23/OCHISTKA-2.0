<script lang="ts">
  import { toCommandError } from '../lib/api';
  import { tweakSet, tweaksOverview, tweaksRevert } from '../lib/tweaksApi';
  import { CONFIRM_TEXT, SETTINGS_TWEAKS, type TweakText } from '../lib/tweaksText';
  import type { CommandError, TweaksOverview } from '../lib/types';
  import ConfirmDialog from './ConfirmDialog.svelte';
  import TweakCard from './TweakCard.svelte';

  let data = $state<TweaksOverview | null>(null);
  let error = $state<CommandError | null>(null);
  let busy = $state(false);
  type Pending = { kind: 'set'; tweak: TweakText; enable: boolean } | { kind: 'revert' };
  let pending = $state<Pending | null>(null);

  async function run(job: () => Promise<TweaksOverview>) {
    busy = true;
    try {
      data = await job();
      error = null;
    } catch (err) {
      error = toCommandError(err);
    } finally {
      busy = false;
    }
  }

  const refresh = () => run(tweaksOverview);

  $effect(() => {
    void refresh();
  });

  const unsupported = $derived(error?.kind === 'unsupported');

  function applied(id: string): boolean {
    return data?.tweaks.find((tweak) => tweak.id === id)?.applied ?? false;
  }

  const hasBackup = $derived(
    SETTINGS_TWEAKS.some((tweak) => data?.tweaks.find((t) => t.id === tweak.id)?.hasBackup),
  );

  function confirm() {
    const action = pending;
    pending = null;
    if (!action) {
      return;
    }
    if (action.kind === 'set') {
      void run(() => tweakSet(action.tweak.id, action.enable));
    } else {
      void run(() => tweaksRevert('settings'));
    }
  }
</script>

<section class="page">
  <h2>Настройки Windows</h2>
  <p class="lead">
    Изменения интерфейса, рекламы и приватности. Выключение возвращает точные исходные значения.
  </p>
  <div class="toolbar">
    <button type="button" disabled={busy} onclick={() => void refresh()}>Обновить состояние</button>
    <button
      type="button"
      disabled={busy || unsupported || !hasBackup}
      onclick={() => (pending = { kind: 'revert' })}>Вернуть изменения раздела</button
    >
  </div>

  {#if error}
    <p class="error" role="alert">
      {unsupported ? 'Настройки Windows недоступны на этой системе.' : error.message}
    </p>
  {/if}

  <div class="list">
    {#each SETTINGS_TWEAKS as tweak (tweak.id)}
      <TweakCard
        title={tweak.title}
        description={tweak.description}
        checked={applied(tweak.id)}
        disabled={busy || data === null || unsupported}
        onchange={(next) => (pending = { kind: 'set', tweak, enable: next })}
      />
    {/each}
  </div>
</section>

{#if pending}
  <ConfirmDialog
    title={pending.kind === 'revert'
      ? 'Вернуть изменения раздела?'
      : pending.enable
        ? `Применить: ${pending.tweak.title}?`
        : `Вернуть: ${pending.tweak.title}?`}
    text={pending.kind === 'revert'
      ? 'Все настройки этого раздела вернутся к значениям, сохранённым до применения.'
      : pending.enable
        ? CONFIRM_TEXT
        : 'Настройка вернётся к значению, сохранённому до применения.'}
    confirmLabel={pending.kind === 'revert' || !pending.enable ? 'Вернуть' : 'Применить'}
    onconfirm={confirm}
    oncancel={() => (pending = null)}
  />
{/if}

<style>
  .page h2 {
    margin: 0 0 8px;
    font-size: var(--text-title);
    text-transform: uppercase;
  }

  .lead {
    margin: 0 0 12px;
    font-size: var(--text-small);
    color: var(--page-text);
  }

  .toolbar {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
    margin-bottom: 14px;
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
    gap: 12px;
    margin-bottom: 16px;
  }

  .error {
    margin: 0 0 12px;
    padding: 10px 14px;
    border: 1px solid #da3633;
    border-radius: var(--radius-sm);
    background: var(--black, #080808);
    color: var(--ink, #fafafa);
  }
</style>
