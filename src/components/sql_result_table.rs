//! SQL 查询结果表格组件。
//!
//! 消费后端 [`SqlResult`]，按单元格的 `serde_json::Value` variant 渲染：
//! NULL 显示为斜体灰色字面量，布尔显示为彩色小徽章，数字右对齐 + 等宽数字，
//! 文本截断显示。长文本可通过点击行在行下方展开跨列详情区查看完整内容。
//! 表头 sticky，列宽有界（`max-width`），避免长文本撑爆横向布局。

use dioxus::prelude::*;

use crate::api::database::sql_console::SqlResult;
use crate::components::ui::ADMIN_TABLE_CLASS;

/// 一行展开详情区中 `<pre>` 的最大高度（超出纵向滚动，避免大 jsonb 撑爆页面）。
const EXPAND_MAX_HEIGHT_CLASS: &str = "max-h-80";
/// 文本单元格的列宽上限（Tailwind 任意值，约束长文本不无限拉伸）。
const TEXT_CELL_MAX_WIDTH_CLASS: &str = "max-w-[24rem]";

/// SQL 查询结果表格。
#[derive(Props, Clone, PartialEq)]
pub struct SqlResultTableProps {
    pub result: SqlResult,
}

/// 渲染 SQL 查询结果表格。
///
/// 一次只允许展开一行（`expanded_row` 信号记录行索引）。
/// `mut` 信号仅在 WASM 端被 `.set()`；server 构建下 `.set()` 调用在 cfg 门控块内被
/// strip，故加 `cfg_attr` 抑制 server 目标的 `unused_mut` 警告。
#[component]
#[allow(non_snake_case)]
#[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut))]
pub fn SqlResultTable(props: SqlResultTableProps) -> Element {
    let mut expanded_row: Signal<Option<usize>> = use_signal(|| None);

    rsx! {
        div { class: "{ADMIN_TABLE_CLASS}",
            div { class: "overflow-auto max-h-[70vh]",
                table { class: "w-full text-sm border-collapse",
                    // 占位：后续 task 填充 thead / tbody
                }
            }
        }
    }
}
