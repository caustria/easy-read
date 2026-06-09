<script>
  import { fly } from "svelte/transition";
  import { modalTransition } from "$lib/components/panelTransitions.js";

  let { onClose } = $props();
</script>

<div class="panel-overlay" role="dialog" aria-modal="true" aria-label="Keyboard shortcuts">
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div class="panel-backdrop" role="presentation" onclick={onClose}></div>
  <div class="hotkeys-modal" transition:fly={modalTransition}>
    <div class="hotkeys-header">
      <span>Keyboard Shortcuts</span>
      <button class="icon-btn" onclick={onClose} aria-label="Close">&times;</button>
    </div>
    <table class="hotkeys-table">
      <tbody>
        <tr><td><kbd>Ctrl+O</kbd></td><td>Open book</td></tr>
        <tr><td><kbd>F11</kbd></td><td>Toggle fullscreen</td></tr>
        <tr><td><kbd>Ctrl+F</kbd></td><td>Search</td></tr>
        <tr><td><kbd>Ctrl+B</kbd></td><td>Toggle bookmarks</td></tr>
        <tr><td><kbd>Ctrl+Q</kbd></td><td>Toggle quotes</td></tr>
        <tr><td><kbd>Ctrl+H</kbd></td><td>Book history</td></tr>
        <tr><td><kbd>Ctrl++</kbd> / <kbd>Ctrl+-</kbd></td><td>Adjust font size</td></tr>
        <tr><td><kbd>b</kbd></td><td>Place / remove bookmark</td></tr>
        <tr><td><kbd>q</kbd></td><td>Save selection as quote</td></tr>
        <tr><td><kbd>j</kbd> / <kbd>&rarr;</kbd></td><td>Next page</td></tr>
        <tr><td><kbd>k</kbd> / <kbd>&larr;</kbd></td><td>Previous page</td></tr>
        <tr><td><kbd>Esc</kbd></td><td>Close panel</td></tr>
        <tr><td><kbd>;</kbd> / <kbd>?</kbd></td><td>Show this menu</td></tr>
      </tbody>
    </table>
  </div>
</div>
