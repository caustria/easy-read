use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::{SystemTime, UNIX_EPOCH};

// In-memory search index for the currently loaded book (not persisted)
#[derive(Default)]
pub struct BookIndex {
    pub file_path: String,
    pub chapters: Vec<ChapterText>,
}

#[derive(Clone)]
pub struct ChapterText {
    pub index: usize,
    pub title: Option<String>,
    pub plain_text: String,
}

pub fn recover_lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Strip HTML tags from a string, returning plain text.
pub fn strip_html(html: &str) -> String {
    let mut result = String::with_capacity(html.len());
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' => {
                in_tag = false;
                result.push(' ');
            }
            _ if !in_tag => result.push(c),
            _ => {}
        }
    }
    result
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct AppState {
    pub last_opened: Option<String>,
    pub books: HashMap<String, BookRecord>,
    pub preferences: UserPreferences,
}

pub struct LoadedState {
    pub state: AppState,
    pub state_recovered: bool,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct BookRecord {
    pub title: String,
    pub author: String,
    pub last_chapter: usize,
    pub last_page: usize,
    #[serde(default)]
    pub last_scroll_top: f64,
    pub bookmarks: Vec<Bookmark>,
    pub highlights: Vec<Highlight>,
    #[serde(default)]
    pub quotes: Vec<Quote>,
}

pub fn default_book_record(title: String, author: String) -> BookRecord {
    BookRecord {
        title,
        author,
        last_chapter: 0,
        last_page: 0,
        last_scroll_top: 0.0,
        bookmarks: vec![],
        highlights: vec![],
        quotes: vec![],
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Bookmark {
    pub chapter_index: usize,
    pub page_index: usize,
    pub label: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Highlight {
    pub chapter_index: usize,
    pub start_offset: usize,
    pub end_offset: usize,
    pub color: String,
    pub note: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Quote {
    pub id: String,
    pub chapter_index: usize,
    pub text: String,
    pub note: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct BookHistoryEntry {
    pub file_path: String,
    pub title: String,
    pub author: String,
    pub last_chapter: usize,
    pub last_page: usize,
    pub has_bookmarks: bool,
    pub has_quotes: bool,
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct UserPreferences {
    pub font_size: Option<f32>,
    pub theme: Option<String>,
    pub font_family: Option<String>,
    pub line_height: Option<f32>,
    pub text_align: Option<String>,
    pub reader_mode: Option<String>,
}

fn unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn corrupt_state_path(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("state.json");
    let timestamp = unix_timestamp();
    let first = path.with_file_name(format!("{file_name}.corrupt-{timestamp}"));
    if !first.exists() {
        return first;
    }

    for suffix in 1..1000 {
        let candidate = path.with_file_name(format!("{file_name}.corrupt-{timestamp}-{suffix}"));
        if !candidate.exists() {
            return candidate;
        }
    }

    path.with_file_name(format!(
        "{file_name}.corrupt-{timestamp}-{}",
        std::process::id()
    ))
}

pub fn load_state(app_data_dir: &Path) -> LoadedState {
    let path = app_data_dir.join("state.json");
    match std::fs::read_to_string(&path) {
        Ok(content) => match serde_json::from_str(&content) {
            Ok(state) => LoadedState {
                state,
                state_recovered: false,
            },
            Err(err) => {
                let corrupt_path = corrupt_state_path(&path);
                match std::fs::rename(&path, &corrupt_path) {
                    Ok(()) => eprintln!(
                        "Recovered from corrupt state file {}; moved it to {}: {err}",
                        path.display(),
                        corrupt_path.display()
                    ),
                    Err(rename_err) => eprintln!(
                        "State file {} is corrupt and could not be moved to {}: {err}; rename failed: {rename_err}",
                        path.display(),
                        corrupt_path.display()
                    ),
                }
                LoadedState {
                    state: AppState::default(),
                    state_recovered: true,
                }
            }
        },
        Err(err) => {
            if err.kind() != std::io::ErrorKind::NotFound {
                eprintln!("Failed to read state file {}: {err}", path.display());
            }
            LoadedState {
                state: AppState::default(),
                state_recovered: false,
            }
        }
    }
}

#[cfg(windows)]
fn replace_file_windows(temp_path: &Path, path: &Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
    };

    let temp_wide: Vec<u16> = temp_path.as_os_str().encode_wide().chain(Some(0)).collect();
    let path_wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    let flags = MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH;
    let result = unsafe { MoveFileExW(temp_wide.as_ptr(), path_wide.as_ptr(), flags) };
    if result == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(windows)]
fn replace_state_file(temp_path: &Path, path: &Path) -> std::io::Result<()> {
    match replace_file_windows(temp_path, path) {
        Ok(()) => Ok(()),
        Err(_) => {
            std::thread::sleep(std::time::Duration::from_millis(75));
            replace_file_windows(temp_path, path)
        }
    }
}

#[cfg(not(windows))]
fn replace_state_file(temp_path: &Path, path: &Path) -> std::io::Result<()> {
    std::fs::rename(temp_path, path)
}

pub fn save_state(app_data_dir: &Path, state: &AppState) -> Result<(), String> {
    let json = serde_json::to_string_pretty(state)
        .map_err(|err| format!("failed to serialize state: {err}"))?;
    std::fs::create_dir_all(app_data_dir).map_err(|err| {
        format!(
            "failed to create data directory {}: {err}",
            app_data_dir.display()
        )
    })?;

    let path = app_data_dir.join("state.json");
    if let Ok(existing_json) = std::fs::read_to_string(&path) {
        if existing_json == json {
            return Ok(());
        }
    }

    let temp_path = app_data_dir.join(format!("state.json.tmp-{}", std::process::id()));
    {
        let mut file = std::fs::File::create(&temp_path).map_err(|err| {
            format!(
                "failed to create temp state file {}: {err}",
                temp_path.display()
            )
        })?;
        file.write_all(json.as_bytes()).map_err(|err| {
            format!(
                "failed to write temp state file {}: {err}",
                temp_path.display()
            )
        })?;
        file.sync_all().map_err(|err| {
            format!(
                "failed to sync temp state file {}: {err}",
                temp_path.display()
            )
        })?;
    }

    replace_state_file(&temp_path, &path).map_err(|err| {
        format!(
            "failed to replace state file {} with {}: {err}",
            path.display(),
            temp_path.display()
        )
    })
}
