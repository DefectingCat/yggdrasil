use dioxus::prelude::*;
use dioxus::router::components::Link;

use crate::router::Route;

#[derive(Clone, PartialEq)]
pub struct NavItemConfig {
    pub route: Route,
    pub label: &'static str,
    pub is_active: bool,
}

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
