//! HTML 空白压缩工具函数。
//!
//! 仅服务端使用。对 HTML 片段做轻量 minify：
//! - 合并标签之间连续空白为一个空格；
//! - 移除普通 HTML 注释，但**保留 Dioxus hydration marker 注释**；
//! - 保留 `<pre>`、`<code>`、`<textarea>`、`<script>`、`<style>` 内部原样空白。
//!
//! Dioxus 0.7 SSR 会在输出中插入若干 load-bearing 的注释 marker，客户端
//! hydration 依赖它们定位动态节点。删除这些注释会导致前端 hydration
//! 失败（节点对不齐、交互失效）。需要保留的格式：
//!   `<!--#-->`              动态文本节点结束 marker
//!   `<!--node-id<N>-->`     动态文本节点起始 marker
//!   `<!--placeholder-->`    / `<!--placeholder<N>-->` 占位节点 marker
//! （参考 others/dioxus/packages/ssr/src/renderer.rs）

use lol_html::{doc_comments, doc_text, element, rewrite_str, RewriteStrSettings};
use std::cell::Cell;
use std::rc::Rc;

/// 判断一段注释文本是否为 Dioxus hydration marker，必须原样保留。
fn is_hydration_marker(comment_text: &str) -> bool {
    comment_text == "#"
        || comment_text.starts_with("node-id")
        || comment_text.starts_with("placeholder")
}

/// 压缩 HTML 中的无用空白。
pub fn minify_html(input: &str) -> String {
    let protected_depth: Rc<Cell<usize>> = Rc::new(Cell::new(0));

    rewrite_str(
        input,
        RewriteStrSettings {
            element_content_handlers: vec![element!(
                "pre, code, textarea, script, style",
                {
                    let depth = protected_depth.clone();
                    move |el| {
                        depth.set(depth.get() + 1);
                        let depth_end = depth.clone();
                        let _ = el.on_end_tag(lol_html::end_tag!(move |_end| {
                            depth_end.set(depth_end.get().saturating_sub(1));
                            Ok(())
                        }));
                        Ok(())
                    }
                }
            )],
            document_content_handlers: vec![
                doc_text!({
                    let depth = protected_depth.clone();
                    move |text| {
                        if depth.get() == 0 {
                            let s = text.as_str();
                            let collapsed = collapse_whitespace(s);
                            if collapsed != s {
                                text.set_str(collapsed);
                            }
                        }
                        Ok(())
                    }
                }),
                doc_comments!(|c| {
                    // 只删除非 hydration marker 的普通注释。
                    if !is_hydration_marker(c.text().as_str()) {
                        c.remove();
                    }
                    Ok(())
                }),
            ],
            ..RewriteStrSettings::default()
        },
    )
    .unwrap_or_else(|_| input.to_string())
}

/// 将连续空白字符合并为一个空格。
fn collapse_whitespace(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut prev_ws = false;
    for ch in s.chars() {
        if ch.is_whitespace() {
            if !prev_ws {
                out.push(' ');
                prev_ws = true;
            }
        } else {
            out.push(ch);
            prev_ws = false;
        }
    }
    out
}
