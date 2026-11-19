<script lang="ts">
  import { joinEntries, splitEntries } from '../lib/vpn';
  import { vpnSetRules, type RoutingRules, type VpnSnapshot } from '../lib/vpnApi';
  import { errorMessage } from '../lib/vpnNetwork';

  let {
    snapshot,
    onsaved,
  }: { snapshot: VpnSnapshot | null; onsaved: (rules: RoutingRules) => void } = $props();

  let directDomains = $state('');
  let proxyDomains = $state('');
  let directApps = $state('');
  let proxyApps = $state('');
  let presets = $state<string[]>([]);
  let loaded = false;
  let saving = $state(false);
  let message = $state<string | null>(null);
  let failed = $state(false);

  // Поля заполняются один раз, когда пришло сохранённое состояние; дальше их ведёт пользователь.
  $effect(() => {
    if (!snapshot || loaded) return;
    loaded = true;
    directDomains = joinEntries(snapshot.rules.directDomains);
    proxyDomains = joinEntries(snapshot.rules.proxyDomains);
    directApps = joinEntries(snapshot.rules.directApps);
    proxyApps = joinEntries(snapshot.rules.proxyApps);
    presets = [...snapshot.rules.presets];
  });

  const connected = $derived(snapshot?.connection.state === 'connected');

  function togglePreset(id: string, on: boolean) {
    presets = on ? [...presets.filter((p) => p !== id), id] : presets.filter((p) => p !== id);
  }

  async function save() {
    saving = true;
    message = null;
    failed = false;
    try {
      const saved = await vpnSetRules({
        directDomains: splitEntries(directDomains),
        proxyDomains: splitEntries(proxyDomains),
        directApps: splitEntries(directApps),
        proxyApps: splitEntries(proxyApps),
        presets,
      });
      directDomains = joinEntries(saved.directDomains);
      proxyDomains = joinEntries(saved.proxyDomains);
      directApps = joinEntries(saved.directApps);
      proxyApps = joinEntries(saved.proxyApps);
      presets = [...saved.presets];
      onsaved(saved);
      message = connected
        ? 'Сохранено. Правила применятся при следующем подключении: отключи и подключи VPN.'
        : 'Сохранено. Правила применятся при подключении.';
    } catch (err) {
      failed = true;
      message = errorMessage(err);
    } finally {
      saving = false;
    }
  }
</script>

{#if !snapshot}
  <p class="state" role="status">Читаем сохранённые правила…</p>
{:else}
  <section class="card">
    <h2>Готовые наборы</h2>
    <ul class="presets" aria-label="Готовые наборы">
      {#each snapshot.presets as preset (preset.id)}
        <li>
          <label class="preset">
            <input
              type="checkbox"
              checked={presets.includes(preset.id)}
              onchange={(e) => togglePreset(preset.id, e.currentTarget.checked)}
            />
            <span class="preset-text">
              <span class="preset-title">{preset.title}</span>
              <span class="note">{preset.description}</span>
            </span>
          </label>
        </li>
      {/each}
    </ul>
  </section>

  <div class="grid">
    <section class="card">
      <h2><label for="vpn-direct-domains">Домены напрямую</label></h2>
      <textarea
        id="vpn-direct-domains"
        rows="4"
        placeholder="example.ru"
        bind:value={directDomains}
        spellcheck="false"
      ></textarea>
    </section>
    <section class="card">
      <h2><label for="vpn-proxy-domains">Домены через VPN</label></h2>
      <textarea
        id="vpn-proxy-domains"
        rows="4"
        placeholder="example.com"
        bind:value={proxyDomains}
        spellcheck="false"
      ></textarea>
    </section>
    <section class="card">
      <h2><label for="vpn-direct-apps">Приложения напрямую</label></h2>
      <textarea
        id="vpn-direct-apps"
        rows="4"
        placeholder="steam.exe"
        bind:value={directApps}
        spellcheck="false"
      ></textarea>
    </section>
    <section class="card">
      <h2><label for="vpn-proxy-apps">Приложения через VPN</label></h2>
      <textarea
        id="vpn-proxy-apps"
        rows="4"
        placeholder="chrome.exe"
        bind:value={proxyApps}
        spellcheck="false"
      ></textarea>
    </section>
  </div>
  <p class="note hint">
    По одной записи в строке. Домен: <code>example.com</code> (с поддоменами),
    <code>full:host.example.com</code>, <code>keyword:слово</code>, адрес или сеть
    <code>10.0.0.0/8</code>. Приложение: имя файла, например <code>chrome.exe</code>. Локальные
    адреса всегда идут напрямую.
  </p>
  <div class="save-row">
    <button type="button" class="save" disabled={saving} onclick={save}>
      {saving ? 'Сохраняем…' : 'Сохранить'}
    </button>
    {#if message}
      <p class="note msg" class:failed role={failed ? 'alert' : 'status'}>{message}</p>
    {/if}
  </div>
{/if}

<style>
  .card {
    padding: 16px;
    border-radius: var(--radius-lg);
    background: var(--surface-alt);
    box-shadow: 0 0 0 2px #000;
  }

  h2 {
    margin: 0 0 8px;
    font-size: var(--text-caption);
    font-weight: 800;
    text-transform: uppercase;
    color: var(--ink);
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
    gap: 8px;
  }

  textarea {
    box-sizing: border-box;
    width: 100%;
    resize: vertical;
    padding: 8px 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--black);
    color: var(--ink);
    font: inherit;
    font-size: var(--text-caption);
  }

  .presets {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .preset {
    display: flex;
    align-items: flex-start;
    gap: 12px;
    cursor: pointer;
  }

  .preset input {
    flex: none;
    width: 20px;
    height: 20px;
    margin: 2px 0 0;
    accent-color: var(--accent);
  }

  .preset-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .preset-title {
    font-weight: 800;
    color: var(--ink);
  }

  .note {
    margin: 0;
    font-size: var(--text-caption);
    color: var(--muted);
  }

  .hint {
    color: #000;
  }

  code {
    font-family: inherit;
    font-weight: 800;
  }

  .state {
    margin: 8px 0;
    color: #000;
  }

  .save-row {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .save {
    min-height: 44px;
    padding: 8px 22px;
    border: 0;
    border-radius: var(--radius-sm);
    background: var(--btn-bg);
    color: var(--ink);
    box-shadow: inset 0 0 0 1px var(--btn-border);
    font: inherit;
    font-size: var(--text-caption);
    font-weight: 800;
    cursor: pointer;
  }

  .save:disabled {
    opacity: 0.6;
    cursor: default;
  }

  .msg {
    color: #000;
    font-weight: 800;
  }

  .msg.failed {
    color: #7a0020;
  }
</style>
