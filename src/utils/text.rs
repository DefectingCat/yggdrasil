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
    plain = LINK_RE.replace_all(&plain, "$1").to_string();
    plain = HEADING_RE.replace_all(&plain, "").to_string();
    plain = IMAGE_RE.replace_all(&plain, "").to_string();
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
