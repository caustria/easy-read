use std::collections::HashMap;
use std::io::Read;
use std::path::Path;

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};

use crate::models::book::{
    AdapterError, Book, BookAdapter, BookFormat, BookMetadata, Chapter, ChapterContent,
};

pub struct EpubAdapter;

/// Resolve a relative path against a base file path within the EPUB zip.
fn resolve_path(base_file: &str, relative: &str) -> String {
    // Strip fragment identifiers (e.g. chapter.xhtml#section1)
    let relative = relative.split('#').next().unwrap_or(relative);

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
    let mut remaining = html;

    while let Some(pos) = remaining.find("src=\"") {
        // Push everything up to and including src="
        output.push_str(&remaining[..pos + 5]);
        remaining = &remaining[pos + 5..];

        if let Some(end) = remaining.find('"') {
            let src = &remaining[..end];

            let inlined = if !src.starts_with("data:")
                && !src.starts_with("http")
                && !src.starts_with("//")
            {
                let resolved = resolve_path(base_path, src);
                if let Some(mime) = guess_image_mime(&resolved) {
                    resources
                        .get(&resolved)
                        .map(|bytes| format!("data:{};base64,{}", mime, BASE64.encode(bytes)))
                } else {
                    None
                }
            } else {
                None
            };

            if let Some(data_uri) = inlined {
                output.push_str(&data_uri);
            } else {
                output.push_str(src);
            }
            remaining = &remaining[end..]; // position at closing "
        }
    }
    output.push_str(remaining);
    output
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
        if lower.starts_with(prefix) {
            let rest = lower[prefix.len()..].trim_start_matches(|c: char| !c.is_ascii_digit());
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

        // Read all entries into a HashMap for random access
        let mut resources: HashMap<String, Vec<u8>> = HashMap::new();
        for i in 0..archive.len() {
            let mut entry = archive
                .by_index(i)
                .map_err(|e| AdapterError::ParseError(e.to_string()))?;
            if !entry.is_file() {
                continue;
            }
            let name = entry.name().to_string();
            let mut buf = Vec::new();
            entry
                .read_to_end(&mut buf)
                .map_err(|e| AdapterError::ParseError(e.to_string()))?;
            resources.insert(name, buf);
        }

        // --- Parse container.xml → OPF path ---
        let container_bytes = resources
            .get("META-INF/container.xml")
            .ok_or_else(|| AdapterError::ParseError("Missing META-INF/container.xml".into()))?;
        let container_str = std::str::from_utf8(container_bytes)
            .map_err(|e| AdapterError::ParseError(e.to_string()))?;
        let container_doc = roxmltree::Document::parse(container_str)
            .map_err(|e| AdapterError::ParseError(e.to_string()))?;
        let opf_path = container_doc
            .descendants()
            .find(|n| n.has_tag_name("rootfile"))
            .and_then(|n| n.attribute("full-path"))
            .ok_or_else(|| AdapterError::ParseError("No rootfile in container.xml".into()))?
            .to_string();

        // --- Parse OPF ---
        let opf_bytes = resources
            .get(&opf_path)
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
            .and_then(|p| resources.get(p))
            .cloned();

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
            if let Some(bytes) = resources.get(nav_path) {
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
                if let Some(bytes) = resources.get(ncx_path) {
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
            let html_bytes = match resources.get(item_path) {
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
                    extract_html_title(&html_str)
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
