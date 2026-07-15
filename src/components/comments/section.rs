//! 评论区段组件
//!
//! 管理单篇文章的评论上下文（回复目标、刷新触发器、待审核评论），
//! 负责加载评论列表、轮询待审核评论状态并渲染表单与列表。

use dioxus::prelude::*;

use crate::api::comments::{check_pending_status, get_comments, CommentTreeResponse};
use crate::components::comments::form::CommentForm;
use crate::components::comments::list::CommentList;
use crate::components::skeletons::comment_skeleton::CommentListSkeleton;
use crate::components::skeletons::delayed_skeleton::DelayedSkeleton;
use crate::utils::comment_storage::{self, PendingComment};

/// 评论上下文，供评论相关组件共享状态。
///
/// 字段：
/// - `active_reply`：当前正在回复的评论 ID
/// - `refresh_trigger`：刷新触发信号，切换时触发评论列表重新加载
/// - `pending_comments`：本地存储的待审核评论
#[derive(Clone, Copy)]
pub struct CommentContext {
    /// 当前正在回复的评论 ID。
    pub active_reply: Signal<Option<i64>>,
    /// 刷新触发信号，切换时触发评论列表重新加载。
    pub refresh_trigger: Signal<bool>,
    /// 本地存储的待审核评论。
    pub pending_comments: Signal<Vec<PendingComment>>,
}

/// 评论区段组件。
///
/// Props：
/// - `post_id`：所属文章 ID
///
/// 负责：
/// - 提供 `CommentContext` 上下文
/// - 加载本地待审核评论并定期轮询其审核状态
/// - 加载已审核评论列表并合并展示
/// - 空评论时展示提示文案
#[component]
pub fn CommentSection(post_id: i32) -> Element {
    let mut ctx = use_context_provider(|| CommentContext {
        active_reply: Signal::new(None),
        refresh_trigger: Signal::new(false),
        pending_comments: Signal::new(Vec::new()),
    });

    // 挂载后从本地存储异步加载待审核评论以防 SSR Hydration Mismatch
    use_effect(move || {
        let pending = comment_storage::load_pending_comments(post_id);
        comment_storage::prune_all_expired();
        ctx.pending_comments.set(pending);
    });

    // 轮询待审核评论状态，已处理（非 pending）的评论从本地移除
    use_future(move || {
        let pending_val = ctx.pending_comments.read().clone();
        async move {
            let ids: Vec<i64> = pending_val.iter().map(|c| c.id).collect();
            if ids.is_empty() {
                return;
            }
            match check_pending_status(ids).await {
                Ok(statuses) => {
                    let to_remove: Vec<i64> = statuses
                        .into_iter()
                        .filter(|s| s.status != "pending")
                        .map(|s| s.id)
                        .collect();
                    if !to_remove.is_empty() {
                        comment_storage::remove_pending_ids(post_id, &to_remove);
                        ctx.pending_comments
                            .write()
                            .retain(|c| !to_remove.contains(&c.id));
                    }
                }
                Err(_e) => {
                    // 在 WASM 环境下静默忽略，服务器端日志不可用
                }
            }
        }
    });

    // 评论数据资源，refresh_trigger 变化时自动重新加载
    let comments_resource = use_resource(move || {
        let _ = (ctx.refresh_trigger)();
        async move { get_comments(post_id).await }
    });

    let data = comments_resource.read();

    // 动态计算总评论数（已审核 + 本地待审核）
    let total_count = if let Some(Ok(CommentTreeResponse { count, .. })) = &*data {
        let approved_count = *count;
        let pending_count = ctx.pending_comments.read().len() as i64;
        Some(approved_count + pending_count)
    } else {
        None
    };

    rsx! {
        div { class: "space-y-8",
            // 标题：加载中显示通用“评论区”，加载成功后附加数量
            if let Some(count) = total_count {
                h2 { class: "text-xl font-bold text-paper-primary", "评论区 ({count})" }
            } else {
                h2 { class: "text-xl font-bold text-paper-primary", "评论区" }
            }

            // 真实的评论输入表单始终立即可见且可交互，避免 CLS
            CommentForm { post_id, parent_id: None, parent_indent: None }

            // 根据数据状态渲染列表区、错误提示或骨架屏
            match &*data {
                Some(Ok(CommentTreeResponse { comments, .. })) => {
                    let approved_count = comments.len();
                    let pending_count = ctx.pending_comments.read().len();
                    let has_any = approved_count > 0 || pending_count > 0;
                    if !has_any {
                        rsx! {
                            p { class: "text-paper-tertiary text-center py-8",
                                "暂无评论，成为第一个评论的人吧！"
                            }
                        }
                    } else {
                        rsx! {
                            CommentList {
                                comments: comments.clone(),
                                pending: ctx.pending_comments.read().clone(),
                                post_id,
                            }
                        }
                    }
                }
                Some(Err(_)) => rsx! {
                    div { class: "text-center text-red-500 dark:text-red-400 py-8", "评论加载失败" }
                },
                None => rsx! {
                    DelayedSkeleton { CommentListSkeleton {} }
                },
            }
        }
    }
}
