<script lang="ts">
  import { getSystemOverview, POLL_INTERVAL_MS, toCommandError } from '../lib/api';
  import { formatBytes, formatPercent } from '../lib/format';
  import type { CommandError, SystemOverview } from '../lib/types';

  let overview = $state<SystemOverview | null>(null);
  let error = $state<CommandError | null>(null);

  $effect(() => {
    // Ответ, пришедший после размонтирования, игнорируется.
    let active = true;

    const load = async () => {
      try {
        const next = await getSystemOverview();
        if (active) {
          overview = next;
          error = null;
        }
      } catch (err) {
        if (active) {
          error = toCommandError(err);
        }
      }
    };

    load();
    const timer = setInterval(load, POLL_INTERVAL_MS);
    return () => {
      active = false;
      clearInterval(timer);
    };
  });
</script>

<h1>Главная</h1>

{#if error}
  <section class="alert" role="alert">
    <h2>
      {error.kind === 'unsupported'
        ? 'Недоступно на этой ОС'
        : 'Не удалось прочитать системные данные'}
    </h2>
    <p>{error.message}</p>
  </section>
{:else if overview}
  <section class="card">
    <h2>Процессор</h2>
    <p>Загрузка: {formatPercent(overview.cpu.usagePercent)}</p>
    <p>Логических ядер: {overview.cpu.logicalCores}</p>
    <div
      class="bar"
      role="progressbar"
      aria-label="Загрузка процессора"
      aria-valuemin="0"
      aria-valuemax="100"
      aria-valuenow={Math.round(overview.cpu.usagePercent)}
    >
      <div class="fill" style:width="{overview.cpu.usagePercent}%"></div>
    </div>
  </section>

  <section class="card">
    <h2>Оперативная память</h2>
    <p>
      {formatBytes(overview.memory.usedBytes)} из {formatBytes(overview.memory.totalBytes)}
    </p>
    <p>{formatPercent(overview.memory.usagePercent)}</p>
    <div
      class="bar"
      role="progressbar"
      aria-label="Использование памяти"
      aria-valuemin="0"
      aria-valuemax="100"
      aria-valuenow={Math.round(overview.memory.usagePercent)}
    >
      <div class="fill" style:width="{overview.memory.usagePercent}%"></div>
    </div>
  </section>

  <section class="card">
    <h2>Диски</h2>
    {#each overview.disks as disk (disk.mountPoint)}
      <div class="disk">
        <p>
          <span>{disk.displayName}</span>
          {#if disk.displayName !== disk.mountPoint}
            <span class="muted">{disk.mountPoint}</span>
          {/if}
        </p>
        <p>
          {formatBytes(disk.usedBytes)} из {formatBytes(disk.totalBytes)} · {formatPercent(
            disk.usagePercent,
          )}
        </p>
        <div
          class="bar"
          role="progressbar"
          aria-label={`Заполнение диска ${disk.displayName}`}
          aria-valuemin="0"
          aria-valuemax="100"
          aria-valuenow={Math.round(disk.usagePercent)}
        >
          <div class="fill" style:width="{disk.usagePercent}%"></div>
        </div>
      </div>
    {:else}
      <p>Диски не найдены.</p>
    {/each}
  </section>
{:else}
  <p>Загрузка…</p>
{/if}

<style>
  h1 {
    margin-top: 0;
  }

  h2 {
    margin: 0 0 8px;
    font-size: 16px;
  }

  p {
    margin: 4px 0;
  }

  .card,
  .alert {
    margin-bottom: 16px;
    padding: 16px;
    border-radius: 8px;
    background: #161b22;
  }

  .alert {
    border: 1px solid #da3633;
  }

  .muted {
    margin-left: 8px;
    color: #8b949e;
  }

  .bar {
    height: 8px;
    margin-top: 8px;
    border-radius: 4px;
    background: #21262d;
    overflow: hidden;
  }

  .fill {
    height: 100%;
    background: #1f6feb;
  }

  .disk + .disk {
    margin-top: 12px;
  }
</style>
