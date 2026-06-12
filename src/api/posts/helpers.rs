#[cfg(feature = "server")]
use crate::api::error::AppError;
#[cfg(feature = "server")]
use crate::models::post::{Post, PostStatus};
#[cfg(feature = "server")]
use crate::utils::text::count_words;

#[cfg(feature = "server")]
pub(super) use crate::api::auth::get_current_admin_user;

#[cfg(feature = "server")]
pub(super) async fn row_to_post_list(
    _client: &tokio_postgres::Client,
    row: &tokio_postgres::Row,
) -> Post {
    let id: i32 = row.get("id");
    let role_str: String = row.get("status");
    let status = PostStatus::from_str(&role_str).unwrap_or(PostStatus::Draft);

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
        tags,
        cover_image: row.get("cover_image"),
        reading_time: (word_count / 200).max(1),
        word_count,
        toc_html: None,
        prev_post: None,
        next_post: None,
    }
}

#[cfg(feature = "server")]
pub(super) async fn row_to_post_full(
    _client: &tokio_postgres::Client,
    row: &tokio_postgres::Row,
) -> Post {
    let id: i32 = row.get("id");
    let role_str: String = row.get("status");
    let status = PostStatus::from_str(&role_str).unwrap_or(PostStatus::Draft);

    let tags: Vec<String> = row
        .try_get::<_, Vec<String>>("tags")
        .unwrap_or_default()
        .into_iter()
        .filter(|t| !t.is_empty())
        .collect();

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
        tags,
        cover_image: row.get("cover_image"),
        reading_time: (word_count / 200).max(1),
        word_count,
        toc_html,
        prev_post,
        next_post,
    }
}

#[cfg(feature = "server")]
pub(super) async fn sync_tags(
    tx: &deadpool_postgres::Transaction<'_>,
    post_id: i32,
    tags: &[String],
) -> Result<(), AppError> {
    for tag_name in tags {
        let tag_id: i32 = {
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

#[cfg(feature = "server")]
pub(super) fn clean_tags(tags: &[String]) -> Vec<String> {
    tags.iter()
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .collect()
}
