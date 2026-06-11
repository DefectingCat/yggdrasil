use dioxus::prelude::*;

use crate::api::comments::{approve_comment, spam_comment, trash_comment};
use crate::components::comments::section::CommentContext;

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
