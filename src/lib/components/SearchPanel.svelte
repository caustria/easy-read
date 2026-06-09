<script>
  let {
    book,
    searchInputEl = $bindable(null),
    searchQuery = $bindable(""),
    searchMode,
    searchResults,
    currentMatchIndex,
    showResults,
    filteredBookmarks,
    filteredQuotes,
    onInput,
    onKeydown,
    onClose,
    onSetMode,
    onPreviousMatch,
    onNextMatch,
    onNavigateToMatch,
    onSelectBookmark,
    onSelectQuote,
  } = $props();
</script>

{#if book}
  <div class="search-overlay">
    <div class="search-bar-row">
      <!-- svelte-ignore a11y_autofocus -->
      <input
        class="search-input"
        bind:this={searchInputEl}
        bind:value={searchQuery}
        oninput={onInput}
        onkeydown={onKeydown}
        placeholder={searchMode === "book" ? "Search in book..." : searchMode === "bookmarks" ? "Filter bookmarks..." : "Filter quotes..."}
        autofocus
      />
      {#if searchMode === "book" && searchResults.length > 0}
        <span class="search-counter">
          {currentMatchIndex >= 0 ? currentMatchIndex + 1 : "-"} / {searchResults.length}
        </span>
        <button class="search-arrow" onclick={onPreviousMatch} title="Previous match">&uarr;</button>
        <button class="search-arrow" onclick={onNextMatch} title="Next match">&darr;</button>
      {/if}
      <button class="search-close" onclick={onClose}>&times;</button>
    </div>
    <div class="search-mode-row">
      <button class="mode-btn" class:active={searchMode === "book"} onclick={() => onSetMode("book")}>Book</button>
      <button class="mode-btn" class:active={searchMode === "bookmarks"} onclick={() => onSetMode("bookmarks")}>Bookmarks</button>
      <button class="mode-btn" class:active={searchMode === "quotes"} onclick={() => onSetMode("quotes")}>Quotes</button>
    </div>
    {#if searchMode === "book"}
      {#if showResults}
        {#if searchResults.length > 0}
          <ul class="search-results">
            {#each searchResults as result, i}
              <li>
                <!-- svelte-ignore a11y_click_events_have_key_events -->
                <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
                <button class="search-result-item" class:current={i === currentMatchIndex} onclick={() => onNavigateToMatch(i)}>
                  <span class="sr-chapter">{result.chapter_title ?? `Chapter ${result.chapter_index + 1}`}</span>
                  <span class="sr-snippet">{result.snippet}</span>
                </button>
              </li>
            {/each}
          </ul>
        {:else}
          <p class="search-empty">No results found.</p>
        {/if}
      {/if}
    {:else if searchMode === "bookmarks"}
      {#if filteredBookmarks.length > 0}
        <ul class="search-results">
          {#each filteredBookmarks as bookmark}
            <li>
              <!-- svelte-ignore a11y_click_events_have_key_events -->
              <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
              <button class="search-result-item" onclick={() => onSelectBookmark(bookmark)}>
                <span class="sr-chapter">Ch. {bookmark.chapter_index + 1}, Page {bookmark.page_index + 1}</span>
                <span class="sr-snippet">{bookmark.label ?? `Chapter ${bookmark.chapter_index + 1}, Page ${bookmark.page_index + 1}`}</span>
              </button>
            </li>
          {/each}
        </ul>
      {:else}
        <p class="search-empty">{searchQuery.trim() ? "No matching bookmarks." : "No bookmarks saved."}</p>
      {/if}
    {:else if searchMode === "quotes"}
      {#if filteredQuotes.length > 0}
        <ul class="search-results">
          {#each filteredQuotes as quote}
            <li>
              <!-- svelte-ignore a11y_click_events_have_key_events -->
              <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
              <button class="search-result-item" onclick={() => onSelectQuote(quote)}>
                <span class="sr-chapter">Ch. {quote.chapter_index + 1}</span>
                <span class="sr-snippet">"{quote.text}"</span>
              </button>
            </li>
          {/each}
        </ul>
      {:else}
        <p class="search-empty">{searchQuery.trim() ? "No matching quotes." : "No quotes saved."}</p>
      {/if}
    {/if}
  </div>
{/if}
