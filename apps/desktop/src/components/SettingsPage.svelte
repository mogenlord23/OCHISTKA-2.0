<script lang="ts">
  import { onMount } from 'svelte';
  import {
    openActivationSettings,
    tweakSet,
    tweaksRevert,
    windowsActivation,
  } from '../lib/tweaksApi';
  import { TweakRunner, tweakErrorText } from '../lib/tweakRunner.svelte';
  import {
    ACTIVATION_TEXT,
    SETTINGS_CATEGORIES,
    SETTINGS_TWEAKS,
    restartText,
  } from '../lib/tweaksText';
  import type { ActivationInfo } from '../lib/types';
  import TweakRow from './TweakRow.svelte';

  const runner = new TweakRunner();

  /** Явно открытые и закрытые группы; без выбора открыта только первая. */
  let opened = $state<Record<string, boolean>>({});
  let activation = $state<ActivationInfo | null>(null);
  let activationFailed = $state(false);

  onMount(() => {
    void runner.refresh();
    windowsActivation().then(
      (info) => (activation = info),
      () => (activationFailed = true),
    );
  });

  const unsupported = $derived(runner.error?.kind === 'unsupported');
  const needsAdminShown = $derived(
    runner.data !== null &&
      !runner.data.elevated &&
      SETTINGS_TWEAKS.some((tweak) => runner.state(tweak.id)?.availability === 'needsAdmin'),
  );

  const groups = SETTINGS_CATEGORIES;

  const hasBackup = $derived(SETTINGS_TWEAKS.some((tweak) => runner.state(tweak.id)?.hasBackup));

  function isOpen(id: string, index: number): boolean {
    return id in opened ? opened[id] : index === 0;
  }

  function toggleGroup(id: string, index: number) {
    opened[id] = !isOpen(id, index);
  }

  function appliedCount(ids: readonly { id: string }[]): number {
    return ids.filter((tweak) => runner.shown(tweak.id, runner.state(tweak.id)?.applied ?? false))
      .length;
  }

  const toggle = (id: string, enable: boolean) =>
    void runner.apply({ [id]: enable }, () => tweakSet(id, enable));

  const activationLine = $derived(
    activationFailed
      ? ACTIVATION_TEXT.unknown
      : activation
        ? ACTIVATION_TEXT[activation.status] +
          (activation.graceDays !== null ? `, осталось дней: ${activation.graceDays}` : '')
        : 'Проверяем активацию…',
  );
</script>

<section class="page">
  <h2 class="sr-only">Настройки Windows</h2>

  <div class="activation" aria-label="Активация Windows">
    <div class="activation-text">
      <span class="activation-title">Активация Windows</span>
      <span class="activation-state" role="status">{activationLine}</span>
    </div>
    <button type="button" onclick={() => void openActivationSettings().catch(() => {})}
      >Открыть параметры активации</button
    >
  </div>

  <div class="toolbar">
    <button
      type="button"
      disabled={unsupported || !hasBackup}
      onclick={() => void runner.apply({}, () => tweaksRevert('settings'))}
      >Вернуть изменения</button
    >
    <button type="button" onclick={() => void runner.refresh()}>Обновить состояние</button>
  </div>

  {#if runner.error}
    <p class="error" role="alert">
      {tweakErrorText(runner.error, 'Настройки Windows недоступны на этой системе.')}
    </p>
  {/if}
  {#if needsAdminShown}
    <p class="admin" role="status">
      Часть настроек меняет параметры всего компьютера. Они серые, пока OCHISTKA запущена без прав
      администратора.
    </p>
  {/if}

  {#each groups as group, index (group.id)}
    <section class="group" id={`group-${group.id}`}>
      <button
        type="button"
        class="group-head"
        aria-expanded={isOpen(group.id, index)}
        onclick={() => toggleGroup(group.id, index)}
      >
        <span class="arrow" aria-hidden="true"></span>
        <span class="group-title">{group.title}</span>
        <span class="count">{appliedCount(group.tweaks)} из {group.tweaks.length}</span>
      </button>
      {#if isOpen(group.id, index)}
        <div class="rows">
          {#each group.tweaks as tweak (tweak.id)}
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
              onchange={(next) => toggle(tweak.id, next)}
            />
          {/each}
        </div>
      {/if}
    </section>
  {/each}
</section>

<style>
  .page h2 {
    margin: 0 0 8px;
    font-size: var(--text-title);
    text-transform: uppercase;
  }

  .activation {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    margin-bottom: 8px;
    padding: 8px 14px;
    border-radius: var(--radius-md);
    background: var(--black, #080808);
    color: var(--ink, #fafafa);
  }

  .activation-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .activation-title {
    font-size: var(--text-body);
    font-weight: 600;
  }

  .activation-state {
    font-size: var(--text-small);
    color: var(--ink-soft, #d4d4d4);
  }

  .toolbar {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin-bottom: 8px;
  }

  .toolbar button,
  .activation button {
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
  .activation button:hover {
    background: #333;
  }

  .toolbar button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .group {
    margin-bottom: 6px;
  }

  .group-head {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    min-height: 40px;
    padding: 6px 14px;
    border: 0;
    border-radius: var(--radius-md);
    background: #1a1a1a;
    color: var(--ink, #fafafa);
    font: inherit;
    text-align: left;
    cursor: pointer;
  }

  .group-head:hover {
    background: #242424;
  }

  .arrow {
    flex: none;
    width: 0;
    height: 0;
    border-left: 6px solid currentcolor;
    border-top: 5px solid transparent;
    border-bottom: 5px solid transparent;
    transition: transform 160ms ease;
  }

  .group-head[aria-expanded='true'] .arrow {
    transform: rotate(90deg);
  }

  .group-title {
    flex: 1;
    font-size: var(--text-body);
    font-weight: 700;
    text-transform: uppercase;
  }

  .count {
    font-size: var(--text-caption);
    color: #b8b8b8;
  }

  .rows {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin-top: 4px;
  }

  .error,
  .admin {
    margin: 0 0 8px;
    padding: 8px 14px;
    border-radius: var(--radius-sm);
    background: var(--black, #080808);
    color: var(--ink, #fafafa);
    font-size: var(--text-small);
  }

  .error {
    border: 1px solid #da3633;
  }
</style>
