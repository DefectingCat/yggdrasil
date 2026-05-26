use dioxus::prelude::*;

use crate::pages::home::{Post, POSTS};

#[component]
pub fn AdminPage() -> Element {
    rsx! {
        div { class: "space-y-8",
            // 统计卡片
            div { class: "grid grid-cols-1 md:grid-cols-3 gap-6",
                StatCard { value: POSTS.len().to_string(), label: "文章总数" }
                StatCard { value: "0".to_string(), label: "草稿数" }
                StatCard { value: POSTS.len().to_string(), label: "已发布" }
            }

            // 快捷操作
            div { class: "grid grid-cols-1 md:grid-cols-2 gap-4",
                button {
                    class: "bg-gray-900 dark:bg-[#dadadb] text-white dark:text-gray-900 rounded-full px-6 py-3 text-center font-medium hover:opacity-80 transition-opacity cursor-pointer",
                    onclick: move |_| {
                        dioxus::router::navigator().push("/admin/write");
                    },
                    "写文章"
                }
                button {
                    class: "bg-gray-200 dark:bg-[#333] text-gray-700 dark:text-[#dadadb] rounded-full px-6 py-3 text-center font-medium hover:opacity-80 transition-opacity",
                    onclick: move |_| {
                        #[cfg(target_arch = "wasm32")]
                        web_sys::window().map(|w| w.alert_with_message("开发中").ok());
                    },
                    "管理文章"
                }
            }

            // 最近文章
            div { class: "mb-8",
                h2 { class: "text-xl font-bold text-gray-900 dark:text-[#dadadb] mb-4",
                    "最近文章"
                }
                div { class: "space-y-0",
                    for post in POSTS.iter().take(5) {
                        RecentPostItem { post: post.clone() }
                    }
                }
            }
        }
    }
}

#[component]
fn StatCard(value: String, label: String) -> Element {
    rsx! {
        div { class: "rounded-xl bg-white dark:bg-[#2e2e33] border border-gray-200 dark:border-[#333] p-6 text-center",
            div { class: "text-3xl font-bold text-gray-900 dark:text-[#dadadb]",
                "{value}"
            }
            div { class: "text-sm text-gray-500 dark:text-[#9b9c9d] mt-2",
                "{label}"
            }
        }
    }
}

#[component]
fn RecentPostItem(post: Post) -> Element {
    rsx! {
        div { class: "flex justify-between items-center py-3 border-b border-gray-100 dark:border-[#333]",
            span { class: "text-gray-700 dark:text-[#dadadb]",
                "{post.title}"
            }
            span { class: "text-sm text-gray-400 dark:text-[#9b9c9d]",
                "{post.date}"
            }
        }
    }
}
