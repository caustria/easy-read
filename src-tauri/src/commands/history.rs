use std::sync::Mutex;

#[tauri::command]
pub fn get_book_history(
    state: tauri::State<Mutex<crate::state::AppState>>,
) -> Vec<crate::state::BookHistoryEntry> {
    let s = crate::state::recover_lock(state.inner());
    let mut entries: Vec<crate::state::BookHistoryEntry> = s
        .books
        .iter()
        .map(|(path, record)| crate::state::BookHistoryEntry {
            file_path: path.clone(),
            title: record.title.clone(),
            author: record.author.clone(),
            last_chapter: record.last_chapter,
            last_page: record.last_page,
            has_bookmarks: !record.bookmarks.is_empty(),
            has_quotes: !record.quotes.is_empty(),
        })
        .collect();
    entries.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
    entries
}

#[tauri::command]
pub fn delete_book_history(
    state: tauri::State<Mutex<crate::state::AppState>>,
    data_dir: tauri::State<std::path::PathBuf>,
    file_path: String,
) -> Result<(), String> {
    let mut s = crate::state::recover_lock(state.inner());
    let removed = s.books.remove(&file_path).is_some();
    let mut changed = removed;
    if s.last_opened.as_deref() == Some(&file_path) {
        s.last_opened = None;
        changed = true;
    }
    if !changed {
        return Ok(());
    }
    let data_dir = data_dir.inner().clone();
    crate::state::save_state(&data_dir, &s)
}
