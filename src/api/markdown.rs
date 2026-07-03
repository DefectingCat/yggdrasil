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
/// 复用 `utils::html::escape_html`（转义 `& < > " '`），避免在仓库内
/// 维护第二份转义实现。原先用 `clean_html` 处理属性上下文会漏掉 `"`，标题形如
/// `" onmouseover="alert(1)` 会越出属性边界。
fn escape_heading_text(s: &str) -> String {
    crate::utils::html::escape_html(s)
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

    // 两遍解析使用相同的 Options，避免 TOC 收集与正文渲染对 Markdown 扩展语法
    // （表格、删除线、脚注等）的处理不一致。
    let opts = Options::all();

    // 1. Parse markdown and collect headings for TOC
    let parser = pulldown_cmark::Parser::new_ext(md, opts);
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
    let parser = pulldown_cmark::Parser::new_ext(md, opts);
    let mut html = String::new();
    let mut heading_idx = 0;
    let mut in_heading = false;
    let mut in_codeblock = false;
    let mut code_lang: Option<String> = None;
    /// 可运行代码块的 (lang, html-escaped overrides JSON)；为 None 表示普通代码块。
    let mut code_runnable: Option<(String, String)> = None;
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
                // 解析围栏 info：识别 `runnable` 标记与可选 ResourceLimits JSON 覆盖。
                // 仅在「标记为 runnable 且语言受支持」时挂 data-*，供阅读器扫描挂载运行器。
                code_runnable = code_lang
                    .as_deref()
                    .map(|info| {
                        let (lang, runnable, overrides) =
                            crate::api::code_runner::languages::parse_fence_info(info);
                        if runnable && crate::api::code_runner::languages::is_supported_lang(&lang) {
                            // overrides 序列化为 JSON 后 HTML 转义，避免属性注入。
                            let ov_json = overrides
                                .map(|o| serde_json::to_string(&o).unwrap_or_default())
                                .unwrap_or_default();
                            let escaped = crate::utils::html::escape_html(&ov_json);
                            Some((lang, escaped))
                        } else {
                            None
                        }
                    })
                    .flatten();
                code_buffer.clear();
            }
            Event::Text(text) if in_codeblock => {
                code_buffer.push_str(&text);
            }
            Event::End(TagEnd::CodeBlock) => {
                // 使用 syntect 对代码块进行服务端语法高亮。
                let highlighted =
                    crate::highlight::server::highlight_code(&code_buffer, code_lang.as_deref());
                // 可运行代码块：在 <pre> 上挂 data-runnable / data-lang / data-overrides，
                // 阅读器（post_content.rs）客户端扫描这些标记原地挂载 CodeRunner 组件。
                if let Some((lang, overrides_escaped)) = code_runnable.take() {
                    html.push_str(&format!(
                        r#"<pre data-runnable="true" data-lang="{lang}" data-overrides="{overrides_escaped}"><code class="language-{lang}">"#
                    ));
                } else {
                    html.push_str("<pre><code");
                    if let Some(lang) = &code_lang {
                        // 围栏语言可能含 info 修饰（如 `python runnable {...}`），
                        // 高亮的 language-xxx 取纯语言 token（首个空白前）。
                        let clean_lang = lang.split_whitespace().next().unwrap_or("");
                        if !clean_lang.is_empty() {
                            html.push_str(&format!(" class=\"language-{clean_lang}\""));
                        }
                    }
                    html.push('>');
                }
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

    let html = wrap_images_with_blur(&html);
    RenderedContent {
        html: clean_html(&html),
        toc_html,
    }
}

/// 把 HTML 里的 /uploads/ 图片转成 blur-up 双层结构。
///
/// 仅处理 src 以 /uploads/ 开头的 img；外链图保持原样。
/// 对每个匹配的 img：
/// 1. 提取 src，解析出 rel_path（去 /uploads/ 前缀和 query）
/// 2. 查 get_image_dimensions 拿真实宽高，算 --ar（如 "16:9"）
/// 3. 生成 `<span class="blur-img" style="--ar:..">` 包裹两层 img
#[cfg(feature = "server")]
fn wrap_images_with_blur(html: &str) -> String {
    wrap_images_with_blur_with(html, crate::api::image::get_image_dimensions)
}

