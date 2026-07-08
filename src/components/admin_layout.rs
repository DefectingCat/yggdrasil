//! 后台管理布局组件
//!
//! 提供全新设计的柔和/软扁平化风格的管理员专属后台布局。
//! 采用圆角矩形、大空间距与友好的交互设计。

use dioxus::prelude::*;
use dioxus::router::components::Link;

use crate::api::auth::{get_current_user, logout};
use crate::components::admin_skeleton::AdminDashboardSkeleton;

use crate::components::write_skeleton::WriteSkeleton;
use crate::context::UserContext;
use crate::router::Route;
use crate::theme::ThemeToggle;

#[component]
pub fn AdminLayout() -> Element {
    let mut ctx: UserContext = use_context();
    let navigator = dioxus::router::navigator();
    let route = use_route::<Route>();


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

    let admin_nav_items = vec![
        (Route::Admin {}, "仪表盘"),
        (Route::Write {}, "写文章"),
        (Route::Posts {}, "管理文章"),
        (Route::Runner {}, "试运行"),
        (Route::System {}, "系统"),
    ];

    let is_write_route =
        matches!(route, Route::Write {}) || matches!(route, Route::WriteEdit { .. });
    // 「管理文章」高亮覆盖其下所有子路由：列表、分页、回收站 tab。
    let is_posts_route = matches!(
        route,
        Route::Posts {}
            | Route::PostsPage { .. }
            | Route::PostsTrash {}
            | Route::PostsTrashPage { .. }
    );

    // 所有 admin 页面共用同一 shell:外层圆角卡片(滚动容器) + 内部 main 负责居中限宽。
    // write 路由例外:卡片不滚动(overflow-hidden),main 作为 flex 容器不带头尾 padding,
    // 由 write 页面自身组织 [内容区 flex-1 overflow-y-auto] + [底栏 flex-shrink-0] 的分区布局,
    // 这样底栏永远贴卡片底部不随内容滚动,也不会出现 sticky + 负 margin 的跳动。
    let card_overflow = if is_write_route { "overflow-hidden" } else { "overflow-y-auto" };
    let main_class = if is_write_route {
        "flex-1 w-full max-w-7xl mx-auto flex flex-col min-h-0"
    } else {
        "flex-1 w-full max-w-7xl mx-auto px-10 py-12"
    };

    let root_class = "min-h-dvh flex bg-[var(--color-paper-entry)] text-[var(--color-paper-primary)] font-sans";

    let nav_content = rsx! {
        aside { class: "w-64 flex-shrink-0 hidden md:flex flex-col h-screen sticky top-0 p-6 bg-[var(--color-paper-entry)]",
            // Logo
            div { class: "mb-10 px-4",
                Link {
                    class: "font-extrabold text-2xl tracking-tight text-[var(--color-paper-primary)] hover:text-[var(--color-paper-accent)] transition-colors",
                    to: Route::Home {},
                    "Yggdrasil."
                }
            }
            // Nav Items
            nav { class: "flex-1 flex flex-col gap-2",
                for (dest, label) in admin_nav_items {
                    {
                        let is_active = route == dest
                            || (label == "写文章" && is_write_route)
                            || (label == "管理文章" && is_posts_route);
                        let base_class = "flex items-center px-4 py-3 rounded-2xl text-sm font-medium transition-all";
                        let text_class = if is_active {
                            "bg-[var(--color-paper-theme)] text-[var(--color-paper-primary)] shadow-sm border border-[var(--color-paper-border)]"
                        } else {
                            "text-[var(--color-paper-secondary)] hover:bg-[var(--color-paper-theme)]/50 hover:text-[var(--color-paper-primary)] border border-transparent"
                        };
                        rsx! {
                            Link {
                                key: "{label}",
                                class: "{base_class} {text_class}",
                                to: dest,
                                "{label}"
                            }
                        }
                    }
                }
            }
            // Bottom Tools
            div { class: "mt-auto pt-6 border-t border-[var(--color-paper-border)] flex items-center justify-between px-4",
                ThemeToggle {}
                button {
                    class: "text-sm font-medium px-4 py-2 rounded-xl bg-[var(--color-paper-theme)] border border-[var(--color-paper-border)] shadow-sm hover:shadow-md transition-all text-[var(--color-paper-secondary)] hover:text-red-500 cursor-pointer",
                    onclick: move |_| {
                        spawn(async move {
                            let _ = logout().await;
                            ctx.user.set(None);
                            ctx.checked.set(false);
                            let _ = navigator.push(Route::Login {});
                        });
                    },
                    "退出"
                }
            }
        }
    };

    match ((ctx.checked)(), (ctx.user)()) {
        (true, Some(_)) => {
            rsx! {
                div { class: "{root_class}",
                    {nav_content}
                    div { class: "flex-1 flex flex-col min-w-0 h-screen p-2 md:p-4",
                        div { class: "flex-1 bg-[var(--color-paper-theme)] rounded-[2rem] shadow-sm border border-[var(--color-paper-border)] {card_overflow} relative flex flex-col",
                            main { class: "{main_class}", Outlet::<Route> {} }
                        }
                    }
                }
            }
        }
        _ => {
            rsx! {
                div { class: "{root_class}",
                    {nav_content}
                    div { class: "flex-1 flex flex-col min-w-0 h-screen p-2 md:p-4",
                        div { class: "flex-1 bg-[var(--color-paper-theme)] rounded-[2rem] shadow-sm border border-[var(--color-paper-border)] overflow-hidden relative flex flex-col",
                            main { class: "{main_class}",
                                div { class: "p-10 animate-pulse",
                                    {
                                        match route {
                                            Route::Write {} => rsx! { WriteSkeleton {} },
                                            _ => rsx! { AdminDashboardSkeleton {} },
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
