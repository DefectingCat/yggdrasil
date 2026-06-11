use dioxus::prelude::*;

use crate::api::comments::create_comment;
use crate::components::comments::section::CommentContext;
use crate::components::forms::{INPUT_CLASS, BUTTON_PRIMARY_CLASS, AlertBox};

#[component]
pub fn CommentForm(post_id: i32, parent_id: Option<i64>) -> Element {
    let ctx: CommentContext = use_context();
    let mut active_reply = ctx.active_reply;
    let mut refresh_trigger = ctx.refresh_trigger;
    let mut author_name = use_signal(String::new);
    let mut author_email = use_signal(String::new);
    let mut author_url = use_signal(String::new);
    let mut content_md = use_signal(String::new);
    let mut honeypot = use_signal(String::new);
    let mut consented = use_signal(|| false);
    let mut submitting = use_signal(|| false);
    let mut message = use_signal(|| Option::<(String, &'static str)>::None);

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
                if !is_reply {
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
                }

                textarea {
                    class: "{INPUT_CLASS} min-h-[100px] resize-y",
                    placeholder: "写下你的评论…",
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

                div { class: "flex items-start gap-2",
                    input {
                        r#type: "checkbox",
                        id: "consent-{post_id}-{parent_id.unwrap_or(0)}",
                        checked: consented(),
                        disabled: submitting(),
                        class: "mt-1 rounded border-gray-300 text-paper-accent focus:ring-paper-accent/30",
                        onchange: move |e| consented.set(e.checked()),
                    }
                    label {
                        r#for: "consent-{post_id}-{parent_id.unwrap_or(0)}",
                        class: "text-sm text-paper-secondary select-none cursor-pointer",
                        "同意隐私政策"
                    }
                }

                button {
                    class: BUTTON_PRIMARY_CLASS,
                    disabled: submitting(),
                    onclick: move |_| {
                        let post_id = post_id;
                        let parent_id = parent_id;
                        let name = author_name();
                        let email = author_email();
                        let url_val = author_url();
                        let content = content_md();
                        let hp = honeypot();
                        let consent = consented();

                        if !hp.is_empty() {
                            return;
                        }

                        if name.trim().is_empty() || email.trim().is_empty() || content.trim().is_empty() {
                            message.set(Some(("请填写所有必填项".to_string(), "error")));
                            return;
                        }

                        if !consent {
                            message.set(Some(("请同意隐私政策".to_string(), "error")));
                            return;
                        }

                        submitting.set(true);
                        message.set(None);

                        spawn(async move {
                            let result = create_comment(
                                post_id,
                                parent_id,
                                name,
                                email,
                                if url_val.trim().is_empty() { None } else { Some(url_val) },
                                content,
                                consent,
                            ).await;

                            submitting.set(false);

                            match result {
                                Ok(resp) => {
                                    if resp.success {
                                        content_md.set(String::new());
                                        consented.set(false);
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
