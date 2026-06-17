//! HTML 消毒器。
//!
//! 基于 lol_html 清理不受信任的 HTML，限制允许的 tag/attribute/URL scheme，
//! 分别提供文章正文（`clean_html`）与评论（`clean_comment_html`）两套白名单策略。
//! 仅在 `feature = "server"` 时执行。

#![allow(clippy::unused_unit, deprecated)]

#[cfg(feature = "server")]
use std::collections::HashSet;

#[cfg(feature = "server")]
fn default_allowed_tags() -> HashSet<&'static str> {
    let mut set = HashSet::new();
    for tag in [
        "a",
        "abbr",
        "acronym",
        "area",
        "article",
        "aside",
        "b",
        "bdi",
        "bdo",
        "blockquote",
        "br",
        "caption",
        "center",
        "cite",
        "code",
        "col",
        "colgroup",
        "data",
        "dd",
        "del",
        "details",
        "dfn",
        "div",
        "dl",
        "dt",
        "em",
        "figcaption",
        "figure",
        "footer",
        "h1",
        "h2",
        "h3",
        "h4",
        "h5",
        "h6",
        "header",
        "hgroup",
        "hr",
        "i",
        "img",
        "ins",
        "kbd",
        "li",
        "map",
        "mark",
        "nav",
        "ol",
        "p",
        "pre",
        "q",
        "rp",
        "rt",
        "rtc",
        "ruby",
        "s",
        "samp",
        "small",
        "span",
        "strike",
        "strong",
        "sub",
        "summary",
        "sup",
        "table",
        "tbody",
        "td",
        "th",
        "thead",
        "time",
        "tr",
        "tt",
        "u",
        "ul",
        "var",
        "wbr",
    ] {
        set.insert(tag);
    }
    set
}

#[cfg(feature = "server")]
fn clean_content_tags() -> HashSet<&'static str> {
    let mut set = HashSet::new();
    set.insert("script");
    set.insert("style");
    set
}

#[cfg(feature = "server")]
fn default_allowed_schemes() -> HashSet<&'static str> {
    let mut set = HashSet::new();
    for scheme in [
        "bitcoin",
        "ftp",
        "ftps",
        "geo",
        "http",
        "https",
        "im",
        "irc",
        "ircs",
        "magnet",
        "mailto",
        "mms",
        "mx",
        "news",
        "nntp",
        "openpgp4fpr",
        "sip",
        "sms",
        "smsto",
        "ssh",
        "tel",
        "url",
        "webcal",
        "wtai",
        "xmpp",
    ] {
        set.insert(scheme);
    }
    set
}

#[cfg(feature = "server")]
fn is_safe_data_uri(url: &str) -> bool {
    // data URI 只允许安全的图片类型；禁止 data:text/html、data:application/javascript 等。
    let url = url.trim();
    let Some(rest) = url.strip_prefix("data:") else {
        return false;
    };
    let media_type = rest.split(',').next().unwrap_or("");
    let media_type = media_type.split(';').next().unwrap_or("").trim();
    matches!(
        media_type.to_lowercase().as_str(),
        "image/png"
            | "image/jpeg"
            | "image/jpg"
            | "image/gif"
            | "image/webp"
            | "image/avif"
            | "image/bmp"
            | "image/tiff"
            | "image/svg+xml"
    )
}

#[cfg(feature = "server")]
fn is_safe_url(url: &str, allowed_schemes: &HashSet<&str>, allow_data_uri: bool) -> bool {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return true;
    }
    // 解析 scheme 并与白名单对比；未知 scheme 默认拒绝。
    if let Some(colon_pos) = trimmed.find(':') {
        let scheme = &trimmed[..colon_pos];
        let scheme_lower = scheme.to_lowercase();
        if scheme_lower == "javascript" || scheme_lower == "vbscript" {
            return false;
        }
        if scheme.contains(|c: char| c.is_ascii_whitespace()) {
            return false;
        }
        if allowed_schemes.contains(scheme_lower.as_str()) {
            return true;
        }
        if scheme_lower == "data" {
            return allow_data_uri && is_safe_data_uri(trimmed);
        }
        // 任何其它 scheme 均拒绝：file://、blob://、about:blank 等。
        return false;
    }
    // 无 scheme 时只允许相对路径与锚点。
    trimmed.starts_with('#') || trimmed.starts_with('/')
}

