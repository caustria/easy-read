<script>
  import { fly } from "svelte/transition";
  import { leftPanelTransition } from "$lib/components/panelTransitions.js";

  let { book, currentChapterIndex, onClose, onSelectChapter } = $props();
</script>

{#if book}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div class="panel-overlay">
    <div class="panel-backdrop" role="presentation" onclick={onClose}></div>
    <nav class="side-panel" transition:fly={leftPanelTransition}>
      <p class="panel-label">TABLE OF CONTENTS</p>
      <ul class="panel-list">
        {#each book.chapters as chapter, i}
          <li>
            <button
              class="panel-item"
              class:active={i === currentChapterIndex}
              onclick={() => onSelectChapter(i)}
            >
              {chapter.title ?? `Chapter ${i + 1}`}
            </button>
          </li>
        {/each}
      </ul>
    </nav>
  </div>
{/if}
