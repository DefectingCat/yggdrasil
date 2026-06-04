use dioxus::prelude::*;
use dioxus::router::components::Link;

use crate::api::posts::{list_published_posts, PostListResponse};
use crate::components::skeletons::delayed_skeleton::DelayedSkeleton;
use crate::components::skeletons::archive_skeleton::ArchiveSkeleton;
use crate::models::post::Post;
use crate::router::Route;

#[derive(Clone, PartialEq)]
struct YearGroup {
    year: String,
    months: Vec<MonthGroup>,
}

#[derive(Clone, PartialEq)]
struct MonthGroup {
    month: String,
    month_en: String,
    posts: Vec<Post>,
}

fn group_posts(posts: &[Post]) -> Vec<YearGroup> {
    let mut years: Vec<YearGroup> = vec![];

    for post in posts {
        let date_str = post.formatted_date();

        let parts: Vec<&str> = date_str.split('-').collect();
        if parts.len() != 3 {
            continue;
        }
        let year = parts[0].to_string();
        let month_num = parts[1];
        let month_en = match month_num {
            "01" => "January",
            "02" => "February",
            "03" => "March",
            "04" => "April",
            "05" => "May",
            "06" => "June",
            "07" => "July",
            "08" => "August",
            "09" => "September",
            "10" => "October",
            "11" => "November",
            "12" => "December",
            _ => month_num,
        };

        if let Some(yg) = years.last_mut() {
            if yg.year == year {
                if let Some(mg) = yg.months.last_mut() {
                    if mg.month_en == month_en {
                        mg.posts.push(post.clone());
                        continue;
                    }
                }
                yg.months.push(MonthGroup {
                    month: month_en.to_string(),
                    month_en: month_en.to_string(),
                    posts: vec![post.clone()],
                });
                continue;
            }
        }
        years.push(YearGroup {
            year,
            months: vec![MonthGroup {
                month: month_en.to_string(),
                month_en: month_en.to_string(),
                posts: vec![post.clone()],
            }],
        });
    }

    years
}

#[component]
pub fn Archives() -> Element {
    rsx! {
        header { class: "page-header mb-6",
            h1 { class: "text-[34px] font-bold text-gray-900 dark:text-[#dadadb]",
                "归档"
            }
        }
        ArchivesContent {}
    }
}

#[component]
fn ArchivesContent() -> Element {
    let posts_res = use_server_future(move || list_published_posts(1, 10000))?;

    let posts_data = posts_res.read();
    match &*posts_data {
        Some(Ok(PostListResponse { posts })) => {
            let grouped = group_posts(posts);
            rsx! {
                div { class: "mt-2 text-base text-gray-500 dark:text-[#9b9c9d]",
                    "共 "
                    span { class: "font-medium text-gray-700 dark:text-[#dadadb]", "{posts.len()}" }
                    " 篇文章"
                }
                for year_group in grouped.iter() {
                    YearSection { year_group: year_group.clone() }
                }
            }
        }
        Some(Err(e)) => {
            rsx! {
                div { class: "text-center text-red-500 dark:text-red-400 py-20",
                    "加载失败: {e}"
                }
            }
        }
        None => {
            rsx! {
                DelayedSkeleton { ArchiveSkeleton {} }
            }
        }
    }
}

#[component]
fn YearSection(year_group: YearGroup) -> Element {
    let total = year_group
        .months
        .iter()
        .map(|m| m.posts.len())
        .sum::<usize>();

    rsx! {
        div { class: "archive-year mt-10",
            h2 {
                class: "archive-year-header text-2xl font-bold text-gray-900 dark:text-[#dadadb] mb-4",
                id: "{year_group.year}",
                a {
                    class: "archive-header-link hover:opacity-80 transition-opacity",
                    href: "#{year_group.year}",
                    "{year_group.year}"
                }
                sup { class: "archive-count text-sm text-gray-400 dark:text-[#9b9c9d] ml-1", "{total}" }
            }
            for month_group in year_group.months.iter() {
                MonthSection { month_group: month_group.clone(), year: year_group.year.clone() }
            }
        }
    }
}

#[component]
fn MonthSection(month_group: MonthGroup, year: String) -> Element {
    let count = month_group.posts.len();

    rsx! {
        div { class: "archive-month flex flex-col md:flex-row md:items-start py-2.5 border-b border-gray-100 dark:border-[#333]/50",
            h3 {
                class: "archive-month-header text-lg font-medium text-gray-700 dark:text-[#9b9c9d] md:w-[200px] shrink-0 mt-0 mb-0 py-1.5",
                id: "{year}-{month_group.month_en}",
                a {
                    class: "archive-header-link hover:opacity-80 transition-opacity",
                    href: "#{year}-{month_group.month_en}",
                    "{month_group.month}"
                }
                sup { class: "archive-count text-sm text-gray-400 dark:text-[#9b9c9d] ml-1", "{count}" }
            }
            div { class: "archive-posts flex-1",
                for post in month_group.posts.iter() {
                    ArchiveEntry { post: post.clone() }
                }
            }
        }
    }
}

#[component]
fn ArchiveEntry(post: Post) -> Element {
    let date_str = post.formatted_date();

    rsx! {
        div { class: "archive-entry relative py-1.5 my-2.5 group",
            h3 { class: "archive-entry-title text-base font-normal text-gray-900 dark:text-[#dadadb] m-0",
                "{post.title}"
            }
            div { class: "archive-meta text-sm text-gray-400 dark:text-[#9b9c9d] mt-1",
                "{date_str}"
            }
            Link {
                class: "entry-link absolute inset-0 z-10",
                aria_label: "post link to {post.title}",
                to: Route::PostDetail { slug: post.slug.clone() },
            }
        }
    }
}
