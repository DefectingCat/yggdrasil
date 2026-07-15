//! HTML 转义工具（零依赖纯函数，前端后端通用）。
//!
//! 仓库内原先存在两份 `escape_html` 实现：
//! - `utils::html::escape_html`（`'` → `&#39;`）
//! - `api::comments::helpers::escape_html`（`'` → `&#x27;`，server-only）
//! 现统一到本模块，单引号采用 HTML5 标准的 `&#x27;`。

/// 转义 HTML 特殊字符：`& < > " '`。
///
/// 单引号统一为 `&#x27;`（HTML5 规范，与原 server 端 `helpers::escape_html` 一致）。
/// 可安全用于文本节点与属性值上下文。
///
/// 单遍扫描：旧实现链式调用 5 次 `str::replace`，每次全量扫描 + 分配一个新 String，
/// 5 个特殊字符意味着 5 次堆分配 + 5 遍扫描。单遍 `match` 只扫描一次、只分配一次。
/// 该函数被 markdown 渲染管线密集调用（每个标题、每个代码块），属于热点路径。
pub fn escape_html(input: &str) -> String {
    // 快速路径：无特殊字符直接 clone（大多纯文本走这里，零额外扫描）。
    // memchr 风格的逐字节查找比总是分配 + 遍历更省。
    if !input
        .as_bytes()
        .iter()
        .any(|&b| matches!(b, b'&' | b'<' | b'>' | b'"' | b'\''))
    {
        return input.to_string();
    }
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#x27;"),
            _ => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_all_five_special_chars() {
        assert_eq!(escape_html("&<>\"'"), "&amp;&lt;&gt;&quot;&#x27;");
    }

    #[test]
    fn escapes_ampersand_first_to_avoid_double_escape() {
        // & 必须先转义，否则后续引入的 &amp; 会被再次处理。
        assert_eq!(escape_html("<&>"), "&lt;&amp;&gt;");
    }

    #[test]
    fn leaves_plain_text_untouched() {
        assert_eq!(escape_html("hello world"), "hello world");
    }

    #[test]
    fn empty_input_returns_empty() {
        assert_eq!(escape_html(""), "");
    }
}
