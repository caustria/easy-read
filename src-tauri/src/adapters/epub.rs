use std::collections::HashMap;
use std::io::Read;
use std::path::Path;

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use percent_encoding::percent_decode_str;

use crate::models::book::{
    AdapterError, Book, BookAdapter, BookFormat, BookMetadata, Chapter, ChapterContent,
};

pub struct EpubAdapter;

const MAX_EPUB_ENTRY_SIZE: u64 = 64 * 1024 * 1024;
const MAX_EPUB_TOTAL_SIZE: u64 = 512 * 1024 * 1024;
const EPUB_TOO_LARGE: &str = "EPUB too large to open safely";

/// Resolve a relative path against a base file path within the EPUB zip.
fn resolve_path(base_file: &str, relative: &str) -> String {
    // Strip fragment identifiers (e.g. chapter.xhtml#section1)
    let relative = decode_url_path(relative);

    if relative.starts_with('/') {
        return relative.trim_start_matches('/').to_string();
    }

    let base_dir = base_file.rfind('/').map(|i| &base_file[..=i]).unwrap_or("");
    let combined = format!("{}{}", base_dir, relative);

    // Collapse . and ..
    let mut parts: Vec<&str> = Vec::new();
    for part in combined.split('/') {
        match part {
            ".." => {
                parts.pop();
            }
            "." | "" => {}
            p => parts.push(p),
        }
    }
    parts.join("/")
}

fn decode_url_path(path: &str) -> String {
    let path = path.split('#').next().unwrap_or(path);
    percent_decode_str(path).decode_utf8_lossy().into_owned()
}

fn get_resource<'a>(resources: &'a HashMap<String, Vec<u8>>, path: &str) -> Option<&'a Vec<u8>> {
    resources.get(path).or_else(|| {
        resources
            .iter()
            .find(|(candidate, _)| candidate.eq_ignore_ascii_case(path))
            .map(|(_, bytes)| bytes)
    })
}

fn guess_image_mime(path: &str) -> Option<&'static str> {
    let lower = path.to_lowercase();
    if lower.ends_with(".png") {
        Some("image/png")
    } else if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
        Some("image/jpeg")
    } else if lower.ends_with(".gif") {
        Some("image/gif")
    } else if lower.ends_with(".svg") {
        Some("image/svg+xml")
    } else if lower.ends_with(".webp") {
        Some("image/webp")
    } else {
        None
    }
}

/// Replace src="..." values in HTML with base64 data URIs using resources from the zip.
fn inline_images(html: &str, base_path: &str, resources: &HashMap<String, Vec<u8>>) -> String {
    let mut output = String::with_capacity(html.len() * 2);
    let mut cursor = 0;

    while let Some((value_start, value_end)) = find_next_src_value(&html[cursor..]) {
        let value_start = cursor + value_start;
        let value_end = cursor + value_end;
        let src = &html[value_start..value_end];

        output.push_str(&html[cursor..value_start]);
        if let Some(data_uri) = inline_image_src(src, base_path, resources) {
            output.push_str(&data_uri);
        } else {
            output.push_str(src);
        }
        cursor = value_end;
    }

    output.push_str(&html[cursor..]);
    output
}

fn find_next_src_value(input: &str) -> Option<(usize, usize)> {
    let bytes = input.as_bytes();
    let mut i = 0;
    while i + 3 <= bytes.len() {
        if bytes[i].eq_ignore_ascii_case(&b's')
            && bytes[i + 1].eq_ignore_ascii_case(&b'r')
            && bytes[i + 2].eq_ignore_ascii_case(&b'c')
            && (i == 0 || !is_attr_name_byte(bytes[i - 1]))
        {
            let mut j = i + 3;
            while j < bytes.len() && bytes[j].is_ascii_whitespace() {
                j += 1;
            }
            if j < bytes.len() && bytes[j] == b'=' {
                j += 1;
                while j < bytes.len() && bytes[j].is_ascii_whitespace() {
                    j += 1;
                }
                if j < bytes.len() && (bytes[j] == b'"' || bytes[j] == b'\'') {
                    let quote = bytes[j];
                    let value_start = j + 1;
                    if let Some(end_offset) = bytes[value_start..].iter().position(|b| *b == quote)
                    {
                        return Some((value_start, value_start + end_offset));
                    }
                    return None;
                }
            }
        }
        i += 1;
    }
    None
}

