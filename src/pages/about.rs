//! 关于页面模块。
//!
//! 对应路由 `/about`。
//!
//! 该页面为静态展示页面，不发起任何 server function 调用。
//! 页面刻意不做常规的站点简介 / 技术栈 / 联系方式罗列，而是围绕一个呼应展开：
//! 「世界……遗忘我……」是大慈树王把自己从世界树（Irminsul）中抹除时的请求，
//! 而本站名为 Yggdrasil——北欧神话中的世界树，记忆的容器。
//! 书写即对遗忘的抵抗：世界遗忘的，树记得。
//!
//! 文末的「年轮」小节收录站点元链接（如 rustdoc 站点文档），
//! 向 [`LINKS`] 数组追加一行即可新增条目。

use dioxus::prelude::*;

/// 「年轮」链接列表：`(URL, 名称, 一句描述)`。
///
/// 这些目标均在 Dioxus SPA 路由之外（静态文件或站外地址），
/// 因此渲染为普通 `<a target="_blank">` 而非 `Link`——
/// `Link` 只接受 `Route` 枚举，指向未注册路径会被路由兜进 404。
const LINKS: &[(&str, &str, &str)] = &[(
    "/doc/yggdrasil/index.html",
    "站点文档",
    "这棵树是如何长成的",
)];

/// 关于页面组件，对应路由 `/about`。
///
/// 静态三段式结构：引言区（引文 + 落款 + 分隔线）→ 短文 → 「年轮」链接列表。
/// 全部内容编译期确定，无 signal、无副作用。
#[component]
pub fn About() -> Element {
    rsx! {
        div { class: "animate-page-enter",
            header { class: "page-header mb-6",
                h1 { class: "text-4xl font-bold text-paper-primary tracking-tight",
                    "关于"
                }
            }

            // 引言区：引文即页面的视觉锚点，仪式感与 404 页保持一致
            div { class: "text-center py-12 md:py-16",
                blockquote { class: "text-2xl md:text-4xl font-medium text-paper-primary leading-relaxed tracking-wide",
                    "世界……遗忘我……"
                }
                p { class: "mt-6 text-sm text-paper-tertiary",
                    "—— 大慈树王"
                }
                div { class: "w-12 h-px bg-paper-border mx-auto mt-10" }
            }

            // 短文：对引文的回应，「树记得」落回站名
            div { class: "max-w-xl mx-auto text-center space-y-4 text-paper-secondary leading-loose",
                p { "Yggdrasil，北欧神话中的世界树。根须贯穿九界，枝叶承载记忆。" }
                p {
                    "人会遗忘，也终将被遗忘——而写下的不会。"
                    span { class: "text-paper-primary", "世界遗忘的，树记得。" }
                }
            }

            // 年轮：站点元链接。行样式镜像归档页的安静感（发丝线 + 排版层级），
            // hover 时名称转为全站唯一强调色，箭头轻移作为确认。
            div { class: "mt-16 md:mt-20",
                p { class: "text-center text-sm font-medium tracking-[0.2em] text-paper-tertiary mb-2",
                    "年轮"
                }
                div { class: "max-w-xl mx-auto",
                    for (href, name, desc) in LINKS.iter().copied() {
                        a {
                            key: "{href}",
                            href: href,
                            target: "_blank",
                            rel: "noopener noreferrer",
                            class: "group flex items-baseline justify-between gap-4 py-3 border-b border-paper-border/50",
                            span { class: "flex items-baseline gap-1.5 text-paper-primary font-medium group-hover:text-paper-accent transition-colors",
                                "{name}"
                                svg {
                                    xmlns: "http://www.w3.org/2000/svg",
                                    width: "14",
                                    height: "14",
                                    view_box: "0 0 24 24",
                                    fill: "none",
                                    stroke: "currentColor",
                                    stroke_width: "2",
                                    stroke_linecap: "round",
                                    stroke_linejoin: "round",
                                    class: "text-paper-tertiary group-hover:text-paper-accent group-hover:-translate-y-0.5 group-hover:translate-x-0.5 transition-all",
                                    path { d: "M7 17L17 7M7 7h10v10" }
                                }
                            }
                            span { class: "text-sm text-paper-secondary", "{desc}" }
                        }
                    }
                }
            }
        }
    }
}
