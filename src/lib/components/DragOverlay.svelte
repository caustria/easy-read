<script>
  let { isDragOver, isDragBad } = $props();
</script>

{#if isDragOver}
  <div class="drag-overlay" class:bad={isDragBad} aria-hidden="true"></div>
  <div class="drag-border" class:bad={isDragBad} aria-hidden="true">
    {#if isDragBad}<span class="drag-bad-msg">Unsupported format &middot; .epub .txt .md</span>{/if}
  </div>
{/if}