fn is_attr_name_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b':')
}

fn inline_image_src(
    src: &str,
    base_path: &str,
    resources: &HashMap<String, Vec<u8>>,
) -> Option<String> {
    let lower = src.trim_start().to_ascii_lowercase();
    if lower.starts_with("data:") || lower.starts_with("http:") || lower.starts_with("https:") {
        return None;
    }
    if src.trim_start().starts_with("//") {
        return None;
    }

    let resolved = resolve_path(base_path, src.trim());
    let mime = guess_image_mime(&resolved)?;
    let bytes = get_resource(resources, &resolved)?;
    Some(format!("data:{};base64,{}", mime, BASE64.encode(bytes)))
}

fn image_data_uri(path: &str, bytes: &[u8]) -> Option<String> {
    let mime = guess_image_mime(path)?;
    Some(format!("data:{};base64,{}", mime, BASE64.encode(bytes)))
}

fn extract_html_title(html: &str) -> Option<String> {
    let start = html.find("<title>")?;
    let after = &html[start + 7..];
    let end = after.find("</title>")?;
    let title = after[..end].trim().to_string();
    if title.is_empty() {
        None
    } else {
        Some(title)
    }
}

fn clean_chapter_title(title: &str, fallback_number: usize) -> Option<String> {
    let trimmed = title.trim();
    if trimmed.is_empty() {
        return None;
    }
    let lower = trimmed.to_lowercase();
    for prefix in &["chapter", "chap", "section", "part", "item", "sec", "ch"] {
        if let Some(stripped) = lower.strip_prefix(prefix) {
            let rest = stripped.trim_start_matches(|c: char| !c.is_ascii_digit());
            if !rest.is_empty()
                && rest
                    .chars()
                    .next()
                    .map(|c| c.is_ascii_digit())
                    .unwrap_or(false)
            {
                let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
                if let Ok(n) = digits.parse::<usize>() {
                    return Some(format!("Chapter {}", n));
                }
            }
            // prefix followed only by non-digit chars (e.g. "part_one") — use fallback
            return Some(format!("Chapter {}", fallback_number));
        }
    }
    Some(trimmed.to_string())
}

/// Build a zip-path → label map from an EPUB2 NCX file.
fn parse_ncx_toc(ncx_path: &str, ncx_bytes: &[u8]) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let Ok(s) = std::str::from_utf8(ncx_bytes) else {
        return map;
    };
    let Ok(doc) = roxmltree::Document::parse(s) else {
        return map;
    };
    for nav_point in doc.descendants().filter(|n| n.has_tag_name("navPoint")) {
        let label = nav_point
            .descendants()
            .find(|n| n.has_tag_name("text"))
            .and_then(|n| n.text())
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty());
        let src = nav_point
            .descendants()
            .find(|n| n.has_tag_name("content"))
            .and_then(|n| n.attribute("src"));
        if let (Some(label), Some(src)) = (label, src) {
            let path = resolve_path(ncx_path, src);
            map.entry(path).or_insert(label);
        }
    }
    map
}

