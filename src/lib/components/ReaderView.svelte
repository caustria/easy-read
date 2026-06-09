<script>
  let {
    book,
    activePanel,
    currentChapter,
    currentChapterIndex,
    chapterHtml,
    scrollMode,
    pageOffset,
    isFullscreen,
    controlsVisible,
    isCurrentPageBookmarked,
    overallProgress,
    scrubbing,
    readerEl = $bindable(null),
    readingArea = $bindable(null),
    progressTrackEl = $bindable(null),
    onTogglePanel,
    onToggleBookmark,
    onReaderClick,
    onProgressDown,
  } = $props();
</script>

<header class="header" class:ctrl-hidden={isFullscreen && !controlsVisible}>
  <div class="header-left">
    <button class="icon-btn" onclick={() => onTogglePanel("toc")} aria-label="Table of contents">&#9776;</button>
    <span class="header-title">{book.metadata.title}</span>
    <span class="header-sep" aria-hidden="true">&mdash;</span>
    <span class="header-author">{book.metadata.author}</span>
  </div>
  <div class="header-right">
    <button
      class="icon-btn bookmark-btn"
      class:bookmarked={isCurrentPageBookmarked}
      onclick={onToggleBookmark}
      aria-label={isCurrentPageBookmarked ? "Remove bookmark" : "Add bookmark"}
      title="Toggle bookmark"
    >{#if isCurrentPageBookmarked}<svg width="13" height="17" viewBox="0 0 13 17" fill="currentColor" aria-hidden="true"><path d="M1 1h11a1 1 0 0 1 1 1v13l-6.5-4L1 15V2a1 1 0 0 1 1-1z"/></svg>{:else}<svg width="13" height="17" viewBox="0 0 13 17" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true"><path d="M1 1h11a1 1 0 0 1 1 1v13l-6.5-4L1 15V2a1 1 0 0 1 1-1z"/></svg>{/if}</button>
    <button class="icon-btn" onclick={() => onTogglePanel("bookmarks")} aria-label="Bookmarks">&#8942;</button>
    <button class="icon-btn" class:quotes-active={activePanel === "quotes"} onclick={() => onTogglePanel("quotes")} aria-label="Quotes" title="Quotes (Ctrl+Q)">&ldquo;</button>
    <button class="icon-btn" onclick={() => onTogglePanel("settings")} aria-label="Settings">&#9881;</button>
  </div>
</header>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<main
  class="reading-area"
  class:scroll-mode={scrollMode}
  bind:this={readingArea}
  onclick={onReaderClick}
>
  {#if chapterHtml}
    <div
      class="page-content"
      bind:this={readerEl}
      style="transform: {scrollMode ? 'none' : `translateY(${-pageOffset}px)`}"
    >
      {@html chapterHtml}
    </div>
  {:else}
    <p class="no-content">No content for this chapter.</p>
  {/if}
</main>

<footer class="footer" class:ctrl-hidden={isFullscreen && !controlsVisible}>
  <div class="footer-meta">
    <span class="footer-left">
      <span class="footer-chapter">{currentChapter?.title ?? `Chapter ${currentChapterIndex + 1}`}</span>
    </span>
  </div>
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="progress-track"
    class:scrubbing
    bind:this={progressTrackEl}
    onmousedown={onProgressDown}
  >
    <div class="progress-fill" style="width: {overallProgress}%"></div>
  </div>
</footer>