#[cfg(feature = "server")]
/// HTML 消毒配置：白名单 tag/attribute、允许 URL scheme 与链接 rel。
struct SanitizerConfig {
    allowed_tags: HashSet<&'static str>,
    extra_generic_attrs: Vec<&'static str>,
    extra_tag_attrs: Vec<(&'static str, Vec<&'static str>)>,
    allowed_schemes: HashSet<&'static str>,
    allow_data_uri: bool,
    link_rel: Option<&'static str>,
    remove_tags: HashSet<&'static str>,
}

#[cfg(feature = "server")]
fn sanitize(input: &str, config: &SanitizerConfig) -> String {
    let allowed_tags = config.allowed_tags.clone();
    let remove_tags = config.remove_tags.clone();
    let generic_attrs: HashSet<&str> = config
        .extra_generic_attrs
        .iter()
        .copied()
        .chain(["lang", "title"])
        .collect();
    let tag_attrs_map: std::collections::HashMap<&str, HashSet<&str>> = {
        let mut m = std::collections::HashMap::new();
        let base = [
            ("a", vec!["href", "hreflang"]),
            ("bdo", vec!["dir"]),
            ("blockquote", vec!["cite"]),
            ("col", vec!["align", "char", "charoff", "span"]),
            ("colgroup", vec!["align", "char", "charoff", "span"]),
            ("del", vec!["cite", "datetime"]),
            ("hr", vec!["align", "size", "width"]),
            ("img", vec!["align", "alt", "height", "src", "width"]),
            ("ins", vec!["cite", "datetime"]),
            ("ol", vec!["start"]),
            ("q", vec!["cite"]),
            ("table", vec!["align", "char", "charoff", "summary"]),
            ("tbody", vec!["align", "char", "charoff"]),
            (
                "td",
                vec!["align", "char", "charoff", "colspan", "headers", "rowspan"],
            ),
            ("tfoot", vec!["align", "char", "charoff"]),
            (
                "th",
                vec![
                    "align", "char", "charoff", "colspan", "headers", "rowspan", "scope",
                ],
            ),
            ("thead", vec!["align", "char", "charoff"]),
            ("tr", vec!["align", "char", "charoff"]),
        ];
        for (tag, attrs) in &base {
            m.insert(*tag, attrs.iter().copied().collect());
        }
        for (tag, attrs) in &config.extra_tag_attrs {
            m.entry(tag)
                .or_insert_with(HashSet::new)
                .extend(attrs.iter().copied());
        }
        m
    };
    let allowed_schemes = config.allowed_schemes.clone();
    let allow_data_uri = config.allow_data_uri;
    let link_rel = config.link_rel;

    let element_handler = move |el: &mut lol_html::html_content::Element| {
        let tag = el.tag_name().to_lowercase();

        if remove_tags.contains(tag.as_str()) {
            el.remove();
            return Ok(());
        }

        if !allowed_tags.contains(tag.as_str()) {
            el.remove_and_keep_content();
            return Ok(());
        }

        let allowed_for_tag: HashSet<&str> = {
            let mut s = generic_attrs.clone();
            if let Some(tag_specific) = tag_attrs_map.get(tag.as_str()) {
                s.extend(tag_specific.iter().copied());
            }
            s
        };

        let attrs_to_remove: Vec<String> = el
            .attributes()
            .iter()
            .filter_map(|attr| {
                let name = attr.name();
                let name_lower = name.to_lowercase();
                // 仅保留白名单属性；对 href/src/cite 额外校验 URL 安全性。
                if allowed_for_tag.contains(name_lower.as_str()) {
                    if name_lower == "href" || name_lower == "src" || name_lower == "cite" {
                        let val = attr.value();
                        if !is_safe_url(&val, &allowed_schemes, allow_data_uri) {
                            return Some(name);
                        }
                    }
                    None
                } else {
                    Some(name)
                }
            })
            .collect();

        for attr_name in attrs_to_remove {
            el.remove_attribute(&attr_name);
        }

        if link_rel.is_some() && tag == "a" {
            if let Some(rel) = link_rel {
                let existing = el.get_attribute("rel").unwrap_or_default();
                if existing != rel {
                    el.set_attribute("rel", rel).ok();
                }
            }
        }

        Ok(())
    };

    lol_html::rewrite_str(
        input,
        lol_html::RewriteStrSettings {
            element_content_handlers: vec![lol_html::element!("*", element_handler)],
            document_content_handlers: vec![lol_html::doc_comments!(|c| {
                c.remove();
                Ok(())
            })],
            ..lol_html::RewriteStrSettings::new()
        },
    )
    .unwrap_or_default()
}

