<script>
  let { selectionPopup, onAddHighlight, onSaveQuote, onDismiss } = $props();
</script>

{#if selectionPopup}
  <div
    class="highlight-popup"
    style="left: {selectionPopup.x}px; top: {selectionPopup.y}px;"
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
