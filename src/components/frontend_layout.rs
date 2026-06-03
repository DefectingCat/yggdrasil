use dioxus::prelude::*;

use crate::components::footer::Footer;
use crate::components::header::Header;
use crate::components::nav::use_nav_items;
use crate::router::Route;
use crate::theme::ThemeToggle;

#[component]
pub fn FrontendLayout() -> Element {
    let route = use_route::<Route>();
    let nav_items = use_nav_items(route);

    rsx! {
        div { class: "min-h-screen flex flex-col bg-white dark:bg-[#1d1e20] transition-colors duration-300",
            Header { nav_items, right_content: rsx! { ThemeToggle {} } }
            main { class: "flex-1 w-full max-w-3xl mx-auto px-6 py-6",
                Outlet::<Route> {}
            }
            Footer {}
        }
    }
}
