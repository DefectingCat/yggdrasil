//! 语法高亮模块。
//!
//! 仅在 `server` feature 启用时可用，使用 `syntect` 将代码块转换为带 CSS class 的 HTML，
//! 配合 `public/highlight.css` 中生成的主题规则实现亮/暗主题高亮。

#[cfg(feature = "server")]
pub mod server {
    use std::sync::LazyLock;

    use syntect::html::{ClassStyle, ClassedHTMLGenerator};
    use syntect::parsing::SyntaxSet;
    use syntect::util::LinesWithEndings;

    /// 全局语法集合，懒加载时合并内置语法与 `syntaxes/` 目录下的自定义语法。
    static SYNTAX_SET: LazyLock<SyntaxSet> = LazyLock::new(|| {
        let mut builder = SyntaxSet::load_defaults_newlines().into_builder();
        // 使用 CARGO_MANIFEST_DIR 派生的绝对路径，避免运行时工作目录不确定导致加载失败
        let syntaxes_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/syntaxes");
        tracing::info!("Loading custom syntaxes from: {}", syntaxes_dir);
        match builder.add_from_folder(syntaxes_dir, true) {
            Ok(()) => tracing::info!("Custom syntaxes loaded successfully"),
            Err(e) => tracing::warn!("Failed to load custom syntaxes: {:?}", e),
        }
        let built = builder.build();
        tracing::info!(
            "SyntaxSet built: {} syntaxes, swift={:?}",
            built.syntaxes().len(),
            built
                .find_syntax_by_extension("swift")
                .map(|s| &s.name)
        );
        built
    });

    /// 根据语言标识查找对应的语法定义。
    ///
    /// 依次尝试：扩展名、语法名称、小写扩展名/名称、常用别名映射。
    /// 如果全部失败，则回退到纯文本语法。
    fn find_syntax(lang: Option<&str>) -> &'static syntect::parsing::SyntaxReference {
        let ss = &*SYNTAX_SET;
        if let Some(lang) = lang {
            if !lang.is_empty() {
                // 尝试按扩展名匹配
                if let Some(s) = ss.find_syntax_by_extension(lang) {
                    return s;
                }
                // 尝试按语法名称匹配
                if let Some(s) = ss.find_syntax_by_name(lang) {
                    return s;
                }
                // 小写后再匹配一次
                let lower = lang.to_lowercase();
                if lower != lang {
                    if let Some(s) = ss.find_syntax_by_extension(&lower) {
                        return s;
                    }
                    if let Some(s) = ss.find_syntax_by_name(&lower) {
                        return s;
                    }
                }
                // 常用语言别名映射表
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
                    // 别名比较同样不区分大小写，保证 "RUST" 与 "rust" 等价。
                    if lang.eq_ignore_ascii_case(from) {
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

    /// 对给定代码字符串按指定语言进行高亮，返回 HTML 字符串。
    ///
    /// 输出使用 spaced CSS class 风格，便于与 `highlight.css` 中的选择器匹配。
    pub fn highlight_code(code: &str, lang: Option<&str>) -> String {
        let trimmed = code.trim();
        let syntax = find_syntax(lang);
        let ss = &*SYNTAX_SET;
        let mut generator =
            ClassedHTMLGenerator::new_with_class_style(syntax, ss, ClassStyle::Spaced);

        // 逐行解析，出错时记录警告并继续
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

    #[test]
    fn highlight_code_uppercase_language_falls_back_via_lowercase() {
        // 大写语言标识应通过小写回退路径匹配到对应语法。
        let lower = highlight_code("fn main() {}", Some("rust"));
        let upper = highlight_code("fn main() {}", Some("RUST"));
        // 大写标识的输出必须与小写标识完全一致，证明回退路径生效。
        assert_eq!(lower, upper);
        assert!(lower.contains(r#"<span class="storage type function rust">fn</span>"#));
    }

    #[test]
    fn highlight_code_resolves_golang_alias() {
        // 别名表中 "golang" 映射到 "go" 扩展名，输出应与直接用 "go" 一致。
        let by_alias = highlight_code("package main", Some("golang"));
        let by_ext = highlight_code("package main", Some("go"));
        assert_eq!(by_alias, by_ext);
        // 别名解析必须产出带 span 的高亮输出，而非纯文本。
        assert!(by_alias.contains("span"));
    }

    #[test]
    fn highlight_code_resolves_bash_alias() {
        // 别名表中 "bash" 映射到 "sh" 扩展名。
        let result = highlight_code("echo hello", Some("bash"));
        assert!(result.contains("span"));
    }

    #[test]
    fn highlight_code_resolves_yml_alias() {
        // 别名表中 "yml" 映射到 "yaml" 扩展名。
        let result = highlight_code("key: value", Some("yml"));
        assert!(!result.is_empty());
    }

    #[test]
    fn highlight_code_unknown_language_falls_back_to_plain_text() {
        // 无法识别的语言应回退到纯文本语法，仍能输出内容。
        let result = highlight_code("hello world", Some("totally-not-a-language-xyz"));
        assert!(result.contains("hello world"));
    }

    #[test]
    fn highlight_code_empty_language_string_falls_back_to_plain_text() {
        // 空字符串语言标识应走纯文本回退路径。
        let result = highlight_code("just text", Some(""));
        assert!(result.contains("just text"));
    }

    #[test]
    fn highlight_code_trims_surrounding_whitespace() {
        // 代码首尾的空白会被 trim 掉再高亮。
        let result = highlight_code("  \nfn main() {}\n  ", Some("rust"));
        assert!(result.contains(r#"<span class="storage type function rust">fn</span>"#));
    }

    #[test]
    fn highlight_code_multiline_output_spans_all_lines() {
        // 多行代码每一行都应被解析为带 span 的输出。
        let code = "fn a() {}\nfn b() {}";
        let result = highlight_code(code, Some("rust"));
        // 两处 fn 关键字都应出现
        assert_eq!(
            result
                .matches(r#"<span class="storage type function rust">fn</span>"#)
                .count(),
            2
        );
    }

    #[test]
    fn highlight_code_swift_keyword_and_func() {
        // Swift 关键字 func/import/let 应生成 declaration/keyword span，而不是纯文本。
        let code = "import Foundation\nfunc greet(person: String) -> String {\n    return \"Hi\"\n}";
        let result = highlight_code(code, Some("swift"));
        assert!(
            result.contains("keyword"),
            "Swift 输出缺少关键字高亮: {}",
            result
        );
        // 函数名应被识别为函数（声明名 entity name function 或调用 variable function）。
        assert!(
            result.contains("name function") || result.contains("variable function"),
            "Swift func 名缺少函数高亮: {}",
            result
        );
    }

    #[test]
    fn highlight_code_swift_types_and_strings() {
        // Swift 标准库类型与字符串字面量都应被识别。
        let code = "let count: Int = 42\nlet name = \"hello\"";
        let result = highlight_code(code, Some("swift"));
        assert!(
            result.contains("support type") || result.contains("entity name type"),
            "Swift Int 类型未被识别为类型: {}",
            result
        );
        assert!(
            result.contains("string"),
            "Swift 字符串未被识别: {}",
            result
        );
    }
}
