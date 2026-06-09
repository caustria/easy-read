<script>
  let { isDragOver, error } = $props();
</script>

<main class="welcome" class:drag-active={isDragOver}>
  {#if isDragOver}
    <p class="drop-cta">Release to open</p>
  {:else}
    <p class="welcome-main">Drop a book here</p>
    <p class="welcome-hint">or press <kbd>Ctrl+O</kbd> to open one</p>
    <p class="welcome-formats">.epub  .txt  .md</p>
    {#if error}
      <p class="welcome-error">{error}</p>
    {/if}
  {/if}
</main>
