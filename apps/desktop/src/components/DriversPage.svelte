<script lang="ts">
  import { DRIVERS_REASON, UNSUPPORTED_OS_REASON } from '../lib/computer';
  import PlaceholderPage from './PlaceholderPage.svelte';

  type OsId = 'windows' | 'macos' | 'linux';

  /** Включена только Windows; остальные ОС выключены с причиной. */
  const OS_OPTIONS: readonly { id: OsId; label: string; reason: string | null }[] = [
    { id: 'windows', label: 'Windows', reason: null },
    { id: 'macos', label: 'macOS', reason: UNSUPPORTED_OS_REASON },
    { id: 'linux', label: 'Linux', reason: UNSUPPORTED_OS_REASON },
  ];

  let selected = $state<OsId>('windows');
</script>

<section class="drivers">
  <div class="os-switch" role="group" aria-label="Операционная система">
    {#each OS_OPTIONS as option (option.id)}
      <button
        type="button"
        aria-pressed={selected === option.id}
        disabled={option.reason !== null}
        aria-describedby={option.reason ? `os-reason-${option.id}` : undefined}
        onclick={() => (selected = option.id)}
      >
        {option.label}
      </button>
    {/each}
  </div>

  {#each OS_OPTIONS as option (option.id)}
    {#if option.reason}
      <p class="reason" id="os-reason-{option.id}">{option.label}: {option.reason}</p>
    {/if}
  {/each}

  {#if selected === 'windows'}
    <PlaceholderPage note={DRIVERS_REASON} />
  {/if}
</section>

<style>
  .os-switch {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin-bottom: 12px;
  }

  button {
    min-height: 44px;
    padding: 10px 16px;
    border: 0;
    border-radius: var(--radius-sm);
    background: var(--black);
    color: var(--ink);
    font: inherit;
    font-weight: 800;
    text-transform: uppercase;
    font-size: var(--text-body);
    cursor: pointer;
  }

  button[aria-pressed='true'] {
    background: var(--ink);
    color: var(--black);
  }

  button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .reason {
    margin: 4px 0;
    font-size: var(--text-small);
    color: var(--page-text);
  }
</style>
