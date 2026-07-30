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
use crate::utils::time::sleep_ms;

/// 待审核评论状态的轮询间隔（毫秒）。
///
/// 仅在本地存在待审核评论时才轮询；30s 在「审核通过后尽快反映」与「不触发 strict
/// 限流（默认 1 req/s, burst 5）」之间取平衡。
const PENDING_POLL_INTERVAL_MS: u32 = 30_000;

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

    // 轮询待审核评论状态：只要本地还有待审核评论，就定期查询其审核状态。
    //
    // 旧实现把 pending_comments 快照进闭包、use_future 仅在信号变化时跑一次——
    // 用户提交评论后立即查（此时仍是 pending），之后管理员通过审核时信号未变，
    // future 不再重跑，导致「审核中」徽章永久残留（issue #9）。这与 backup.rs 的
    // 进度轮询属同类 bug，沿用「长生命周期 loop + 循环内读信号」的模式修复：
    // 一旦某条评论变为非 pending（通常为已通过），就从本地移除并刷新已审核列表，
    // 使其以正式状态进入评论树，而非一直挂着「审核中」占位。
    use_future(move || {
        // 同步段读取建立响应式依赖：pending_comments 变化（新评论提交 / 本轮移除）
        // 时 use_future 自动重启，确保循环 return 退出后仍能重新进入轮询。
        let _ = ctx.pending_comments.read();
        let mut pending_comments = ctx.pending_comments;
        let mut refresh_trigger = ctx.refresh_trigger;
        async move {
            loop {
                let ids: Vec<i64> = pending_comments.read().iter().map(|c| c.id).collect();
                if ids.is_empty() {
                    // 无待审核评论：停止轮询，等待响应式重启（新评论提交时触发）。
                    return;
                }

                if let Ok(statuses) = check_pending_status(ids).await {
                    let to_remove: Vec<i64> = statuses
                        .into_iter()
                        .filter(|s| s.status != "pending")
                        .map(|s| s.id)
                        .collect();
                    if !to_remove.is_empty() {
                        comment_storage::remove_pending_ids(post_id, &to_remove);
                        // 评论状态已变化（多为已通过）：刷新已审核列表，使其以正式
                        // 状态进入评论树。peek 不订阅信号，避免给本 future 引入
                        // 额外响应式依赖；先取出值再 set，规避 peek 守卫与 set 的借用冲突。
                        let next = !*refresh_trigger.peek();
                        refresh_trigger.set(next);
                        pending_comments
                            .write()
                            .retain(|c| !to_remove.contains(&c.id));
                    }
                }
                // Err（如限流）静默忽略，统一在下方 sleep 后下一轮重试。

                sleep_ms(PENDING_POLL_INTERVAL_MS).await;
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
                            p { class: "text-paper-tertiary text-center py-8", "暂无评论，成为第一个评论的人吧！" }
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
