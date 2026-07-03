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
use crate::hooks::delayed_loading::use_delayed_loading;
use crate::router::Route;
use crate::theme::ThemeToggle;

#[component]
pub fn AdminLayout() -> Element {
    let mut ctx: UserContext = use_context();
    let navigator = dioxus::router::navigator();
    let route = use_route::<Route>();
    let show_skeleton = use_delayed_loading(move || !(ctx.checked)());

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
        (Route::Trash {}, "回收站"),
        (Route::System {}, "系统"),
    ];

    let is_write_route =
        matches!(route, Route::Write {}) || matches!(route, Route::WriteEdit { .. });

    let main_class = if is_write_route {
        "flex-1 w-full flex flex-col relative"
    } else {
        "flex-1 w-full max-w-7xl mx-auto px-6 py-10"
    };

    let root_class = "min-h-dvh flex flex-col bg-paper-theme text-paper-primary font-sans";

    let nav_content = rsx! {
        header { class: "w-full border-b border-paper-border bg-paper-theme sticky top-0 z-40",
            div { class: "w-full max-w-7xl mx-auto px-6 h-14 flex items-center justify-between",
                div { class: "flex items-center gap-8",
                    // 品牌标识 / 回前台
                    Link {
                        class: "font-bold text-lg hover:text-[var(--color-paper-accent)] transition-colors tracking-tight",
                        to: Route::Home {},
                        "Yggdrasil"
                    }
                    // 导航链接
                    nav { class: "hidden md:flex items-center gap-6",
                        for (dest, label) in admin_nav_items {
                            {
                                let is_active = route == dest || (label == "写文章" && is_write_route) || (label == "回收站" && matches!(route, Route::TrashPage { .. }));
                                let text_class = if is_active { "text-paper-primary" } else { "text-paper-secondary hover:text-paper-primary" };
                                rsx! {
                                    Link {
                                        key: "{label}",
                                        class: "text-sm font-medium transition-colors {text_class}",
                                        to: dest,
                                        "{label}"
                                    }
                                }
                            }
                        }
                    }
                }
                // 右侧操作
                div { class: "flex items-center gap-4",
                    ThemeToggle {}
                    button {
                        class: "text-sm font-medium px-4 py-1.5 bg-[var(--color-paper-entry)] border border-[var(--color-paper-border)] rounded-full shadow-sm hover:shadow-md transition-all cursor-pointer text-[var(--color-paper-secondary)] hover:text-[var(--color-paper-primary)]",
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
            }
        }
    };

    match ((ctx.checked)(), (ctx.user)()) {
        (true, Some(_)) => {
            rsx! {
                div { class: "{root_class}",
                    {nav_content}
                    main { class: "{main_class}", Outlet::<Route> {} }
                }
            }
        }
        (true, None) => {
            rsx! {
                div { class: "{root_class}",
                    div { class: "flex-1 flex items-center justify-center font-medium text-sm text-[var(--color-paper-secondary)]",
                        "正在验证身份..."
                    }
                }
            }
        }
        (false, _) => {
            rsx! {
                div { class: "{root_class}",
                    {nav_content}
                    main { class: "{main_class}",
                        div { class: if show_skeleton() { "" } else { "opacity-0" },
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
