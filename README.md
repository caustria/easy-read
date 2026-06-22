# Easy Read

A lightweight portable desktop e-reader built with **Rust**, **Tauri v2**, and **Svelte**. Carry it on a USB drive alongside your book collection and read anywhere, no installation required. This is a single-book reader, not a library manager. Open a book, read it, close the app.

Your progress, bookmarks, highlights, quotes, history, and preferences are saved in a single readable `state.json` file.

---

## Keyboard Shortcuts

| Shortcut | Action |
| --- | --- |
| `Ctrl+O` / `Cmd+O` | Open file picker |
| `F11` | Toggle fullscreen |
| `Ctrl+F` / `Cmd+F` | Toggle search panel |
| `Ctrl+B` / `Cmd+B` | Toggle bookmarks panel |
| `Ctrl+Q` / `Cmd+Shift+Q` | Toggle quotes panel |
| `Ctrl+H` / `Cmd+Y` | Toggle history panel |
| `Ctrl++` / `Ctrl+=` / `Cmd++` / `Cmd+=` | Increase font size |
| `Ctrl+-` / `Cmd+-` | Decrease font size |
| `b` | Place or remove bookmark |
| `q` | Save selected text as a quote |
| `j` / `Right` / `Down` | Next page |
| `k` / `Left` / `Up` | Previous page |
| `Shift+Tab` | Cycle search modes while search is open |
| `;` / `?` | Toggle keyboard shortcut reference |
| `Escape` | Close the active overlay or popup |

On macOS, Easy Read uses `Cmd` for standard shortcuts. Quotes and history use `Cmd+Shift+Q` and `Cmd+Y` because `Cmd+Q` and `Cmd+H` are reserved by the system.

In scroll mode, `Up` and `Down` scroll the chapter, while `Left` and `Right` move between chapters.

---

## Features

### Reading

- Open books via drag-and-drop or the `Ctrl+O` / `Cmd+O` file picker
- Automatically resumes the last book at the last saved position on launch
- CSS column-based pagination with a scroll mode option
- Page navigation with arrow keys, `j`/`k`, mouse wheel, or left/right click zones
- Table of contents overlay for chapter jumping
- Auto-advance between chapters at chapter boundaries
- Launching Easy Read while it is already running focuses the existing window instead of opening a second instance

### Current Supported Formats

| Format | Status | Notes |
| --- | --- | --- |
| `.epub` | Supported | Reflowable text with embedded images; DRM-protected or oversized EPUBs are rejected with clear errors |
| `.txt` | Supported | Plain text, split into chapters on blank-section separators; UTF-8, UTF-16, detected legacy encodings, BOMs, and CRLF files are handled |
| `.md` | Supported | Rendered to formatted HTML via `pulldown-cmark`; UTF-8, UTF-16, detected legacy encodings, BOMs, and CRLF files are handled |
| `.pdf` | Not supported | Opening a PDF returns a clear unsupported-format error |

### Annotations & Notes

- **Bookmarks**: place and remove bookmarks on any page; toggle the panel with `Ctrl+B` / `Cmd+B` or toggle the current bookmark with `b`
- **Highlights**: select text to highlight in color; add optional notes
- **Quotes**: save selected text as a quote with `q`; view saved quotes with `Ctrl+Q` / `Cmd+Shift+Q`
- All annotations persist across sessions in `state.json`

### Search

- `Ctrl+F` / `Cmd+F` opens the search panel with three modes:
  - **Book**: full-text search across all chapters
  - **Bookmarks**: live-filter your bookmarks by label
  - **Quotes**: live-filter your saved quotes by content
- `Shift+Tab` cycles search modes while the search panel is open
- Search results show chapter context; clicking navigates directly there

### History

- `Ctrl+H` / `Cmd+Y` opens the History panel
- Lists every book with saved reading state
- Alphabetically sorted with icons indicating bookmark/quote status

### Theming & Typography

- Light, dark, and sepia themes
- Font size slider with live repagination; adjust with `Ctrl++`, `Ctrl+=`, `Ctrl+-`, or the matching `Cmd` shortcuts on macOS
- Font family selector (serif, sans-serif, monospace)
- Line height and text alignment controls
- Bundled reading, UI, and monospace fonts keep pagination consistent across platforms
- All preferences persist in `state.json`

---

## Persistence

Everything is stored in a single easy-to-read `state.json` file. Easy Read first tries to use an app-adjacent `data` folder so portable builds can keep the app and state together. If that folder is not writable, it falls back to the operating system's app data directory and logs the chosen location at startup.

Portable and production builds store it here when the folder is writable:

```text
<folder containing the app executable>/data/state.json
```

Linux AppImage builds use the folder containing the `.AppImage` file:

```text
<folder containing Easy Read.AppImage>/data/state.json
```

Development builds use the folder containing the dev executable, usually:

```text
src-tauri/target/debug/data/state.json
```

Fallback storage uses the platform app data directory for Easy Read:

```text
<OS app data directory>/state.json
```

If `state.json` is malformed on launch, Easy Read starts with a fresh state file, keeps the bad file beside it as `state.json.corrupt-<timestamp>`, and shows a recovery warning.

Example state shape:

```json
{
  "last_opened": "/books/dune.epub",
  "books": {
    "/books/dune.epub": {
      "title": "Dune",
      "author": "Frank Herbert",
      "last_chapter": 3,
      "last_page": 12,
      "last_scroll_top": 0.0,
      "bookmarks": [
        { "chapter_index": 1, "page_index": 5, "label": "Litany against fear" }
      ],
      "highlights": [],
      "quotes": []
    }
  },
  "preferences": {
    "font_size": 18.0,
    "theme": "dark",
    "font_family": "serif",
    "line_height": 1.6,
    "text_align": "left",
    "reader_mode": "paginated"
  }
}
```

No database. No cloud sync. No telemetry. Just a file you can read in a text editor and copy to any device.

---

## Downloads

Prebuilt installers and portable executables are published on the [GitHub Releases page](https://github.com/klokkish/ereader/releases).

## Building from Source

**Prerequisites:** Rust, Node.js v20+, and npm. On Windows, install the Rust MSVC toolchain and Visual Studio Build Tools with the C++ desktop workload. The Tauri CLI is installed as a project dev dependency and is run through npm/npx.

```bash
npm install

# Development
npm run tauri dev

# Production build - outputs to releases/<platform>/
npm run release
```

`npm run release` wipes any prior Tauri bundle, sets `NO_STRIP=1`, runs `npx tauri build`, and stages installers plus portable executables into a top-level `releases/<platform>/` folder:

```text
releases/
|-- windows/
|   |-- installers/
|   `-- portable/
|-- linux/
|   |-- installers/
|   `-- portable/
`-- macos/
    `-- installers/
```

On Windows, the raw built executable is created at:

```text
src-tauri/target/release/easy-read.exe
```

The staged portable copy is named with the app name and package version:

```text
releases/windows/portable/easy-read_<version>_x64.exe
```