/// `wrap_images_with_blur` 的纯函数核心，接受 dimensions 查询闭包以便单测。
///
/// `dims_fn(rel_path) -> Option<(u32, u32)>` 注入真实或测试用的 dimensions 来源，
/// 使本函数不依赖文件系统——测试可注入已知宽高，生产注入 get_image_dimensions。
#[cfg(feature = "server")]
fn wrap_images_with_blur_with<F>(html: &str, dims_fn: F) -> String
where
    F: Fn(&str) -> Option<(u32, u32)>,
{
    use regex::Regex;
    use std::sync::LazyLock;

    // 匹配 pulldown-cmark 产出的 <img src="..." alt="..." /> 或 <img src="..." alt="...">
    // pulldown-cmark 格式可控：src 在前，alt 在后，属性用双引号
    static IMG_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"<img\s+src="(/uploads/[^"]+)"(?:\s+alt="([^"]*)")?\s*/?>"#).unwrap()
    });

    IMG_RE
        .replace_all(html, |caps: &regex::Captures| {
            let src = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            let alt = caps.get(2).map(|m| m.as_str()).unwrap_or("");

            // 从 src 解析 rel_path：去 /uploads/ 前缀 + 去 query
            let rel_path = src
                .strip_prefix("/uploads/")
                .unwrap_or(src)
                .split('?')
                .next()
                .unwrap_or("");

            // 查 dimensions，算 aspect-ratio
            // 注意：CSS aspect-ratio 用斜杠分隔（width / height），不是冒号
            let ar_style = dims_fn(rel_path)
                .map(|(w, h)| format!(" style=\"--ar:{} / {};\"", w, h))
                .unwrap_or_default();

            // alt 转义（src/alt 来自 markdown，pulldown-cmark 已转义过，这里直接用）
            let alt_attr = if alt.is_empty() {
                String::new()
            } else {
                format!(" alt=\"{}\"", alt)
            };

            format!(
                "<span class=\"blur-img\"{ar}><img class=\"blur-img-placeholder\" src=\"{src}?w=20\"{alt_attr}><img class=\"blur-img-full\" data-src=\"{src}?w=800\"{alt_attr}></span>",
                ar = ar_style,
                src = src,
                alt_attr = alt_attr,
            )
        })
        .to_string()
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
    fn wrap_images_with_blur_wraps_uploads_image() {
        // 注入返回 None 的 dims_fn,验证 --ar 缺省时的结构正确性。
        // 不依赖 uploads/ 文件系统。
        let html = r#"<p><img src="/uploads/nonexistent/test.webp" alt="test"></p>"#;
        let result = wrap_images_with_blur_with(html, |_| None);
        assert!(
            result.contains("blur-img-placeholder"),
            "should have placeholder"
        );
        assert!(result.contains("blur-img-full"), "should have full layer");
        assert!(result.contains("?w=20"), "placeholder should use ?w=20");
        assert!(result.contains("?w=800"), "full should use ?w=800");
        assert!(result.contains("data-src"), "full should use data-src");
    }

    #[test]
    fn wrap_images_with_blur_skips_external_image() {
        // 外链图不进入 dims_fn,保持原样。
        let html = r#"<img src="https://example.com/img.png" alt="ext">"#;
        let result = wrap_images_with_blur_with(html, |_| None);
        assert!(
            !result.contains("blur-img"),
            "external image should not be wrapped"
        );
    }

    #[test]
    fn wrap_images_with_blur_uses_slash_in_aspect_ratio() {
        // 注入已知 dimensions,验证 --ar 用斜杠分隔(如 "--ar:800 / 600;")。
        // 此前依赖 uploads/ 真实文件,现已解耦。
        let html = r#"<img src="/uploads/2026/06/18/abc.webp" alt="t">"#;
        let result = wrap_images_with_blur_with(html, |_| Some((800, 600)));
        assert!(result.contains("--ar:"), "should have --ar");
        assert!(
            result.contains(" / "),
            "aspect-ratio must use slash separator, got: {}",
            result
        );
        assert!(result.contains("--ar:800 / 600;"), "应含精确宽高");
    }

    #[test]
    fn wrap_images_with_blur_omits_ar_when_no_dimensions() {
        // dims_fn 返回 None 时不输出 --ar,避免空 style 属性。
        let html = r#"<img src="/uploads/x.webp" alt="t">"#;
        let result = wrap_images_with_blur_with(html, |_| None);
        assert!(!result.contains("--ar"), "无 dimensions 不应有 --ar");
        // 但包裹结构仍应生成
        assert!(result.contains("blur-img"));
    }

    #[test]
    fn wrap_images_with_blur_strips_query_from_rel_path() {
        // rel_path 提取应去 query 后缀,dims_fn 收到的是去 query 的路径。
        // dims_fn 是 Fn(replace_all 要求),用 RefCell 捕获传入值。
        use std::cell::RefCell;
        let html = r#"<img src="/uploads/2026/x.webp?w=100" alt="t">"#;
        let received = RefCell::new(String::new());
        let _ = wrap_images_with_blur_with(html, |p| {
            *received.borrow_mut() = p.to_string();
            Some((100, 100))
        });
        assert_eq!(received.borrow().as_str(), "2026/x.webp", "rel_path 应去 query");
    }

    #[test]
    fn wrap_images_with_blur_preserves_alt() {
        let html = r#"<img src="/uploads/x.webp" alt="描述">"#;
        let result = wrap_images_with_blur_with(html, |_| Some((10, 10)));
        assert!(result.contains(r#"alt="描述""#), "placeholder 应保留 alt");
        assert!(
            result.matches(r#"alt="描述""#).count() == 2,
            "两层 img 都应带 alt"
        );
    }

    #[test]
    fn wrap_images_with_blur_omits_alt_attr_when_empty() {
        let html = r#"<img src="/uploads/x.webp">"#;
        let result = wrap_images_with_blur_with(html, |_| None);
        assert!(
            !result.contains("alt=\""),
            "无 alt 时不应生成空 alt 属性"
        );
    }

    #[test]
    fn full_pipeline_wrap_then_clean_preserves_slash() {
        // 模拟完整渲染管线:wrap → clean_html,验证 sanitizer 不破坏斜杠。
        // 注入确定 dimensions,脱离文件系统依赖。
        let html = r#"<img src="/uploads/2026/06/18/abc.webp" alt="t">"#;
        let wrapped = wrap_images_with_blur_with(html, |_| Some((800, 600)));
        let cleaned = clean_html(&wrapped);
        assert!(
            cleaned.contains(" / "),
            "clean_html must preserve slash in --ar, got: {}",
            cleaned
        );
    }

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
    fn render_markdown_runnable_block_emits_data_attrs() {
        // `python runnable` 围栏：pre 上挂 data-runnable / data-lang / data-overrides，
        // 阅读器据此原地挂载 CodeRunner 组件。
        let result = render_markdown_enhanced("```python runnable\nprint('hi')\n```");
        assert!(
            result.html.contains(r#"data-runnable="true""#),
            "应输出 data-runnable, got: {}",
            result.html
        );
        assert!(
            result.html.contains(r#"data-lang="python""#),
            "应输出 data-lang=python, got: {}",
            result.html
        );
        // 无 overrides 时 data-overrides 为空。
        assert!(
            result.html.contains(r#"data-overrides="""#),
            "无 overrides 时 data-overrides 应为空, got: {}",
            result.html
        );
        // 内部仍带高亮 code 与 language-python。
        assert!(
            result.html.contains(r#"<code class="language-python">"#),
            "应保留高亮 code, got: {}",
            result.html
        );
    }

    #[test]
    fn render_markdown_runnable_block_with_overrides() {
        let result = render_markdown_enhanced(
            r#"```node runnable {"timeout_secs":10,"memory_mb":512,"allow_network":false,"cpu_cores":1.0,"output_bytes":1024}
console.log(1)
```"#,
        );
        assert!(result.html.contains(r#"data-lang="node""#));
        // overrides JSON 应被 HTML 转义后放入属性（双引号变 &quot;），不得出现裸引号越界。
        // 字段顺序按 serde 派生默认（字母序），cpu_cores 在前。
        assert!(
            result.html.contains("data-overrides=\"{&quot;cpu_cores&quot;:1.0"),
            "overrides 应 HTML 转义, got: {}",
            result.html
        );
        // 安全：不得出现可越出属性边界的裸双引号 JSON。
        assert!(
            !result.html.contains(r#"data-overrides="{"timeout""#),
            "overrides 裸引号越界, got: {}",
            result.html
        );
    }

    #[test]
    fn render_markdown_runnable_marker_on_unsupported_lang_ignored() {
        // 语言不在白名单：runnable 标记被忽略，输出普通代码块。
        let result = render_markdown_enhanced("```rust runnable\nfn main(){}\n```");
        assert!(
            !result.html.contains("data-runnable"),
            "不支持的语言不应挂 data-runnable, got: {}",
            result.html
        );
    }

    #[test]
    fn render_markdown_plain_fence_without_runnable_no_data_attrs() {
        let result = render_markdown_enhanced("```python\nprint(1)\n```");
        assert!(!result.html.contains("data-runnable"));
        assert!(result.html.contains(r#"<pre><code class="language-python">"#));
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
    fn render_markdown_data_uri_image_removed() {
        let result = render_markdown_enhanced("![alt](data:image/svg+xml,%3csvg%3e%3c/svg%3e)");
        // 出于 XSS 防护，文章正文不再保留 data URI src。
        assert!(
            !result.html.contains("data:image/svg+xml"),
            "data URI should be removed from img src, got: {}",
            result.html
        );
        assert!(result.html.contains("alt=\"alt\""));
    }

    #[test]
    fn render_markdown_task_list() {
        // 端到端验证：pulldown-cmark 解析任务列表 → sanitizer 清理后 checkbox 不丢失。
        // 覆盖「编辑器写入 → 入库 → 服务端重渲染」链路的最终 HTML 形态。
        let result = render_markdown_enhanced("- [ ] 未完成\n- [x] 已完成\n");
        // 两个 checkbox 都应保留
        assert!(
            result.html.contains(r#"type="checkbox""#),
            "checkbox type 应保留, got: {}",
            result.html
        );
        // 已勾选项的 checked 属性应保留
        assert!(
            result.html.contains("checked"),
            "checked 属性应保留, got: {}",
            result.html
        );
        // 文本内容保留
        assert!(result.html.contains("未完成"));
        assert!(result.html.contains("已完成"));
    }
}
