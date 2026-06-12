//! 评论 Markdown 渲染与 HTML 清洗。
//!
//! 对评论内容做轻量 Markdown 解析，限制标签白名单并转义危险字符。
//! 仅在 `feature = "server"` 启用的服务端构建中实际执行渲染。

#![allow(clippy::unused_unit, deprecated)]

/// 转义 HTML 特殊字符，用于无语言信息的代码块。
#[cfg(feature = "server")]
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// 清洗评论 HTML，移除危险标签与属性。
///
/// 实际委托给 `crate::api::sanitizer::clean_comment_html` 实现。
#[cfg(feature = "server")]
pub fn clean_comment_html(input: &str) -> String {
    crate::api::sanitizer::clean_comment_html(input)
}

/// 将评论 Markdown 渲染为安全的 HTML。
///
/// 支持表格与删除线；标题统一渲染为 `<strong>` 以避免层级混乱；
/// 代码块若指定语言则调用服务端高亮，否则转义 HTML；
/// 最终调用 `clean_comment_html` 过滤危险内容。
#[cfg(feature = "server")]
pub fn render_comment_markdown(md: &str) -> String {
    use pulldown_cmark::{CodeBlockKind, Event, Options, Tag, TagEnd};

    let opts = Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH;
    let parser = pulldown_cmark::Parser::new_ext(md, opts);

    let mut events: Vec<Event> = Vec::new();
    let mut in_codeblock = false;
    let mut code_lang: Option<String> = None;
    let mut code_buffer = String::new();

    // 逐事件处理 Markdown AST，转换标题并收集代码块内容。
    for event in parser {
        match event {
            Event::Start(Tag::Heading { .. }) => {
                // 评论中不保留标题层级，统一加粗。
                events.push(Event::Start(Tag::Strong));
            }
            Event::End(TagEnd::Heading(_)) => {
                events.push(Event::End(TagEnd::Strong));
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                in_codeblock = true;
                code_lang = match kind {
                    CodeBlockKind::Fenced(lang) if !lang.is_empty() => Some(lang.to_string()),
                    _ => None,
                };
                code_buffer.clear();
            }
            Event::Text(text) if in_codeblock => {
                code_buffer.push_str(&text);
            }
            Event::End(TagEnd::CodeBlock) => {
                // 根据是否有语言信息决定高亮或转义。
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
            "# H1",
            "## H2",
            "### H3",
            "#### H4",
            "##### H5",
            "###### H6",
        ] {
            let result = render_comment_markdown(md);
            assert!(
                result.contains("<strong>"),
                "heading not converted for: {}",
                md
            );
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
    fn render_comment_link_javascript_removed() {
        let result = render_comment_markdown("[click](javascript:alert(1))");
        assert!(result.contains("click"));
        assert!(!result.contains("javascript:"));
    }

    #[test]
    fn render_comment_onerror_attribute_removed() {
        let result = render_comment_markdown("<div onerror=\"alert(1)\">text</div>");
        assert!(result.contains("text"));
        assert!(!result.contains("onerror"));
    }

    #[test]
    fn render_comment_link_data_uri_removed() {
        let result =
            render_comment_markdown("[click](data:text/html,<script>alert(1)</script>)");
        assert!(result.contains("click"));
        assert!(!result.contains("data:"));
    }

    #[test]
    fn render_comment_code_block_escapes_html_entities() {
        let result = render_comment_markdown("```\n&amp;\n```");
        assert!(result.contains("&amp;amp;"));
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
        let result =
            clean_comment_html("<details><summary>Click</summary><p>Content</p></details>");
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
