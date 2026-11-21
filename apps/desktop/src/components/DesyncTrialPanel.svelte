<script lang="ts">
  import { onMount } from 'svelte';
  import Check from './Check.svelte';
  import {
    TRIAL_SERVICES,
    TRIAL_VERDICTS,
    desyncTrialNote,
    desyncTrialStart,
    desyncTrialStatus,
    desyncTrialStop,
    formatRemaining,
    trialErrorText,
    type TrialService,
    type TrialStatus,
    type TrialVerdict,
  } from '../lib/desyncTrial';

  let status = $state<TrialStatus | null>(null);
  let consent = $state(false);
  let busy = $state(false);
  let error = $state<string | null>(null);

  async function refresh() {
    try {
      status = await desyncTrialStatus();
    } catch (err) {
      error = trialErrorText(err);
    }
  }

  async function run(action: () => Promise<TrialStatus>) {
    busy = true;
    error = null;
    try {
      status = await action();
    } catch (err) {
      error = trialErrorText(err);
      await refresh();
    } finally {
      busy = false;
    }
  }

  const start = () => run(() => desyncTrialStart(consent));
  const stop = () => run(desyncTrialStop);

  function verdictOf(service: TrialService): TrialVerdict {
    return status?.notes.find((n) => n.service === service)?.verdict ?? 'unchecked';
  }

  // Повторное нажатие на выбранную оценку возвращает «не проверено».
  function mark(service: TrialService, verdict: TrialVerdict) {
    const next = verdictOf(service) === verdict ? 'unchecked' : verdict;
    void run(() => desyncTrialNote(service, next));
  }

  onMount(() => {
    void refresh();
    // Только чтение состояния из Rust; сетевых запросов нет.
    const timer = setInterval(() => void refresh(), 1000);
    return () => clearInterval(timer);
  });

  const canStart = $derived(
    consent && !busy && status != null && !status.active && status.unavailable === null,
  );
</script>

<section class="card trial" aria-label="Пробная проверка NetZapret">
  <h3 class="title">Пробная проверка NetZapret</h3>
  <p class="note">
    Это испытание, а не готовая функция. Запускается только вами и сама выключится через 2 минуты.
    Во время испытания Windows-драйвер перехватывает сетевой трафик: возможны сбои соединения. Если
    что-то пошло не так, нажмите «Остановить». VPN на это время должен быть выключен.
  </p>

  {#if status?.active}
    <p class="state" role="status">
      Идёт испытание · осталось {formatRemaining(status.remainingSecs)}
    </p>
    <div class="notes">
      <p class="note">
        Проверьте нужные сервисы сами и отметьте, что получилось. Ваши отметки — только наблюдение,
        они не подтверждают работу обхода.
      </p>
      {#each TRIAL_SERVICES as svc (svc.id)}
        <div class="note-row">
          <span class="svc">{svc.label}</span>
          <span class="verdicts" role="group" aria-label={svc.label}>
            {#each TRIAL_VERDICTS as v (v.id)}
              <button
                type="button"
                class="verdict"
                class:selected={verdictOf(svc.id) === v.id}
                aria-pressed={verdictOf(svc.id) === v.id}
                disabled={busy}
                onclick={() => mark(svc.id, v.id)}>{v.label}</button
              >
            {/each}
          </span>
        </div>
      {/each}
    </div>
    <button type="button" class="action stop" disabled={busy} onclick={stop}>Остановить</button>
  {:else}
    {#if status?.unavailable}
      <p class="note why" role="status">{status.unavailable}</p>
    {:else if status}
      <p class="note" role="status">{status.message}</p>
    {/if}
    <Check bind:checked={consent} label="Я понимаю риски и согласен запустить пробную проверку" />
    <button type="button" class="action" disabled={!canStart} onclick={start}
      >Запустить пробную проверку</button
    >
  {/if}

  {#if error}<p class="state error" role="alert">{error}</p>{/if}
</section>

<style>
  .trial {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 16px 18px;
    border-radius: var(--radius-md);
    background: var(--surface-alt);
    box-shadow: 0 0 0 2px #000;
  }

  .title {
    margin: 0;
    font-size: 1.1rem;
    font-weight: 800;
    color: var(--ink);
  }

  .note,
  .state {
    margin: 0;
    font-size: var(--text-caption);
    color: var(--muted);
  }

  .state {
    color: var(--ink);
    font-weight: 700;
  }

  .error {
    color: var(--danger, var(--ink));
  }

  .notes,
  .note-row {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .note-row {
    flex-direction: row;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
  }

  .svc {
    font-weight: 700;
    color: var(--ink);
  }

  .verdicts {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .verdict,
  .action {
    padding: 6px 12px;
    border: 0;
    border-radius: var(--radius-md);
    background: var(--surface);
    color: var(--ink);
    box-shadow: 0 0 0 2px transparent;
    font: inherit;
    cursor: pointer;
  }

  .verdict.selected {
    box-shadow: 0 0 0 2px var(--accent, var(--ink));
  }

  .action {
    align-self: flex-start;
    font-weight: 700;
    box-shadow: 0 0 0 2px #000;
  }

  .action:disabled,
  .verdict:disabled {
    opacity: 0.5;
    cursor: default;
  }
</style>
