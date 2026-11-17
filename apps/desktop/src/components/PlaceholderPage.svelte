<script lang="ts">
  let {
    note = 'В разработке',
  }: {
    note?: string;
  } = $props();
</script>

<section class="placeholder">
  <p>{note}</p>
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
