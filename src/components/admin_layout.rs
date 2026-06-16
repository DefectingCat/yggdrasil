//! 后台管理布局组件
//!
//! 包裹所有后台路由，提供管理员专属导航、登录校验、主题切换与登出入口。
//! 在未完成身份校验前显示与真实布局结构一致的骨架屏，避免切换闪烁。

use dioxus::prelude::*;

use crate::api::auth::{get_current_user, logout};
use crate::components::admin_skeleton::AdminDashboardSkeleton;
use crate::components::footer::Footer;
use crate::components::header::{Header, NavItemConfig};
use crate::components::write_skeleton::WriteSkeleton;
use crate::context::UserContext;
use crate::hooks::delayed_loading::use_delayed_loading;
use crate::router::Route;
use crate::theme::ThemeToggle;

/// 后台管理整体布局组件。
///
/// 负责：
/// - 通过 `get_current_user` 校验登录状态，未登录时跳转登录页
/// - 渲染顶部导航（仪表盘、写文章、管理文章）与主题切换/登出按钮
/// - 根据当前路由切换主区域样式（Write 路由固定高度，其他路由可滚动）
/// - 校验完成前使用骨架屏保持布局稳定
#[component]
pub fn AdminLayout() -> Element {
    let mut ctx: UserContext = use_context();
    let navigator = dioxus::router::navigator();
    let route = use_route::<Route>();
    let show_skeleton = use_delayed_loading(move || !(ctx.checked)());

    // 仅在首次挂载时执行一次登录校验
    use_effect(move || {
        if !(ctx.checked)() {
            (ctx.checked).set(true);
            spawn(async move {
                match get_current_user().await {
                    Ok(response) => {
                        if let Some(user) = response.user {
                            ctx.user.set(Some(std::sync::Arc::new(user)));
                        } else {
                            let _ = navigator.push(Route::Login {});
                        }
                    }
                    Err(_) => {
                        let _ = navigator.push(Route::Login {});
                    }
                }
            });
        }
    });

    // 后台导航项，当前路由高亮
    let admin_nav_items = vec![
        NavItemConfig {
            route: Route::Admin {},
            label: "仪表盘",
            is_active: matches!(route, Route::Admin {}),
        },
        NavItemConfig {
            route: Route::Write {},
            label: "写文章",
            is_active: matches!(route, Route::Write {}) || matches!(route, Route::WriteEdit { .. }),
        },
        NavItemConfig {
            route: Route::Posts {},
            label: "管理文章",
            is_active: matches!(route, Route::Posts {}),
        },
        NavItemConfig {
            route: Route::Trash {},
            label: "回收站",
            is_active: matches!(route, Route::Trash {}) || matches!(route, Route::TrashPage { .. }),
        },
    ];

    // 右侧操作区：主题切换 + 登出按钮
    let right_content = rsx! {
        div { class: "flex items-center gap-3",
            ThemeToggle {}
            button {
                class: "text-sm text-gray-600 dark:text-[#9b9c9d] hover:text-gray-900 dark:hover:text-[#dadadb] transition-colors",
                onclick: move |_| {
                    spawn(async move {
                        let _ = logout().await;
                        ctx.user.set(None);
                        ctx.checked.set(false);
                        let _ = navigator.push(Route::Login {});
                    });
                },
                "登出"
            }
        }
    };

    let is_write_route =
        matches!(route, Route::Write {}) || matches!(route, Route::WriteEdit { .. });
    let main_class = if is_write_route {
        "flex-1 w-full max-w-5xl mx-auto px-6 flex flex-col overflow-hidden"
    } else {
        "flex-1 w-full max-w-5xl mx-auto px-6 py-8"
    };

    // Write 路由：页面固定高度，不滚动，由编辑器内部处理滚动
    let root_class = if is_write_route {
        "h-dvh flex flex-col overflow-hidden bg-white dark:bg-[#1d1e20]"
    } else {
        "min-h-screen flex flex-col bg-white dark:bg-[#1d1e20]"
    };

    // 根据校验状态与用户状态渲染真实布局、跳转提示或骨架屏
    match ((ctx.checked)(), (ctx.user)()) {
        (true, Some(_)) => {
            rsx! {
                div { class: "{root_class}",
                    Header { nav_items: admin_nav_items, right_content: right_content }
                    main { class: "{main_class}",
                        Outlet::<Route> {}
                    }
                    Footer {}
                }
            }
        }
        (true, None) => {
            rsx! {
                div { class: "{root_class}",
                    div { class: "flex-1 flex items-center justify-center",
                        p { class: "text-gray-600 dark:text-[#9b9c9d]", "未登录，正在跳转..." }
                    }
                }
            }
        }
        (false, _) => {
            // 使用与真实布局完全相同的结构包裹内容骨架，避免 checked 变化时的布局闪烁
            rsx! {
                div { class: "{root_class}",
                    Header { nav_items: admin_nav_items, right_content: right_content }
                    main { class: "{main_class}",
                        div { class: if show_skeleton() { "" } else { "opacity-0" },
                            {match route {
                                Route::Write {} => rsx! { WriteSkeleton {} },
                                _ => rsx! { AdminDashboardSkeleton {} },
                            }}
                        }
                    }
                    Footer {}
                }
            }
        }
    }
}
