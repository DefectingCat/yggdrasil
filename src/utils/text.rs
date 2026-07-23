//! Markdown 与文本处理工具。
//!
//! 提供移除 Markdown 标记、字数统计、自动生成摘要等功能。

use std::sync::LazyLock;

/// 匹配 fenced code block（```...```）的正则。
static CODE_BLOCK_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"```[\s\S]*?```").expect("CODE_BLOCK_RE 正则模式应在编译期通过校验")
});

/// 匹配行内代码（`...`）的正则。
static INLINE_CODE_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"`[^`]*`").expect("INLINE_CODE_RE 正则模式应在编译期通过校验")
});

/// 匹配 Markdown 链接 `[text](url)` 的正则。
static LINK_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"\[([^\]]*)\]\([^)]*\)").expect("LINK_RE 正则模式应在编译期通过校验")
});

/// 匹配 Markdown 标题（# 到 ######）的正则。
static HEADING_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"^#{1,6}\s*").expect("HEADING_RE 正则模式应在编译期通过校验")
});

/// 匹配 Markdown 图片 `![alt](url)` 的正则。
static IMAGE_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"!\[([^\]]*)\]\([^)]*\)").expect("IMAGE_RE 正则模式应在编译期通过校验")
});

/// 匹配任意空白字符的正则，用于把多个空白合并为单个空格。
static WHITESPACE_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"\s+").expect("WHITESPACE_RE 正则模式应在编译期通过校验")
});

/// 去除 Markdown 标记，返回近似纯文本。
///
/// 处理顺序：代码块 → 行内代码 → 图片 → 链接（保留文字）→ 标题 → 加粗/斜体 → 合并空白。
pub fn strip_markdown(md: &str) -> String {
    let mut plain = CODE_BLOCK_RE.replace_all(md, "").to_string();
    plain = INLINE_CODE_RE.replace_all(&plain, "").to_string();
    // 必须先移除图片再处理链接，否则 `![](url)` 会残留 `!`
    plain = IMAGE_RE.replace_all(&plain, "").to_string();
    plain = LINK_RE.replace_all(&plain, "$1").to_string();
    plain = HEADING_RE.replace_all(&plain, "").to_string();
    plain = plain
        .replace("**", "")
        .replace('*', "")
        .replace("__", "")
        .replace('_', "");
    plain = WHITESPACE_RE.replace_all(&plain, " ").to_string();
    plain.trim().to_string()
}

/// 统计 Markdown 文本的有效字数。
///
/// 中文字符每个计 1；英文字母按连续字母串计 1 个词。
/// 空文本返回 1，避免摘要或列表中出现 0 字的显示问题。
pub fn count_words(md: &str) -> u32 {
    let plain = strip_markdown(md);
    let mut count = 0u32;
    let mut in_word = false;

    for c in plain.chars() {
        // CJK 统一表意文字范围（基本区）
        if c as u32 >= 0x4E00 && c as u32 <= 0x9FFF {
            count += 1;
            in_word = false;
        } else if c.is_alphabetic() {
            if !in_word {
                count += 1;
                in_word = true;
            }
        } else {
            in_word = false;
        }
    }
    count.max(1)
}

/// 由字数估算阅读时长（分钟）。
///
/// 按每分钟 200 字计算，至少返回 1 分钟。
pub fn reading_time(word_count: u32) -> u32 {
    (word_count / 200).max(1)
}

/// 自动生成文本摘要，取去除 Markdown 后的前 200 个字符。
pub fn auto_summary(md: &str) -> String {
    let plain = strip_markdown(md);
    plain.chars().take(200).collect()
}

#[cfg(all(test, feature = "server"))]
mod tests {
    use super::*;

    #[test]
    fn strip_markdown_removes_code_blocks() {
        let input = "before```code here```after";
        assert_eq!(strip_markdown(input), "beforeafter");
    }

    #[test]
    fn strip_markdown_removes_inline_code() {
        assert_eq!(strip_markdown("text `code` more"), "text more");
    }

    #[test]
    fn strip_markdown_removes_images() {
        assert_eq!(strip_markdown("![alt](url)"), "");
    }

    #[test]
    fn strip_markdown_keeps_link_text() {
        assert_eq!(
            strip_markdown("[click me](https://example.com)"),
            "click me"
        );
    }

    #[test]
    fn strip_markdown_removes_headings() {
        assert_eq!(strip_markdown("## Hello"), "Hello");
    }

    #[test]
    fn strip_markdown_removes_bold_and_italic() {
        assert_eq!(
            strip_markdown("**bold** *italic* __bold__ _italic_"),
            "bold italic bold italic"
        );
    }

    #[test]
    fn strip_markdown_empty_input() {
        assert_eq!(strip_markdown(""), "");
    }

    #[test]
    fn strip_markdown_mixed() {
        let md = "# Title\n\nSome **bold** and `code` [link](url)\n\n![img](img.png)";
        let result = strip_markdown(md);
        assert_eq!(result, "Title Some bold and link");
    }

    #[test]
    fn count_words_english() {
        assert_eq!(count_words("hello world"), 2);
    }

    #[test]
    fn count_words_chinese() {
        assert_eq!(count_words("你好世界"), 4);
    }

    #[test]
    fn count_words_mixed() {
        let count = count_words("Hello 你好 world 世界");
        assert_eq!(count, 6);
    }

    #[test]
    fn count_words_with_markdown() {
        let count = count_words("# Hello **World**\n\nSome `code` here");
        assert_eq!(count, 4);
    }

    #[test]
    fn count_words_empty_returns_one() {
        assert_eq!(count_words(""), 1);
    }

    #[test]
    fn reading_time_defaults_to_one() {
        assert_eq!(reading_time(0), 1);
        assert_eq!(reading_time(1), 1);
        assert_eq!(reading_time(199), 1);
    }

    #[test]
    fn reading_time_scales_by_two_hundred() {
        assert_eq!(reading_time(200), 1);
        assert_eq!(reading_time(201), 1);
        assert_eq!(reading_time(400), 2);
        assert_eq!(reading_time(1000), 5);
    }

    #[test]
    fn auto_summary_truncates_at_200_chars() {
        let long_md: String = "a ".repeat(200);
        let summary = auto_summary(&long_md);
        assert_eq!(summary.chars().count(), 200);
    }

    #[test]
    fn auto_summary_short_input() {
        assert_eq!(auto_summary("short"), "short");
    }

    #[test]
    fn auto_summary_strips_markdown() {
        let summary = auto_summary("**bold** and `code`");
        assert_eq!(summary, "bold and");
    }
}
