use crate::components::header::NavItemConfig;
use crate::router::Route;

pub fn use_nav_items(route: Route) -> Vec<NavItemConfig> {
    vec![
        NavItemConfig {
            href: "/",
            label: "首页",
            is_active: matches!(route, Route::Home {}),
        },
        NavItemConfig {
            href: "/archives",
            label: "归档",
            is_active: matches!(route, Route::Archives {}),
        },
        NavItemConfig {
            href: "/tags",
            label: "标签",
            is_active: matches!(route, Route::Tags {}) || matches!(route, Route::TagDetail { .. }),
        },
        NavItemConfig {
            href: "/search",
            label: "搜索",
            is_active: matches!(route, Route::Search {}),
        },
        NavItemConfig {
            href: "/about",
            label: "关于",
            is_active: matches!(route, Route::About {}),
        },
    ]
}
