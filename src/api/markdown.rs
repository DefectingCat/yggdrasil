//! Markdown 渲染与目录生成。
//!
//! 使用 pulldown-cmark 解析 Markdown，为标题生成锚点与目录（TOC），
//! 代码块调用 `highlight` 模块进行语法高亮，最终通过 sanitizer 清理 HTML。
//! 仅在 `feature = "server"` 时执行实际渲染。

#![allow(clippy::unused_unit, deprecated)]

#[cfg(feature = "server")]
/// 对外暴露的 HTML 清理函数，委托给 sanitizer 模块。
pub fn clean_html(input: &str) -> String {
    crate::api::sanitizer::clean_html(input)
}

#[cfg(feature = "server")]
/// 将标题纯文本转义，用于安全地拼进 TOC 的 `aria-label="..."` 与 `<a>` 正文。
///
/// 复用 `hooks::comment_storage::escape_html`（转义 `& < > " '`），避免在仓库内
/// 维护第二份转义实现。原先用 `clean_html` 处理属性上下文会漏掉 `"`，标题形如
/// `" onmouseover="alert(1)` 会越出属性边界。
fn escape_heading_text(s: &str) -> String {
    crate::hooks::comment_storage::escape_html(s)
}

#[derive(Debug, Clone)]
#[cfg(feature = "server")]
/// Markdown 渲染结果。
pub struct RenderedContent {
    /// 清理后的正文 HTML。
    pub html: String,
    /// 目录 HTML（无标题时为空字符串）。
    pub toc_html: String,
}

