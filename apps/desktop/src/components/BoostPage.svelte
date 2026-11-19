<script lang="ts">
  import { onMount } from 'svelte';
  import { boostSetProfile, tweakSet, tweaksRevert } from '../lib/tweaksApi';
  import { TweakRunner, tweakErrorText } from '../lib/tweakRunner.svelte';
  import { BOOST_EXTRAS, BOOST_PROFILES, type ProfileText } from '../lib/tweaksText';
  import TweakCard from './TweakCard.svelte';

  const runner = new TweakRunner();

  onMount(() => {
    void runner.refresh();
  });

  const unsupported = $derived(runner.error?.kind === 'unsupported');
  const hasBackup = $derived(
    (runner.data?.boost.hasBackup ?? false) ||
      BOOST_EXTRAS.some((tweak) => runner.data?.tweaks.find((t) => t.id === tweak.id)?.hasBackup),
  );

  function applied(id: string): boolean {
    return runner.data?.tweaks.find((tweak) => tweak.id === id)?.applied ?? false;
  }

  /** Профили взаимоисключающие: выбранный включается, остальные гаснут в тот же кадр. */
  function chooseProfile(profile: ProfileText, enable: boolean) {
    const shownNow: Record<string, boolean> = {};
    for (const item of BOOST_PROFILES)
      shownNow[`profile:${item.id}`] = enable && item.id === profile.id;
    void runner.apply(shownNow, () => boostSetProfile(enable ? profile.id : null));
  }

  const toggleExtra = (id: string, enable: boolean) =>
    void runner.apply({ [id]: enable }, () => tweakSet(id, enable));
</script>

<section class="page">
  <h2>Boost / Игры</h2>
  <p class="lead">
    Профили уменьшают лишнюю фоновую нагрузку. Результат зависит от игры и оборудования:
    гарантированных процентов FPS нет.
  </p>
  <div class="toolbar">
    <button type="button" onclick={() => void runner.refresh()}>Обновить состояние</button>
    <button
      type="button"
      disabled={unsupported || !hasBackup}
      onclick={() => void runner.apply({}, () => tweaksRevert('boost'))}
      >Вернуть изменения раздела</button
    >
  </div>

  {#if runner.error}
    <p class="error" role="alert">
      {tweakErrorText(runner.error, 'Boost недоступен на этой системе.')}
    </p>
  {/if}

  <h3>Профиль для игр</h3>
  <div class="list">
    {#each BOOST_PROFILES as profile (profile.id)}
      <TweakCard
        title={profile.title}
        description={profile.description}
        checked={runner.shown(`profile:${profile.id}`, runner.data?.boost.profile === profile.id)}
        disabled={runner.data === null || unsupported}
        note={null}
        onchange={(next) => chooseProfile(profile, next)}
      />
    {/each}
  </div>

  <h3>Отдельные параметры</h3>
  <div class="list">
    {#each BOOST_EXTRAS as tweak (tweak.id)}
      <TweakCard
        title={tweak.title}
        description={tweak.description}
        checked={runner.shown(tweak.id, applied(tweak.id))}
        disabled={runner.data === null || unsupported}
        onchange={(next) => toggleExtra(tweak.id, next)}
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

  h3 {
    margin: 16px 0 8px;
    font-size: var(--text-body);
    text-transform: uppercase;
    color: var(--page-text);
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
    margin: 8px 0 0;
    padding: 10px 14px;
    border: 1px solid #da3633;
    border-radius: var(--radius-sm);
    background: var(--black, #080808);
    color: var(--ink, #fafafa);
  }
</style>
