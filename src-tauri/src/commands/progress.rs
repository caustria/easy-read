use std::sync::Mutex;

#[tauri::command]
pub fn update_progress(
    state: tauri::State<Mutex<crate::state::AppState>>,
    data_dir: tauri::State<std::path::PathBuf>,
    file_path: String,
    chapter_index: usize,
    page_index: usize,
    title: String,
    author: String,
) -> Result<(), String> {
    let mut s = crate::state::recover_lock(state.inner());
    let last_opened_changed = s.last_opened.as_deref() != Some(file_path.as_str());
    let inserted_record = !s.books.contains_key(&file_path);
    let record = s
        .books
        .entry(file_path.clone())
        .or_insert_with(|| crate::state::default_book_record(title.clone(), author.clone()));
    let progress_changed = record.last_chapter != chapter_index
        || record.last_page != page_index
        || record.title != title
        || record.author != author;
    if !inserted_record && !progress_changed && !last_opened_changed {
        return Ok(());
    }
    record.last_chapter = chapter_index;
    record.last_page = page_index;
    record.title = title;
    record.author = author;
    s.last_opened = Some(file_path);
    let data_dir = data_dir.inner().clone();
    crate::state::save_state(&data_dir, &s)
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn save_session_state(
    state: tauri::State<Mutex<crate::state::AppState>>,
    data_dir: tauri::State<std::path::PathBuf>,
    reader_mode: String,
    file_path: Option<String>,
    chapter_index: Option<usize>,
    page_index: Option<usize>,
    scroll_top: Option<f64>,
    title: Option<String>,
    author: Option<String>,
) -> Result<(), String> {
    let mut s = crate::state::recover_lock(state.inner());
    let next_mode = match reader_mode.as_str() {
        "scroll" => String::from("scroll"),
        _ => String::from("paginated"),
    };
    let mut changed = false;
    if s.preferences.reader_mode.as_deref() != Some(next_mode.as_str()) {
        s.preferences.reader_mode = Some(next_mode);
        changed = true;
    }

    if let (Some(file_path), Some(chapter_index), Some(page_index), Some(title), Some(author)) =
        (file_path, chapter_index, page_index, title, author)
    {
        let last_opened_changed = s.last_opened.as_deref() != Some(file_path.as_str());
        let inserted_record = !s.books.contains_key(&file_path);
        let record = s
            .books
            .entry(file_path.clone())
            .or_insert_with(|| crate::state::default_book_record(title.clone(), author.clone()));
        let next_scroll_top = scroll_top
            .filter(|value| value.is_finite())
            .map(|value| value.max(0.0));
        let scroll_changed = next_scroll_top
            .map(|value| (record.last_scroll_top - value).abs() > f64::EPSILON)
            .unwrap_or(false);
        let progress_changed = record.last_chapter != chapter_index
            || record.last_page != page_index
            || scroll_changed
            || record.title != title
            || record.author != author;

        if inserted_record || progress_changed || last_opened_changed {
            record.last_chapter = chapter_index;
            record.last_page = page_index;
            if let Some(value) = next_scroll_top {
                record.last_scroll_top = value;
            }
            record.title = title;
            record.author = author;
            s.last_opened = Some(file_path);
            changed = true;
        }
    }

    if !changed {
        return Ok(());
    }
    let data_dir = data_dir.inner().clone();
    crate::state::save_state(&data_dir, &s)
}
