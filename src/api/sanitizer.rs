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
fn is_safe_url(url: &str, allowed_schemes: &HashSet<&str>, allow_data_uri: bool) -> bool {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return true;
    }
    if let Some(colon_pos) = trimmed.find(':') {
        let scheme = &trimmed[..colon_pos];
        let scheme_lower = scheme.to_lowercase();
        if allowed_schemes.contains(scheme_lower.as_str()) {
            return true;
        }
        if scheme_lower == "data" {
            return allow_data_uri;
        }
        if scheme_lower == "javascript" || scheme_lower == "vbscript" {
            return false;
        }
        if scheme.contains(|c: char| c.is_ascii_whitespace()) {
            return false;
        }
    }
    if trimmed.starts_with('#') || trimmed.starts_with('/') {
        return true;
    }
    true
}

#[cfg(feature = "server")]
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
        allow_data_uri: true,
        link_rel: Some("noopener noreferrer"),
        remove_tags: clean_content_tags(),
    };
    sanitize(input, &config)
}

#[cfg(feature = "server")]
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