#[cfg(feature = "server")]
/// 增强版 Markdown 渲染：生成 TOC、标题锚点与语法高亮代码块。
pub fn render_markdown_enhanced(md: &str) -> RenderedContent {
    use pulldown_cmark::{Event, HeadingLevel, Options, Tag, TagEnd};

    // 1. Parse markdown and collect headings for TOC
    let parser = pulldown_cmark::Parser::new_ext(md, Options::all());
    // (level, text, id)
    let mut headings: Vec<(u8, String, String)> = Vec::new();
    let mut current_heading: Option<(u8, String)> = None;

    for event in parser {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                let lvl = match level {
                    HeadingLevel::H1 => 1,
                    HeadingLevel::H2 => 2,
                    HeadingLevel::H3 => 3,
                    HeadingLevel::H4 => 4,
                    HeadingLevel::H5 => 5,
                    HeadingLevel::H6 => 6,
                };
                current_heading = Some((lvl, String::new()));
            }
            Event::Text(text) => {
                if let Some((_, ref mut content)) = current_heading {
                    content.push_str(&text);
                }
            }
            Event::Code(code) => {
                if let Some((_, ref mut content)) = current_heading {
                    content.push_str(&code);
                }
            }
            Event::End(TagEnd::Heading(_)) => {
                if let Some((lvl, text)) = current_heading.take() {
                    let id = slugify_heading(&text);
                    headings.push((lvl, text, id));
                }
            }
            _ => {}
        }
    }

    // 2. Generate TOC HTML
    let toc_html = generate_toc_html(&headings);

    // 3. Generate HTML with heading anchors
    let parser = pulldown_cmark::Parser::new_ext(md, Options::ENABLE_TABLES);
    let mut html = String::new();
    let mut heading_idx = 0;
    let mut in_heading = false;
    let mut in_codeblock = false;
    let mut code_lang: Option<String> = None;
    let mut code_buffer = String::new();
    let mut non_heading_events: Vec<Event> = Vec::new();

    for event in parser {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                // 先把累积的普通事件刷入 HTML，再开始新标题。
                if !non_heading_events.is_empty() {
                    pulldown_cmark::html::push_html(&mut html, non_heading_events.into_iter());
                    non_heading_events = Vec::new();
                }
                in_heading = true;
                if heading_idx < headings.len() {
                    let (_, _, ref id) = headings[heading_idx];
                    let tag = match level {
                        HeadingLevel::H1 => "h1",
                        HeadingLevel::H2 => "h2",
                        HeadingLevel::H3 => "h3",
                        HeadingLevel::H4 => "h4",
                        HeadingLevel::H5 => "h5",
                        HeadingLevel::H6 => "h6",
                    };
                    html.push_str(&format!("<{} id=\"{}\">", tag, id));
                }
            }
            Event::End(TagEnd::Heading(level)) => {
                if heading_idx < headings.len() {
                    let (_, _, ref id) = headings[heading_idx];
                    let tag = match level {
                        HeadingLevel::H1 => "h1",
                        HeadingLevel::H2 => "h2",
                        HeadingLevel::H3 => "h3",
                        HeadingLevel::H4 => "h4",
                        HeadingLevel::H5 => "h5",
                        HeadingLevel::H6 => "h6",
                    };
                    html.push_str(&format!(
                        "<a class=\"anchor\" aria-hidden=\"true\" href=\"#{}\">#</a></{}>",
                        id, tag
                    ));
                    heading_idx += 1;
                }
                in_heading = false;
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                // 代码块开始前同样先刷入未处理的普通事件。
                if !non_heading_events.is_empty() {
                    pulldown_cmark::html::push_html(&mut html, non_heading_events.into_iter());
                    non_heading_events = Vec::new();
                }
                in_codeblock = true;
                code_lang = match kind {
                    pulldown_cmark::CodeBlockKind::Fenced(lang) => {
                        if lang.is_empty() {
                            None
                        } else {
                            Some(lang.to_string())
                        }
                    }
                    _ => None,
                };
                code_buffer.clear();
            }
            Event::Text(text) if in_codeblock => {
                code_buffer.push_str(&text);
            }
            Event::End(TagEnd::CodeBlock) => {
                // 使用 syntect 对代码块进行服务端语法高亮。
                let highlighted =
                    crate::highlight::server::highlight_code(&code_buffer, code_lang.as_deref());
                html.push_str("<pre><code");
                if let Some(lang) = &code_lang {
                    html.push_str(&format!(" class=\"language-{lang}\""));
                }
                html.push('>');
                html.push_str(&highlighted);
                html.push_str("</code></pre>");
                in_codeblock = false;
            }
            _ => {
                if in_heading {
                    // 标题内部只保留文本与行内代码，避免嵌套块级元素。
                    match event {
                        Event::Text(text) => html.push_str(&clean_html(&text)),
                        Event::Code(code) => {
                            html.push_str("<code>");
                            html.push_str(&clean_html(&code));
                            html.push_str("</code>");
                        }
                        _ => {}
                    }
                } else if !in_codeblock {
                    non_heading_events.push(event);
                }
            }
        }
    }

    // Flush remaining non-heading events
    if !non_heading_events.is_empty() {
        pulldown_cmark::html::push_html(&mut html, non_heading_events.into_iter());
    }

    RenderedContent {
        html: crate::utils::html_minify::minify_html(&clean_html(&html)),
        toc_html: crate::utils::html_minify::minify_html(&toc_html),
    }
}

#[cfg(feature = "server")]
/// 根据标题层级生成嵌套目录 HTML。
fn generate_toc_html(headings: &[(u8, String, String)]) -> String {
    if headings.is_empty() {
        return String::new();
    }

    let mut html = String::from("<ul>");
    let mut stack: Vec<u8> = vec![headings[0].0];

    for (i, (level, text, id)) in headings.iter().enumerate() {
        let level = *level;

        if i > 0 {
            let prev_level = headings[i - 1].0;
            if level > prev_level {
                // 标题层级升高：打开新的嵌套列表。
                for _ in prev_level..level {
                    html.push_str("<ul>");
                    stack.push(level);
                }
            } else if level < prev_level {
                // 标题层级降低：关闭多余的嵌套列表。
                while let Some(top) = stack.last() {
                    if *top > level {
                        html.push_str("</li></ul>");
                        stack.pop();
                    } else {
                        break;
                    }
                }
                html.push_str("</li>");
            } else {
                html.push_str("</li>");
            }
        }

        // 标题 text 是 pulldown-cmark 收集的纯文本（Text/Code 字面字符），不是 HTML 片段，
        // 因此正文与属性两处都走 escape_heading_text（转义 & < > " '）。原先用 clean_html
        // 处理属性上下文会漏掉 `"`，标题中的双引号会越出 aria-label 边界。
        let escaped_text = escape_heading_text(text);
        html.push_str(&format!(
            "<li><a href=\"#{}\" aria-label=\"{}\">{}</a>",
            id, escaped_text, escaped_text
        ));
    }

    // Close remaining lists
    // 闭合所有残留的嵌套列表。
    while stack.len() > 1 {
        html.push_str("</li></ul>");
        stack.pop();
    }
    html.push_str("</li></ul>");

    html
}

