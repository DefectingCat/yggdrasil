use dioxus::prelude::*;

use crate::api::auth::{get_current_user, logout};
use crate::components::header::{Header, NavItemConfig};
use crate::components::footer::Footer;
use crate::components::admin_skeleton::{AdminSkeleton, AdminDashboardSkeleton};
use crate::components::write_skeleton::WriteSkeleton;
use crate::context::UserContext;
use crate::router::Route;

#[component]
pub fn AdminLayout() -> Element {
    let mut ctx: UserContext = use_context();
    let navigator = dioxus::router::navigator();
    let route = use_route::<Route>();

    // 只在首次挂载时加载用户数据
    use_effect(move || {
        if !(ctx.checked)() {
            (ctx.checked).set(true);
            spawn(async move {
                match get_current_user().await {
                    Ok(response) => {
                        if let Some(user) = response.user {
                            ctx.user.set(Some(std::sync::Arc::new(user)));
                        } else {
                            let _ = navigator.push("/login");
                        }
                    }
                    Err(_) => {
                        let _ = navigator.push("/login");
                    }
                }
            });
        }
    });

    let admin_nav_items = vec![
        NavItemConfig {
            href: "/admin",
            label: "仪表盘",
            is_active: matches!(route, Route::AdminPage {}),
        },
        NavItemConfig {
            href: "/admin/write",
            label: "写文章",
            is_active: matches!(route, Route::WritePage {}),
        },
        NavItemConfig {
            href: "/",
            label: "前台",
            is_active: false,
        },
    ];

    let nav = navigator.clone();
    let logout_button = rsx! {
        button {
            class: "text-sm text-gray-600 dark:text-[#9b9c9d] hover:text-gray-900 dark:hover:text-[#dadadb] transition-colors",
            onclick: move |_| {
                let nav = nav.clone();
                spawn(async move {
                    let _ = logout().await;
                    let _ = nav.push("/login");
                });
            },
            "登出"
        }
    };

    match ((ctx.checked)(), (ctx.user)()) {
        (true, Some(_)) => {
            rsx! {
                div { class: "min-h-screen flex flex-col bg-white dark:bg-[#1d1e20]",
                    Header { nav_items: admin_nav_items, right_content: logout_button }
                    main { class: "flex-1 w-full max-w-5xl mx-auto px-6 py-8",
                        Outlet::<Route> {}
                    }
                    Footer {}
                }
            }
        }
        (true, None) => {
            rsx! {
                div { class: "min-h-screen flex items-center justify-center bg-white dark:bg-[#1d1e20]",
                    p { class: "text-gray-600 dark:text-[#9b9c9d]", "未登录，正在跳转..." }
                }
            }
        }
        (false, _) => {
            // 使用与真实布局完全相同的结构包裹内容骨架，避免 checked 变化时的布局闪烁
            rsx! {
                div { class: "min-h-screen flex flex-col bg-white dark:bg-[#1d1e20]",
                    Header { nav_items: admin_nav_items, right_content: logout_button }
                    main { class: "flex-1 w-full max-w-5xl mx-auto px-6 py-8",
                        {match route {
                            Route::WritePage {} => rsx! { WriteSkeleton {} },
                            _ => rsx! { AdminDashboardSkeleton {} },
                        }}
                    }
                    Footer {}
                }
            }
        }
    }
}
