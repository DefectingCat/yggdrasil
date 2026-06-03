#[cfg(feature = "server")]
pub mod server {
    use std::sync::{LazyLock, OnceLock};

    use syntect::highlighting::{Theme, ThemeSet};
    use syntect::html::{ClassedHTMLGenerator, ClassStyle};
    use syntect::parsing::{SyntaxReference, SyntaxSet};
    use syntect::util::LinesWithEndings;

    static SYNTAX_SET: LazyLock<SyntaxSet> =
        LazyLock::new(|| SyntaxSet::load_defaults_newlines());

    static LATTE_THEME: LazyLock<Theme> = LazyLock::new(|| {
        ThemeSet::get_theme("themes/Catppuccin Latte.tmTheme")
            .expect("Failed to load Catppuccin Latte theme")
    });

    static MOCHA_THEME: LazyLock<Theme> = LazyLock::new(|| {
        ThemeSet::get_theme("themes/Catppuccin Mocha.tmTheme")
            .expect("Failed to load Catppuccin Mocha theme")
    });

    static FALLBACK_SYNTAX: OnceLock<SyntaxReference> = OnceLock::new();

    fn find_syntax(lang: Option<&str>) -> &'static SyntaxReference {
        let ss = &*SYNTAX_SET;
        if let Some(lang) = lang {
            if !lang.is_empty() {
                if let Some(s) = ss.find_syntax_by_extension(lang) {
                    return s;
                }
                if let Some(s) = ss.find_syntax_by_name(lang) {
                    return s;
                }
            }
        }
        let plain = ss
            .find_syntax_by_extension("txt")
            .or_else(|| ss.find_syntax_by_name("Plain Text"))
            .expect("no plain text syntax");
        FALLBACK_SYNTAX.get_or_init(|| plain.clone())
    }

    pub fn highlight_code(code: &str, lang: Option<&str>) -> String {
        let syntax = find_syntax(lang);
        let ss = &*SYNTAX_SET;
        let mut generator =
            ClassedHTMLGenerator::new_with_class_style(syntax, ss, ClassStyle::Spaced);

        for line in LinesWithEndings::from(code) {
            let _ = generator.parse_html_for_line_which_includes_newline(line);
        }

        generator.finalize()
    }

    pub fn get_latte_theme() -> &'static Theme {
        &*LATTE_THEME
    }

    pub fn get_mocha_theme() -> &'static Theme {
        &*MOCHA_THEME
    }

    pub fn get_syntax_set() -> &'static SyntaxSet {
        &*SYNTAX_SET
    }
}
