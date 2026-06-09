<script>
  import { fly } from "svelte/transition";
  import { leftPanelTransition } from "$lib/components/panelTransitions.js";

  let { book, bookmarks, currentChapterIndex, currentPage, onClose, onSelectBookmark } = $props();
</script>

{#if book}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div class="panel-overlay">
    <div class="panel-backdrop" role="presentation" onclick={onClose}></div>
    <nav class="side-panel" transition:fly={leftPanelTransition}>
      <p class="panel-label">BOOKMARKS</p>
      {#if bookmarks.length === 0}
        <p class="panel-empty">No bookmarks yet.</p>
      {:else}
        <ul class="panel-list">
          {#each bookmarks as bookmark}
            <li>
              <button
                class="panel-item"
                class:active={bookmark.chapter_index === currentChapterIndex && bookmark.page_index === currentPage}
                onclick={() => onSelectBookmark(bookmark)}
              >
                {bookmark.label ?? `Ch. ${bookmark.chapter_index + 1}, Page ${bookmark.page_index + 1}`}
              </button>
            </li>
          {/each}
        </ul>
      {/if}
    </nav>
  </div>
{/if}