#[cfg(feature = "server")]
/// 将标题文本转换为可用于锚点的 slug。
fn slugify_heading(text: &str) -> String {
    let mut slug = String::new();
    let mut prev_dash = true;

    for c in text.to_lowercase().chars() {
        if c.is_alphanumeric() {
            slug.push(c);
            prev_dash = false;
        } else if !prev_dash {
            slug.push('-');
            prev_dash = true;
        }
    }

    if slug.ends_with('-') {
        slug.pop();
    }

    if slug.is_empty() {
        slug.push_str("heading");
    }

    slug
}

#[cfg(all(test, feature = "server"))]
mod tests {
    use super::*;

    #[test]
    fn slugify_heading_simple() {
        assert_eq!(slugify_heading("Hello World"), "hello-world");
    }

    #[test]
    fn slugify_heading_special_chars() {
        assert_eq!(slugify_heading("What's new? (2024)"), "what-s-new-2024");
    }

    #[test]
    fn slugify_heading_chinese() {
        let slug = slugify_heading("你好世界");
        assert!(!slug.is_empty());
    }

    #[test]
    fn slugify_heading_collapses_dashes() {
        assert_eq!(slugify_heading("a--b"), "a-b");
    }

    #[test]
    fn slugify_heading_strips_trailing_dash() {
        assert_eq!(slugify_heading("hello!"), "hello");
    }

    #[test]
    fn slugify_heading_empty_returns_heading() {
        assert_eq!(slugify_heading(""), "heading");
    }

    #[test]
    fn clean_html_allows_safe_tags() {
        let input = "<p>Hello <strong>world</strong></p>";
        assert_eq!(clean_html(input), input);
    }

    #[test]
    fn clean_html_removes_script() {
        let input = "<script>alert('xss')</script><p>safe</p>";
        let result = clean_html(input);
        assert!(!result.contains("script"));
        assert!(result.contains("safe"));
    }

    #[test]
    fn clean_html_allows_id_attribute() {
        let input = "<h2 id=\"my-heading\">Title</h2>";
        let result = clean_html(input);
        assert!(result.contains("id=\"my-heading\""));
    }

    #[test]
    fn clean_html_allows_class_attribute() {
        let input = "<span class=\"highlight\">text</span>";
        let result = clean_html(input);
        assert!(result.contains("class=\"highlight\""));
    }

    #[test]
    fn generate_toc_html_empty() {
        assert_eq!(generate_toc_html(&[]), "");
    }

    #[test]
    fn generate_toc_html_single_heading() {
        let headings = vec![(2u8, "Title".to_string(), "title".to_string())];
        let html = generate_toc_html(&headings);
        assert!(html.contains("href=\"#title\""));
        assert!(html.contains("<ul>"));
        assert!(html.contains("</ul>"));
    }

    #[test]
    fn generate_toc_html_nested() {
        let headings = vec![
            (2u8, "Chapter".to_string(), "chapter".to_string()),
            (3u8, "Section".to_string(), "section".to_string()),
        ];
        let html = generate_toc_html(&headings);
        assert!(html.contains("href=\"#chapter\""));
        assert!(html.contains("href=\"#section\""));
        let ul_count = html.matches("<ul>").count();
        assert_eq!(ul_count, 2);
    }

