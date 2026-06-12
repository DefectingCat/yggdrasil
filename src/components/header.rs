//! 顶部导航栏组件
//!
//! 提供站点 Logo、响应式导航菜单项与右侧自定义内容区，
//! 支持前台布局与后台布局复用。

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
#[component]
pub fn Header(nav_items: Vec<NavItemConfig>, right_content: Element) -> Element {
    rsx! {
        header { class: "sticky top-0 z-40 w-full border-b border-paper-border bg-paper-theme/80 backdrop-blur-sm",
            nav { class: "max-w-3xl mx-auto px-6 h-[60px] flex items-center justify-between",
                Link {
                    class: "text-2xl font-bold font-serif text-paper-primary hover:text-paper-accent transition-colors duration-200",
                    to: Route::Home {},
                    "Yggdrasil"
                }
                div { class: "flex items-center gap-2",
                    ul { class: "hidden md:flex items-center gap-1",
                        for item in nav_items.iter().cloned() {
                            NavItem {
                                route: item.route,
                                label: item.label,
                                is_active: item.is_active,
                            }
                        }
                    }
                    {right_content}
                }
            }
        }
    }
}

/// 单个导航项组件，根据 `is_active` 切换高亮样式。
///
/// Props：
/// - `route`：目标路由
/// - `label`：显示文本
/// - `is_active`：是否高亮
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
            Link {
                class: "{class_str}",
                to: route,
                "{label}"
            }
        }
    }
}
