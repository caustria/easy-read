<script>
  import { fly } from "svelte/transition";
  import { leftPanelTransition } from "$lib/components/panelTransitions.js";

  let { book, quotes, onClose, onSelectQuote, onDeleteQuote } = $props();
</script>

{#if book}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div class="panel-overlay">
    <div class="panel-backdrop" role="presentation" onclick={onClose}></div>
    <nav class="side-panel" transition:fly={leftPanelTransition}>
      <p class="panel-label">QUOTES</p>
      {#if quotes.length === 0}
        <p class="panel-empty">No saved quotes yet.</p>
      {:else}
        <ul class="panel-list">
          {#each quotes as quote}
            <li class="quote-list-item">
              <button class="quote-nav-btn" onclick={() => onSelectQuote(quote)}>
                <span class="quote-chapter-label">Ch. {quote.chapter_index + 1}</span>
                <span class="quote-body">"{quote.text}"</span>
              </button>
              <button class="hl-remove" onclick={() => onDeleteQuote(quote.id)} title="Delete quote">&times;</button>
            </li>
          {/each}
        </ul>
      {/if}
    </nav>
  </div>
{/if}
