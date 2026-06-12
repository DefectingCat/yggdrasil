//! 评论管理操作组件
//!
//! 在后台管理文章评论时，提供通过、标记垃圾、删除（移入回收站）三种操作按钮。

use dioxus::prelude::*;

use crate::api::comments::{approve_comment, spam_comment, trash_comment};
use crate::components::comments::section::CommentContext;

/// 评论管理操作按钮组件。
///
/// Props：
/// - `comment_id`：目标评论 ID
/// - `post_id`：所属文章 ID（当前未使用，保留用于未来扩展）
///
/// 关键事件：
/// - 点击"通过"/"垃圾"/"删除"按钮后调用对应 API，操作完成后触发评论列表刷新
/// - 操作期间禁用按钮，防止重复提交
#[component]
pub fn CommentActions(comment_id: i64, post_id: i32) -> Element {
    let ctx: CommentContext = use_context();
    let refresh_trigger = ctx.refresh_trigger;
    let mut busy = use_signal(|| false);

    let _ = post_id;

    rsx! {
        div { class: "flex items-center gap-1.5",
            button {
                class: "text-xs px-2 py-0.5 rounded-full text-green-700 dark:text-green-400 bg-green-50 dark:bg-green-900/20 hover:bg-green-100 dark:hover:bg-green-900/40 transition-colors cursor-pointer",
                disabled: busy(),
                onclick: move |_| {
                    busy.set(true);
                    let mut refresh_trigger = refresh_trigger;
                    spawn(async move {
                        let _ = approve_comment(comment_id).await;
                        refresh_trigger.set(!refresh_trigger());
                        busy.set(false);
                    });
                },
                "通过"
            }
            button {
                class: "text-xs px-2 py-0.5 rounded-full text-amber-700 dark:text-amber-400 bg-amber-50 dark:bg-amber-900/20 hover:bg-amber-100 dark:hover:bg-amber-900/40 transition-colors cursor-pointer",
                disabled: busy(),
                onclick: move |_| {
                    busy.set(true);
                    let mut refresh_trigger = refresh_trigger;
                    spawn(async move {
                        let _ = spam_comment(comment_id).await;
                        refresh_trigger.set(!refresh_trigger());
                        busy.set(false);
                    });
                },
                "垃圾"
            }
            button {
                class: "text-xs px-2 py-0.5 rounded-full text-red-700 dark:text-red-400 bg-red-50 dark:bg-red-900/20 hover:bg-red-100 dark:hover:bg-red-900/40 transition-colors cursor-pointer",
                disabled: busy(),
                onclick: move |_| {
                    busy.set(true);
                    let mut refresh_trigger = refresh_trigger;
                    spawn(async move {
                        let _ = trash_comment(comment_id).await;
                        refresh_trigger.set(!refresh_trigger());
                        busy.set(false);
                    });
                },
                "删除"
            }
        }
    }
}
