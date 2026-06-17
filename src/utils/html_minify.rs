//! HTML 空白压缩工具函数。
//!
//! 仅服务端使用。对 HTML 片段做轻量 minify：
//! - 合并标签之间连续空白为一个空格；
//! - 移除 HTML 注释；
//! - 保留 `<pre>`、`<code>`、`<textarea>`、`<script>`、`<style>` 内部原样空白。

use lol_html::{doc_comments, doc_text, element, rewrite_str, RewriteStrSettings};
use std::cell::Cell;
use std::rc::Rc;

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
                    c.remove();
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
