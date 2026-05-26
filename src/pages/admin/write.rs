use dioxus::prelude::*;

use crate::components::admin_layout::AdminLayout;

fn markdown_to_html(input: &str) -> String {
    let parser = pulldown_cmark::Parser::new(input);
    let mut html = String::new();
    pulldown_cmark::html::push_html(&mut html, parser);
    html
}

#[component]
pub fn WritePage() -> Element {
    let mut title = use_signal(|| "".to_string());
    let mut content = use_signal(|| "".to_string());
    let preview_html = use_memo(move || {
        markdown_to_html(&content())
    });

    rsx! {
        AdminLayout {
            div { class: "space-y-4",
                // 标题输入
                input {
                    class: "w-full text-2xl font-bold bg-transparent border-b border-gray-200 dark:border-[#333] py-2 mb-4 text-gray-900 dark:text-[#dadadb] placeholder-gray-400 dark:placeholder-[#9b9c9d] focus:outline-none",
                    placeholder: "文章标题...",
                    value: "{title}",
                    oninput: move |evt| title.set(evt.value()),
                }

                // 两栏布局
                div { class: "grid grid-cols-1 md:grid-cols-2 gap-6",
                    // 编辑区
                    div { class: "space-y-2",
                        label { class: "text-sm text-gray-500 dark:text-[#9b9c9d]",
                            "Markdown"
                        }
                        textarea {
                            class: "w-full h-[500px] bg-gray-50 dark:bg-[#2e2e33] rounded-lg p-4 font-mono text-sm text-gray-800 dark:text-[#dadadb] placeholder-gray-400 dark:placeholder-[#9b9c9d] border border-gray-200 dark:border-[#333] focus:outline-none focus:border-gray-400 dark:focus:border-gray-600 resize-none",
                            placeholder: "在此输入 Markdown...",
                            value: "{content}",
                            oninput: move |evt| content.set(evt.value()),
                        }
                    }

                    // 预览区
                    div { class: "space-y-2",
                        label { class: "text-sm text-gray-500 dark:text-[#9b9c9d]",
                            "预览"
                        }
                        div {
                            class: "w-full h-[500px] overflow-y-auto bg-white dark:bg-[#2e2e33] rounded-lg p-4 border border-gray-200 dark:border-[#333] prose dark:prose-invert max-w-none",
                            dangerous_inner_html: "{preview_html}",
                        }
                    }
                }

                // 保存按钮
                button {
                    class: "mt-4 px-6 py-2 bg-gray-900 dark:bg-[#dadadb] text-white dark:text-gray-900 rounded-full font-medium hover:opacity-80 transition-opacity",
                    onclick: move |_| {
                        let t = title();
                        let c = content();
                        println!("保存文章: title={}, content_len={}", t, c.len());
                    },
                    "保存草稿"
                }
            }
        }
    }
}
