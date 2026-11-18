<script lang="ts">
  import { t } from '../lib/i18n.svelte';
  let {
    note,
  }: {
    note?: string;
  } = $props();
</script>

<section class="placeholder">
  <p>{note ?? t('inDevelopment')}</p>
</section>

<style>
  .placeholder {
    padding: 24px;
    border: 1px dashed var(--page-text);
    border-radius: var(--radius-md);
    color: var(--page-text);
  }

  p {
    margin: 0;
  }
</style>
