use std::sync::LazyLock;

static CODE_BLOCK_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"```[\s\S]*?```").unwrap()
});

static INLINE_CODE_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"`[^`]*`").unwrap()
});

static LINK_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"\[([^\]]*)\]\([^)]*\)").unwrap()
});

static HEADING_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"^#{1,6}\s*").unwrap()
});

static IMAGE_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"!\[([^\]]*)\]\([^)]*\)").unwrap()
});

static WHITESPACE_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"\s+").unwrap()
});

pub fn strip_markdown(md: &str) -> String {
    let mut plain = CODE_BLOCK_RE.replace_all(md, "").to_string();
    plain = INLINE_CODE_RE.replace_all(&plain, "").to_string();
    // Must strip images BEFORE links, otherwise `![](url)` becomes `!`
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

pub fn count_words(md: &str) -> u32 {
    let plain = strip_markdown(md);
    let mut count = 0u32;
    let mut in_word = false;

    for c in plain.chars() {
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

pub fn auto_summary(md: &str) -> String {
    let plain = strip_markdown(md);
    plain.chars().take(200).collect()
}

#[cfg(test)]
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
        assert_eq!(strip_markdown("[click me](https://example.com)"), "click me");
    }

    #[test]
    fn strip_markdown_removes_headings() {
        assert_eq!(strip_markdown("## Hello"), "Hello");
    }

    #[test]
    fn strip_markdown_removes_bold_and_italic() {
        assert_eq!(strip_markdown("**bold** *italic* __bold__ _italic_"), "bold italic bold italic");
    }

    #[test]
    fn strip_markdown_empty_input() {
        assert_eq!(strip_markdown(""), "");
    }

    #[test]
    fn strip_markdown_mixed() {
        let md = "# Title\n\nSome **bold** and `code` [link](url)\n\n![img](img.png)";
        let result = strip_markdown(md);
        assert!(result.contains("Title"));
        assert!(result.contains("bold"));
        assert!(result.contains("link"));
        assert!(!result.contains("img"));
        assert!(!result.contains("**"));
        assert!(!result.contains("`"));
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
