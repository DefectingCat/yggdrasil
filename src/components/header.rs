//! 顶部导航栏组件
//!
//! 提供站点 Logo、响应式导航菜单项与右侧自定义内容区，
//! 支持前台布局与后台布局复用，并包含小屏幕下的汉堡菜单。

use dioxus::prelude::*;
use dioxus::router::components::Link;

use crate::router::Route;

/// 导航项配置，用于描述 Header 中的一个链接。
///
/// 字段：
/// - `route`：目标路由
/// - `label`：显示文本
/// - `is_active`：当前是否处于激活状态
#[derive(Clone, PartialEq)]
pub struct NavItemConfig {
    /// 目标路由。
    pub route: Route,
    /// 显示文本。
    pub label: &'static str,
    /// 当前是否处于激活状态。
    pub is_active: bool,
}

/// 顶部导航栏组件。
///
/// Props：
/// - `nav_items`：导航项列表
/// - `right_content`：右侧自定义内容（如主题切换、登出按钮）
/// - `max_width`：内部导航的宽度类，需与正文 `max-w-*` 一致以保证左右边缘对齐。
///   默认 `max-w-3xl`（前台阅读宽度）；后台传 `max-w-5xl` 与之同宽。
#[component]
pub fn Header(
    nav_items: Vec<NavItemConfig>,
    right_content: Element,
    #[props(default = "max-w-3xl")] max_width: &'static str,
) -> Element {
    let mut mobile_open = use_signal(|| false);
    let menu_id = use_memo(|| "mobile-nav-menu".to_string());

    rsx! {
        header { class: "sticky top-0 z-40 w-full border-b border-paper-border bg-paper-theme/80 backdrop-blur-sm",
            nav { class: "{max_width} mx-auto px-6 h-[60px] flex items-center justify-between",
                Link {
                    class: "text-2xl font-bold font-serif text-paper-primary hover:text-paper-accent transition-colors duration-200",
                    to: Route::Home {},
                    "Yggdrasil"
                }
                div { class: "flex items-center gap-2",
                    // 桌面端导航
                    ul { class: "hidden md:flex items-center gap-1",
                        for item in nav_items.iter().cloned() {
                            NavItem {
                                key: "{item.label}",
                                route: item.route,
                                label: item.label,
                                is_active: item.is_active,
                            }
                        }
                    }

                    {right_content}

                    // 移动端汉堡菜单按钮
                    button {
                        class: "md:hidden p-2 rounded-lg text-paper-secondary hover:text-paper-primary hover:bg-paper-entry transition-colors",
                        r#type: "button",
                        aria_label: "切换导航菜单",
                        aria_expanded: "{mobile_open()}",
                        aria_controls: "{menu_id()}",
                        onclick: move |_| mobile_open.set(!mobile_open()),
                        if mobile_open() {
                            // 关闭图标（X）
                            svg {
                                class: "w-6 h-6",
                                fill: "none",
                                stroke: "currentColor",
                                view_box: "0 0 24 24",
                                path {
                                    stroke_linecap: "round",
                                    stroke_linejoin: "round",
                                    stroke_width: "2",
                                    d: "M6 18L18 6M6 6l12 12",
                                }
                            }
                        } else {
                            // 汉堡图标
                            svg {
                                class: "w-6 h-6",
                                fill: "none",
                                stroke: "currentColor",
                                view_box: "0 0 24 24",
                                path {
                                    stroke_linecap: "round",
                                    stroke_linejoin: "round",
                                    stroke_width: "2",
                                    d: "M4 6h16M4 12h16M4 18h16",
                                }
                            }
                        }
                    }
                }
            }

            // 移动端导航面板
            if mobile_open() {
                div {
                    id: "{menu_id()}",
                    class: "md:hidden border-t border-paper-border bg-paper-theme/95 backdrop-blur-sm",
                    ul { class: "py-2 px-6 space-y-1",
                        for item in nav_items.iter().cloned() {
                            li { key: "{item.label}",
                                MobileNavItem {
                                    route: item.route,
                                    label: item.label,
                                    is_active: item.is_active,
                                    on_navigate: move |_| mobile_open.set(false),
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// 单个桌面导航项组件，根据 `is_active` 切换高亮样式。
#[component]
fn NavItem(route: Route, label: &'static str, is_active: bool) -> Element {
    let base_class = "px-3 py-1 text-base rounded-lg transition-all duration-200";
    let class_str = if is_active {
        format!("{} font-medium text-paper-accent underline underline-offset-[0.3rem] decoration-2 decoration-paper-accent", base_class)
    } else {
        format!(
            "{} text-paper-secondary hover:text-paper-primary",
            base_class
        )
    };

    rsx! {
        li {
            Link { class: "{class_str}", to: route, "{label}" }
        }
    }
}

/// 单个移动端导航项组件，点击后关闭菜单。
#[component]
fn MobileNavItem(
    route: Route,
    label: &'static str,
    is_active: bool,
    on_navigate: EventHandler<()>,
) -> Element {
    let class_str = if is_active {
        "block w-full px-3 py-2 text-base font-medium text-paper-accent rounded-lg bg-paper-entry"
    } else {
        "block w-full px-3 py-2 text-base text-paper-secondary hover:text-paper-primary hover:bg-paper-entry rounded-lg transition-colors"
    };

    rsx! {
        Link {
            class: "{class_str}",
            to: route,
            onclick: move |_| on_navigate.call(()),
            "{label}"
        }
    }
}