    #[test]
    fn generate_toc_html_escapes_quote_in_attr() {
        // 标题中的双引号不得越出 aria-label 属性边界。
        let headings = vec![(
            2u8,
            "\" onmouseover=\"alert(1)".to_string(),
            "heading".to_string(),
        )];
        let html = generate_toc_html(&headings);
        // aria-label 中的双引号被转义为 &quot;，无法越出属性边界注入新属性。
        assert!(
            html.contains("aria-label=\"&quot; onmouseover=&quot;alert(1)\""),
            "aria-label 应转义内部双引号，got: {html}"
        );
        // 关键：不得出现「未被引号包裹、可被解析为真实属性」的 onmouseover= 片段。
        // 正文中作为纯文本出现 "onmouseover" 字符串是安全的（无 < 或属性结构）。
        let attr_injection = "\" onmouseover=\"";
        let injected = html.matches(attr_injection).count();
        // 原始输入里有 1 个裸双引号起头；转义后该模式不应再作为属性边界出现。
        // 注意 aria-label 内部的双引号已变成 &quot;，因此裸的 `" onmouseover="` 不应存在。
        assert_eq!(
            injected, 0,
            "不应存在未转义的属性边界 `\" onmouseover=\"`，got: {html}"
        );
    }

    #[test]
    fn generate_toc_html_escapes_ampersand_in_attr() {
        let headings = vec![(2u8, "A & B".to_string(), "heading".to_string())];
        let html = generate_toc_html(&headings);
        assert!(
            html.contains("aria-label=\"A &amp; B\""),
            "& 应在属性中转义，got: {html}"
        );
    }

    #[test]
    fn generate_toc_html_escapes_less_than_in_attr() {
        // `<` 在属性与正文中都应被转义，避免被误解析为标签起始。
        let headings = vec![(2u8, "a < b".to_string(), "heading".to_string())];
        let html = generate_toc_html(&headings);
        assert!(
            html.contains("aria-label=\"a &lt; b\""),
            "< 应在属性中转义，got: {html}"
        );
        assert!(!html.contains("a < b"));
    }

    #[test]
    fn render_markdown_simple_paragraph() {
        let result = render_markdown_enhanced("Hello **world**");
        assert!(result.html.contains("<strong>world</strong>"));
        assert!(result.toc_html.is_empty());
    }

    #[test]
    fn render_markdown_with_heading_generates_toc() {
        let result = render_markdown_enhanced("## My Heading\n\nSome text.");
        assert!(result.toc_html.contains("My Heading"));
        assert!(result.html.contains("id=\"my-heading\""));
    }

    #[test]
    fn render_markdown_empty() {
        let result = render_markdown_enhanced("");
        assert_eq!(result.html, "");
        assert_eq!(result.toc_html, "");
    }

    #[test]
    fn render_markdown_code_block() {
        let result = render_markdown_enhanced("```rust\nfn main() {}\n```");
        assert!(result.html.contains(r#"<pre><code class="language-rust">"#));
        assert!(result
            .html
            .contains(r#"<span class="entity name function rust">main</span>"#));
        assert!(result
            .html
            .contains(r#"<span class="storage type function rust">fn</span>"#));
    }

    #[test]
    fn render_markdown_code_block_without_language() {
        let result = render_markdown_enhanced("```\nplain text\n```");
        assert!(result.html.contains("<pre><code>"));
        assert!(!result.html.contains("class=\"language-"));
        assert!(result.html.contains("plain text"));
    }

    #[test]
    fn render_markdown_code_block_preserves_html_content() {
        let result = render_markdown_enhanced("```html\n<script>alert(1)</script>\n```");
        assert!(result.html.contains("<pre><code class=\"language-html\">"));
        assert!(!result.html.contains("<script>"));
        assert!(result
            .html
            .contains(r#"<span class="variable function js">alert</span>"#));
        assert!(result
            .html
            .contains(r#"<span class="constant numeric js">1</span>"#));
    }

    #[test]
    fn render_markdown_data_uri_image() {
        let result = render_markdown_enhanced("![alt](data:image/svg+xml,%3csvg%3e%3c/svg%3e)");
        assert!(
            result.html.contains("data:image/svg+xml"),
            "data URI should be preserved in img src, got: {}",
            result.html
        );
        assert!(result.html.contains("alt=\"alt\""));
    }
}
