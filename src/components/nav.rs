use crate::components::header::NavItemConfig;
use crate::router::Route;

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
