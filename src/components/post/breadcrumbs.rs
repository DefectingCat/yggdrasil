//! 面包屑组件
//!
//! 在文章详情页展示从首页到当前文章标题的导航路径。

use dioxus::prelude::*;
use dioxus::router::components::Link;

use crate::router::Route;

/// 面包屑导航组件。
///
/// Props：
/// - `title`：当前文章标题
///
/// 渲染 `Home > 当前标题` 的面包屑路径。
#[component]
pub fn Breadcrumbs(title: String) -> Element {
    rsx! {
        nav {
            class: "breadcrumbs",
            role: "navigation",
            aria_label: "Breadcrumb",
            Link {
                to: Route::Home {},
                "Home"
            }
            svg {
                xmlns: "http://www.w3.org/2000/svg",
                view_box: "0 0 24 24",
                fill: "none",
                stroke: "currentColor",
                stroke_width: "2",
                stroke_linecap: "round",
                stroke_linejoin: "round",
                class: "feather feather-chevron-right",
                width: "16",
                height: "16",
                polyline { points: "9 18 15 12 9 6" }
            }
            span { "{title}" }
        }
    }
}
