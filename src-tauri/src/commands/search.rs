use serde::Serialize;
use std::sync::Mutex;

#[derive(Serialize)]
pub struct SearchResult {
    pub chapter_index: usize,
    pub chapter_title: Option<String>,
    pub snippet: String,
    pub offset: usize,
}

#[tauri::command]
pub fn search_book(
    book_index: tauri::State<Mutex<crate::state::BookIndex>>,
    query: String,
) -> Vec<SearchResult> {
    if query.trim().is_empty() {
        return vec![];
    }
    let idx = book_index.lock().unwrap();
    let query_lower = query.to_lowercase();
    let mut results: Vec<SearchResult> = Vec::new();

    'outer: for chapter in &idx.chapters {
        let text = &chapter.plain_text;
        let text_lower = text.to_lowercase();
        let mut byte_pos = 0usize;

        while let Some(rel) = text_lower[byte_pos..].find(&query_lower) {
            let abs = byte_pos + rel;

            let snip_start = {
                let mut s = abs.saturating_sub(60);
                while s > 0 && !text.is_char_boundary(s) {
                    s -= 1;
                }
                s
            };
            let snip_end = {
                let mut e = (abs + query.len() + 60).min(text.len());
                while e < text.len() && !text.is_char_boundary(e) {
                    e += 1;
                }
                e
            };

            results.push(SearchResult {
                chapter_index: chapter.index,
                chapter_title: chapter.title.clone(),
                snippet: text[snip_start..snip_end].to_string(),
                offset: abs,
            });

            byte_pos = abs + query_lower.len().max(1);
            if results.len() >= 30 {
                break 'outer;
            }
        }
    }
    results
}
