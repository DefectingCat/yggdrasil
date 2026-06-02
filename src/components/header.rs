use dioxus::prelude::*;

#[derive(Clone, PartialEq)]
pub struct NavItemConfig {
    pub href: &'static str,
    pub label: &'static str,
    pub is_active: bool,
}

#[component]
pub fn Header(nav_items: Vec<NavItemConfig>, right_content: Element) -> Element {
    rsx! {
        header { class: "sticky top-0 z-40 w-full border-b border-gray-200 dark:border-[#333] bg-white/80 dark:bg-[#1d1e20]/80 backdrop-blur-sm",
            nav { class: "max-w-3xl mx-auto px-6 h-[60px] flex items-center justify-between",
                a {
                    class: "text-2xl font-bold text-gray-900 dark:text-[#dadadb] hover:opacity-80 transition-opacity",
                    href: "/",
                    onclick: move |evt| {
                        evt.prevent_default();
                        dioxus::router::navigator().push("/");
                    },
                    "Yggdrasil"
                }
                div { class: "flex items-center gap-2",
                    ul { class: "hidden md:flex items-center gap-1",
                        for item in nav_items.iter().cloned() {
                            NavItem {
                                href: item.href,
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
fn NavItem(href: &'static str, label: &'static str, is_active: bool) -> Element {
    let base_class = "px-3 py-1 text-base rounded-lg transition-colors";
    let class_str = if is_active {
        format!("{} font-medium text-gray-900 dark:text-[#dadadb] underline underline-offset-[0.3rem] decoration-2 decoration-gray-900 dark:decoration-[#dadadb]", base_class)
    } else {
        format!(
            "{} text-gray-600 dark:text-[#9b9c9d] hover:text-gray-900 dark:hover:text-[#dadadb]",
            base_class
        )
    };

    let href = href;
    rsx! {
        li {
            a {
                class: "{class_str}",
                href: "{href}",
                onclick: move |evt| {
                    evt.prevent_default();
                    dioxus::router::navigator().push(href);
                },
                "{label}"
            }
        }
    }
}
