<script lang="ts">
  import { onMount } from 'svelte';
  import { boostSetProfile, tweakSet, tweaksRevert } from '../lib/tweaksApi';
  import { TweakRunner, tweakErrorText } from '../lib/tweakRunner.svelte';
  import {
    BOOST_EXTRAS,
    BOOST_PROFILES,
    SAVED_NOTE,
    VBS_CONFIRM,
    profileStepTitle,
    restartText,
    type ProfileText,
  } from '../lib/tweaksText';
  import TweakRow from './TweakRow.svelte';

  const runner = new TweakRunner();

  /** Опасная настройка, которая ждёт ответа в окне подтверждения. */
  let confirming = $state<string | null>(null);
  let cancelButton = $state<HTMLButtonElement | null>(null);

  onMount(() => {
    void runner.refresh();
  });

  $effect(() => {
    if (confirming) cancelButton?.focus();
  });

  const unsupported = $derived(runner.error?.kind === 'unsupported');
  const hasBackup = $derived(
    (runner.data?.boost.hasBackup ?? false) ||
      BOOST_EXTRAS.some((tweak) => runner.state(tweak.id)?.hasBackup),
  );
  const needsAdminShown = $derived(
    runner.data !== null &&
      !runner.data.elevated &&
      BOOST_EXTRAS.some((tweak) => runner.state(tweak.id)?.availability === 'needsAdmin'),
  );

  function profileOn(id: string): boolean {
    return runner.shown(`profile:${id}`, runner.data?.boost.profile === id);
  }

  const steps = $derived(runner.data?.boost.steps ?? []);
  const activeProfile = $derived(BOOST_PROFILES.find((item) => profileOn(item.id)) ?? null);

  /** Профили взаимоисключающие: выбранный включается, остальные гаснут в тот же кадр;
      повторное нажатие на выбранный снимает выбор. */
  function chooseProfile(profile: ProfileText) {
    const enable = !profileOn(profile.id);
    const shownNow: Record<string, boolean> = {};
    for (const item of BOOST_PROFILES)
      shownNow[`profile:${item.id}`] = enable && item.id === profile.id;
    void runner.apply(shownNow, () => boostSetProfile(enable ? profile.id : null));
  }

  function toggleExtra(id: string, enable: boolean) {
    if (enable && runner.state(id)?.dangerous) {
      confirming = id;
      return;
    }
    void runner.apply({ [id]: enable }, () => tweakSet(id, enable));
  }

  function confirmDanger() {
    const id = confirming;
    confirming = null;
    if (id) void runner.apply({ [id]: true }, () => tweakSet(id, true, true));
  }

  const profileFailure = $derived(
    BOOST_PROFILES.map((item) => runner.failed[`profile:${item.id}`]).find(Boolean) ?? '',
  );
</script>

