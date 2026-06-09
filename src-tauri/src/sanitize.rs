use std::collections::HashSet;

use ammonia::{Builder, UrlRelative};

fn is_safe_data_image(value: &str) -> bool {
    let lower = value.trim_start().to_ascii_lowercase();
    [
        "data:image/png;base64,",
        "data:image/jpeg;base64,",
        "data:image/jpg;base64,",
        "data:image/gif;base64,",
        "data:image/webp;base64,",
        "data:image/svg+xml;base64,",
    ]
    .iter()
    .any(|prefix| lower.starts_with(prefix))
}

pub fn sanitize_book_html(html: &str) -> String {
    let mut builder = Builder::default();
    builder
        .add_tags(&["main", "section"])
        .url_schemes(HashSet::from(["data"]))
        .url_relative(UrlRelative::Deny)
        .link_rel(None)
        .attribute_filter(|element, attribute, value| match (element, attribute) {
            ("img", "src") if is_safe_data_image(value) => Some(value.into()),
            ("img", "src") => None,
            ("a", "href") => None,
            _ => Some(value.into()),
        });

    builder.clean(html).to_string()
}

#[cfg(test)]
mod tests {
    use super::sanitize_book_html;

    #[test]
    fn removes_scripts_handlers_and_external_urls() {
        let input = r#"
            <section onclick="alert(1)">
                <script>alert(1)</script>
                <a href="https://example.com">external</a>
                <img alt="remote" src="https://example.com/image.png">
                <img alt="inline" src="data:image/png;base64,AAAA">
            </section>
        "#;

        let sanitized = sanitize_book_html(input);

        assert!(!sanitized.contains("script"));
        assert!(!sanitized.contains("onclick"));
        assert!(!sanitized.contains("https://example.com"));
        assert!(sanitized.contains("<a>external</a>"));
        assert!(sanitized.contains(r#"<img alt="remote">"#));
        assert!(sanitized.contains(r#"<img alt="inline" src="data:image/png;base64,AAAA">"#));
    }

    #[test]
    fn strips_style_and_link_content() {
        let input = r#"
            <link rel="stylesheet" href="https://example.com/book.css">
            <style>body { background: red; }</style>
            <p style="background:url(https://example.com/x)">Text</p>
        "#;

        let sanitized = sanitize_book_html(input);

        assert!(!sanitized.contains("<link"));
        assert!(!sanitized.contains("<style"));
        assert!(!sanitized.contains("style="));
        assert_eq!(sanitized.trim(), "<p>Text</p>");
    }
}
