<script lang="ts">
  import type { Snippet } from 'svelte';

  let {
    title,
    lead = '',
    tabs,
    footer = '',
    el = $bindable(),
    children,
  }: {
    title: string;
    lead?: string;
    tabs?: Snippet;
    footer?: string;
    el?: HTMLElement;
    children: Snippet;
  } = $props();
</script>

<!-- Общий шаблон страницы: одинаковое начало содержимого во всех разделах.
     Сверху фиксированный отступ, по бокам поля, контейнер по центру с предельной шириной. -->
<main bind:this={el}>
  <div class="container">
    <h1>{title}</h1>
    {#if lead}<p class="lead">{lead}</p>{/if}
    {#if tabs}{@render tabs()}{/if}
    <div class="page-body">
      {@render children()}
    </div>
    {#if footer}<footer>{footer}</footer>{/if}
  </div>
</main>

<style>
  main {
    flex: 1;
    min-width: 0;
    min-height: 0;
    overflow: hidden auto;
    padding: 0 var(--content-gutter, 24px) 8px;
    overscroll-behavior: contain;
  }

  .container {
    display: flex;
    flex-direction: column;
    min-height: 100%;
    max-width: 1480px;
    margin-inline: auto;
  }

  h1 {
    margin: 8px 0 0;
    font-family: var(--font-heading);
    font-size: 32px;
    font-weight: 900;
    line-height: 40px;
    text-transform: uppercase;
    color: var(--page-text);
  }

  .lead {
    margin: 8px 0 0;
    font-size: var(--text-small);
    font-weight: 700;
    color: var(--page-text);
  }

  .page-body {
    margin-top: 16px;
  }

  footer {
    margin-top: auto;
    padding-top: 16px;
    font-size: var(--text-caption);
    font-weight: 700;
    color: var(--page-text);
  }
</style>
