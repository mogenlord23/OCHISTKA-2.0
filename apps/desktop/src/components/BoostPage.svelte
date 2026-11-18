<script lang="ts">
  import { toCommandError } from '../lib/api';
  import { boostSetProfile, tweakSet, tweaksOverview, tweaksRevert } from '../lib/tweaksApi';
  import {
    BOOST_EXTRAS,
    BOOST_PROFILES,
    CONFIRM_TEXT,
    type ProfileText,
    type TweakText,
  } from '../lib/tweaksText';
  import type { CommandError, TweaksOverview } from '../lib/types';
  import ConfirmDialog from './ConfirmDialog.svelte';
  import TweakCard from './TweakCard.svelte';

  let data = $state<TweaksOverview | null>(null);
  let error = $state<CommandError | null>(null);
  let busy = $state(false);
  type Pending =
    | { kind: 'profile'; profile: ProfileText; enable: boolean }
    | { kind: 'extra'; tweak: TweakText; enable: boolean }
    | { kind: 'revert' };
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
  const hasBackup = $derived(
    (data?.boost.hasBackup ?? false) ||
      BOOST_EXTRAS.some((tweak) => data?.tweaks.find((t) => t.id === tweak.id)?.hasBackup),
  );

  function applied(id: string): boolean {
    return data?.tweaks.find((tweak) => tweak.id === id)?.applied ?? false;
  }

  function confirm() {
    const action = pending;
    pending = null;
    if (!action) {
      return;
    }
    if (action.kind === 'profile') {
      void run(() => boostSetProfile(action.enable ? action.profile.id : null));
    } else if (action.kind === 'extra') {
      void run(() => tweakSet(action.tweak.id, action.enable));
    } else {
      void run(() => tweaksRevert('boost'));
    }
  }

  const dialogTitle = $derived.by(() => {
    if (!pending) {
      return '';
    }
    if (pending.kind === 'revert') {
      return 'Вернуть изменения раздела?';
    }
    const name =
      pending.kind === 'profile' ? `профиль «${pending.profile.title}»` : pending.tweak.title;
    return pending.enable ? `Включить ${name}?` : `Выключить ${name}?`;
  });
</script>

<section class="page">
  <h2>Boost / Игры</h2>
  <p class="lead">
    Профили уменьшают лишнюю фоновую нагрузку. Результат зависит от игры и оборудования:
    гарантированных процентов FPS нет.
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
      {unsupported ? 'Boost недоступен на этой системе.' : error.message}
    </p>
  {/if}

  <h3>Профиль для игр</h3>
  <div class="list">
    {#each BOOST_PROFILES as profile (profile.id)}
      <TweakCard
        title={profile.title}
        description={profile.description}
        checked={data?.boost.profile === profile.id}
        disabled={busy || data === null || unsupported}
        note={null}
        onchange={(next) => (pending = { kind: 'profile', profile, enable: next })}
      />
    {/each}
  </div>

  <h3>Отдельные параметры</h3>
  <div class="list">
    {#each BOOST_EXTRAS as tweak (tweak.id)}
      <TweakCard
        title={tweak.title}
        description={tweak.description}
        checked={applied(tweak.id)}
        disabled={busy || data === null || unsupported}
        onchange={(next) => (pending = { kind: 'extra', tweak, enable: next })}
      />
    {/each}
  </div>
</section>

{#if pending}
  <ConfirmDialog
    title={dialogTitle}
    text={pending.kind === 'revert'
      ? 'Всё, что изменил Boost, включая схему электропитания, вернётся к сохранённому состоянию.'
      : pending.enable
        ? CONFIRM_TEXT
        : 'Профиль выключится: изменения Boost и схема питания вернутся к сохранённым значениям.'}
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

  h3 {
    margin: 18px 0 10px;
    font-size: var(--text-body);
    text-transform: uppercase;
    color: var(--page-text);
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
    margin: 12px 0 0;
    padding: 10px 14px;
    border: 1px solid #da3633;
    border-radius: var(--radius-sm);
    background: var(--black, #080808);
    color: var(--ink, #fafafa);
  }
</style>
