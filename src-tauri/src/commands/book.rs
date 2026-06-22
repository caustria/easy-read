use std::sync::Mutex;

use tauri_plugin_dialog::DialogExt;

#[tauri::command]
pub async fn pick_file(app: tauri::AppHandle) -> Result<Option<String>, String> {
    let file = app
        .dialog()
        .file()
        .add_filter("Books", &["epub", "txt", "md"])
        .blocking_pick_file();

    Ok(file.map(|p| p.to_string()))
}

#[tauri::command]
pub async fn open_book(
    book_index: tauri::State<'_, Mutex<crate::state::BookIndex>>,
    path: String,
) -> Result<crate::models::book::Book, String> {
    use crate::adapters::epub::EpubAdapter;
    use crate::models::book::{BookAdapter, ChapterContent};
    use crate::state::ChapterText;

    {
        let mut idx = crate::state::recover_lock(book_index.inner());
        idx.file_path.clear();
        idx.chapters.clear();
    }

    let parse_path = path.clone();
    let book = tauri::async_runtime::spawn_blocking(move || {
        let path_obj = std::path::PathBuf::from(parse_path);
        let ext = path_obj
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_ascii_lowercase());
        match ext.as_deref() {
            Some("epub") => EpubAdapter::parse(&path_obj).map_err(|e| e.to_string()),
            Some("txt") | Some("md") => {
                crate::adapters::txt::TxtAdapter::parse(&path_obj).map_err(|e| e.to_string())
            }
            Some("pdf") => {
                Err("PDF files are not supported. Supported formats: .epub, .txt, .md".into())
            }
            _ => Err("Unsupported format. Supported: .epub, .txt, .md".into()),
        }
    })
    .await
    .map_err(|err| format!("failed to parse book: {err}"))??;

    let mut idx = crate::state::recover_lock(book_index.inner());
    idx.file_path = path.clone();
    idx.chapters = book
        .chapters
        .iter()
        .map(|ch| {
            let plain_text = match &ch.content {
                ChapterContent::Html(html) => crate::state::strip_html(html),
                ChapterContent::PlainText(text) => text.clone(),
            };
            ChapterText {
                index: ch.index,
                title: ch.title.clone(),
                plain_text,
            }
        })
        .collect();

    Ok(book)
}