#[cfg(feature = "server")]
/// 文章正文 HTML 清理：允许较完整的标签与 data URI，外链添加 `noopener noreferrer`。
pub fn clean_html(input: &str) -> String {
    let config = SanitizerConfig {
        allowed_tags: default_allowed_tags(),
        extra_generic_attrs: vec![
            "class",
            "aria-hidden",
            "aria-label",
            "id",
            "role",
            "accesskey",
            "title",
        ],
        extra_tag_attrs: vec![
            ("a", vec!["class", "aria-hidden", "aria-label"]),
            ("span", vec!["class"]),
            ("h1", vec!["id", "class"]),
            ("h2", vec!["id", "class"]),
            ("h3", vec!["id", "class"]),
            ("h4", vec!["id", "class"]),
            ("h5", vec!["id", "class"]),
            ("h6", vec!["id", "class"]),
        ],
        allowed_schemes: default_allowed_schemes(),
        allow_data_uri: false,
        link_rel: Some("noopener noreferrer"),
        remove_tags: clean_content_tags(),
    };
    sanitize(input, &config)
}

#[cfg(feature = "server")]
/// 评论 HTML 清理：移除图片与折叠块，禁用 data URI，外链添加 `nofollow noopener`。
pub fn clean_comment_html(input: &str) -> String {
    let mut tags = default_allowed_tags();
    tags.remove("img");
    tags.remove("details");
    tags.remove("summary");

    let config = SanitizerConfig {
        allowed_tags: tags,
        extra_generic_attrs: vec![
            "class",
            "title",
            "aria-hidden",
            "aria-label",
            "role",
            "accesskey",
        ],
        extra_tag_attrs: vec![
            ("a", vec!["class", "aria-hidden", "aria-label"]),
            ("span", vec!["class"]),
        ],
        allowed_schemes: default_allowed_schemes(),
        allow_data_uri: false,
        link_rel: Some("nofollow noopener"),
        remove_tags: clean_content_tags(),
    };
    sanitize(input, &config)
}

#[cfg(all(test, feature = "server"))]
mod tests {
    use super::*;

    #[test]
    fn safe_tags_preserved() {
        assert_eq!(clean_html("<p>safe</p>"), "<p>safe</p>");
        assert_eq!(
            clean_html("<p><strong>bold</strong></p>"),
            "<p><strong>bold</strong></p>"
        );
    }

    #[test]
    fn script_and_style_removed() {
        assert_eq!(
            clean_html("<script>alert(1)</script><style>.x{}</style><p>ok</p>"),
            "<p>ok</p>"
        );
    }

    #[test]
    fn id_and_class_preserved() {
        assert_eq!(
            clean_html("<h1 id=\"toc\" class=\"title\">x</h1>"),
            "<h1 id=\"toc\" class=\"title\">x</h1>"
        );
        assert_eq!(
            clean_html("<p id=\"note\" class=\"hint\">x</p>"),
            "<p id=\"note\" class=\"hint\">x</p>"
        );
    }

    #[test]
    fn javascript_url_stripped() {
        assert_eq!(
            clean_html("<a href=\"javascript:alert(1)\">x</a>"),
            "<a rel=\"noopener noreferrer\">x</a>"
        );
    }

    #[test]
    fn vbscript_url_stripped() {
        assert_eq!(
            clean_html("<a href=\"vbscript:msgbox\">x</a>"),
            "<a rel=\"noopener noreferrer\">x</a>"
        );
    }

    #[test]
    fn unknown_tags_removed_content_kept() {
        assert_eq!(clean_html("<custom>keep me</custom>"), "keep me");
    }

