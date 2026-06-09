<script>
  let { menu, onSaveQuote, onDismiss } = $props();
</script>

{#if menu}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div class="ctx-backdrop" role="presentation" onclick={onDismiss}></div>
  <div class="highlight-ctx-menu" style="left: {menu.x}px; top: {menu.y}px;">
    <button class="ctx-item" onclick={() => onSaveQuote(menu?.text ?? "")}>Save as Quote</button>
  </div>
{/if}
