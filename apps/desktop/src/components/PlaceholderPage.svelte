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
    border: 1px dashed #343434;
    border-radius: 12px;
    color: #8b8b8b;
  }

  p {
    margin: 0;
  }
</style>
