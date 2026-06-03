#[cfg(feature = "server")]
pub mod server {
    use std::sync::LazyLock;

    use syntect::html::{ClassedHTMLGenerator, ClassStyle};
    use syntect::parsing::SyntaxSet;
    use syntect::util::LinesWithEndings;

    static SYNTAX_SET: LazyLock<SyntaxSet> =
        LazyLock::new(SyntaxSet::load_defaults_newlines);

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
                    ("js", "javascript"),
                    ("ts", "typescript"),
                    ("py", "python"),
                    ("rb", "ruby"),
                    ("sh", "bash"),
                    ("yaml", "yml"),
                    ("md", "markdown"),
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
