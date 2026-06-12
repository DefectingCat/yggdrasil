#![allow(clippy::unused_unit, deprecated, unused_imports)]

#[cfg(feature = "server")]
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(feature = "server")]
static COMMENT_AMMONIA_BUILDER: std::sync::LazyLock<ammonia::Builder> = std::sync::LazyLock::new(|| {
    let mut builder = ammonia::Builder::default();
    builder
        .rm_tags(["img", "details", "summary"])
        .add_generic_attributes(&[
            "class",
            "title",
            "aria-hidden",
            "aria-label",
            "role",
            "accesskey",
        ])
        .url_relative(ammonia::UrlRelative::PassThrough)
        .add_tag_attributes("a", &["class", "aria-hidden", "aria-label"])
        .add_tag_attributes("span", &["class"])
        .link_rel(Some("nofollow noopener"));
    builder
});

#[cfg(feature = "server")]
pub fn clean_comment_html(input: &str) -> String {
    COMMENT_AMMONIA_BUILDER.clean(input).to_string()
}

#[cfg(feature = "server")]
pub fn render_comment_markdown(md: &str) -> String {
    use pulldown_cmark::{CodeBlockKind, Event, Options, Tag, TagEnd};

    let opts = Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH;
    let parser = pulldown_cmark::Parser::new_ext(md, opts);

    let mut events: Vec<Event> = Vec::new();
    let mut in_codeblock = false;
    let mut code_lang: Option<String> = None;
    let mut code_buffer = String::new();

    for event in parser {
        match event {
            Event::Start(Tag::Heading { .. }) => {
                events.push(Event::Start(Tag::Strong));
            }
            Event::End(TagEnd::Heading(_)) => {
                events.push(Event::End(TagEnd::Strong));
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                in_codeblock = true;
                code_lang = match kind {
                    CodeBlockKind::Fenced(lang) if !lang.is_empty() => {
                        Some(lang.to_string())
                    }
                    _ => None,
                };
                code_buffer.clear();
            }
            Event::Text(text) if in_codeblock => {
                code_buffer.push_str(&text);
            }
            Event::End(TagEnd::CodeBlock) => {
                let html = if let Some(ref lang) = code_lang {
                    let highlighted =
                        crate::highlight::server::highlight_code(&code_buffer, Some(lang));
                    format!("<pre><code>{}</code></pre>", highlighted)
                } else {
                    format!("<pre><code>{}</code></pre>", html_escape(&code_buffer))
                };
                events.push(Event::Html(html.into()));
                in_codeblock = false;
            }
            _ if !in_codeblock => {
                events.push(event);
            }
            _ => {}
        }
    }

    let mut html = String::new();
    pulldown_cmark::html::push_html(&mut html, events.into_iter());
    clean_comment_html(&html)
}

#[cfg(all(test, feature = "server"))]
mod tests {
    use super::*;

    #[test]
    fn render_comment_heading_converted_to_strong() {
        let result = render_comment_markdown("## Hello World");
        assert!(result.contains("<strong>Hello World</strong>"));
        assert!(!result.contains("<h2>"));
    }

    #[test]
    fn render_comment_heading_all_levels() {
        for md in &[
            "# H1", "## H2", "### H3", "#### H4", "##### H5", "###### H6",
        ] {
            let result = render_comment_markdown(md);
            assert!(result.contains("<strong>"), "heading not converted for: {}", md);
        }
    }

    #[test]
    fn render_comment_paragraph() {
        let result = render_comment_markdown("Hello **world**");
        assert!(result.contains("<strong>world</strong>"));
    }

    #[test]
    fn render_comment_code_block_with_language() {
        let result = render_comment_markdown("```rust\nfn main() {}\n```");
        assert!(result.contains("<pre><code>"));
        assert!(result.contains("main"));
    }

    #[test]
    fn render_comment_code_block_without_language() {
        let result = render_comment_markdown("```\nplain text\n```");
        assert!(result.contains("<pre><code>"));
        assert!(result.contains("plain text"));
    }

    #[test]
    fn render_comment_code_block_without_language_escapes_html() {
        let result = render_comment_markdown("```\n<div>alert('xss')</div>\n```");
        assert!(result.contains("&lt;div&gt;"));
        assert!(!result.contains("<div>"));
    }

    #[test]
    fn render_comment_strips_script() {
        let result = render_comment_markdown("<script>alert('xss')</script>");
        assert!(!result.contains("script"));
    }

    #[test]
    fn render_comment_no_img_tags() {
        let result = render_comment_markdown("![alt](https://example.com/img.png)");
        assert!(!result.contains("<img"));
    }

    #[test]
    fn render_comment_link_has_nofollow() {
        let result = render_comment_markdown("[link](https://example.com)");
        assert!(result.contains("nofollow"));
        assert!(result.contains("noopener"));
    }

    #[test]
    fn render_comment_no_id_attribute() {
        let result = render_comment_markdown("<div id=\"test\">text</div>");
        assert!(!result.contains("id="));
    }

    #[test]
    fn render_comment_table() {
        let result = render_comment_markdown("| a | b |\n|---|---|\n| 1 | 2 |");
        assert!(result.contains("<table>"));
    }

    #[test]
    fn render_comment_strikethrough() {
        let result = render_comment_markdown("~~deleted~~");
        assert!(result.contains("<del>deleted</del>"));
    }

    #[test]
    fn render_comment_inline_code() {
        let result = render_comment_markdown("Use `println!` to print");
        assert!(result.contains("<code>println!</code>"));
    }

    #[test]
    fn clean_comment_html_removes_details_summary() {
        let result = clean_comment_html("<details><summary>Click</summary><p>Content</p></details>");
        assert!(!result.contains("details"));
        assert!(!result.contains("summary"));
    }

    #[test]
    fn clean_comment_html_removes_data_uri() {
        let result =
            clean_comment_html("<a href=\"data:text/html,<script>alert(1)</script>\">click</a>");
        assert!(!result.contains("data:"));
    }

    #[test]
    fn render_comment_empty() {
        let result = render_comment_markdown("");
        assert!(result.is_empty());
    }

    #[test]
    fn render_comment_heading_with_inline_code() {
        let result = render_comment_markdown("## Using `foo()`");
        assert!(result.contains("<strong>"));
        assert!(result.contains("<code>foo()</code>"));
        assert!(!result.contains("<h2>"));
    }
}
