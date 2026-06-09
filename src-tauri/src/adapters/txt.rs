use crate::models::book::{
    AdapterError, Book, BookAdapter, BookFormat, BookMetadata, Chapter, ChapterContent,
};

pub struct TxtAdapter;

impl BookAdapter for TxtAdapter {
    fn parse(path: &std::path::Path) -> Result<Book, AdapterError> {
        if !path.exists() {
            return Err(AdapterError::FileNotFound);
        }

        let raw = std::fs::read(path).map_err(|e| AdapterError::ParseError(e.to_string()))?;
        let content = String::from_utf8_lossy(&raw).into_owned();

        let title = path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Untitled".to_string());

        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_ascii_lowercase());

        let chapters = match ext.as_deref() {
            Some("txt") => parse_txt(&content),
            _ => parse_md(&content), // "md"
        };

        Ok(Book {
            metadata: BookMetadata {
                title,
                author: "Unknown".to_string(),
                cover_image: None,
            },
            chapters,
            format: BookFormat::Txt,
            file_path: path.to_string_lossy().into_owned(),
        })
    }
}

fn parse_txt(content: &str) -> Vec<Chapter> {
    let parts: Vec<&str> = content
        .split("\n\n\n")
        .flat_map(|s| s.split("\n---\n"))
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();

    if parts.is_empty() {
        return vec![Chapter {
            index: 0,
            title: None,
            content: ChapterContent::PlainText(String::new()),
        }];
    }

    parts
        .iter()
        .enumerate()
        .map(|(i, seg)| Chapter {
            index: i,
            title: None,
            content: ChapterContent::PlainText(seg.to_string()),
        })
        .collect()
}

fn parse_md(content: &str) -> Vec<Chapter> {
    use pulldown_cmark::{html, Options, Parser};
    let parser = Parser::new_ext(content, Options::all());
    let mut html_out = String::new();
    html::push_html(&mut html_out, parser);
    let sanitized_html = crate::sanitize::sanitize_book_html(&html_out);
    vec![Chapter {
        index: 0,
        title: None,
        content: ChapterContent::Html(sanitized_html),
    }]
}
