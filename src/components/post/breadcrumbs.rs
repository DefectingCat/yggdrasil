use dioxus::prelude::*;

#[component]
pub fn Breadcrumbs(title: String) -> Element {
    rsx! {
        nav {
            class: "breadcrumbs",
            role: "navigation",
            aria_label: "Breadcrumb",
            a {
                href: "/",
                onclick: move |evt| {
                    evt.prevent_default();
                    dioxus::router::navigator().push("/");
                },
                "Home"
            }
            svg {
                xmlns: "http://www.w3.org/2000/svg",
                view_box: "0 0 24 24",
                fill: "none",
                stroke: "currentColor",
                stroke_width: "2",
                stroke_linecap: "round",
                stroke_linejoin: "round",
                class: "feather feather-chevron-right",
                width: "16",
                height: "16",
                polyline { points: "9 18 15 12 9 6" }
            }
            span { "{title}" }
        }
    }
}
