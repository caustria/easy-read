<script>
  import { tick } from "svelte";

  let { selectionPopup, onAddHighlight, onSaveQuote, onDismiss } = $props();
  /** @type {HTMLDivElement | null} */
  let popupEl = $state(null);
  let popupPosition = $state({ x: 0, y: 0 });

  $effect(() => {
    const popup = selectionPopup;
    if (!popup) return;
    popupPosition = { x: popup.x, y: popup.y };
    tick().then(() => {
      if (!popupEl || selectionPopup !== popup) return;
      const rect = popupEl.getBoundingClientRect();
      const margin = 8;
      popupPosition = {
        x: Math.min(window.innerWidth - margin - rect.width / 2, Math.max(margin + rect.width / 2, popup.x)),
        y: Math.min(window.innerHeight - margin, Math.max(margin + rect.height, popup.y)),
      };
    });
  });
</script>

{#if selectionPopup}
  <div
    class="highlight-popup"
    bind:this={popupEl}
    style="left: {popupPosition.x}px; top: {popupPosition.y}px;"
  >
    <button class="hl-swatch" style="background: rgba(255,220,0,0.55)" onclick={() => onAddHighlight("rgba(255,220,0,0.55)")} title="Yellow"></button>
    <button class="hl-swatch" style="background: rgba(100,200,100,0.55)" onclick={() => onAddHighlight("rgba(100,200,100,0.55)")} title="Green"></button>
    <button class="hl-swatch" style="background: rgba(100,160,255,0.55)" onclick={() => onAddHighlight("rgba(100,160,255,0.55)")} title="Blue"></button>
    <button class="hl-swatch" style="background: rgba(255,120,120,0.55)" onclick={() => onAddHighlight("rgba(255,120,120,0.55)")} title="Red"></button>
    <span class="hl-popup-sep" aria-hidden="true"></span>
    <button class="hl-quote-btn" onclick={onSaveQuote} title="Save as Quote">&ldquo;</button>
    <button class="hl-dismiss" onclick={onDismiss}>&times;</button>
  </div>
{/if}
