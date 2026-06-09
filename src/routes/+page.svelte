<script>
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { onMount, onDestroy, tick } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import {
    calculateOverallProgress as calculateProgressPercent,
    calculatePageCount,
    calculatePageOffset,
    calculatePageStep,
    clampIndex,
    clampNonNegativeInteger,
    clampPage,
    nextReadingPosition,
    positionFromPercent,
    previousReadingPosition,
  } from "$lib/pagination.js";
  import {
    findTextRange,
    getTextOffset,
    isValidTextRange,
    unwrapMark,
    unwrapMarks,
    wrapTextRange,
  } from "$lib/textRanges.js";
  import "$lib/reader.css";
  import BookmarksPanel from "$lib/components/BookmarksPanel.svelte";
  import DragOverlay from "$lib/components/DragOverlay.svelte";
  import HighlightContextMenu from "$lib/components/HighlightContextMenu.svelte";
  import HighlightSelectionPopup from "$lib/components/HighlightSelectionPopup.svelte";
  import HistoryModal from "$lib/components/HistoryModal.svelte";
  import HotkeysModal from "$lib/components/HotkeysModal.svelte";
  import QuotesPanel from "$lib/components/QuotesPanel.svelte";
  import ReaderView from "$lib/components/ReaderView.svelte";
  import SearchPanel from "$lib/components/SearchPanel.svelte";
  import SettingsPanel from "$lib/components/SettingsPanel.svelte";
  import TocPanel from "$lib/components/TocPanel.svelte";
  import WelcomeView from "$lib/components/WelcomeView.svelte";

  /**
   * @typedef {{ title: string, author: string }} BookMetadata
   * @typedef {{ Html?: string, PlainText?: string }} ChapterContent
   * @typedef {{ title?: string | null, content: ChapterContent }} Chapter
   * @typedef {{ metadata: BookMetadata, chapters: Chapter[] }} Book
   * @typedef {{ chapter_index: number, page_index: number, label?: string | null }} Bookmark
   * @typedef {{ chapter_index: number, start_offset: number, end_offset: number, color: string, note?: string | null }} Highlight
   * @typedef {{ id: string, chapter_index: number, text: string, note?: string | null }} Quote
   * @typedef {{ last_chapter: number, last_page: number, last_scroll_top?: number | null, title: string, author: string, bookmarks?: Bookmark[], highlights?: Highlight[], quotes?: Quote[] }} BookRecord
   * @typedef {{ font_size?: number | null, theme?: Theme | null, font_family?: FontFamily | null, line_height?: number | null, text_align?: TextAlign | null, reader_mode?: ReaderMode | null } & Record<string, string | number | null | undefined>} Preferences
   * @typedef {{ preferences?: Preferences, last_opened?: string | null, books?: Record<string, BookRecord> }} AppState
   * @typedef {{ filePath: string, chapterIndex: number, pageIndex: number, title: string, author: string }} ProgressSnapshot
   * @typedef {{ chapter_index: number, chapter_title?: string | null, snippet: string }} SearchResult
   * @typedef {{ file_path: string, title: string, author: string, last_chapter: number, has_bookmarks?: boolean, has_quotes?: boolean }} HistoryEntry
   * @typedef {{ x: number, y: number, start: number, end: number, text: string }} SelectionPopup
   * @typedef {{ x: number, y: number, text: string }} HighlightContextMenu
   * @typedef {{ text: string, kind: "search" | "quote" }} PendingTextCue
   * @typedef {"toc" | "bookmarks" | "settings" | "search" | "quotes" | "hotkeys" | "history"} Panel
   * @typedef {"book" | "bookmarks" | "quotes"} SearchMode
   * @typedef {"serif" | "sans" | "mono"} FontFamily
   * @typedef {"left" | "justify"} TextAlign
   * @typedef {"light" | "dark" | "sepia"} Theme
   * @typedef {"paginated" | "scroll"} ReaderMode
   * @typedef {() => void} Unlisten
   */

  // ── Core reading state ──
  /** @type {Book | null} */
  let book = $state(null);
  /** @type {string | null} */
  let currentBookPath = $state(null);
  let currentChapterIndex = $state(0);
  let currentPage = $state(0);
  let totalPages = $state(1);

  // ── Typography preferences ──
  let fontSize = $state(18);
  let lineHeight = $state(1.6);
  /** @type {FontFamily} */
  let fontFamily = $state("serif"); // "serif" | "sans" | "mono"
  /** @type {TextAlign} */
  let textAlign = $state("left");   // "left" | "justify"
  /** @type {Theme} */
  let theme = $state("light");      // "light" | "dark" | "sepia"

  // ── UI panel state ──
  /** @type {Panel | null} */
  let activePanel = $state(null); // "toc" | "bookmarks" | "settings" | "search" | "quotes" | "hotkeys" | "history"
  let scrollMode = $state(false);
  let isDragOver = $state(false);
  let isDragBad = $state(false);
  let isFullscreen = $state(false);
  let controlsVisible = $state(true);
  /** @type {number | null} */
  let controlsHideTimer = null;
  /** @type {number | null} */
  let saveProgressTimer = null;
  /** @type {number | null} */
  let layoutSettlingTimer = null;
  let layoutSettlingVersion = 0;
  let pendingProgressSave = false;
  /** @type {number | null} */
  let pendingLastPageChapterIndex = null;
  /** @type {ProgressSnapshot | null} */
  let lastSavedProgress = null;
  /** @type {number | null} */
  let pendingScrollTopRestore = null;
  /** @type {Record<string, string | number | null | undefined>} */
  let savedPreferences = {};
  let layoutSettling = false;
  let scrubbing = $state(false);
  /** @type {HTMLDivElement | null} */
  let progressTrackEl = $state(null);

  // ── Bookmarks & app state ──
  /** @type {Bookmark[]} */
  let bookmarks = $state([]);
  /** @type {AppState | null} */
  let appState = $state(null);
  let error = $state("");

  // ── Search ──
  let searchQuery = $state("");
  /** @type {SearchResult[]} */
  let searchResults = $state([]);
  /** @type {HTMLInputElement | null} */
  let searchInputEl = $state(null);
  /** @type {PendingTextCue | null} */
  let pendingTextCue = $state(null);
  let searchTrigger = $state(0);        // incremented on each result click to force effect re-run
  let currentMatchIndex = $state(-1);   // which result we're currently on
  let showResults = $state(false);      // show dropdown only while actively typing
  /** @type {number | null} */
  let searchTimer = null;
  let searchRequestId = 0;

  // ── Highlights ──
  /** @type {Highlight[]} */
  let highlights = $state([]);
  /** @type {SelectionPopup | null} */
  let selectionPopup = $state(null);

  // ── Quotes ──
  /** @type {Quote[]} */
  let quotes = $state([]);
  /** @type {HistoryEntry[]} */
  let historyEntries = $state([]);
  /** @type {HighlightContextMenu | null} */
  let highlightCtxMenu = $state(null); // { x, y, text } for right-click on highlight

  // ── Search mode ──
  /** @type {SearchMode} */
  let searchMode = $state("book"); // "book" | "bookmarks" | "quotes"

  // ── DOM refs ──
  /** @type {HTMLDivElement | null} */
  let readerEl = $state(null);
  /** @type {HTMLElement | null} */
  let readingArea = $state(null);
  let containerHeight = $state(0);
  let contentHeight = $state(0);

  // ── Derived ──
  let pageStep = $derived(calculatePageStep(containerHeight, fontSize));
  let pageOffset = $derived(calculatePageOffset(contentHeight, containerHeight, pageStep, currentPage));
  let isCurrentPageBookmarked = $derived(
    bookmarks.some(b => b.chapter_index === currentChapterIndex && b.page_index === currentPage)
  );
  let currentChapter = $derived(getCurrentChapter(book, currentChapterIndex));
  let chapterHtml = $derived(getChapterHtml(currentChapter));

  let overallProgress = $derived(
    calculateProgressPercent(chapterCount(), currentChapterIndex, currentPage, totalPages)
  );

  let filteredBookmarks = $derived.by(() => {
    if (searchMode !== "bookmarks") return [];
    const q = searchQuery.toLowerCase();
    return bookmarks.filter(b => !q || (b.label ?? "").toLowerCase().includes(q));
  });

  let filteredQuotes = $derived.by(() => {
    if (searchMode !== "quotes") return [];
    const q = searchQuery.toLowerCase();
    return quotes.filter(qt => !q || qt.text.toLowerCase().includes(q));
  });

  /** @type {Unlisten[]} */
  let unlistens = [];
  let isClosing = false;

  onMount(async () => {
    const supported = new Set(['.epub', '.txt', '.md']);
    const ul1 = await listen("tauri://drag-drop", async (event) => {
      isDragOver = false;
      isDragBad = false;
      const paths = event.payload.paths;
      if (paths?.length > 0) await loadBook(paths[0]);
    });
    const ul2 = await listen("tauri://drag-enter", (event) => {
      const paths = event.payload?.paths;
      if (paths?.length > 0) {
        const ext = paths[0].toLowerCase().match(/\.[^.]+$/)?.[0] ?? '';
        isDragBad = !supported.has(ext);
      } else {
        isDragBad = false;
      }
      isDragOver = true;
    });
    const ul3 = await listen("tauri://drag-leave", () => { isDragOver = false; isDragBad = false; });
    const appWindow = getCurrentWindow();
    const closeUnlisten = await appWindow.onCloseRequested(async (event) => {
      event.preventDefault();
      if (isClosing) return;
      isClosing = true;
      await Promise.race([persistSessionState(), delay(750)]);
      await appWindow.destroy();
    });
    unlistens = [ul1, ul2, ul3, closeUnlisten];
    document.addEventListener("keydown", handleKeydown);

    const state = /** @type {AppState} */ (await invoke("get_state"));
    appState = state;
    const prefs = state.preferences ?? {};
    if (prefs.font_size)   fontSize   = prefs.font_size;
    if (prefs.theme)       theme      = prefs.theme;
    if (prefs.font_family) fontFamily = prefs.font_family;
    if (prefs.line_height) lineHeight = prefs.line_height;
    if (prefs.text_align)  textAlign  = prefs.text_align;
    scrollMode = prefs.reader_mode === "scroll";
    savedPreferences = {
      font_size: prefs.font_size ?? null,
      theme: prefs.theme ?? null,
      font_family: prefs.font_family ?? null,
      line_height: prefs.line_height ?? null,
      text_align: prefs.text_align ?? null,
      reader_mode: prefs.reader_mode ?? null,
    };

    if (state.last_opened) {
      await loadBook(state.last_opened, true);
      if (!book) await invoke("clear_last_opened").catch(() => {});
    }
  });

  onDestroy(() => {
    unlistens.forEach(u => u());
    document.removeEventListener("keydown", handleKeydown);
    clearPendingSearch();
    if (saveProgressTimer !== null) clearTimeout(saveProgressTimer);
    if (layoutSettlingTimer !== null) clearTimeout(layoutSettlingTimer);
  });

  // Apply theme attribute to <html>
  $effect(() => {
    document.documentElement.setAttribute("data-theme", theme);
  });

  // Sync typography CSS variables to :root
  $effect(() => {
    const root = document.documentElement;
    root.style.setProperty("--font-size-reading", `${fontSize}px`);
    root.style.setProperty("--line-height-reading", String(lineHeight));
    root.style.setProperty("--font-current", fontFamilyStack(fontFamily));
    root.style.setProperty("--text-align-reading", textAlign);
  });

  /**
   * @param {Book | null} bookValue
   * @param {number} chapterIndex
   * @returns {Chapter | null}
   */
  function getCurrentChapter(bookValue, chapterIndex) {
    return bookValue?.chapters?.[chapterIndex] ?? null;
  }

  /** @param {FontFamily} f */
  function fontFamilyStack(f) {
    if (f === "sans") return "'Inter', system-ui, sans-serif";
    if (f === "mono") return "'JetBrains Mono', 'Fira Code', Consolas, monospace";
    return "Georgia, 'Noto Serif', serif";
  }

  /**
   * @param {Chapter | null} chapter
   * @returns {string | null}
   */
  function getChapterHtml(chapter) {
    if (!chapter) return null;
    const c = chapter.content;
    if (c.Html !== undefined) {
      return c.Html;
    }
    if (c.PlainText !== undefined) {
      return `<pre class="plain-text-chapter">${escapeHtml(c.PlainText)}</pre>`;
    }
    return null;
  }

  /** @param {string} s */
  function escapeHtml(s) {
    return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
  }

  // Track reading area height
  $effect(() => {
    const area = readingArea;
    if (!area) return;
    const update = () => { containerHeight = area.clientHeight; };
    const ro = new ResizeObserver(update);
    ro.observe(area);
    update();
    return () => ro.disconnect();
  });

  // Wheel-to-page in paginated mode (must be non-passive to call preventDefault)
  $effect(() => {
    const area = readingArea;
    if (!area) return;
    area.addEventListener('wheel', handleReaderWheel, { passive: false });
    return () => area.removeEventListener('wheel', handleReaderWheel);
  });

  // Repaginate on content/font/height change
  $effect(() => {
    void chapterHtml;
    void fontSize;
    void lineHeight;
    void containerHeight;
    void scrollMode;
    tick().then(recalcPages);
  });

  // Scroll mode: track scroll position → currentPage
  $effect(() => {
    const area = readingArea;
    if (!scrollMode || !area) return;
    const onScroll = () => {
      if (pageStep === 0) return;
      const maxOffset = Math.max(0, contentHeight - containerHeight);
      const rawPage = maxOffset > 0 && area.scrollTop >= maxOffset - 1
        ? totalPages - 1
        : Math.floor(area.scrollTop / pageStep);
      const page = clampPage(rawPage, totalPages);
      if (page !== currentPage) {
        setReadingPosition(
          { chapterIndex: currentChapterIndex, pageIndex: page },
          { save: false, restore: false }
        );
      }
    };
    area.addEventListener('scroll', onScroll, { passive: true });
    return () => area.removeEventListener('scroll', onScroll);
  });

  // Scroll mode: restore scroll position on chapter load or mode toggle
  $effect(() => {
    void chapterHtml;
    void scrollMode;
    if (!scrollMode) return;
    tick().then(scrollToCurrentPage);
  });

  /**
   * @param {number} [releaseDelay]
   * @returns {number}
   */
  function beginLayoutSettling(releaseDelay = 250) {
    const version = ++layoutSettlingVersion;
    layoutSettling = true;
    if (saveProgressTimer !== null) {
      clearTimeout(saveProgressTimer);
      saveProgressTimer = null;
    }
    if (layoutSettlingTimer !== null) clearTimeout(layoutSettlingTimer);
    layoutSettlingTimer = setTimeout(() => finishLayoutSettling(version), releaseDelay);
    return version;
  }

  /** @param {number} [version] */
  function finishLayoutSettling(version = layoutSettlingVersion) {
    if (version !== layoutSettlingVersion) return;
    if (layoutSettlingTimer !== null) clearTimeout(layoutSettlingTimer);
    layoutSettlingTimer = null;
    layoutSettling = false;
    if (pendingProgressSave) {
      pendingProgressSave = false;
      scheduleSaveProgress(250);
    }
  }

  /** @returns {number} */
  function chapterCount() {
    return book?.chapters.length ?? 0;
  }

  /**
   * @param {number} targetChapterIndex
   * @param {number} requestedPageIndex
   */
  function pageCountForTarget(targetChapterIndex, requestedPageIndex) {
    if (targetChapterIndex === currentChapterIndex) return totalPages;
    return Math.max(1, clampNonNegativeInteger(requestedPageIndex) + 1);
  }

  /**
   * @param {{ chapterIndex: number, pageIndex: number }} position
   * @param {{ save?: boolean, restore?: boolean, pendingLayout?: boolean, preservePendingLastPage?: boolean }} [options]
   */
  function setReadingPosition(position, options = {}) {
    const chapters = chapterCount();
    if (chapters <= 0) return false;
    const chapterIndex = clampIndex(position.chapterIndex, chapters);
    if (!options.preservePendingLastPage && pendingLastPageChapterIndex !== null) {
      pendingLastPageChapterIndex = null;
    }
    const targetPageCount = options.pendingLayout
      ? Math.max(1, clampNonNegativeInteger(position.pageIndex) + 1)
      : pageCountForTarget(chapterIndex, position.pageIndex);
    const pageIndex = clampPage(position.pageIndex, targetPageCount);
    const changed = chapterIndex !== currentChapterIndex || pageIndex !== currentPage;
    currentChapterIndex = chapterIndex;
    currentPage = pageIndex;
    if (!scrollMode || (options.restore ?? true)) tick().then(scrollToCurrentPage);
    if (options.save) requestProgressSave(true);
    return changed;
  }

  /** @param {boolean} [immediate] */
  function requestProgressSave(immediate = false) {
    if (layoutSettling) {
      pendingProgressSave = true;
      return;
    }
    if (immediate) saveProgress();
    else scheduleSaveProgress();
  }

  function recalcPages() {
    if (!readerEl || pageStep === 0) return;
    const settlingVersion = beginLayoutSettling();
    contentHeight = readerEl.scrollHeight;
    const pages = calculatePageCount(contentHeight, pageStep, containerHeight);
    totalPages = pages;
    const shouldLandOnLastPage = pendingLastPageChapterIndex === currentChapterIndex;
    if (shouldLandOnLastPage) pendingLastPageChapterIndex = null;
    const changed = setReadingPosition(
      {
        chapterIndex: currentChapterIndex,
        pageIndex: shouldLandOnLastPage ? pages - 1 : currentPage,
      },
      { save: false, restore: true }
    );
    if (changed || shouldLandOnLastPage) pendingProgressSave = true;
    tick().then(() => {
      setTimeout(() => finishLayoutSettling(settlingVersion), 80);
    });
  }

  /**
   * @param {string} path
   * @param {boolean} [silent]
   */
  async function loadBook(path, silent = false) {
    error = "";
    pendingLastPageChapterIndex = null;
    try {
      const state = /** @type {AppState} */ (await invoke("get_state"));
      const loadedBook = /** @type {Book} */ (await invoke("open_book", { path }));
      appState = state;
      book = loadedBook;
      currentBookPath = path;
      const record = state.books?.[path];
      if (record) {
        const chapterIndex = clampIndex(record.last_chapter, loadedBook.chapters.length);
        const pageIndex = clampNonNegativeInteger(record.last_page);
        pendingScrollTopRestore = scrollMode && Number.isFinite(record.last_scroll_top)
          ? Math.max(0, Number(record.last_scroll_top))
          : null;
        setReadingPosition({ chapterIndex, pageIndex }, { save: false, restore: false, pendingLayout: true });
        lastSavedProgress = progressSnapshot(path, chapterIndex, pageIndex, record.title, record.author);
        bookmarks = record.bookmarks ?? [];
        highlights = record.highlights ?? [];
        quotes = record.quotes ?? [];
      } else {
        pendingScrollTopRestore = null;
        setReadingPosition({ chapterIndex: 0, pageIndex: 0 }, { save: false, restore: false });
        bookmarks = [];
        highlights = [];
        quotes = [];
        await invoke("update_progress", {
          filePath: path, chapterIndex: 0, pageIndex: 0,
          title: loadedBook.metadata.title, author: loadedBook.metadata.author,
        });
        lastSavedProgress = progressSnapshot(path, 0, 0, loadedBook.metadata.title, loadedBook.metadata.author);
      }
    } catch (e) {
      if (!silent) error = String(e);
      book = null;
      currentBookPath = null;
      lastSavedProgress = null;
      pendingScrollTopRestore = null;
    }
  }

  async function openFilePicker() {
    const path = await invoke("pick_file");
    if (path) await loadBook(path);
  }

  /**
   * @param {string} filePath
   * @param {number} chapterIndex
   * @param {number} pageIndex
   * @param {string} title
   * @param {string} author
   * @returns {ProgressSnapshot}
   */
  function progressSnapshot(filePath, chapterIndex, pageIndex, title, author) {
    return { filePath, chapterIndex, pageIndex, title, author };
  }

  /**
   * @param {ProgressSnapshot | null} a
   * @param {ProgressSnapshot | null} b
   */
  function progressMatches(a, b) {
    return a && b
      && a.filePath === b.filePath
      && a.chapterIndex === b.chapterIndex
      && a.pageIndex === b.pageIndex
      && a.title === b.title
      && a.author === b.author;
  }

  function saveProgress() {
    if (!book || !currentBookPath) return;
    if (layoutSettling) {
      pendingProgressSave = true;
      return;
    }
    const nextProgress = progressSnapshot(
      currentBookPath,
      currentChapterIndex,
      currentPage,
      book.metadata.title,
      book.metadata.author
    );
    if (progressMatches(lastSavedProgress, nextProgress)) return;
    lastSavedProgress = nextProgress;
    invoke("update_progress", {
      filePath: currentBookPath,
      chapterIndex: currentChapterIndex,
      pageIndex: currentPage,
      title: book.metadata.title,
      author: book.metadata.author,
    }).catch(() => {});
  }

  /** @param {number} [delay] */
  function scheduleSaveProgress(delay = 500) {
    if (saveProgressTimer !== null) clearTimeout(saveProgressTimer);
    saveProgressTimer = setTimeout(saveProgress, delay);
  }

  function flushScheduledProgress() {
    if (saveProgressTimer !== null) clearTimeout(saveProgressTimer);
    saveProgressTimer = null;
    if (layoutSettling) {
      pendingProgressSave = true;
      return;
    }
    if (scrubbing) scheduleSaveProgress(250);
    else saveProgress();
  }

  async function persistSessionState() {
    try {
      if (saveProgressTimer !== null) clearTimeout(saveProgressTimer);
      saveProgressTimer = null;
      pendingProgressSave = false;
      const readerMode = /** @type {ReaderMode} */ (scrollMode ? "scroll" : "paginated");
      await invoke("save_session_state", {
        readerMode,
        filePath: currentBookPath,
        chapterIndex: book ? currentChapterIndex : null,
        pageIndex: book ? currentPage : null,
        scrollTop: scrollMode && readingArea ? readingArea.scrollTop : null,
        title: book?.metadata.title ?? null,
        author: book?.metadata.author ?? null,
      });
    } catch (_) {}
  }

  /** @param {number} pct */
  function seekToPercent(pct) {
    if (!book || !book.chapters.length) return;
    const position = positionFromPercent(pct, book.chapters.length, currentChapterIndex, totalPages);
    setReadingPosition(position, { save: false });
    requestProgressSave(!scrubbing);
  }

  /** @param {MouseEvent} e */
  function handleProgressDown(e) {
    if (!book || !progressTrackEl) return;
    e.preventDefault();
    scrubbing = true;
    const track = progressTrackEl;
    const rect = track.getBoundingClientRect();
    seekToPercent((e.clientX - rect.left) / rect.width * 100);

    /** @param {MouseEvent} me */
    const onMove = (me) => {
      const r = track.getBoundingClientRect();
      seekToPercent((me.clientX - r.left) / r.width * 100);
    };
    const onUp = () => {
      scrubbing = false;
      window.removeEventListener('mousemove', onMove);
      window.removeEventListener('mouseup', onUp);
      flushScheduledProgress();
    };
    window.addEventListener('mousemove', onMove);
    window.addEventListener('mouseup', onUp);
  }

  /**
   * @param {string} key
   * @param {string | number} value
   */
  function savePref(key, value) {
    const normalized = String(value);
    if (savedPreferences[key] != null && String(savedPreferences[key]) === normalized) return;
    savedPreferences = { ...savedPreferences, [key]: value };
    invoke("save_preference", { key, value: String(value) }).catch(() => {});
  }

  function resetSearchPanel() {
    clearPendingSearch();
    searchQuery = "";
    searchResults = [];
    currentMatchIndex = -1;
    showResults = false;
    searchMode = "book";
  }

  function closePanel() {
    if (activePanel === "search") resetSearchPanel();
    activePanel = null;
  }

  /** @param {Panel} panel */
  async function openPanel(panel) {
    if (activePanel === "search" && panel !== "search") resetSearchPanel();
    activePanel = panel;
    if (panel === "history") {
      historyEntries = /** @type {HistoryEntry[]} */ (await invoke("get_book_history"));
    }
  }

  /** @param {Panel} panel */
  async function togglePanel(panel) {
    if (activePanel === panel) {
      closePanel();
    } else {
      await openPanel(panel);
    }
  }

  async function toggleFullscreen() {
    const appWindow = getCurrentWindow();
    try {
      const nextFullscreen = !(await appWindow.isFullscreen());
      await appWindow.setFullscreen(nextFullscreen);
      isFullscreen = nextFullscreen;
    } catch (err) {
      console.error("Failed to toggle fullscreen", err);
    }
  }

  /** @param {KeyboardEvent} e */
  async function handleKeydown(e) {
    if (e.ctrlKey && e.key === "o") { e.preventDefault(); openFilePicker(); return; }
    if (e.ctrlKey && (e.key === "+" || e.key === "=")) { e.preventDefault(); adjustFontSize(1); return; }
    if (e.ctrlKey && e.key === "-") { e.preventDefault(); adjustFontSize(-1); return; }
    if (e.ctrlKey && e.key === "f") {
      e.preventDefault();
      await togglePanel("search");
      return;
    }
    if (e.ctrlKey && e.key === "b") {
      e.preventDefault();
      await togglePanel("bookmarks");
      return;
    }
    if (e.ctrlKey && e.key === "q") {
      e.preventDefault();
      await togglePanel("quotes");
      return;
    }
    if (e.ctrlKey && e.key === "h") {
      e.preventDefault();
      await togglePanel("history");
      return;
    }
    if (e.key === "Escape") {
      if (highlightCtxMenu) { highlightCtxMenu = null; return; }
      if (activePanel) { closePanel(); return; }
      selectionPopup = null;
      return;
    }
    if (e.key === "F11") {
      e.preventDefault();
      await toggleFullscreen();
      return;
    }
    if (!book) return;
    const target = e.target;
    const inInput = target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement;
    if (!inInput && !e.ctrlKey && !e.altKey && !e.metaKey) {
      if (e.key === ";" || e.key === "?") {
        e.preventDefault();
        await togglePanel("hotkeys");
        return;
      }
      if (e.key === "b") { e.preventDefault(); toggleBookmark(); return; }
      if (e.key === "q") {
        if (!selectionPopup) handleTextSelection();
        if (selectionPopup) { e.preventDefault(); saveAsQuote(); return; }
      }
      if (e.key === "j") { e.preventDefault(); nextPage(); return; }
      if (e.key === "k") { e.preventDefault(); prevPage(); return; }
    }
    if (e.shiftKey && e.key === "Tab" && activePanel === "search") {
      e.preventDefault();
      /** @type {SearchMode[]} */
      const modes = ["book", "bookmarks", "quotes"];
      setSearchMode(modes[(modes.indexOf(searchMode) + 1) % modes.length]);
      return;
    }
    if (activePanel === "search") return; // don't turn pages while searching
    if (scrollMode) {
      if (e.key === "ArrowDown") { e.preventDefault(); readingArea?.scrollBy({ top: 40 }); }
      else if (e.key === "ArrowUp") { e.preventDefault(); readingArea?.scrollBy({ top: -40 }); }
      else if (e.key === "ArrowRight") { e.preventDefault(); goToAdjacentScrollChapter(1); }
      else if (e.key === "ArrowLeft") { e.preventDefault(); goToAdjacentScrollChapter(-1); }
    } else {
      if (e.key === "ArrowRight" || e.key === "ArrowDown") nextPage();
      else if (e.key === "ArrowLeft" || e.key === "ArrowUp") prevPage();
    }
  }

  /** @param {MouseEvent} e */
  function handleReaderClick(e) {
    const target = e.target instanceof Element ? e.target : null;
    if (highlightCtxMenu && !target?.closest('.highlight-ctx-menu')) {
      highlightCtxMenu = null;
    }
    // Dismiss highlight popup if click is outside it
    if (selectionPopup && !target?.closest('.highlight-popup')) {
      selectionPopup = null;
    }
    if (activePanel) return;
    // Don't turn page if user has text selected
    if (window.getSelection()?.toString().trim()) return;
    const currentTarget = /** @type {HTMLElement} */ (e.currentTarget);
    const rect = currentTarget.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const w = rect.width;
    if (x < w / 3) prevPage();
    else if (x > (w * 2) / 3) nextPage();
  }

  /** @param {WheelEvent} e */
  function handleReaderWheel(e) {
    if (scrollMode) return;
    e.preventDefault();
    if (e.deltaY > 0) nextPage();
    else if (e.deltaY < 0) prevPage();
  }

  function scrollToCurrentPage() {
    if (!readingArea) return;
    if (!scrollMode) {
      readingArea.scrollTop = 0;
      return;
    }
    if (pageStep === 0) return;
    const targetScrollTop = pendingScrollTopRestore ?? pageOffset;
    readingArea.scrollTop = targetScrollTop;
    if (pendingScrollTopRestore !== null) {
      requestAnimationFrame(() => {
        if (readingArea) readingArea.scrollTop = targetScrollTop;
        pendingScrollTopRestore = null;
      });
    }
  }

  /** @param {number} direction */
  function goToAdjacentScrollChapter(direction) {
    const targetChapter = currentChapterIndex + direction;
    if (!book || targetChapter < 0 || targetChapter >= book.chapters.length) return;
    setReadingPosition({ chapterIndex: targetChapter, pageIndex: 0 }, { save: true });
  }

  function nextPage() {
    if (!book) return;
    const position = nextReadingPosition(
      { chapterIndex: currentChapterIndex, pageIndex: currentPage },
      book.chapters.length,
      totalPages
    );
    setReadingPosition(position, { save: true });
  }

  function prevPage() {
    if (!book) return;
    if (currentPage === 0 && currentChapterIndex > 0) {
      const previousChapterIndex = currentChapterIndex - 1;
      pendingLastPageChapterIndex = previousChapterIndex;
      setReadingPosition(
        { chapterIndex: previousChapterIndex, pageIndex: 0 },
        { save: false, restore: false, pendingLayout: true, preservePendingLastPage: true }
      );
      return;
    }
    const position = previousReadingPosition(
      { chapterIndex: currentChapterIndex, pageIndex: currentPage },
      book.chapters.length,
      totalPages
    );
    setReadingPosition(position, { save: true });
  }

  /** @param {number} index */
  function goToChapter(index) {
    closePanel();
    setReadingPosition({ chapterIndex: index, pageIndex: 0 }, { save: true });
  }

  async function toggleBookmark() {
    if (!currentBookPath) return;
    try {
      const added = await invoke("toggle_bookmark", {
        filePath: currentBookPath,
        chapterIndex: currentChapterIndex,
        pageIndex: currentPage,
      });
      if (added) {
        bookmarks = [...bookmarks, { chapter_index: currentChapterIndex, page_index: currentPage, label: null }];
      } else {
        bookmarks = bookmarks.filter(
          b => !(b.chapter_index === currentChapterIndex && b.page_index === currentPage)
        );
      }
    } catch (_) {}
  }

  /** @param {Bookmark} bm */
  function goToBookmark(bm) {
    closePanel();
    setReadingPosition(
      { chapterIndex: bm.chapter_index, pageIndex: bm.page_index },
      { save: true, pendingLayout: bm.chapter_index !== currentChapterIndex }
    );
  }

  /** @param {Theme} t */
  function setTheme(t) {
    if (theme === t) return;
    theme = t;
    savePref("theme", t);
  }
  /** @param {FontFamily} f */
  function setFontFamily(f) {
    beginLayoutSettling(400);
    fontFamily = f;
    savePref("font_family", f);
  }
  /** @param {TextAlign} a */
  function setTextAlign(a) {
    beginLayoutSettling(400);
    textAlign = a;
    savePref("text_align", a);
  }

  function toggleScrollMode() {
    beginLayoutSettling(400);
    pendingScrollTopRestore = null;
    scrollMode = !scrollMode;
    savePref("reader_mode", scrollMode ? "scroll" : "paginated");
  }

  /** @param {number} d */
  function adjustFontSize(d) {
    beginLayoutSettling(400);
    fontSize = Math.min(28, Math.max(14, fontSize + d));
    savePref("font_size", fontSize);
  }

  /** @param {number} d */
  function adjustLineHeight(d) {
    beginLayoutSettling(400);
    lineHeight = Math.round(Math.min(2.0, Math.max(1.3, lineHeight + d)) * 10) / 10;
    savePref("line_height", lineHeight);
  }

  // ── Search ──
  function clearPendingSearch() {
    if (searchTimer !== null) clearTimeout(searchTimer);
    searchTimer = null;
    searchRequestId++;
  }

  function doSearch() {
    clearPendingSearch();
    currentMatchIndex = -1;
    if (searchMode !== "book") {
      // bookmarks/quotes mode: filtering is purely reactive, nothing to do
      showResults = false;
      return;
    }
    if (!searchQuery.trim()) { searchResults = []; showResults = false; return; }
    const query = searchQuery.trim();
    const requestId = searchRequestId;
    searchTimer = setTimeout(() => {
      runBookSearch(query, requestId);
    }, 200);
  }

  /**
   * @param {string} query
   * @param {number} requestId
   * @param {boolean} [navigateAfter]
   */
  async function runBookSearch(query, requestId, navigateAfter = false) {
    const results = /** @type {SearchResult[]} */ (await invoke("search_book", { query }).catch(() => []));
    if (requestId !== searchRequestId || searchMode !== "book" || searchQuery.trim() !== query) return;
    searchResults = results;
    showResults = true;
    if (navigateAfter && results.length > 0) navigateToMatch(0);
  }

  /** @param {KeyboardEvent} e */
  function handleSearchKeydown(e) {
    if (e.key !== "Enter" || searchMode !== "book") return;
    e.preventDefault();
    if (searchTimer) {
      clearTimeout(searchTimer);
      searchTimer = null;
      runBookSearch(searchQuery.trim(), searchRequestId, true);
      return;
    }
    navigateToMatch(currentMatchIndex >= 0 ? currentMatchIndex : 0);
  }

  /** @param {SearchMode} mode */
  function setSearchMode(mode) {
    clearPendingSearch();
    searchMode = mode;
    searchResults = [];
    currentMatchIndex = -1;
    showResults = false;
  }

  function closeSearch() {
    closePanel();
  }

  /** @param {Bookmark} bm */
  function goToBookmarkFromSearch(bm) {
    closePanel();
    setReadingPosition(
      { chapterIndex: bm.chapter_index, pageIndex: bm.page_index },
      { save: true, pendingLayout: bm.chapter_index !== currentChapterIndex }
    );
  }

  /** @param {Quote} qt */
  function goToQuoteFromSearch(qt) {
    queueTextCue(qt.text, "quote");
    closePanel();
    setReadingPosition({ chapterIndex: qt.chapter_index, pageIndex: 0 }, { save: true });
  }

  /** @param {number} index */
  function navigateToMatch(index) {
    if (!searchResults.length) return;
    const i = ((index % searchResults.length) + searchResults.length) % searchResults.length;
    currentMatchIndex = i;
    const result = searchResults[i];
    const term = searchQuery.trim();
    queueTextCue(term, "search");
    showResults = false;
    setReadingPosition({ chapterIndex: result.chapter_index, pageIndex: 0 }, { save: true });
  }

  function navigateToPreviousMatch() {
    navigateToMatch(currentMatchIndex >= 0 ? currentMatchIndex - 1 : searchResults.length - 1);
  }

  function navigateToNextMatch() {
    navigateToMatch(currentMatchIndex >= 0 ? currentMatchIndex + 1 : 0);
  }

  // ── Highlights ──

  /**
   * @param {string} text
   * @param {"search" | "quote"} kind
   */
  function queueTextCue(text, kind) {
    pendingTextCue = { text, kind };
    searchTrigger++;
  }

  /**
   * @param {Highlight[]} chapterHighlights
   * @returns {Highlight[]}
   */
  function validatedChapterHighlights(chapterHighlights) {
    if (!readerEl) return [];
    const sorted = [...chapterHighlights].sort((a, b) =>
      a.start_offset - b.start_offset || a.end_offset - b.end_offset
    );
    /** @type {Highlight[]} */
    const valid = [];
    let lastEnd = -1;

    for (const highlight of sorted) {
      const start = highlight.start_offset;
      const end = highlight.end_offset;
      if (!isValidTextRange(readerEl, start, end)) continue;
      if (start < lastEnd) continue;
      valid.push(highlight);
      lastEnd = end;
    }

    return valid;
  }

  // Apply / re-apply all highlights for the current chapter to the DOM.
  function applyHighlightsToDOM() {
    if (!readerEl) return;
    unwrapMarks(readerEl, "mark[data-search], mark[data-quote], mark[data-hl]");
    const chHls = validatedChapterHighlights(highlights.filter(h => h.chapter_index === currentChapterIndex));
    for (const hl of chHls) {
      wrapTextRange(readerEl, hl.start_offset, hl.end_offset, () => {
        const mark = document.createElement("mark");
        mark.setAttribute("data-hl", "1");
        mark.dataset.hlStart = String(hl.start_offset);
        mark.dataset.hlEnd = String(hl.end_offset);
        mark.style.setProperty("background", hl.color, "important");
        mark.style.setProperty("background-color", hl.color, "important");
        mark.style.borderRadius = "2px";
        mark.style.cursor = "pointer";
        if (hl.note) mark.title = hl.note;
        return mark;
      });
    }
  }

  // Re-apply highlights (and handle pending text cues) whenever chapter or highlights change
  $effect(() => {
    void chapterHtml;
    void highlights;
    void currentChapterIndex;
    void searchTrigger;
    tick().then(async () => {
      applyHighlightsToDOM();
      // pendingTextCue is read outside reactive tracking so it won't cause a re-run loop
      const cue = pendingTextCue;
      if (cue) {
        pendingTextCue = null;
        await waitForReaderSettled();
        applyHighlightsToDOM();
        flashTextCue(cue.text, cue.kind);
      }
    });
  });

  function waitForAnimationFrame() {
    return new Promise(resolve => requestAnimationFrame(resolve));
  }

  async function waitForReaderSettled() {
    await tick();
    await waitForAnimationFrame();
    if (layoutSettling) {
      await new Promise(resolve => setTimeout(resolve, 120));
      await tick();
      await waitForAnimationFrame();
    }
  }

  /**
   * @param {"search" | "quote"} kind
   * @returns {HTMLElement}
   */
  function createTemporaryCueMark(kind) {
    const mark = document.createElement("mark");
    mark.setAttribute(kind === "quote" ? "data-quote" : "data-search", "1");
    return mark;
  }

  /**
   * @param {HTMLElement[]} marks
   * @param {number} [visibleDelay]
   */
  function fadeAndRemoveTemporaryMarks(marks, visibleDelay = 900) {
    setTimeout(() => {
      for (const mark of marks) {
        mark.style.transition = "opacity 0.25s";
        mark.style.opacity = "0";
      }
      setTimeout(() => {
        for (const mark of marks) {
          if (mark.parentNode) unwrapMark(mark);
        }
      }, 250);
    }, visibleDelay);
  }

  /** @param {number} ms */
  function delay(ms) {
    return new Promise(resolve => setTimeout(resolve, ms));
  }

  /**
   * @param {Element} element
   * @param {number} [idleDelay]
   * @param {number} [maxDelay]
   */
  function waitForScrollIdle(element, idleDelay = 140, maxDelay = 1200) {
    return new Promise(resolve => {
      let done = false;
      /** @type {number | null} */
      let idleTimer = null;

      const finish = () => {
        if (done) return;
        done = true;
        if (idleTimer !== null) clearTimeout(idleTimer);
        element.removeEventListener("scroll", onScroll);
        element.removeEventListener("scrollend", finish);
        resolve(undefined);
      };

      const onScroll = () => {
        if (idleTimer !== null) clearTimeout(idleTimer);
        idleTimer = setTimeout(finish, idleDelay);
      };

      element.addEventListener("scroll", onScroll, { passive: true });
      element.addEventListener("scrollend", finish, { once: true });
      idleTimer = setTimeout(finish, idleDelay);
      setTimeout(finish, maxDelay);
    });
  }

  /**
   * Find text, navigate to its page, and show a temporary cue.
   * @param {string} query
   * @param {"search" | "quote"} kind
   */
  async function flashTextCue(query, kind) {
    if (!readerEl || !query) return;
    unwrapMarks(readerEl, kind === "quote" ? "mark[data-quote]" : "mark[data-search]");

    const range = findTextRange(readerEl, query);
    if (!range) return;

    const marks = wrapTextRange(readerEl, range.start, range.end, () => createTemporaryCueMark(kind));
    const firstMark = marks[0];
    if (!firstMark) return;

    // Determine which page this mark is on (at currentPage=0, transform is translateY(0))
    if (!scrollMode && readingArea && pageStep > 0) {
      const relTop = firstMark.getBoundingClientRect().top - readingArea.getBoundingClientRect().top;
      const targetPage = clampPage(Math.floor(relTop / pageStep), totalPages);
      setReadingPosition(
        { chapterIndex: currentChapterIndex, pageIndex: targetPage },
        { save: true, restore: false }
      );
    } else if (scrollMode) {
      const scrollFinished = readingArea ? waitForScrollIdle(readingArea) : Promise.resolve();
      firstMark.scrollIntoView({ behavior: 'smooth', block: 'center' });
      await scrollFinished;
    }

    await tick();
    await waitForAnimationFrame();
    await delay(scrollMode ? 120 : 80);
    fadeAndRemoveTemporaryMarks(marks);
  }

  // Register mouseup on reading area for text selection detection
  $effect(() => {
    const area = readingArea;
    if (!area) return;
    area.addEventListener("mouseup", handleTextSelection, false);
    return () => area.removeEventListener("mouseup", handleTextSelection);
  });

  // Right-click on highlighted text → context menu
  $effect(() => {
    const area = readingArea;
    if (!area) return;
    area.addEventListener("contextmenu", handleHighlightContextMenu, false);
    return () => area.removeEventListener("contextmenu", handleHighlightContextMenu);
  });

  /** @param {MouseEvent} e */
  function handleHighlightContextMenu(e) {
    const target = e.target instanceof Element ? e.target : null;
    const mark = target?.closest("mark[data-hl]");
    if (!mark) return;
    e.preventDefault();
    const start = mark.getAttribute("data-hl-start");
    const end = mark.getAttribute("data-hl-end");
    const matchingMarks = readerEl && start && end
      ? Array.from(readerEl.querySelectorAll("mark[data-hl]")).filter(candidate =>
          candidate.getAttribute("data-hl-start") === start &&
          candidate.getAttribute("data-hl-end") === end
        )
      : [mark];
    const text = matchingMarks.map(candidate => candidate.textContent ?? "").join("");
    highlightCtxMenu = { x: e.clientX, y: e.clientY, text };
  }

  /** @param {string} text */
  async function saveHighlightAsQuote(text) {
    if (!currentBookPath || !text) return;
    const id = Date.now().toString();
    highlightCtxMenu = null;
    await invoke("add_quote", {
      filePath: currentBookPath,
      chapterIndex: currentChapterIndex,
      text,
      note: null,
      id,
    }).catch(() => {});
    quotes = [...quotes, { id, chapter_index: currentChapterIndex, text, note: null }];
  }

  function handleTextSelection() {
    const sel = window.getSelection();
    if (!sel || sel.isCollapsed || !sel.toString().trim() || !readerEl) {
      selectionPopup = null;
      return;
    }
    const range = sel.getRangeAt(0);
    if (!readerEl.contains(range.commonAncestorContainer)) {
      selectionPopup = null;
      return;
    }
    const start = getTextOffset(readerEl, range.startContainer, range.startOffset);
    const end   = getTextOffset(readerEl, range.endContainer,   range.endOffset);
    const rect  = range.getBoundingClientRect();
    selectionPopup = { x: rect.left + rect.width / 2, y: rect.top - 8, start, end, text: sel.toString() };
  }

  /** @param {string} color */
  async function addHighlight(color) {
    if (!selectionPopup || !currentBookPath) return;
    const { start, end } = selectionPopup;
    selectionPopup = null;
    window.getSelection()?.removeAllRanges();

    await invoke("add_highlight", {
      filePath: currentBookPath,
      chapterIndex: currentChapterIndex,
      startOffset: start,
      endOffset: end,
      color,
      note: null,
    }).catch(() => {});

    highlights = [...highlights, {
      chapter_index: currentChapterIndex,
      start_offset: start,
      end_offset: end,
      color,
      note: null,
    }];
  }

  /** @param {Highlight} hl */
  function unwrapHighlightMarks(hl) {
    if (!readerEl) return;
    for (const mark of Array.from(readerEl.querySelectorAll("mark[data-hl]"))) {
      if (
        mark instanceof HTMLElement &&
        mark.dataset.hlStart === String(hl.start_offset) &&
        mark.dataset.hlEnd === String(hl.end_offset)
      ) {
        unwrapMark(mark);
      }
    }
  }

  /** @param {Highlight} hl */
  async function removeHighlightAt(hl) {
    await invoke("remove_highlight", {
      filePath: currentBookPath,
      chapterIndex: hl.chapter_index,
      startOffset: hl.start_offset,
      endOffset: hl.end_offset,
    }).catch(() => {});
    unwrapHighlightMarks(hl);
    highlights = highlights.filter(h =>
      !(h.chapter_index === hl.chapter_index &&
        h.start_offset === hl.start_offset &&
        h.end_offset === hl.end_offset)
    );
  }

  // ── Quotes ──

  async function saveAsQuote() {
    if (!selectionPopup || !currentBookPath) return;
    const { text } = selectionPopup;
    const id = Date.now().toString();
    selectionPopup = null;
    window.getSelection()?.removeAllRanges();
    await invoke("add_quote", {
      filePath: currentBookPath,
      chapterIndex: currentChapterIndex,
      text,
      note: null,
      id,
    }).catch(() => {});
    quotes = [...quotes, { id, chapter_index: currentChapterIndex, text, note: null }];
  }

  /** @param {Quote} q */
  function goToQuote(q) {
    queueTextCue(q.text, "quote");
    closePanel();
    setReadingPosition({ chapterIndex: q.chapter_index, pageIndex: 0 }, { save: true });
  }

  /** @param {string} id */
  async function deleteQuote(id) {
    if (!currentBookPath) return;
    await invoke("remove_quote", { filePath: currentBookPath, quoteId: id }).catch(() => {});
    quotes = quotes.filter(q => q.id !== id);
  }

  /** @param {HistoryEntry} entry */
  async function deleteHistoryEntry(entry) {
    await invoke("delete_book_history", { filePath: entry.file_path }).catch(() => {});
    historyEntries = historyEntries.filter(e => e.file_path !== entry.file_path);
  }

  // Focus search input when panel opens
  $effect(() => {
    if (activePanel === "search" && searchInputEl) {
      tick().then(() => searchInputEl?.focus());
    }
  });

  // ── Fullscreen ──
  function showControlsTemporarily() {
    if (!isFullscreen) return;
    controlsVisible = true;
    if (controlsHideTimer !== null) clearTimeout(controlsHideTimer);
    controlsHideTimer = setTimeout(() => { controlsVisible = false; }, 2000);
  }

  // Manage fullscreen: body class, mouse tracking, hide timer
  $effect(() => {
    document.body.classList.toggle('fullscreen', isFullscreen);
    if (!isFullscreen) {
      if (controlsHideTimer !== null) clearTimeout(controlsHideTimer);
      controlsVisible = true;
      return;
    }
    showControlsTemporarily();
    document.addEventListener('mousemove', showControlsTemporarily, { passive: true });
    return () => {
      document.removeEventListener('mousemove', showControlsTemporarily);
      if (controlsHideTimer !== null) clearTimeout(controlsHideTimer);
    };
  });