/// Build a zip-path → label map from an EPUB3 Nav document.
fn parse_nav_toc(nav_path: &str, nav_bytes: &[u8]) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let Ok(s) = std::str::from_utf8(nav_bytes) else {
        return map;
    };
    let Ok(doc) = roxmltree::Document::parse(s) else {
        return map;
    };
    for nav in doc.descendants().filter(|n| n.has_tag_name("nav")) {
        let is_toc = nav.attributes().any(|a| a.value().contains("toc"));
        if !is_toc {
            continue;
        }
        for anchor in nav.descendants().filter(|n| n.has_tag_name("a")) {
            if let Some(href) = anchor.attribute("href") {
                let label: String = anchor
                    .descendants()
                    .filter(|n| n.is_text())
                    .filter_map(|n| n.text())
                    .collect::<Vec<_>>()
                    .join(" ")
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ");
                if !label.is_empty() {
                    let path = resolve_path(nav_path, href);
                    map.entry(path).or_insert(label);
                }
            }
        }
        break;
    }
    map
}

impl BookAdapter for EpubAdapter {
    fn parse(path: &Path) -> Result<Book, AdapterError> {
        let file_bytes = std::fs::read(path).map_err(|_| AdapterError::FileNotFound)?;
        let cursor = std::io::Cursor::new(file_bytes);
        let mut archive =
            zip::ZipArchive::new(cursor).map_err(|e| AdapterError::ParseError(e.to_string()))?;

        // Read all entries into a HashMap for random access, with conservative caps.
        let mut resources: HashMap<String, Vec<u8>> = HashMap::new();
        let mut total_decompressed_size: u64 = 0;
        for i in 0..archive.len() {
            let mut entry = match archive.by_index(i) {
                Ok(entry) => entry,
                Err(e) => {
                    eprintln!("Skipping unreadable EPUB entry at index {}: {}", i, e);
                    continue;
                }
            };
            if !entry.is_file() {
                continue;
            }
            let entry_size = entry.size();
            if entry_size > MAX_EPUB_ENTRY_SIZE {
                return Err(AdapterError::ParseError(EPUB_TOO_LARGE.into()));
            }
            total_decompressed_size = total_decompressed_size
                .checked_add(entry_size)
                .ok_or_else(|| AdapterError::ParseError(EPUB_TOO_LARGE.into()))?;
            if total_decompressed_size > MAX_EPUB_TOTAL_SIZE {
                return Err(AdapterError::ParseError(EPUB_TOO_LARGE.into()));
            }

            let name = entry.name().to_string();
            let mut buf = Vec::new();
            if let Err(e) = entry.read_to_end(&mut buf) {
                eprintln!("Skipping unreadable EPUB entry {}: {}", name, e);
                continue;
            }
            resources.insert(name, buf);
        }

        if get_resource(&resources, "META-INF/encryption.xml").is_some() {
            return Err(AdapterError::ParseError(
                "This EPUB is DRM-protected and cannot be opened".into(),
            ));
        }

        // --- Parse container.xml → OPF path ---
        let container_bytes = get_resource(&resources, "META-INF/container.xml")
            .ok_or_else(|| AdapterError::ParseError("Missing META-INF/container.xml".into()))?;
        let container_str = std::str::from_utf8(container_bytes)
            .map_err(|e| AdapterError::ParseError(e.to_string()))?;
        let container_doc = roxmltree::Document::parse(container_str)
            .map_err(|e| AdapterError::ParseError(e.to_string()))?;
        let opf_path = container_doc
            .descendants()
            .find(|n| n.has_tag_name("rootfile"))
            .and_then(|n| n.attribute("full-path"))
            .map(decode_url_path)
            .ok_or_else(|| AdapterError::ParseError("No rootfile in container.xml".into()))?;

        // --- Parse OPF ---
        let opf_bytes = get_resource(&resources, &opf_path)
            .ok_or_else(|| AdapterError::ParseError(format!("OPF not found: {}", opf_path)))?;
        let opf_str =
            std::str::from_utf8(opf_bytes).map_err(|e| AdapterError::ParseError(e.to_string()))?;
        let opf_doc = roxmltree::Document::parse(opf_str)
            .map_err(|e| AdapterError::ParseError(e.to_string()))?;

        // Metadata
        let title = opf_doc
            .descendants()
            .find(|n| n.has_tag_name("title"))
            .and_then(|n| n.text())
            .unwrap_or("Unknown Title")
            .to_string();
        let author = opf_doc
            .descendants()
            .find(|n| n.has_tag_name("creator"))
            .and_then(|n| n.text())
            .unwrap_or("Unknown Author")
            .to_string();

        // Manifest: id → full zip path
        let mut manifest: HashMap<String, String> = HashMap::new();
        let mut cover_id: Option<String> = None;

        for node in opf_doc.descendants() {
            if node.has_tag_name("item") {
                if let (Some(id), Some(href)) = (node.attribute("id"), node.attribute("href")) {
                    let full_path = resolve_path(&opf_path, href);
                    manifest.insert(id.to_string(), full_path.clone());

                    // Cover image by properties attribute
                    if node
                        .attribute("properties")
                        .map(|p| p.contains("cover-image"))
                        .unwrap_or(false)
                    {
                        cover_id = Some(id.to_string());
                    }
                    // Cover image by id convention
                    if cover_id.is_none()
                        && id.to_lowercase().contains("cover")
                        && node
                            .attribute("media-type")
                            .map(|m| m.starts_with("image/"))
                            .unwrap_or(false)
                    {
                        cover_id = Some(id.to_string());
                    }
                }
            }
            // Cover via <meta name="cover" content="...">
            if node.has_tag_name("meta") && node.attribute("name") == Some("cover") {
                if let Some(content) = node.attribute("content") {
                    cover_id = Some(content.to_string());
                }
            }
        }

        let cover_image = cover_id
            .as_deref()
            .and_then(|id| manifest.get(id))
            .and_then(|p| get_resource(&resources, p).and_then(|bytes| image_data_uri(p, bytes)));

        // Build TOC label map: zip-path → chapter label
        // Prefer EPUB3 nav, fall back to EPUB2 NCX.
        let mut toc_labels: HashMap<String, String> = HashMap::new();

        // EPUB3 nav
        let nav_path = opf_doc
            .descendants()
            .filter(|n| n.has_tag_name("item"))
            .find(|n| {
                n.attribute("properties")
                    .map(|p| p.contains("nav"))
                    .unwrap_or(false)
            })
            .and_then(|n| n.attribute("href"))
            .map(|href| resolve_path(&opf_path, href));
        if let Some(ref nav_path) = nav_path {
            if let Some(bytes) = get_resource(&resources, nav_path) {
                toc_labels = parse_nav_toc(nav_path, bytes);
            }
        }

        // EPUB2 NCX (fallback)
        if toc_labels.is_empty() {
            let ncx_path = opf_doc
                .descendants()
                .filter(|n| n.has_tag_name("item"))
                .find(|n| n.attribute("media-type") == Some("application/x-dtbncx+xml"))
                .and_then(|n| n.attribute("href"))
                .map(|href| resolve_path(&opf_path, href));
            if let Some(ref ncx_path) = ncx_path {
                if let Some(bytes) = get_resource(&resources, ncx_path) {
                    toc_labels = parse_ncx_toc(ncx_path, bytes);
                }
            }
        }

        // Spine: ordered chapter paths
        let mut spine_paths: Vec<String> = Vec::new();
        for node in opf_doc.descendants() {
            if node.has_tag_name("itemref") {
                if let Some(idref) = node.attribute("idref") {
                    if let Some(href) = manifest.get(idref) {
                        spine_paths.push(href.clone());
                    }
                }
            }
        }

        if spine_paths.is_empty() {
            return Err(AdapterError::ParseError("Spine is empty".into()));
        }

        // Build chapters
        let mut chapters: Vec<Chapter> = Vec::new();
        for (index, item_path) in spine_paths.iter().enumerate() {
            let html_bytes = match get_resource(&resources, item_path) {
                Some(b) => b,
                None => continue,
            };
            let html_str = match std::str::from_utf8(html_bytes) {
                Ok(s) => s,
                Err(_) => continue,
            };

            let html_with_images = inline_images(html_str, item_path, &resources);
            let sanitized_html = crate::sanitize::sanitize_book_html(&html_with_images);

            let book_title_lower = title.trim().to_lowercase();
            let is_placeholder = |s: &str| -> bool {
                let lower = s.trim().to_lowercase();
                // Also catches common misspellings and single-word generic labels
                lower.is_empty()
                    || lower == "unnamed"
                    || lower == "unamed"
                    || lower == "untitled"
                    || lower == "unknown"
                    || lower == "no title"
                    || lower == "none"
                    || lower == book_title_lower
            };

            // Use TOC label if meaningful, then HTML <title>, then numbered fallback.
            let chapter_title = toc_labels
                .get(item_path)
                .map(|s| s.as_str())
                .filter(|t| !is_placeholder(t))
                .map(|t| t.trim().to_string())
                .or_else(|| {
                    extract_html_title(html_str)
                        .filter(|t| !is_placeholder(t))
                        .and_then(|t| clean_chapter_title(&t, index + 1))
                })
                .or_else(|| Some(format!("Chapter {}", index + 1)));

            chapters.push(Chapter {
                index,
                title: chapter_title,
                content: ChapterContent::Html(sanitized_html),
            });
        }

        if chapters.is_empty() {
            return Err(AdapterError::ParseError(
                "No readable chapters found".into(),
            ));
        }

        Ok(Book {
            metadata: BookMetadata {
                title,
                author,
                cover_image,
            },
            chapters,
            format: BookFormat::Epub,
            file_path: path.to_string_lossy().into_owned(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{inline_images, EpubAdapter};
    use crate::models::book::{BookAdapter, ChapterContent};
    use std::collections::HashMap;
    use std::fs::File;
    use std::io::Write;
    use std::time::{SystemTime, UNIX_EPOCH};
    use zip::write::SimpleFileOptions;

    fn write_test_epub(entries: &[(&str, &[u8])]) -> std::path::PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "easy-read-phase5-{}-{}.epub",
            std::process::id(),
            unique
        ));
        let file = File::create(&path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);

        for (name, bytes) in entries {
            zip.start_file(name, options).unwrap();
            zip.write_all(bytes).unwrap();
        }
        zip.finish().unwrap();

        path
    }

    #[test]
    fn parses_percent_encoded_epub_paths_with_case_fallback_and_cover_data_uri() {
        let container = br#"<?xml version="1.0"?>
            <container>
              <rootfiles>
                <rootfile full-path="OPS/package.opf"/>
              </rootfiles>
            </container>"#;
        let opf = br#"<?xml version="1.0"?>
            <package>
              <metadata>
                <title>Phase 5 Fixture</title>
                <creator>Fixture Author</creator>
              </metadata>
              <manifest>
                <item id="nav" href="nav.xhtml" properties="nav" media-type="application/xhtml+xml"/>
                <item id="chapter" href="Text/My%20Chapter.xhtml" media-type="application/xhtml+xml"/>
                <item id="cover" href="Images/Cover%20Image.PNG" properties="cover-image" media-type="image/png"/>
                <item id="image" href="Images/Pic%20One.PNG" media-type="image/png"/>
              </manifest>
              <spine>
                <itemref idref="chapter"/>
              </spine>
            </package>"#;
        let nav = br#"<html xmlns:epub="http://www.idpf.org/2007/ops">
            <body>
              <nav epub:type="toc">
                <ol><li><a href="Text/My%20Chapter.xhtml">Decoded Chapter</a></li></ol>
              </nav>
            </body>
          </html>"#;
        let chapter = br#"<html>
            <head><title>Ignored Placeholder</title></head>
            <body><p>Hello</p><img ALT='pic' SRC = '../Images/Pic%20One.PNG'></body>
          </html>"#;
        let path = write_test_epub(&[
            ("META-INF/container.xml", container.as_slice()),
            ("OPS/package.opf", opf.as_slice()),
            ("OPS/nav.xhtml", nav.as_slice()),
            ("OPS/Text/My Chapter.XHTML", chapter.as_slice()),
            ("OPS/Images/Pic One.PNG", b"image bytes"),
            ("OPS/Images/Cover Image.PNG", b"cover bytes"),
        ]);

        let book = EpubAdapter::parse(&path).unwrap();
        std::fs::remove_file(path).unwrap();

        assert_eq!(book.metadata.title, "Phase 5 Fixture");
        assert_eq!(book.metadata.author, "Fixture Author");
        assert!(book
            .metadata
            .cover_image
            .as_deref()
            .unwrap()
            .starts_with("data:image/png;base64,"));
        assert_eq!(book.chapters.len(), 1);
        assert_eq!(book.chapters[0].title.as_deref(), Some("Decoded Chapter"));
        assert!(matches!(
            &book.chapters[0].content,
            ChapterContent::Html(html) if html.contains("data:image/png;base64,")
        ));
    }

    #[test]
    fn rejects_epubs_with_encryption_xml() {
        let container = br#"<?xml version="1.0"?>
            <container>
              <rootfiles>
                <rootfile full-path="OPS/package.opf"/>
              </rootfiles>
            </container>"#;
        let path = write_test_epub(&[
            ("META-INF/container.xml", container.as_slice()),
            ("META-INF/encryption.xml", b"<encryption/>"),
        ]);

        let err = match EpubAdapter::parse(&path) {
            Ok(_) => panic!("encrypted EPUB should be rejected"),
            Err(err) => err.to_string(),
        };
        std::fs::remove_file(path).unwrap();

        assert!(err.contains("DRM-protected"));
    }

    #[test]
    fn opens_epub_when_unused_entry_is_corrupt() {
        let container = br#"<?xml version="1.0"?>
            <container>
              <rootfiles>
                <rootfile full-path="OPS/package.opf"/>
              </rootfiles>
            </container>"#;
        let opf = br#"<?xml version="1.0"?>
            <package>
              <metadata>
                <title>Corrupt Entry Fixture</title>
                <creator>Fixture Author</creator>
              </metadata>
              <manifest>
                <item id="chapter" href="chapter.xhtml" media-type="application/xhtml+xml"/>
              </manifest>
              <spine>
                <itemref idref="chapter"/>
              </spine>
            </package>"#;
        let chapter = br#"<html><body><p>Readable</p></body></html>"#;
        let corrupt_payload = b"CORRUPT_ME_PAYLOAD";
        let path = write_test_epub(&[
            ("bad.bin", corrupt_payload),
            ("META-INF/container.xml", container.as_slice()),
            ("OPS/package.opf", opf.as_slice()),
            ("OPS/chapter.xhtml", chapter.as_slice()),
        ]);
        let mut bytes = std::fs::read(&path).unwrap();
        let corrupt_at = bytes
            .windows(corrupt_payload.len())
            .position(|window| window == corrupt_payload)
            .unwrap();
        bytes[corrupt_at] = b'X';
        std::fs::write(&path, bytes).unwrap();

        let book = EpubAdapter::parse(&path).unwrap();
        std::fs::remove_file(path).unwrap();

        assert_eq!(book.chapters.len(), 1);
    }

    #[test]
    fn inline_images_handles_single_quotes_case_whitespace_and_percent_encoding() {
        let mut resources = HashMap::new();
        resources.insert(
            "OPS/Images/Pic One.PNG".to_string(),
            b"image bytes".to_vec(),
        );

        let html = "<img ALT='pic' SRC = '../Images/Pic%20One.PNG'>";
        let inlined = inline_images(html, "OPS/Text/chapter.xhtml", &resources);

        assert!(inlined.contains("data:image/png;base64,"));
    }
}
