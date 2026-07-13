//! 文章编辑器页骨架屏
//!
//! 在写文章/编辑文章页面加载时展示，镜像 Write 页面的左右两栏结构：
//! 左栏(标题+编辑器) + 右栏(链接/标签/摘要/封面) + 底部操作栏。

use crate::components::skeletons::atoms::*;
use dioxus::prelude::*;

/// 文章编辑器页骨架屏组件。
///
/// 镜像 Write 页面的左右两栏布局：左栏主写作区(标题、编辑器)，
/// 右栏侧边栏(链接、标签、摘要、封面图)，底部贴底操作栏。
#[component]
pub fn WriteSkeleton() -> Element {
    rsx! {
        // 根:父层是 flex 容器(write.rs 覆盖层 / admin_layout 包裹层均为 flex flex-col),
        // 用 flex-1 撑满父层高度(比 height:100% 更可靠,不依赖父显式 height)。
        div { class: "relative flex-1 flex flex-col min-h-0 overflow-hidden",
            // 两栏容器:与真实页面一致,左 flex-1 + 右 w-80
            div { class: "flex-1 min-h-0 flex",
                // 左栏(主写作区)
                div { class: "flex-1 min-w-0 min-h-0 overflow-y-auto px-6 py-8 flex flex-col",
                    // 标题输入
                    SkeletonBox { class: "h-10 w-3/4 rounded-lg" }

                    // 编辑器区域:flex-1 撑满左栏剩余高度
                    div { class: "flex-1 min-h-[400px] flex flex-col my-4",
                        div { class: "flex-1 min-h-0 w-full border border-[var(--color-paper-border)] rounded-2xl overflow-hidden bg-[var(--color-paper-entry)] shadow-sm p-4",
                            // 工具栏
                            div { class: "flex gap-2 pb-3 border-b border-[var(--color-paper-border)]",
                                for _ in 0..8 {
                                    SkeletonBox { class: "w-8 h-8 rounded" }
                                }
                            }
                            // 正文占位行
                            div { class: "space-y-3 pt-4",
                                SkeletonBox { class: "h-4 w-[90%] rounded" }
                                SkeletonBox { class: "h-4 w-full rounded" }
                                SkeletonBox { class: "h-4 w-[85%] rounded" }
                                SkeletonBox { class: "h-4 w-[95%] rounded" }
                                SkeletonBox { class: "h-4 w-[60%] rounded" }
                                SkeletonBox { class: "h-4 w-full rounded" }
                                SkeletonBox { class: "h-4 w-[75%] rounded" }
                            }
                        }
                    }
                }

                // 右栏(侧边栏):w-80 border-l,分节式
                div { class: "w-80 flex-shrink-0 min-h-0 overflow-y-auto border-l border-[var(--color-paper-border)] flex flex-col",
                    // 链接节
                    div { class: "p-6 border-b border-[var(--color-paper-border)]",
                        SkeletonBox { class: "h-3 w-10 rounded mb-3" }
                        SkeletonBox { class: "h-8 w-full rounded-2xl" }
                    }
                    // 标签节
                    div { class: "p-6 border-b border-[var(--color-paper-border)]",
                        SkeletonBox { class: "h-3 w-10 rounded mb-3" }
                        SkeletonBox { class: "h-8 w-full rounded-2xl" }
                    }
                    // 摘要节
                    div { class: "p-6 border-b border-[var(--color-paper-border)]",
                        SkeletonBox { class: "h-3 w-10 rounded mb-3" }
                        SkeletonBox { class: "h-16 w-full rounded-2xl" }
                    }
                    // 封面图节
                    div { class: "p-6",
                        SkeletonBox { class: "h-3 w-12 rounded mb-3" }
                        SkeletonBox { class: "h-14 w-full rounded-2xl" }
                    }
                }
            }

            // 底部操作栏:与真实页面 px-6 py-4 border-t 一致
            div { class: "flex-shrink-0 px-6 py-4 flex items-center gap-4 border-t border-[var(--color-paper-border)]",
                SkeletonBox { class: "h-9 w-[56px] rounded-full" }
                div { class: "w-px h-5 bg-[var(--color-paper-border)]" }
                SkeletonBox { class: "h-9 w-[96px] rounded-full" }
                div { class: "w-px h-5 bg-[var(--color-paper-border)]" }
                SkeletonBox { class: "h-9 w-[72px] rounded-full" }
            }
        }
    }
}
