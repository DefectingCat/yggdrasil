//! 前台导航项配置
//!
//! 根据当前路由生成前台 Header 所需的导航项列表。

use crate::components::header::NavItemConfig;
use crate::router::Route;

/// 生成前台导航项列表，当前访问的路由会被标记为激活。
///
/// 参数：
/// - `route`：当前路由
///
/// 返回：包含首页、归档、标签、搜索、关于的导航配置数组。
pub fn use_nav_items(route: Route) -> Vec<NavItemConfig> {
    vec![
        NavItemConfig {
            route: Route::Home {},
            label: "首页",
            is_active: matches!(route, Route::Home {}),
        },
        NavItemConfig {
            route: Route::Archives {},
            label: "归档",
            is_active: matches!(route, Route::Archives {}),
        },
        NavItemConfig {
            route: Route::Tags {},
            label: "标签",
            is_active: matches!(route, Route::Tags {}) || matches!(route, Route::TagDetail { .. }),
        },
        NavItemConfig {
            route: Route::Search {},
            label: "搜索",
            is_active: matches!(route, Route::Search {}),
        },
        NavItemConfig {
            route: Route::About {},
            label: "关于",
            is_active: matches!(route, Route::About {}),
        },
    ]
}
