use dioxus::prelude::*;

use crate::components::footer::Footer;
use crate::components::header::Header;
use crate::components::nav::use_nav_items;
use crate::components::skeletons::archive_skeleton::ArchiveSkeleton;
use crate::components::skeletons::delayed_skeleton::DelayedSkeleton;
use crate::components::skeletons::home_skeleton::HomeSkeleton;
use crate::components::skeletons::post_detail_skeleton::PostDetailSkeleton;
use crate::components::skeletons::search_skeleton::SearchSkeleton;
use crate::components::skeletons::tags_skeleton::TagsSkeleton;
use crate::router::Route;
use crate::theme::ThemeToggle;

fn route_skeleton(route: &Route) -> Element {
    match route {
        Route::Archives {} => rsx! { DelayedSkeleton { ArchiveSkeleton {} } },
        Route::Tags {} | Route::TagDetail { .. } => rsx! { DelayedSkeleton { TagsSkeleton {} } },
        Route::Search {} => rsx! { DelayedSkeleton { SearchSkeleton {} } },
        Route::PostDetail { .. } => rsx! { DelayedSkeleton { PostDetailSkeleton {} } },
        _ => rsx! { DelayedSkeleton { HomeSkeleton {} } },
    }
}

#[component]
pub fn FrontendLayout() -> Element {
    let route = use_route::<Route>();
    let nav_items = use_nav_items(route.clone());

    rsx! {
        div { class: "min-h-screen flex flex-col bg-white dark:bg-[#1d1e20]",
            Header { nav_items, right_content: rsx! { ThemeToggle {} } }
            main { class: "flex-1 w-full max-w-3xl mx-auto px-6 py-6",
                SuspenseBoundary {
                    fallback: move |_| route_skeleton(&route),
                    Outlet::<Route> {}
                }
            }
            Footer {}
        }
    }
}
