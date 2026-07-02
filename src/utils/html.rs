//! HTML 转义工具（零依赖纯函数，前端后端通用）。
//!
//! 仓库内原先存在两份 `escape_html` 实现：
//! - `hooks::comment_storage::escape_html`（`'` → `&#39;`）
//! - `api::comments::helpers::escape_html`（`'` → `&#x27;`，server-only）
//! 现统一到本模块，单引号采用 HTML5 标准的 `&#x27;`。

/// 转义 HTML 特殊字符：`& < > " '`。
///
/// 单引号统一为 `&#x27;`（HTML5 规范，与原 server 端 `helpers::escape_html` 一致）。
/// 可安全用于文本节点与属性值上下文。
pub fn escape_html(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
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
