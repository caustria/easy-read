<script>
  import { tick } from "svelte";

  let { menu, onSaveQuote, onDismiss } = $props();
  /** @type {HTMLDivElement | null} */
  let menuEl = $state(null);
  let menuPosition = $state({ x: 0, y: 0 });

  $effect(() => {
    const currentMenu = menu;
    if (!currentMenu) return;
    menuPosition = { x: currentMenu.x, y: currentMenu.y };
    tick().then(() => {
      if (!menuEl || menu !== currentMenu) return;
      const rect = menuEl.getBoundingClientRect();
      const margin = 8;
      menuPosition = {
        x: Math.min(window.innerWidth - margin - rect.width, Math.max(margin, currentMenu.x)),
        y: Math.min(window.innerHeight - margin - rect.height, Math.max(margin, currentMenu.y)),
      };
    });
  });
</script>

{#if menu}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div class="ctx-backdrop" role="presentation" onclick={onDismiss}></div>
  <div class="highlight-ctx-menu" bind:this={menuEl} style="left: {menuPosition.x}px; top: {menuPosition.y}px;">
    <button class="ctx-item" onclick={() => onSaveQuote(menu?.text ?? "")}>Save as Quote</button>
  </div>
{/if}
