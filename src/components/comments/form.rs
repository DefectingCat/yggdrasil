use dioxus::prelude::*;

use crate::api::comments::create_comment;
use crate::components::comments::section::CommentContext;
use crate::components::forms::{INPUT_CLASS, BUTTON_PRIMARY_CLASS, AlertBox};
use crate::hooks::comment_storage::{self, PendingComment};

#[component]
pub fn CommentForm(post_id: i32, parent_id: Option<i64>) -> Element {
    let ctx: CommentContext = use_context();
    let mut active_reply = ctx.active_reply;
    let mut refresh_trigger = ctx.refresh_trigger;
    let mut pending_comments = ctx.pending_comments;

    let mut author_name = use_signal(String::new);
    let mut author_email = use_signal(String::new);
    let mut author_url = use_signal(String::new);
    let mut content_md = use_signal(String::new);
    let mut honeypot = use_signal(String::new);
    let mut submitting = use_signal(|| false);
    let mut message = use_signal(|| Option::<(String, &'static str)>::None);
    let mut loaded = use_signal(|| false);

    use_effect(move || {
        if loaded() {
            return;
        }
        loaded.set(true);
        if let Some(info) = comment_storage::load_author() {
            author_name.set(info.name);
            author_email.set(info.email);
            author_url.set(info.url);
        }
    });

    if let Some(pid) = parent_id {
        if active_reply() != Some(pid) {
            return rsx! {};
        }
    }

    let is_reply = parent_id.is_some();

    rsx! {
        div {
            class: if is_reply { "mt-3 pt-3 border-t border-gray-100 dark:border-[#333]" } else { "" },
            role: "form",
            aria_label: if is_reply { "回复评论" } else { "发表评论" },

            if let Some((msg, variant)) = message() {
                div { aria_live: "polite",
                    AlertBox { message: msg, variant }
                }
            }

            div { class: "space-y-3",
                div { class: "grid grid-cols-1 sm:grid-cols-2 gap-3",
                    div {
                        label { class: "block text-sm font-medium text-paper-secondary mb-1",
                            "昵称 *"
                        }
                        input {
                            class: INPUT_CLASS,
                            r#type: "text",
                            placeholder: "你的昵称",
                            value: "{author_name}",
                            disabled: submitting(),
                            oninput: move |e| author_name.set(e.value()),
                        }
                    }
                    div {
                        label { class: "block text-sm font-medium text-paper-secondary mb-1",
                            "邮箱 *"
                        }
                        input {
                            class: INPUT_CLASS,
                            r#type: "email",
                            placeholder: "your@email.com",
                            value: "{author_email}",
                            disabled: submitting(),
                            oninput: move |e| author_email.set(e.value()),
                        }
                    }
                }
                div {
                    label { class: "block text-sm font-medium text-paper-secondary mb-1",
                        "网站"
                    }
                    input {
                        class: INPUT_CLASS,
                        r#type: "url",
                        placeholder: "https://example.com（可选）",
                        value: "{author_url}",
                        disabled: submitting(),
                        oninput: move |e| author_url.set(e.value()),
                    }
                }

                textarea {
                    class: "{INPUT_CLASS} min-h-[100px] resize-y",
                    value: "{content_md}",
                    disabled: submitting(),
                    oninput: move |e| content_md.set(e.value()),
                }

                p { class: "text-xs text-paper-tertiary",
                    "支持 Markdown 语法"
                }

                textarea {
                    class: "hidden",
                    aria_hidden: "true",
                    tabindex: "-1",
                    value: "{honeypot}",
                    oninput: move |e| honeypot.set(e.value()),
                }

                button {
                    class: BUTTON_PRIMARY_CLASS,
                    disabled: submitting(),
                    onclick: move |_| {
                        if submitting() {
                            return;
                        }

                        let post_id = post_id;
                        let parent_id = parent_id;
                        let name = author_name();
                        let email = author_email();
                        let url_val = author_url();
                        let content = content_md();
                        let hp = honeypot();

                        if !hp.is_empty() {
                            return;
                        }

                        if name.trim().is_empty() || email.trim().is_empty() || content.trim().is_empty() {
                            message.set(Some(("请填写所有必填项".to_string(), "error")));
                            return;
                        }

                        submitting.set(true);
                        message.set(None);

                        spawn(async move {
                            let result = create_comment(
                                post_id,
                                parent_id,
                                name.clone(),
                                email.clone(),
                                if url_val.trim().is_empty() { None } else { Some(url_val.clone()) },
                                content.clone(),
                            ).await;

                            submitting.set(false);

                            match result {
                                Ok(resp) => {
                                    if resp.success {
                                        comment_storage::save_author(
                                            &name,
                                            &email,
                                            &url_val,
                                        );

                                        if let Some(comment_id) = resp.comment_id {
                                            let avatar_url = resp.avatar_url.unwrap_or_default();
                                            let depth = resp.depth.unwrap_or(0);

                                            let now = chrono::Utc::now().to_rfc3339();
                                            let pending = PendingComment {
                                                id: comment_id,
                                                parent_id,
                                                depth,
                                                author_name: name.clone(),
                                                author_url: if url_val.trim().is_empty() { None } else { Some(url_val) },
                                                avatar_url,
                                                content_md: content,
                                                created_at: now.clone(),
                                                stored_at: now,
                                            };

                                            comment_storage::save_pending_comment(post_id, pending.clone());
                                            pending_comments.write().push(pending);
                                        }

                                        content_md.set(String::new());
                                        message.set(Some((resp.message, "success")));
                                        if parent_id.is_some() {
                                            active_reply.set(None);
                                        }
                                        refresh_trigger.set(!refresh_trigger());
                                    } else {
                                        message.set(Some((resp.message, "error")));
                                    }
                                }
                                Err(_) => {
                                    message.set(Some(("提交失败，请稍后重试".to_string(), "error")));
                                }
                            }
                        });
                    },

                    if submitting() {
                        "提交中…"
                    } else if is_reply {
                        "回复"
                    } else {
                        "发表评论"
                    }
                }
            }
        }
    }
}
