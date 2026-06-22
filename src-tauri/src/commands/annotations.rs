use std::sync::Mutex;

#[tauri::command]
pub fn toggle_bookmark(
    state: tauri::State<Mutex<crate::state::AppState>>,
    data_dir: tauri::State<std::path::PathBuf>,
    file_path: String,
    chapter_index: usize,
    page_index: usize,
) -> Result<bool, String> {
    let mut s = crate::state::recover_lock(state.inner());
    let record = s
        .books
        .entry(file_path)
        .or_insert_with(|| crate::state::default_book_record(String::new(), String::new()));
    let existing = record
        .bookmarks
        .iter()
        .position(|b| b.chapter_index == chapter_index && b.page_index == page_index);
    let added = if let Some(idx) = existing {
        record.bookmarks.remove(idx);
        false
    } else {
        record.bookmarks.push(crate::state::Bookmark {
            chapter_index,
            page_index,
            label: None,
        });
        true
    };
    let data_dir = data_dir.inner().clone();
    crate::state::save_state(&data_dir, &s)?;
    Ok(added)
}

#[tauri::command]
pub fn get_bookmarks(
    state: tauri::State<Mutex<crate::state::AppState>>,
    file_path: String,
) -> Vec<crate::state::Bookmark> {
    let s = crate::state::recover_lock(state.inner());
    s.books
        .get(&file_path)
        .map(|r| r.bookmarks.clone())
        .unwrap_or_default()
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn add_highlight(
    state: tauri::State<Mutex<crate::state::AppState>>,
    data_dir: tauri::State<std::path::PathBuf>,
    file_path: String,
    chapter_index: usize,
    start_offset: usize,
    end_offset: usize,
    color: String,
    note: Option<String>,
) -> Result<(), String> {
    let mut s = crate::state::recover_lock(state.inner());
    let Some(record) = s.books.get_mut(&file_path) else {
        return Err("Book not found in state".into());
    };
    record.highlights.push(crate::state::Highlight {
        chapter_index,
        start_offset,
        end_offset,
        color,
        note,
    });
    let data_dir = data_dir.inner().clone();
    crate::state::save_state(&data_dir, &s)
}

#[tauri::command]
pub fn remove_highlight(
    state: tauri::State<Mutex<crate::state::AppState>>,
    data_dir: tauri::State<std::path::PathBuf>,
    file_path: String,
    chapter_index: usize,
    start_offset: usize,
    end_offset: usize,
) -> Result<(), String> {
    let mut s = crate::state::recover_lock(state.inner());
    let Some(record) = s.books.get_mut(&file_path) else {
        return Err("Book not found in state".into());
    };
    let original_len = record.highlights.len();
    record.highlights.retain(|h| {
        !(h.chapter_index == chapter_index
            && h.start_offset == start_offset
            && h.end_offset == end_offset)
    });
    if record.highlights.len() == original_len {
        return Ok(());
    }
    let data_dir = data_dir.inner().clone();
    crate::state::save_state(&data_dir, &s)
}

#[tauri::command]
pub fn get_highlights(
    state: tauri::State<Mutex<crate::state::AppState>>,
    file_path: String,
) -> Vec<crate::state::Highlight> {
    let s = crate::state::recover_lock(state.inner());
    s.books
        .get(&file_path)
        .map(|r| r.highlights.clone())
        .unwrap_or_default()
}

#[tauri::command]
pub fn add_quote(
    state: tauri::State<Mutex<crate::state::AppState>>,
    data_dir: tauri::State<std::path::PathBuf>,
    file_path: String,
    chapter_index: usize,
    text: String,
    note: Option<String>,
    id: String,
) -> Result<(), String> {
    let mut s = crate::state::recover_lock(state.inner());
    let Some(record) = s.books.get_mut(&file_path) else {
        return Err("Book not found in state".into());
    };
    record.quotes.push(crate::state::Quote {
        id,
        chapter_index,
        text,
        note,
    });
    let data_dir = data_dir.inner().clone();
    crate::state::save_state(&data_dir, &s)
}

#[tauri::command]
pub fn remove_quote(
    state: tauri::State<Mutex<crate::state::AppState>>,
    data_dir: tauri::State<std::path::PathBuf>,
    file_path: String,
    quote_id: String,
) -> Result<(), String> {
    let mut s = crate::state::recover_lock(state.inner());
    let Some(record) = s.books.get_mut(&file_path) else {
        return Err("Book not found in state".into());
    };
    let original_len = record.quotes.len();
    record.quotes.retain(|q| q.id != quote_id);
    if record.quotes.len() == original_len {
        return Ok(());
    }
    let data_dir = data_dir.inner().clone();
    crate::state::save_state(&data_dir, &s)
}

#[tauri::command]
pub fn get_quotes(
    state: tauri::State<Mutex<crate::state::AppState>>,
    file_path: String,
) -> Vec<crate::state::Quote> {
    let s = crate::state::recover_lock(state.inner());
    s.books
        .get(&file_path)
        .map(|r| r.quotes.clone())
        .unwrap_or_default()
}
