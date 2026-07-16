//! KaTeX 服务端数学公式渲染。
//!
//! 用纯 Rust 的 [`katex`](https://crates.io/crates/katex-rs) crate 把 TeX 公式
//! 渲染成 HTML span，供 pulldown-cmark 的 `InlineMath` / `DisplayMath` 事件调用。
//! 仅在 `feature = "server"` 时编译——前端 WASM 不参与公式渲染（SSR 即终态）。
//!
//! 渲染策略：
//! - `OutputFormat::Html`：只产出视觉层 `<span class="katex">…</span>`，不含 MathML
//!   语义层（`<math>` 等）。这样 sanitizer 无需为 MathML 标签开白名单，XSS 面最小。
//!   屏幕阅读器等无障碍场景的语义损失可接受（本站数学公式占比低）。
//! - `throw_on_error = false`：坏公式渲染成红色错误 span 而非中断整篇文章。
//!
//! 配套资源：前端必须加载 `public/katex/katex.min.css` + `fonts/`（见 Makefile
//! `katex-css`），否则只有裸 span、无数学字体排版。crate 本身不打包 CSS。

#![cfg(feature = "server")]

use katex::{KatexContext, OutputFormat, Settings};

/// 内联公式（`$...$`）渲染配置工厂：`display_mode = false`。
fn inline_settings() -> Settings {
    let mut s = Settings::default();
    s.output = OutputFormat::Html;
    s.display_mode = false;
    s.throw_on_error = false;
    s
}

/// 块级公式（`$$...$$`）渲染配置工厂：`display_mode = true`（居中独占一行）。
fn display_settings() -> Settings {
    let mut s = Settings::default();
    s.output = OutputFormat::Html;
    s.display_mode = true;
    s.throw_on_error = false;
    s
}

thread_local! {
    /// KaTeX 上下文：含全部内置符号 / 宏表，应在多次渲染间复用（README 建议）。
    /// 用 thread_local 而非全局 static：`KatexContext` 内含 `RefCell<HashMap>`
    /// 宏表，非 `Sync`，不能放 `LazyLock`。tokio 多线程 runtime 下每线程各持一份。
    static KATEX_CTX: KatexContext = KatexContext::default();

    /// 每线程缓存的渲染配置，避免每次渲染都重建宏表 HashMap。
    /// `Settings` 同样因 `RefCell` 宏表非 `Sync`。
    static INLINE_SETTINGS: Settings = inline_settings();
    static DISPLAY_SETTINGS: Settings = display_settings();
}

/// 渲染内联公式 `$...$`（定界符由 pulldown-cmark 剥除）→ HTML 字符串。
///
/// 渲染失败（坏 TeX）时回退到 HTML 转义后的原文，保证文章不因一个坏公式全篇崩。
pub fn render_inline(tex: &str) -> String {
    KATEX_CTX.with(|ctx| {
        INLINE_SETTINGS.with(|settings| {
            katex::render_to_string(ctx, tex, settings)
                .unwrap_or_else(|_| crate::utils::html::escape_html(tex))
        })
    })
}

/// 渲染块级公式 `$$...$$`（定界符由 pulldown-cmark 剥除）→ HTML 字符串。
///
/// 与 [`render_inline`] 同样在失败时回退到转义原文。调用方负责块级包裹
/// （如 `<p class="math-display">`），这里只产出 KaTeX 的 span 串。
pub fn render_display(tex: &str) -> String {
    KATEX_CTX.with(|ctx| {
        DISPLAY_SETTINGS.with(|settings| {
            katex::render_to_string(ctx, tex, settings)
                .unwrap_or_else(|_| crate::utils::html::escape_html(tex))
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_inline_produces_katex_span() {
        let html = render_inline("E = mc^2");
        assert!(
            html.contains("katex"),
            "内联公式应产出含 katex class 的 span, got: {html}"
        );
    }

    #[test]
    fn render_display_produces_katex_display() {
        let html = render_display("\\frac{a}{b}");
        assert!(
            html.contains("katex-display"),
            "块级公式应产出含 katex-display class 的结构, got: {html}"
        );
    }

    #[test]
    fn render_bad_tex_does_not_panic() {
        // throw_on_error=false：坏 TeX 不应 panic、不应返回 Err。
        // KaTeX 可能渲染成红色错误 span，也可能把未知宏当字面文本处理。
        // 关键契约：返回非空字符串、不中断调用方。
        let html = render_inline("\\thisisnotarealmacroxyz{");
        assert!(!html.is_empty(), "坏公式应返回非空 HTML, got empty");
    }

    #[test]
    fn render_inline_does_not_emit_math_tag() {
        // OutputFormat::Html 不应产出 <math> 标签（那是 HtmlAndMathml / Mathml 模式）。
        let html = render_inline("a^2 + b^2 = c^2");
        assert!(
            !html.contains("<math"),
            "Html 输出不应含 <math> 标签, got: {html}"
        );
    }
}
