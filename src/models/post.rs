use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PostStatus {
    Draft,
    Published,
}

impl PostStatus {
    #[allow(dead_code)]
    pub fn as_str(&self) -> &'static str {
        match self {
            PostStatus::Draft => "draft",
            PostStatus::Published => "published",
        }
    }

    #[allow(dead_code)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "draft" => Some(PostStatus::Draft),
            "published" => Some(PostStatus::Published),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Post {
    pub id: i32,
    pub author_id: i32,
    pub title: String,
    pub slug: String,
    pub summary: Option<String>,
    pub content_md: String,
    pub content_html: Option<String>,
    pub status: PostStatus,
    pub published_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub tags: Vec<String>,
    pub cover_image: Option<String>,
    pub reading_time: u32,
    pub word_count: u32,
    pub toc_html: Option<String>,
    pub prev_post: Option<PostNav>,
    pub next_post: Option<PostNav>,
}

impl Post {
    pub fn formatted_date(&self) -> String {
        self.published_at
            .map(|d| d.format("%Y-%m-%d").to_string())
            .unwrap_or_else(|| self.created_at.format("%Y-%m-%d").to_string())
    }

    pub fn status_label(&self) -> &'static str {
        match self.status {
            PostStatus::Published => "已发布",
            PostStatus::Draft => "草稿",
        }
    }

    pub fn status_class(&self) -> &'static str {
        match self.status {
            PostStatus::Published => "text-green-600 dark:text-green-400",
            PostStatus::Draft => "text-gray-400 dark:text-[#9b9c9d]",
        }
    }

    pub fn status_badge_class(&self) -> &'static str {
        match self.status {
            PostStatus::Published => "bg-green-100 dark:bg-green-900/30 text-green-700 dark:text-green-300",
            PostStatus::Draft => "bg-gray-100 dark:bg-[#333] text-gray-600 dark:text-[#9b9c9d]",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PostNav {
    pub title: String,
    pub slug: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tag {
    pub id: i32,
    pub name: String,
    pub post_count: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PostStats {
    pub total: i64,
    pub drafts: i64,
    pub published: i64,
}