</script>

<DragOverlay {isDragOver} {isDragBad} />

{#if activePanel === "toc"}
  <TocPanel {book} {currentChapterIndex} onClose={closePanel} onSelectChapter={goToChapter} />
{/if}

{#if activePanel === "bookmarks"}
  <BookmarksPanel
    {book}
    {bookmarks}
    {currentChapterIndex}
    {currentPage}
    onClose={closePanel}
    onSelectBookmark={goToBookmark}
  />
{/if}

{#if activePanel === "settings"}
  <SettingsPanel
    {theme}
    {fontFamily}
    {fontSize}
    {lineHeight}
    {textAlign}
    {scrollMode}
    {currentChapterIndex}
    {highlights}
    onClose={closePanel}
    onSetTheme={setTheme}
    onSetFontFamily={setFontFamily}
    onAdjustFontSize={adjustFontSize}
    onAdjustLineHeight={adjustLineHeight}
    onSetTextAlign={setTextAlign}
    onToggleScrollMode={toggleScrollMode}
    onRemoveHighlight={removeHighlightAt}
  />
{/if}

{#if activePanel === "search"}
  <SearchPanel
    {book}
    bind:searchInputEl
    bind:searchQuery
    {searchMode}
    {searchResults}
    {currentMatchIndex}
    {showResults}
    {filteredBookmarks}
    {filteredQuotes}
    onInput={doSearch}
    onKeydown={handleSearchKeydown}
    onClose={closeSearch}
    onSetMode={setSearchMode}
    onPreviousMatch={navigateToPreviousMatch}
    onNextMatch={navigateToNextMatch}
    onNavigateToMatch={navigateToMatch}
    onSelectBookmark={goToBookmarkFromSearch}
    onSelectQuote={goToQuoteFromSearch}
  />
{/if}

<HighlightSelectionPopup
  {selectionPopup}
  onAddHighlight={addHighlight}
  onSaveQuote={saveAsQuote}
  onDismiss={() => { selectionPopup = null; }}
/>

<HighlightContextMenu
  menu={highlightCtxMenu}
  onSaveQuote={saveHighlightAsQuote}
  onDismiss={() => { highlightCtxMenu = null; }}
/>

{#if activePanel === "hotkeys"}
  <HotkeysModal onClose={closePanel} />
{/if}

{#if activePanel === "history"}
  <HistoryModal {historyEntries} onClose={closePanel} onDeleteEntry={deleteHistoryEntry} />
{/if}

{#if activePanel === "quotes"}
  <QuotesPanel {book} {quotes} onClose={closePanel} onSelectQuote={goToQuote} onDeleteQuote={deleteQuote} />
{/if}

{#if book}
  <ReaderView
    {book}
    {activePanel}
    {currentChapter}
    {currentChapterIndex}
    {chapterHtml}
    {scrollMode}
    {pageOffset}
    {isFullscreen}
    {controlsVisible}
    {isCurrentPageBookmarked}
    {overallProgress}
    {scrubbing}
    bind:readerEl
    bind:readingArea
    bind:progressTrackEl
    onTogglePanel={togglePanel}
    onToggleBookmark={toggleBookmark}
    onReaderClick={handleReaderClick}
    onProgressDown={handleProgressDown}
  />
{:else}
  <WelcomeView {isDragOver} {error} />
{/if}
