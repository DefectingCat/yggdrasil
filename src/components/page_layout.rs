use dioxus::prelude::*;

use crate::components::footer::Footer;
use crate::components::header::{Header, NavItemConfig};
use crate::theme::ThemeToggle;

#[component]
pub fn PageLayout(nav_items: Vec<NavItemConfig>, children: Element) -> Element {
    rsx! {
        div { class: "min-h-screen flex flex-col bg-white dark:bg-[#1d1e20] transition-colors duration-300",
            Header { nav_items, right_content: rsx! { ThemeToggle {} } }
            main { class: "flex-1 w-full max-w-3xl mx-auto px-6 py-6",
                {children}
            }
            Footer {}
        }
    }
}
