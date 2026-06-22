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
pub async fn search_book(
    book_index: tauri::State<'_, Mutex<crate::state::BookIndex>>,
    query: String,
) -> Result<Vec<SearchResult>, String> {
    if query.trim().is_empty() {
        return Ok(vec![]);
    }
    let chapters = {
        let idx = crate::state::recover_lock(book_index.inner());
        idx.chapters.clone()
    };

    let results = tauri::async_runtime::spawn_blocking(move || search_chapters(chapters, query))
        .await
        .map_err(|err| format!("failed to search book: {err}"))?;
    Ok(results)
}

fn search_chapters(chapters: Vec<crate::state::ChapterText>, query: String) -> Vec<SearchResult> {
    let query_folded = fold_for_search(&query);
    if query_folded.is_empty() {
        return vec![];
    }

    let mut results: Vec<SearchResult> = Vec::new();

    'outer: for chapter in &chapters {
        let text = &chapter.plain_text;
        for (start, end) in find_case_insensitive_matches(text, &query_folded) {
            let snip_start = {
                let mut s = start.saturating_sub(60);
                while s > 0 && !text.is_char_boundary(s) {
                    s -= 1;
                }
                s
            };
            let snip_end = {
                let mut e = end.saturating_add(60).min(text.len());
                while e < text.len() && !text.is_char_boundary(e) {
                    e += 1;
                }
                e
            };

            results.push(SearchResult {
                chapter_index: chapter.index,
                chapter_title: chapter.title.clone(),
                snippet: text[snip_start..snip_end].to_string(),
                offset: start,
            });

            if results.len() >= 30 {
                break 'outer;
            }
        }
    }
    results
}

fn find_case_insensitive_matches(text: &str, folded_query: &str) -> Vec<(usize, usize)> {
    let mut matches = Vec::new();
    let mut next_start = 0;

    for (start, _) in text.char_indices() {
        if start < next_start {
            continue;
        }
        let mut folded = String::new();

        for (rel, ch) in text[start..].char_indices() {
            let end = start + rel + ch.len_utf8();
            for folded_ch in ch.to_lowercase() {
                folded.push(folded_ch);
            }

            if folded == folded_query {
                matches.push((start, end));
                next_start = end;
                break;
            }
            if !folded_query.starts_with(&folded) {
                break;
            }
        }
    }

    matches
}

fn fold_for_search(text: &str) -> String {
    text.chars().flat_map(char::to_lowercase).collect()
}

#[cfg(test)]
mod tests {
    use super::search_chapters;
    use crate::state::ChapterText;

    #[test]
    fn reports_original_offsets_after_expanding_unicode_lowercase() {
        let text = "Intro İstanbul has extra lowercase bytes. Target lands later.".to_string();
        let expected_offset = text.find("Target").unwrap();
        let results = search_chapters(
            vec![ChapterText {
                index: 0,
                title: Some("Unicode".to_string()),
                plain_text: text,
            }],
            "target".to_string(),
        );

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].offset, expected_offset);
        assert!(results[0].snippet.contains("Target lands later"));
    }

    #[test]
    fn keeps_ascii_case_insensitive_non_overlapping_behavior() {
        let results = search_chapters(
            vec![ChapterText {
                index: 0,
                title: None,
                plain_text: "Banana banana BANANA".to_string(),
            }],
            "ana".to_string(),
        );

        assert_eq!(
            results.iter().map(|r| r.offset).collect::<Vec<_>>(),
            vec![1, 8, 15]
        );
    }
}
