use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// In-memory search index for the currently loaded book (not persisted)
#[derive(Default)]
pub struct BookIndex {
    pub file_path: String,
    pub chapters: Vec<ChapterText>,
}

pub struct ChapterText {
    pub index: usize,
    pub title: Option<String>,
    pub plain_text: String,
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

pub fn load_state(app_data_dir: &std::path::Path) -> AppState {
    let path = app_data_dir.join("state.json");
    match std::fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => AppState::default(),
    }
}

pub fn save_state(app_data_dir: &std::path::Path, state: &AppState) {
    if let Ok(json) = serde_json::to_string_pretty(state) {
        let _ = std::fs::create_dir_all(app_data_dir);
        let path = app_data_dir.join("state.json");
        if let Ok(existing_json) = std::fs::read_to_string(&path) {
            if existing_json == json {
                return;
            }
        }
        let temp_path = app_data_dir.join("state.json.tmp");
        if std::fs::write(&temp_path, json).is_ok() {
            let _ = std::fs::rename(temp_path, path);
        }
    }
}