<section class="page">
  <h2 class="sr-only">Boost / Игры</h2>
  <p class="note">
    {SAVED_NOTE} Результат зависит от игры и оборудования: прироста FPS никто не обещает.
  </p>

  <div class="toolbar">
    <button
      type="button"
      disabled={unsupported || !hasBackup}
      onclick={() => void runner.apply({}, () => tweaksRevert('boost'))}
      >Вернуть изменения раздела</button
    >
    <button type="button" onclick={() => void runner.refresh()}>Обновить состояние</button>
  </div>

  {#if runner.error}
    <p class="error" role="alert">
      {tweakErrorText(runner.error, 'Boost недоступен на этой системе.')}
    </p>
  {/if}
  {#if needsAdminShown}
    <p class="admin" role="status">
      Часть оптимизаций меняет параметры всего компьютера. Они серые, пока OCHISTKA запущена без
      прав администратора.
    </p>
  {/if}

  <h3>Профиль для игр</h3>
  <div class="segments" role="radiogroup" aria-label="Профиль для игр">
    {#each BOOST_PROFILES as profile (profile.id)}
      <button
        type="button"
        role="radio"
        class="segment"
        class:on={profileOn(profile.id)}
        aria-checked={profileOn(profile.id)}
        disabled={runner.data === null || unsupported}
        onclick={() => chooseProfile(profile)}>{profile.title}</button
      >
    {/each}
  </div>
  <p class="profile-text">
    {activeProfile?.description ?? 'Профиль не выбран. Нажми на профиль, чтобы включить его.'}
  </p>
  {#if profileFailure}<p class="fail" role="alert">{profileFailure}</p>{/if}
  {#if steps.length > 0}
    <ul class="steps" aria-label="Состав профиля">
      {#each steps as step (step.id)}
        <li class:skipped={!step.applied}>
          <span class="mark" aria-hidden="true">{step.applied ? '✓' : '—'}</span>
          <span>{profileStepTitle(step.id)}</span>
          {#if step.reason}<span class="why">{step.reason}</span>{/if}
        </li>
      {/each}
    </ul>
  {/if}

  <h3>Отдельные оптимизации</h3>
  <div class="rows">
    {#each BOOST_EXTRAS as tweak (tweak.id)}
      {@const info = runner.state(tweak.id)}
      <TweakRow
        title={tweak.title}
        description={tweak.description}
        checked={runner.shown(tweak.id, info?.applied ?? false)}
        disabled={!info || unsupported || info.availability !== 'available'}
        pending={runner.isPending(tweak.id)}
        failure={runner.failed[tweak.id] ?? ''}
        reason={info?.reason ?? null}
        restart={info ? restartText(info.restart) : ''}
        onchange={(next) => toggleExtra(tweak.id, next)}
      />
    {/each}
  </div>
</section>

{#if confirming}
  <div class="backdrop">
    <div class="dialog" role="dialog" aria-modal="true" aria-labelledby="vbs-title">
      <h3 id="vbs-title">{VBS_CONFIRM.title}</h3>
      <p>{VBS_CONFIRM.body}</p>
      <div class="dialog-actions">
        <button type="button" bind:this={cancelButton} onclick={() => (confirming = null)}
          >{VBS_CONFIRM.cancel}</button
        >
        <button type="button" class="danger" onclick={confirmDanger}>{VBS_CONFIRM.ok}</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .page h2 {
    margin: 0 0 8px;
    font-size: var(--text-title);
    text-transform: uppercase;
  }

  h3 {
    margin: 12px 0 8px;
    font-size: var(--text-body);
    text-transform: uppercase;
    color: var(--page-text);
  }

  .note {
    margin: 0 0 8px;
    font-size: var(--text-small);
    color: var(--page-text);
  }

  .toolbar {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  .toolbar button,
  .dialog-actions button {
    min-height: 36px;
    padding: 6px 14px;
    border: 0;
    border-radius: var(--radius-sm);
    background: #222;
    color: var(--ink, #fafafa);
    font: inherit;
    font-size: var(--text-small);
    cursor: pointer;
    transition: background-color 160ms ease;
  }

  .toolbar button:hover:not(:disabled),
  .dialog-actions button:hover {
    background: #333;
  }

  .toolbar button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  /* Сегмент-переключатель профилей: один активный. */
  .segments {
    display: inline-flex;
    max-width: 100%;
    padding: 3px;
    border-radius: var(--radius-pill);
    background: var(--black, #080808);
    box-shadow: 0 0 0 2px var(--ink, #fafafa);
  }

  .segment {
    min-height: 32px;
    padding: 4px 20px;
    border: 0;
    border-radius: var(--radius-pill);
    background: transparent;
    color: var(--ink, #fafafa);
    font: inherit;
    font-size: var(--text-small);
    font-weight: 700;
    cursor: pointer;
    transition: background-color 160ms ease;
  }

  .segment.on {
    background: var(--accent, #f83c5c);
    color: var(--page-text, #fff);
  }

  .segment:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .profile-text {
    margin: 8px 0 0;
    font-size: var(--text-small);
    color: var(--page-text);
  }

  .steps {
    display: flex;
    flex-direction: column;
    gap: 2px;
    margin: 8px 0 0;
    padding: 0;
    list-style: none;
    font-size: var(--text-small);
    color: var(--page-text);
  }

  .steps li {
    display: flex;
    flex-wrap: wrap;
    gap: 2px 8px;
  }

  .steps .mark {
    width: 1.2em;
    text-align: center;
  }

  .steps .why {
    flex-basis: 100%;
    padding-left: calc(1.2em + 8px);
    font-size: var(--text-caption);
    opacity: 0.85;
  }

  .fail {
    margin: 4px 0 0;
    font-size: var(--text-small);
    color: var(--page-text);
    font-weight: 700;
  }

  .rows {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin-bottom: 16px;
  }

  .error,
  .admin {
    margin: 8px 0 0;
    padding: 8px 14px;
    border-radius: var(--radius-sm);
    background: var(--black, #080808);
    color: var(--ink, #fafafa);
    font-size: var(--text-small);
  }

  .error {
    border: 1px solid #da3633;
  }

  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 50;
    display: grid;
    place-items: center;
    padding: 16px;
    background: rgb(0 0 0 / 60%);
  }

  .dialog {
    max-width: 440px;
    padding: 20px;
    border-radius: var(--radius-lg);
    background: var(--black, #080808);
    color: var(--ink, #fafafa);
    box-shadow: 0 0 0 2px var(--ink, #fafafa);
  }

  .dialog h3 {
    margin: 0 0 8px;
    color: var(--ink, #fafafa);
  }

  .dialog p {
    margin: 0 0 16px;
    font-size: var(--text-small);
  }

  .dialog-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }

  .dialog-actions .danger {
    background: #da3633;
  }
</style>
