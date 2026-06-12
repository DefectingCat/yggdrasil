use dioxus::prelude::*;

use super::types::TagListResponse;
#[cfg(feature = "server")]
use crate::api::error::AppError;
use crate::db::pool::get_conn;
use crate::models::post::Tag;

#[server(ListTags, "/api")]
pub async fn list_tags() -> Result<TagListResponse, ServerFnError> {
    #[cfg(feature = "server")]
    {
        if let Some(cached) = crate::cache::get_tag_list().await {
            return Ok(TagListResponse { tags: cached });
        }

        let client = get_conn().await.map_err(AppError::db_conn)?;

        let rows = client
            .query(
                "SELECT t.id, t.name, COUNT(pt.post_id) as post_count
                 FROM tags t
                 LEFT JOIN post_tags pt ON t.id = pt.tag_id
                 LEFT JOIN posts p ON pt.post_id = p.id AND p.deleted_at IS NULL AND p.status = 'published'
                 GROUP BY t.id, t.name
                 ORDER BY t.name",
                &[],
            )
            .await
            .map_err(AppError::query)?;

        let tags: Vec<Tag> = rows
            .iter()
            .map(|r| Tag {
                id: r.get("id"),
                name: r.get("name"),
                post_count: r.get("post_count"),
            })
            .collect();

        crate::cache::set_tag_list(tags.clone()).await;
        Ok(TagListResponse { tags })
    }

    #[cfg(not(feature = "server"))]
    {
        Ok(TagListResponse { tags: Vec::new() })
    }
}
