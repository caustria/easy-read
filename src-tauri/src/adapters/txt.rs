use crate::models::book::{
    AdapterError, Book, BookAdapter, BookFormat, BookMetadata, Chapter, ChapterContent,
};
use chardetng::EncodingDetector;

pub struct TxtAdapter;

impl BookAdapter for TxtAdapter {
    fn parse(path: &std::path::Path) -> Result<Book, AdapterError> {
        if !path.exists() {
            return Err(AdapterError::FileNotFound);
        }

        let raw = std::fs::read(path).map_err(|e| AdapterError::ParseError(e.to_string()))?;
        let content = normalize_content(decode_text(&raw));

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

fn decode_text(raw: &[u8]) -> String {
    if let Some((encoding, bom_len)) = encoding_rs::Encoding::for_bom(raw) {
        let (decoded, _had_errors) = encoding.decode_without_bom_handling(&raw[bom_len..]);
        return decoded.into_owned();
    }

    if let Ok(content) = std::str::from_utf8(raw) {
        return content.to_string();
    }

    let mut detector = EncodingDetector::new();
    detector.feed(raw, true);
    let detected = detector.guess(None, true);
    let (decoded, _encoding_used, had_errors) = detected.decode(raw);
    if !had_errors {
        return decoded.into_owned();
    }

    let (decoded, _encoding_used, _had_errors) = encoding_rs::WINDOWS_1252.decode(raw);
    decoded.into_owned()
}

fn normalize_content(mut content: String) -> String {
    if content.starts_with('\u{feff}') {
        content.remove(0);
    }

    if content.contains('\r') {
        content = content.replace("\r\n", "\n").replace('\r', "\n");
    }

    content
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

#[cfg(test)]
mod tests {
    use super::{decode_text, normalize_content, parse_txt};
    use crate::models::book::ChapterContent;

    #[test]
    fn normalizes_bom_and_windows_newlines_before_splitting() {
        let content = normalize_content("\u{feff}First\r\n\r\n\r\nSecond\r\n---\r\nThird".into());
        let chapters = parse_txt(&content);

        assert_eq!(chapters.len(), 3);
        assert!(matches!(
            &chapters[0].content,
            ChapterContent::PlainText(text) if text == "First"
        ));
        assert!(matches!(
            &chapters[1].content,
            ChapterContent::PlainText(text) if text == "Second"
        ));
        assert!(matches!(
            &chapters[2].content,
            ChapterContent::PlainText(text) if text == "Third"
        ));
    }

    #[test]
    fn decodes_windows_1252_when_utf8_is_invalid() {
        let content = decode_text(b"Caf\xe9");

        assert_eq!(content, "Caf\u{e9}");
    }

    #[test]
    fn preserves_existing_lf_utf8_txt_splitting() {
        let chapters = parse_txt("First\n\n\nSecond\n---\nThird");

        assert_eq!(chapters.len(), 3);
        assert!(matches!(
            &chapters[0].content,
            ChapterContent::PlainText(text) if text == "First"
        ));
        assert!(matches!(
            &chapters[1].content,
            ChapterContent::PlainText(text) if text == "Second"
        ));
        assert!(matches!(
            &chapters[2].content,
            ChapterContent::PlainText(text) if text == "Third"
        ));
    }
}
