<script lang="ts">
  import { cachedIcon, loadIcon } from '../lib/iconLoader';
  import { iconColor } from '../lib/processes';

  /**
   * Значок приложения: настоящая иконка файла в скруглённом квадрате или цветная буква.
   * `resolve` вызывается лениво, когда строка попадает в видимую область, один раз на `cacheKey`.
   */
  let {
    name,
    cacheKey = null,
    resolve = null,
    size = 28,
  }: {
    name: string;
    cacheKey?: string | null;
    resolve?: (() => Promise<string | null>) | null;
    size?: number;
  } = $props();

  let host = $state<HTMLSpanElement | null>(null);
  let url = $state<string | null>(null);
  let failed = $state(false);

  $effect(() => {
    const key = cacheKey;
    const source = resolve;
    url = null;
    failed = false;
    if (!key || !source || !host) return;
    const known = cachedIcon(key);
    if (known !== undefined) {
      url = known;
      return;
    }
    let cancelled = false;
    const start = () => {
      const ready = cachedIcon(key);
      if (ready !== undefined) {
        url = ready;
        return;
      }
      void loadIcon(key, source).then((value) => {
        if (!cancelled) url = value;
      });
    };
    if (typeof IntersectionObserver === 'undefined') {
      start();
      return () => (cancelled = true);
    }
    const observer = new IntersectionObserver((entries) => {
      if (entries.some((entry) => entry.isIntersecting)) {
        observer.disconnect();
        start();
      }
    });
    observer.observe(host);
    return () => {
      cancelled = true;
      observer.disconnect();
    };
  });
</script>

<span
  class="icon"
  class:has-image={url !== null && !failed}
  style:--size="{size}px"
  style:background={url !== null && !failed ? 'transparent' : iconColor(name)}
  aria-hidden="true"
  bind:this={host}
>
  {#if url !== null && !failed}
    <img src={url} alt="" draggable="false" onerror={() => (failed = true)} />
  {:else}
    {(/[\p{L}\p{N}]/u.exec(name)?.[0] ?? name.charAt(0)).toUpperCase()}
  {/if}
</span>

<style>
  .icon {
    display: inline-flex;
    flex: none;
    align-items: center;
    justify-content: center;
    width: var(--size);
    height: var(--size);
    border-radius: var(--radius-lg);
    color: #080808;
    font-size: calc(var(--size) * 0.5);
    font-weight: 800;
    overflow: hidden;
  }
  .has-image {
    background: rgba(255, 255, 255, 0.06) !important;
  }
  img {
    width: 100%;
    height: 100%;
    object-fit: contain;
    padding: 2px;
    image-rendering: auto;
  }
</style>
