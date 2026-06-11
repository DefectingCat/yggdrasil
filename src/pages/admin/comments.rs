use std::collections::HashSet;

use dioxus::prelude::*;
use dioxus::router::components::Link;

use crate::api::comments::{
    approve_comment, batch_update_comment_status, get_all_comments, spam_comment,
    AllCommentsResponse,
};
#[cfg(target_arch = "wasm32")]
use crate::api::comments::trash_comment;
use crate::components::skeletons::delayed_skeleton::DelayedSkeleton;
use crate::models::comment::CommentStatus;
use crate::router::Route;

const COMMENTS_PER_PAGE: i32 = 20;

#[component]
pub fn AdminComments() -> Element {
    rsx! { AdminCommentsPage { page: 1 } }
}

#[component]
pub fn AdminCommentsPage(page: i32) -> Element {
    let current_page = page.max(1);
    let mut active_filter = use_signal(|| {
        #[cfg(target_arch = "wasm32")]
        {
            web_sys::window()
                .and_then(|w| w.location().search().ok())
                .and_then(|s| {
                    let params = s.trim_start_matches('?');
                    for pair in params.split('&') {
                        if let Some(val) = pair.strip_prefix("status=") {
                            return Some(val.to_string());
                        }
                    }
                    None
                })
                .unwrap_or_default()
        }
        #[cfg(not(target_arch = "wasm32"))]
        String::new()
    });
    let mut selected_ids: Signal<HashSet<i64>> = use_signal(HashSet::new);
    let filter_status = move || {
        let f = active_filter();
        if f.is_empty() { None } else { Some(f) }
    };
    let mut comments_res =
        use_server_future(move || get_all_comments(current_page, filter_status()))?;

    rsx! {
        div { class: "space-y-6",
            h1 { class: "text-2xl font-bold text-gray-900 dark:text-[#dadadb]",
                "评论管理"
            }

            div { class: "flex gap-1 border-b border-gray-200 dark:border-[#333]",
                for (status, label) in [("", "全部"), ("pending", "待审核"), ("approved", "已通过"), ("spam", "垃圾箱")] {
                    button {
                        class: if active_filter() == status {
                            "px-4 py-2 text-sm font-medium border-b-2 border-gray-900 dark:border-[#dadadb] text-gray-900 dark:text-[#dadadb]"
                        } else {
                            "px-4 py-2 text-sm font-medium text-gray-500 dark:text-[#9b9c9d] hover:text-gray-700 dark:hover:text-[#dadadb] transition-colors"
                        },
                        onclick: move |_| active_filter.set(status.to_string()),
                        "{label}"
                    }
                }
            }

            if !selected_ids().is_empty() {
                { rsx! {
                    div { class: "flex items-center gap-3 p-3 bg-gray-50 dark:bg-[#2a2a2a] rounded-lg",
                        span { class: "text-sm text-gray-600 dark:text-[#9b9c9d]",
                            "已选择 {selected_ids().len()} 条"
                        }
                        button {
                            class: "px-3 py-1.5 text-xs font-medium bg-green-600 text-white rounded hover:bg-green-700 transition-colors",
                            onclick: move |_| {
                                let ids: Vec<i64> = selected_ids().iter().copied().collect();
                                spawn(async move {
                                    let _ = batch_update_comment_status(ids, "approved".to_string()).await;
                                });
                                selected_ids.set(HashSet::new());
                                comments_res.restart();
                            },
                            "批量通过"
                        }
                        button {
                            class: "px-3 py-1.5 text-xs font-medium bg-amber-600 text-white rounded hover:bg-amber-700 transition-colors",
                            onclick: move |_| {
                                let ids: Vec<i64> = selected_ids().iter().copied().collect();
                                spawn(async move {
                                    let _ = batch_update_comment_status(ids, "spam".to_string()).await;
                                });
                                selected_ids.set(HashSet::new());
                                comments_res.restart();
                            },
                            "批量垃圾"
                        }
                        button {
                            class: "px-3 py-1.5 text-xs font-medium bg-red-600 text-white rounded hover:bg-red-700 transition-colors",
                            onclick: move |_| {
                                #[cfg(target_arch = "wasm32")]
                                {
                                    if web_sys::window()
                                        .and_then(|w| w.confirm_with_message("确定要删除这些评论吗？").ok())
                                        .unwrap_or(false)
                                    {
                                        let ids: Vec<i64> = selected_ids().iter().copied().collect();
                                        spawn(async move {
                                            let _ = batch_update_comment_status(ids, "trash".to_string()).await;
                                        });
                                        selected_ids.set(HashSet::new());
                                        comments_res.restart();
                                    }
                                }
                            },
                            "批量删除"
                        }
                    }
                } }
            }

            {
                let data = comments_res.read().as_ref().map(|r| match r {
                    Ok(AllCommentsResponse { comments, total }) => Ok((comments.clone(), *total)),
                    Err(e) => Err(e.to_string()),
                });
                match data {
                    Some(Ok((comments, total))) => {
                        if comments.is_empty() {
                            rsx! {
                                div { class: "text-center py-20 text-gray-500 dark:text-[#9b9c9d]",
                                    "暂无评论"
                                }
                            }
                        } else {
                            let all_selected = comments.iter().all(|c| selected_ids().contains(&c.id));
                            let all_ids: Vec<i64> = comments.iter().map(|c| c.id).collect();
                            rsx! {
                                div { class: "bg-white dark:bg-[#2e2e33] rounded-xl border border-gray-200 dark:border-[#333] overflow-hidden",
                                    div { class: "overflow-x-auto",
                                        table { class: "w-full text-sm",
                                            thead {
                                                tr { class: "border-b border-gray-200 dark:border-[#333] text-left text-gray-500 dark:text-[#9b9c9d]",
                                                    th { class: "px-4 py-3 font-medium w-10",
                                                        input {
                                                            r#type: "checkbox",
                                                            class: "rounded border-gray-300 dark:border-[#555]",
                                                            checked: all_selected,
                                                            onchange: {
                                                                move |_| {
                                                                    let mut s = selected_ids();
                                                                    if all_selected {
                                                                        for id in &all_ids { s.remove(id); }
                                                                    } else {
                                                                        for id in &all_ids { s.insert(*id); }
                                                                    }
                                                                    selected_ids.set(s);
                                                                }
                                                            }
                                                        }
                                                    }
                                                    th { class: "px-4 py-3 font-medium", "作者" }
                                                    th { class: "px-4 py-3 font-medium", "内容" }
                                                    th { class: "px-4 py-3 font-medium", "文章" }
                                                    th { class: "px-4 py-3 font-medium text-center", "状态" }
                                                    th { class: "px-4 py-3 font-medium w-28", "日期" }
                                                    th { class: "px-4 py-3 font-medium w-32 text-right", "操作" }
                                                }
                                            }
                                            tbody {
                                                for comment in comments.iter() {
                                                    CommentRow {
                                                        key: "{comment.id}",
                                                        comment: comment.clone(),
                                                        selected: selected_ids().contains(&comment.id),
                                                        on_select: {
                                                            let id = comment.id;
                                                            move |checked: bool| {
                                                                let mut s = selected_ids();
                                                                if checked { s.insert(id); } else { s.remove(&id); }
                                                                selected_ids.set(s);
                                                            }
                                                        },
                                                        on_approve: {
                                                            let mut comments_res = comments_res;
                                                            let id = comment.id;
                                                            move |_| {
                                                                spawn(async move {
                                                                    let _ = approve_comment(id).await;
                                                                    comments_res.restart();
                                                                });
                                                            }
                                                        },
                                                        on_spam: {
                                                            let mut comments_res = comments_res;
                                                            let id = comment.id;
                                                            move |_| {
                                                                spawn(async move {
                                                                    let _ = spam_comment(id).await;
                                                                    comments_res.restart();
                                                                });
                                                            }
                                                        },
                                                        on_trash: {
                                                            #[allow(unused_variables)]
                                                            let mut comments_res = comments_res;
                                                            let _id = comment.id;
                                                            move |_| {
                                                                #[cfg(target_arch = "wasm32")]
                                                                {
                                                                    if web_sys::window()
                                                                        .and_then(|w| w.confirm_with_message("确定要删除这条评论吗？").ok())
                                                                        .unwrap_or(false)
                                                                    {
                                                                        let id = _id;
                                                                        spawn(async move {
                                                                            let _ = trash_comment(id).await;
                                                                            comments_res.restart();
                                                                        });
                                                                    }
                                                                }
                                                            }
                                                        },
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                CommentsPagination { current_page, total }
                            }
                        }
                    }
                    Some(Err(_e)) => {
                        rsx! {
                            div { class: "text-center text-red-500 dark:text-red-400 py-20",
                                "加载失败"
                            }
                        }
                    }
                    None => {
                        rsx! {
                            DelayedSkeleton {
                                div { class: "bg-white dark:bg-[#2e2e33] rounded-xl border border-gray-200 dark:border-[#333] p-6 space-y-4",
                                    for _ in 0..5 {
                                        div { class: "flex items-center gap-4",
                                            div { class: "h-4 w-4 bg-gray-200 dark:bg-[#2a2a2a] rounded" }
                                            div { class: "h-8 w-8 bg-gray-200 dark:bg-[#2a2a2a] rounded-full" }
                                            div { class: "h-4 w-32 bg-gray-200 dark:bg-[#2a2a2a] rounded" }
                                            div { class: "h-4 flex-1 bg-gray-200 dark:bg-[#2a2a2a] rounded" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn CommentRow(
    comment: crate::models::comment::AdminComment,
    selected: bool,
    on_select: EventHandler<bool>,
    on_approve: EventHandler,
    on_spam: EventHandler,
    on_trash: EventHandler,
) -> Element {
    let (badge_class, status_label) = match &comment.status {
        CommentStatus::Pending => ("bg-amber-100 text-amber-700 dark:bg-amber-900/30 dark:text-amber-400", "待审核"),
        CommentStatus::Approved => ("bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400", "已通过"),
        CommentStatus::Spam => ("bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400", "垃圾"),
        CommentStatus::Trash => ("bg-gray-100 text-gray-700 dark:bg-gray-900/30 dark:text-gray-400", "已删除"),
    };
    let date_str = comment.created_at.format("%Y-%m-%d").to_string();
    let preview = if comment.content_md.len() > 100 {
        format!("{}...", &comment.content_md[..comment.content_md.ceil_char_boundary(100)])
    } else {
        comment.content_md.clone()
    };

    rsx! {
        tr { class: "border-b border-gray-100 dark:border-[#333] last:border-0 hover:bg-gray-50 dark:hover:bg-[#2a2a2a] transition-colors",
            td { class: "px-4 py-3",
                input {
                    r#type: "checkbox",
                    class: "rounded border-gray-300 dark:border-[#555]",
                    checked: selected,
                    onchange: move |e| on_select.call(e.checked()),
                }
            }
            td { class: "px-4 py-3",
                div { class: "flex items-center gap-2",
                    img {
                        class: "w-8 h-8 rounded-full flex-shrink-0",
                        src: "{comment.avatar_url}",
                        alt: "{comment.author_name}",
                    }
                    div { class: "min-w-0",
                        div { class: "text-sm font-medium text-gray-900 dark:text-[#dadadb] truncate",
                            "{comment.author_name}"
                        }
                        div { class: "text-xs text-gray-400 dark:text-[#666] truncate",
                            "{comment.author_email}"
                        }
                    }
                }
            }
            td { class: "px-4 py-3 max-w-xs",
                p { class: "text-sm text-gray-600 dark:text-[#9b9c9d] truncate",
                    "{preview}"
                }
            }
            td { class: "px-4 py-3",
                Link {
                    class: "text-sm text-gray-700 dark:text-[#dadadb] hover:opacity-80 transition-opacity",
                    to: Route::PostDetail { slug: comment.post_slug.clone() },
                    "{comment.post_title}"
                }
            }
            td { class: "px-4 py-3 text-center",
                span { class: "inline-flex items-center px-2 py-0.5 rounded text-xs font-medium whitespace-nowrap {badge_class}",
                    "{status_label}"
                }
            }
            td { class: "px-4 py-3 text-sm text-gray-500 dark:text-[#9b9c9d]",
                "{date_str}"
            }
            td { class: "px-4 py-3 text-right",
                div { class: "flex justify-end gap-2",
                    if !matches!(comment.status, CommentStatus::Approved) {
                        button {
                            class: "text-xs text-green-600 hover:text-green-800 dark:text-green-400 dark:hover:text-green-300 transition-colors cursor-pointer",
                            onclick: move |_| on_approve.call(()),
                            "通过"
                        }
                    }
                    if !matches!(comment.status, CommentStatus::Spam) {
                        button {
                            class: "text-xs text-amber-600 hover:text-amber-800 dark:text-amber-400 dark:hover:text-amber-300 transition-colors cursor-pointer",
                            onclick: move |_| on_spam.call(()),
                            "垃圾"
                        }
                    }
                    if !matches!(comment.status, CommentStatus::Trash) {
                        button {
                            class: "text-xs text-red-500 hover:text-red-700 dark:hover:text-red-300 transition-colors cursor-pointer",
                            onclick: move |_| on_trash.call(()),
                            "删除"
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn CommentsPagination(current_page: i32, total: i64) -> Element {
    let has_prev = current_page > 1;
    let total_pages =
        ((total + COMMENTS_PER_PAGE as i64 - 1) / COMMENTS_PER_PAGE as i64).max(1) as i32;
    let has_next = current_page < total_pages;

    let prev_route = if current_page - 1 <= 1 {
        Route::AdminComments {}
    } else {
        Route::AdminCommentsPage { page: current_page - 1 }
    };
    let next_route = Route::AdminCommentsPage { page: current_page + 1 };

    rsx! {
        nav { class: "flex mt-6 justify-between",
            if has_prev {
                Link {
                    class: "inline-flex items-center px-4 py-2 text-sm text-white bg-gray-900 dark:bg-[#dadadb] dark:text-gray-900 rounded-full hover:opacity-80 transition-opacity cursor-pointer",
                    to: prev_route,
                    span { class: "mr-1", "«" }
                    "上一页"
                }
            } else {
                span { class: "inline-flex items-center px-4 py-2 text-sm text-gray-400 bg-gray-100 dark:bg-[#2a2a2a] rounded-full cursor-not-allowed",
                    span { class: "mr-1", "«" }
                    "上一页"
                }
            }
            span { class: "text-sm text-gray-500 dark:text-[#9b9c9d] self-center",
                "{current_page} / {total_pages} 页 (共 {total} 条)"
            }
            if has_next {
                Link {
                    class: "inline-flex items-center px-4 py-2 text-sm text-white bg-gray-900 dark:bg-[#dadadb] dark:text-gray-900 rounded-full hover:opacity-80 transition-opacity cursor-pointer",
                    to: next_route,
                    "下一页"
                    span { class: "ml-1", "»" }
                }
            } else {
                span { class: "inline-flex items-center px-4 py-2 text-sm text-gray-400 bg-gray-100 dark:bg-[#2a2a2a] rounded-full cursor-not-allowed",
                    "下一页"
                    span { class: "ml-1", "»" }
                }
            }
        }
    }
}
