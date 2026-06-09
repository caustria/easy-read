use serde::Serialize;

#[derive(Serialize)]
pub struct BookMetadata {
    pub title: String,
    pub author: String,
    pub cover_image: Option<Vec<u8>>,
}

#[derive(Serialize)]
pub struct Chapter {
    pub index: usize,
    pub title: Option<String>,
    pub content: ChapterContent,
}

#[derive(Serialize)]
pub enum ChapterContent {
    Html(String),
    PlainText(String),
}

#[derive(Serialize)]
pub struct Book {
    pub metadata: BookMetadata,
    pub chapters: Vec<Chapter>,
    pub format: BookFormat,
    pub file_path: String,
}

#[derive(Serialize)]
pub enum BookFormat {
    Epub,
    Txt,
}

#[derive(Debug)]
pub enum AdapterError {
    FileNotFound,
    ParseError(String),
    UnsupportedFormat,
}

impl std::fmt::Display for AdapterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AdapterError::FileNotFound => write!(f, "File not found"),
            AdapterError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            AdapterError::UnsupportedFormat => write!(f, "Unsupported format"),
        }
    }
}

pub trait BookAdapter {
    fn parse(path: &std::path::Path) -> Result<Book, AdapterError>;
}
