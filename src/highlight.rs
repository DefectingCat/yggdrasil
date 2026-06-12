#[cfg(feature = "server")]
pub mod server {
    use std::sync::LazyLock;

    use syntect::html::{ClassStyle, ClassedHTMLGenerator};
    use syntect::parsing::SyntaxSet;
    use syntect::util::LinesWithEndings;

    static SYNTAX_SET: LazyLock<SyntaxSet> = LazyLock::new(|| {
        let mut builder = SyntaxSet::load_defaults_newlines().into_builder();
        if let Err(e) = builder.add_from_folder("syntaxes/", true) {
            tracing::warn!("Failed to load custom syntaxes: {:?}", e);
        }
        builder.build()
    });

    fn find_syntax(lang: Option<&str>) -> &'static syntect::parsing::SyntaxReference {
        let ss = &*SYNTAX_SET;
        if let Some(lang) = lang {
            if !lang.is_empty() {
                if let Some(s) = ss.find_syntax_by_extension(lang) {
                    return s;
                }
                if let Some(s) = ss.find_syntax_by_name(lang) {
                    return s;
                }
                let lower = lang.to_lowercase();
                if lower != lang {
                    if let Some(s) = ss.find_syntax_by_extension(&lower) {
                        return s;
                    }
                    if let Some(s) = ss.find_syntax_by_name(&lower) {
                        return s;
                    }
                }
                let aliases: &[(&str, &str)] = &[
                    ("rust", "rs"),
                    ("js", "js"),
                    ("javascript", "js"),
                    ("ts", "js"),
                    ("typescript", "js"),
                    ("tsx", "js"),
                    ("py", "py"),
                    ("python", "py"),
                    ("rb", "rb"),
                    ("ruby", "rb"),
                    ("sh", "sh"),
                    ("bash", "sh"),
                    ("yaml", "yaml"),
                    ("yml", "yaml"),
                    ("md", "md"),
                    ("markdown", "md"),
                    ("kotlin", "kt"),
                    ("swift", "swift"),
                    ("golang", "go"),
                ];
                for &(from, to) in aliases {
                    if lang == from {
                        if let Some(s) = ss.find_syntax_by_extension(to) {
                            return s;
                        }
                    }
                }
            }
        }
        ss.find_syntax_by_extension("txt")
            .or_else(|| ss.find_syntax_by_name("Plain Text"))
            .expect("no plain text syntax")
    }

    pub fn highlight_code(code: &str, lang: Option<&str>) -> String {
        let trimmed = code.trim();
        let syntax = find_syntax(lang);
        let ss = &*SYNTAX_SET;
        let mut generator =
            ClassedHTMLGenerator::new_with_class_style(syntax, ss, ClassStyle::Spaced);

        for line in LinesWithEndings::from(trimmed) {
            if let Err(e) = generator.parse_html_for_line_which_includes_newline(line) {
                tracing::warn!("syntect parse error: {:?}", e);
            }
        }

        generator.finalize()
    }
}

#[cfg(all(test, feature = "server"))]
mod tests {
    use super::server::*;

    #[test]
    fn highlight_code_rust() {
        let result = highlight_code("fn main() {}", Some("rust"));
        assert!(result.contains(r#"<span class="storage type function rust">fn</span>"#));
        assert!(result.contains(r#"<span class="entity name function rust">main</span>"#));
    }

    #[test]
    fn highlight_code_javascript_alias() {
        let result = highlight_code("console.log('hi')", Some("js"));
        assert!(result.contains(r#"<span class="support type object console js">console</span>"#));
        assert!(result.contains(r#"<span class="support function console js">log</span>"#));
    }

    #[test]
    fn highlight_code_python_alias() {
        let result = highlight_code("print('hi')", Some("python"));
        assert!(result.contains(r#"<span class="support function builtin python">print</span>"#));
    }

    #[test]
    fn highlight_code_unknown_language() {
        let result = highlight_code("some text", Some("brainfuck"));
        assert!(result.contains(r#"<span class="text plain">some text</span>"#));
    }

    #[test]
    fn highlight_code_none_language() {
        let result = highlight_code("plain text", None);
        assert!(result.contains(r#"<span class="text plain">plain text</span>"#));
    }

    #[test]
    fn highlight_code_empty() {
        let result = highlight_code("", None);
        assert!(result.is_empty());
    }

    #[test]
    fn highlight_code_produces_span_tags() {
        let result = highlight_code("let x = 1;", Some("rust"));
        assert!(result.contains(r#"<span class="storage type rust">let</span>"#));
        assert!(result.contains(r#"<span class="constant numeric integer decimal rust">1</span>"#));
    }
}
