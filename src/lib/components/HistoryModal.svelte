<script>
  import { fly } from "svelte/transition";
  import { modalTransition } from "$lib/components/panelTransitions.js";

  let { historyEntries, onClose, onDeleteEntry } = $props();
</script>

<div class="panel-overlay" role="dialog" aria-modal="true" aria-label="Book history">
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div class="panel-backdrop" role="presentation" onclick={onClose}></div>
  <div class="hotkeys-modal" transition:fly={modalTransition}>
    <div class="hotkeys-header">
      <span>Book History</span>
      <button class="icon-btn" onclick={onClose} aria-label="Close">&times;</button>
    </div>
    {#if historyEntries.length === 0}
      <p class="panel-empty" style="padding: 1rem 0.5rem;">No books in history yet.</p>
    {:else}
      <ul class="history-list">
        {#each historyEntries as entry}
          <li class="history-item">
            <div class="history-info">
              <span class="history-title">{entry.title}</span>
              <span class="history-author">{entry.author}</span>
              <span class="history-meta">
                Ch. {entry.last_chapter + 1}
                {#if entry.has_bookmarks}<span class="history-badge" title="Has bookmarks">&#128209;</span>{/if}
                {#if entry.has_quotes}<span class="history-badge" title="Has quotes">&#128172;</span>{/if}
              </span>
            </div>
            <button class="hl-remove" title="Remove from history" onclick={() => onDeleteEntry(entry)}>&times;</button>
          </li>
        {/each}
      </ul>
    {/if}
  </div>
</div>
