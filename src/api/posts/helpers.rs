//! 文章模块内部辅助函数。
//!
//! 提供数据库行到 `Post` 模型的转换、标签同步与标签清洗等工具函数。
//! 仅在 `feature = "server"` 启用的服务端构建中可用。

#[cfg(feature = "server")]
use crate::api::error::AppError;
#[cfg(feature = "server")]
use crate::models::post::{Post, PostStatus};
#[cfg(feature = "server")]
use crate::utils::text::count_words;

/// 复用认证模块的当前 admin 用户获取逻辑。
#[cfg(feature = "server")]
pub(super) use crate::api::auth::get_current_admin_user;

/// 将数据库行转换为文章列表项。
///
/// 用于列表接口，包含标签聚合、字数与阅读时长估算，
/// 不包含上下篇导航与目录。
#[cfg(feature = "server")]
pub(super) async fn row_to_post_list(
    _client: &tokio_postgres::Client,
    row: &tokio_postgres::Row,
) -> Post {
    let id: i32 = row.get("id");
    let role_str: String = row.get("status");
    let status = PostStatus::from_str(&role_str).unwrap_or(PostStatus::Draft);

    // 聚合标签并过滤空字符串。
    let tags: Vec<String> = row
        .try_get::<_, Vec<String>>("tags")
        .unwrap_or_default()
        .into_iter()
        .filter(|t| !t.is_empty())
        .collect();

    let content_md: String = row.get("content_md");
    let word_count = count_words(&content_md);

    Post {
        id,
        author_id: row.get("author_id"),
        title: row.get("title"),
        slug: row.get("slug"),
        summary: row.get("summary"),
        content_md,
        content_html: row.get("content_html"),
        status,
        published_at: row.get("published_at"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
        deleted_at: row.try_get("deleted_at").ok(),
        tags,
        cover_image: row.get("cover_image"),
        reading_time: (word_count / 200).max(1),
        word_count,
        toc_html: None,
        prev_post: None,
        next_post: None,
    }
}

/// 将数据库行转换为完整文章详情。
///
/// 相比列表项额外包含上一篇/下一篇导航，
/// 并在 content_html 为空时重新渲染 Markdown 以兼容旧数据。
#[cfg(feature = "server")]
pub(super) async fn row_to_post_full(
    _client: &tokio_postgres::Client,
    row: &tokio_postgres::Row,
) -> Post {
    let id: i32 = row.get("id");
    let role_str: String = row.get("status");
    let status = PostStatus::from_str(&role_str).unwrap_or(PostStatus::Draft);

    // 聚合标签并过滤空字符串。
    let tags: Vec<String> = row
        .try_get::<_, Vec<String>>("tags")
        .unwrap_or_default()
        .into_iter()
        .filter(|t| !t.is_empty())
        .collect();

    // 解析上一篇文章导航。
    let prev_post = if let Ok(prev_title) = row.try_get::<_, String>("prev_title") {
        if let Ok(prev_slug) = row.try_get::<_, String>("prev_slug") {
            Some(crate::models::post::PostNav {
                title: prev_title,
                slug: prev_slug,
            })
        } else {
            None
        }
    } else {
        None
    };

    // 解析下一篇文章导航。
    let next_post = if let Ok(next_title) = row.try_get::<_, String>("next_title") {
        if let Ok(next_slug) = row.try_get::<_, String>("next_slug") {
            Some(crate::models::post::PostNav {
                title: next_title,
                slug: next_slug,
            })
        } else {
            None
        }
    } else {
        None
    };

    let content_html: Option<String> = row.get("content_html");
    let toc_html_row: Option<String> = row.get("toc_html");

    // 若数据库中未渲染 HTML（旧数据兼容），则现场渲染 Markdown。
    let (content_html, toc_html) = if let Some(html) = content_html {
        (html, toc_html_row)
    } else {
        let content_md: String = row.get("content_md");
        let rendered = crate::api::markdown::render_markdown_enhanced(&content_md);
        (
            rendered.html,
            if rendered.toc_html.is_empty() {
                None
            } else {
                Some(rendered.toc_html)
            },
        )
    };

    let content_md: String = row.get("content_md");
    let word_count = count_words(&content_md);

    Post {
        id,
        author_id: row.get("author_id"),
        title: row.get("title"),
        slug: row.get("slug"),
        summary: row.get("summary"),
        content_md,
        content_html: Some(content_html),
        status,
        published_at: row.get("published_at"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
        deleted_at: row.try_get("deleted_at").ok(),
        tags,
        cover_image: row.get("cover_image"),
        reading_time: (word_count / 200).max(1),
        word_count,
        toc_html,
        prev_post,
        next_post,
    }
}

/// 在事务中同步文章的标签关联。
///
/// 对传入的每个标签：若不存在则插入 tags 表，否则查询已有 id，
/// 然后在 post_tags 表中建立关联。不会删除旧关联，调用方需先清理。
#[cfg(feature = "server")]
pub(super) async fn sync_tags(
    tx: &deadpool_postgres::Transaction<'_>,
    post_id: i32,
    tags: &[String],
) -> Result<(), AppError> {
    for tag_name in tags {
        let tag_id: i32 = {
            // 先尝试插入，若已存在则返回空。
            let row = tx
                .query_opt(
                    "INSERT INTO tags (name) VALUES ($1) ON CONFLICT (name) DO NOTHING RETURNING id",
                    &[&tag_name.as_str()],
                )
                .await
                .map_err(AppError::tx)?;

            match row {
                Some(r) => r.get(0),
                None => {
                    // 插入冲突时回查标签 id。
                    let row = tx
                        .query_opt("SELECT id FROM tags WHERE name = $1", &[&tag_name.as_str()])
                        .await
                        .map_err(AppError::query)?;
                    row.map(|r| r.get(0))
                        .ok_or(AppError::NotFound("标签不存在"))?
                }
            }
        };

        tx.execute(
            "INSERT INTO post_tags (post_id, tag_id) VALUES ($1, $2)",
            &[&post_id, &tag_id],
        )
        .await
        .map_err(AppError::tx)?;
    }

    Ok(())
}

/// 清洗标签列表：去头尾空白、过滤空字符串并去重（保留原始顺序）。
#[cfg(feature = "server")]
pub(super) fn clean_tags(tags: &[String]) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    tags.iter()
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .filter(|t| seen.insert(t.to_lowercase()))
        .collect()
}

#[cfg(all(test, feature = "server"))]
mod tests {
    use super::clean_tags;

    #[test]
    fn clean_tags_trims_whitespace() {
        let input = vec!["  rust ".to_string(), "\t\nwasm\t".to_string()];
        assert_eq!(
            clean_tags(&input),
            vec!["rust".to_string(), "wasm".to_string()]
        );
    }

    #[test]
    fn clean_tags_filters_empty_strings() {
        let input = vec![
            "".to_string(),
            "  ".to_string(),
            "\t".to_string(),
            "valid".to_string(),
        ];
        assert_eq!(clean_tags(&input), vec!["valid".to_string()]);
    }

    #[test]
    fn clean_tags_removes_duplicates_case_insensitive() {
        let input = vec![
            "rust".to_string(),
            "  rust  ".to_string(),
            "Rust".to_string(),
            "wasm".to_string(),
        ];
        assert_eq!(
            clean_tags(&input),
            vec!["rust".to_string(), "wasm".to_string()]
        );
    }

    #[test]
    fn clean_tags_keeps_already_clean_input() {
        let input = vec!["rust".to_string(), "wasm".to_string(), "dioxus".to_string()];
        assert_eq!(
            clean_tags(&input),
            vec!["rust".to_string(), "wasm".to_string(), "dioxus".to_string()]
        );
    }
}
