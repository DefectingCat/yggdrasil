#![allow(clippy::unused_unit, deprecated, unused_imports)]

#[cfg(feature = "server")]
pub fn clean_html(input: &str) -> String {
    let mut builder = ammonia::Builder::default();
    builder
        .add_generic_attributes(&[
            "class",
            "aria-hidden",
            "aria-label",
            "id",
            "role",
            "accesskey",
            "title",
        ])
        .add_tags(&["details", "summary"])
        .url_relative(ammonia::UrlRelative::PassThrough)
        .add_tag_attributes("a", &["class", "aria-hidden", "aria-label"])
        .add_tag_attributes("span", &["class"])
        .add_tag_attributes("h1", &["id", "class"])
        .add_tag_attributes("h2", &["id", "class"])
        .add_tag_attributes("h3", &["id", "class"])
        .add_tag_attributes("h4", &["id", "class"])
        .add_tag_attributes("h5", &["id", "class"])
        .add_tag_attributes("h6", &["id", "class"]);

    builder.clean(input).to_string()
}

#[derive(Debug, Clone)]
#[cfg(feature = "server")]
pub struct RenderedContent {
    pub html: String,
    pub toc_html: String,
}

#[cfg(feature = "server")]
pub fn render_markdown_enhanced(md: &str) -> RenderedContent {
    use pulldown_cmark::{Event, HeadingLevel, Options, Tag, TagEnd};

    // 1. Parse markdown and collect headings for TOC
    let parser = pulldown_cmark::Parser::new_ext(md, Options::all());
    let mut headings: Vec<(u8, String, String)> = Vec::new(); // (level, text, id)
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
                let highlighted =
                    crate::highlight::server::highlight_code(&code_buffer, code_lang.as_deref());
                html.push_str("<pre><code>");
                html.push_str(&highlighted);
                html.push_str("</code></pre>");
                in_codeblock = false;
            }
            _ => {
                if in_heading {
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
        html: clean_html(&html),
        toc_html,
    }
}

#[cfg(feature = "server")]
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
                // Open new nested lists
                for _ in prev_level..level {
                    html.push_str("<ul>");
                    stack.push(level);
                }
            } else if level < prev_level {
                // Close nested lists
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

        html.push_str(&format!(
            "<li><a href=\"#{}\" aria-label=\"{}\">{}</a>",
            id,
            clean_html(text),
            clean_html(text)
        ));
    }

    // Close remaining lists
    while stack.len() > 1 {
        html.push_str("</li></ul>");
        stack.pop();
    }
    html.push_str("</li></ul>");

    html
}

#[cfg(feature = "server")]
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
