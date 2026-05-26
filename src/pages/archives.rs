use dioxus::prelude::*;

use crate::pages::home::{Footer, Header};

#[derive(Clone, PartialEq)]
pub struct Post {
    pub title: &'static str,
    pub date: &'static str,
    pub slug: &'static str,
}

const POSTS: &[Post] = &[
    Post {
        title: "开始使用 Rust 构建 Web 应用",
        date: "2026-05-20",
        slug: "rust-web-app",
    },
    Post {
        title: "Tailwind CSS 的设计理念与实践",
        date: "2026-05-15",
        slug: "tailwind-css",
    },
    Post {
        title: "PostgreSQL 在 Rust 项目中的最佳实践",
        date: "2026-05-10",
        slug: "postgresql-rust",
    },
    Post {
        title: "暗色模式的设计思考",
        date: "2026-05-05",
        slug: "dark-mode-design",
    },
    Post {
        title: "博客系统的架构演进",
        date: "2026-04-28",
        slug: "blog-architecture",
    },
    Post {
        title: "Dioxus 0.7 新特性一览",
        date: "2026-04-20",
        slug: "dioxus-07",
    },
];

#[derive(Clone, PartialEq)]
struct YearGroup {
    year: &'static str,
    months: Vec<MonthGroup>,
}

#[derive(Clone, PartialEq)]
struct MonthGroup {
    month: &'static str,
    month_en: &'static str,
    posts: Vec<Post>,
}

fn group_posts(posts: &[Post]) -> Vec<YearGroup> {
    let mut years: Vec<YearGroup> = vec![];

    for post in posts {
        let parts: Vec<&str> = post.date.split('-').collect();
        if parts.len() != 3 {
            continue;
        }
        let year = parts[0];
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
                    month: month_en,
                    month_en,
                    posts: vec![post.clone()],
                });
                continue;
            }
        }
        years.push(YearGroup {
            year,
            months: vec![MonthGroup {
                month: month_en,
                month_en,
                posts: vec![post.clone()],
            }],
        });
    }

    years
}

#[component]
pub fn ArchivesPage() -> Element {
    let grouped = group_posts(POSTS);

    rsx! {
        div { class: "min-h-screen flex flex-col bg-white dark:bg-[#1d1e20] transition-colors duration-300",
            Header {}
            main { class: "flex-1 w-full max-w-3xl mx-auto px-6 py-6",
                header { class: "page-header mb-6",
                    h1 { class: "text-[34px] font-bold text-gray-900 dark:text-[#dadadb]",
                        "归档"
                    }
                    div { class: "mt-2 text-base text-gray-500 dark:text-[#9b9c9d]",
                        "共 "
                        span { class: "font-medium text-gray-700 dark:text-[#dadadb]", "{POSTS.len()}" }
                        " 篇文章"
                    }
                }
                for year_group in grouped.iter() {
                    YearSection { year_group: year_group.clone() }
                }
            }
            Footer {}
        }
    }
}

#[component]
fn YearSection(year_group: YearGroup) -> Element {
    let total = year_group.months.iter().map(|m| m.posts.len()).sum::<usize>();

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
                MonthSection { month_group: month_group.clone(), year: year_group.year }
            }
        }
    }
}

#[component]
fn MonthSection(month_group: MonthGroup, year: &'static str) -> Element {
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
    rsx! {
        div { class: "archive-entry relative py-1.5 my-2.5 group",
            h3 { class: "archive-entry-title text-base font-normal text-gray-900 dark:text-[#dadadb] m-0",
                "{post.title}"
            }
            div { class: "archive-meta text-sm text-gray-400 dark:text-[#9b9c9d] mt-1",
                "{post.date}"
            }
            a {
                class: "entry-link absolute inset-0 z-10",
                aria_label: "post link to {post.title}",
                href: "/post/{post.slug}",
            }
        }
    }
}