    #[test]
    fn comment_removes_img_details_summary() {
        assert_eq!(
            clean_comment_html("<img src=\"x\"><details><summary>sum</summary>body</details>"),
            "sumbody"
        );
    }

    #[test]
    fn comment_removes_data_uris() {
        assert_eq!(
            clean_comment_html("<a href=\"data:text/html,hi\">x</a>"),
            "<a rel=\"nofollow noopener\">x</a>"
        );
    }

    // ---- is_safe_url 直接分支测试 ----
    // is_safe_url 是安全敏感的内部函数，以下测试锁定其各分支的行为契约。

    #[test]
    fn is_safe_url_allows_https() {
        let schemes = default_allowed_schemes();
        assert!(is_safe_url("https://example.com", &schemes, false));
        assert!(is_safe_url("http://example.com", &schemes, false));
    }

    #[test]
    fn is_safe_url_rejects_javascript() {
        let schemes = default_allowed_schemes();
        assert!(!is_safe_url("javascript:alert(1)", &schemes, false));
    }

    #[test]
    fn is_safe_url_rejects_vbscript() {
        let schemes = default_allowed_schemes();
        assert!(!is_safe_url("vbscript:msgbox", &schemes, false));
    }

    #[test]
    fn is_safe_url_data_uri_respects_flag_and_media_type() {
        let schemes = default_allowed_schemes();
        // 仅在显式允许且 media type 为图片时通过
        assert!(is_safe_url("data:image/png;base64,iVBOR", &schemes, true));
        assert!(is_safe_url("data:image/svg+xml;base64,PHN2Zz4=", &schemes, true));
        // 禁用 data URI 时拒绝
        assert!(!is_safe_url("data:image/png;base64,iVBOR", &schemes, false));
        // 非图片 data URI 拒绝
        assert!(!is_safe_url("data:text/html,<script>alert(1)</script>", &schemes, true));
        assert!(!is_safe_url("data:application/javascript,alert(1)", &schemes, true));
    }

    #[test]
    fn is_safe_url_allows_relative_and_fragment() {
        let schemes = default_allowed_schemes();
        // 绝对路径
        assert!(is_safe_url("/path/to/page", &schemes, false));
        // 锚点
        assert!(is_safe_url("#section", &schemes, false));
    }

    #[test]
    fn is_safe_url_empty_is_safe() {
        let schemes = default_allowed_schemes();
        // 空 URL（如 img 无 src）视为安全。
        assert!(is_safe_url("", &schemes, false));
        assert!(is_safe_url("   ", &schemes, false));
    }

    #[test]
    fn is_safe_url_allows_other_whitelisted_schemes() {
        let schemes = default_allowed_schemes();
        // mailto / tel / ftp 等均在默认白名单中。
        assert!(is_safe_url("mailto:user@example.com", &schemes, false));
        assert!(is_safe_url("tel:+8613800138000", &schemes, false));
        assert!(is_safe_url("ftp://example.com/file", &schemes, false));
    }

    #[test]
    fn is_safe_url_rejects_scheme_with_whitespace() {
        let schemes = default_allowed_schemes();
        // 含空格的 scheme 名是已知的混淆手法，应被拒绝。
        assert!(!is_safe_url("java\tscript:alert(1)", &schemes, false));
    }

    #[test]
    fn is_safe_url_rejects_unknown_schemes() {
        let schemes = default_allowed_schemes();
        // 未知 scheme 默认拒绝。
        assert!(!is_safe_url("file:///etc/passwd", &schemes, false));
        assert!(!is_safe_url("blob:https://example.com/abc", &schemes, false));
        assert!(!is_safe_url("about:blank", &schemes, false));
        assert!(!is_safe_url("custom-app://open", &schemes, false));
    }

    #[test]
    fn is_safe_url_scheme_matching_is_case_insensitive() {
        let schemes = default_allowed_schemes();
        // scheme 大小写不敏感：HTTPS 与 https 等价。
        assert!(is_safe_url("HTTPS://example.com", &schemes, false));
        assert!(!is_safe_url("JAVASCRIPT:alert(1)", &schemes, false));
    }
}
