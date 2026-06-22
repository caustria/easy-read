<script>
  import { fly } from "svelte/transition";
  import { rightPanelTransition } from "$lib/components/panelTransitions.js";

  let {
    theme,
    fontFamily,
    fontSize,
    lineHeight,
    textAlign,
    scrollMode,
    currentChapterIndex,
    highlights,
    onClose,
    onSetTheme,
    onSetFontFamily,
    onAdjustFontSize,
    onAdjustLineHeight,
    onSetTextAlign,
    onToggleScrollMode,
    onRemoveHighlight,
  } = $props();

  /** @param {{ chapter_index: number }} highlight */
  function isCurrentChapterHighlight(highlight) {
    return highlight.chapter_index === currentChapterIndex;
  }

  let chapterHighlights = $derived(highlights.filter(isCurrentChapterHighlight));
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div class="panel-overlay">
  <div class="panel-backdrop" role="presentation" onclick={onClose}></div>
  <aside class="side-panel settings-panel" transition:fly={rightPanelTransition}>
    <p class="panel-label">THEME</p>
    <div class="theme-swatches">
      <button class="swatch" class:active={theme === "light"} onclick={() => onSetTheme("light")}>
        <span class="swatch-dot" style="background:#FFFFFF; border:1px solid #E5E7EB"></span>Light
      </button>
      <button class="swatch" class:active={theme === "dark"} onclick={() => onSetTheme("dark")}>
        <span class="swatch-dot" style="background:#1E1E1E"></span>Dark
      </button>
      <button class="swatch" class:active={theme === "sepia"} onclick={() => onSetTheme("sepia")}>
        <span class="swatch-dot" style="background:#FBF7EE; border:1px solid #D4CDBA"></span>Sepia
      </button>
    </div>

    <p class="panel-label">FONT</p>
    <div class="font-options">
      <button class="font-opt" class:active={fontFamily === "serif"} onclick={() => onSetFontFamily("serif")}
        style="font-family: var(--font-reading)">Serif</button>
      <button class="font-opt" class:active={fontFamily === "sans"} onclick={() => onSetFontFamily("sans")}
        style="font-family: var(--font-ui)">Sans</button>
      <button class="font-opt" class:active={fontFamily === "mono"} onclick={() => onSetFontFamily("mono")}
        style="font-family: var(--font-mono)">Mono</button>
    </div>

    <p class="panel-label">SIZE</p>
    <div class="step-control">
      <button class="step-btn" onclick={() => onAdjustFontSize(-1)}>&minus;</button>
      <span class="step-value">{fontSize}px</span>
      <button class="step-btn" onclick={() => onAdjustFontSize(1)}>+</button>
    </div>

    <p class="panel-label">LINE HEIGHT</p>
    <div class="step-control">
      <button class="step-btn" onclick={() => onAdjustLineHeight(-0.1)}>&minus;</button>
      <span class="step-value">{lineHeight.toFixed(1)}</span>
      <button class="step-btn" onclick={() => onAdjustLineHeight(0.1)}>+</button>
    </div>

    <p class="panel-label">ALIGNMENT</p>
    <div class="align-options">
      <button class="align-btn" class:active={textAlign === "left"} onclick={() => onSetTextAlign("left")} title="Left">
        Left
      </button>
      <button class="align-btn" class:active={textAlign === "justify"} onclick={() => onSetTextAlign("justify")} title="Justify">
        Justify
      </button>
    </div>

    <p class="panel-label">VIEW</p>
    <button class="view-toggle" onclick={onToggleScrollMode}>
      {scrollMode ? "Switch to paginated" : "Switch to scroll"}
    </button>

    {#if chapterHighlights.length > 0}
      <p class="panel-label">HIGHLIGHTS (this chapter)</p>
      <ul class="panel-list">
        {#each chapterHighlights as highlight}
          <li class="hl-list-item">
            <span class="hl-color-dot" style="background:{highlight.color}"></span>
            <span class="hl-note">{highlight.note ?? "No note"}</span>
            <button class="hl-remove" onclick={() => onRemoveHighlight(highlight)} title="Remove">&times;</button>
          </li>
        {/each}
      </ul>
    {/if}
  </aside>
</div>
