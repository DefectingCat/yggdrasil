#![allow(clippy::unused_unit, deprecated, unused_imports)]

use dioxus::prelude::*;

#[cfg(feature = "server")]
use crate::api::utils::query_error;

#[cfg(feature = "server")]
pub async fn set_post_tags(
    client: &tokio_postgres::Client,
    post_id: i32,
    tags: &[String],
) -> Result<(), ServerFnError> {
    // Remove existing tags
    client
        .execute("DELETE FROM post_tags WHERE post_id = $1", &[&post_id])
        .await
        .map_err(|e| {
            tracing::error!("delete tag links failed: {:?}", e);
            ServerFnError::new(format!("删除标签关联失败: {}", e))
        })?;

    for tag_name in tags {
        let tag_name = tag_name.trim();
        if tag_name.is_empty() {
            continue;
        }

        // Insert or get tag
        let tag_id: i32 = {
            let row = client
                .query_opt(
                    "INSERT INTO tags (name) VALUES ($1) ON CONFLICT (name) DO NOTHING RETURNING id",
                    &[&tag_name],
                )
                .await
                .map_err(|e| {
                    tracing::error!("create tag failed: {:?}", e);
                    ServerFnError::new(format!("创建标签失败: {}", e))
                })?;

            match row {
                Some(r) => r.get(0),
                None => {
                    // Tag already exists, fetch its id
                    let row = client
                        .query_opt("SELECT id FROM tags WHERE name = $1", &[&tag_name])
                        .await
                        .map_err(|e| {
                            tracing::error!("query tag failed: {:?}", e);
                            ServerFnError::new(format!("查询标签失败: {}", e))
                        })?;
                    row.map(|r| r.get(0))
                        .ok_or_else(|| ServerFnError::new(format!("标签不存在: {}", tag_name)))?
                }
            }
        };

        client
            .execute(
                "INSERT INTO post_tags (post_id, tag_id) VALUES ($1, $2)",
                &[&post_id, &tag_id],
            )
            .await
            .map_err(|e| {
                tracing::error!("link tag failed: {:?}", e);
                ServerFnError::new(format!("关联标签失败: {}", e))
            })?;
    }

    Ok(())
}

#[cfg(feature = "server")]
pub async fn get_post_tags(client: &tokio_postgres::Client, post_id: i32) -> Vec<String> {
    let rows = client
        .query(
            "SELECT t.name FROM tags t JOIN post_tags pt ON t.id = pt.tag_id WHERE pt.post_id = $1 ORDER BY t.name",
            &[&post_id],
        )
        .await;

    match rows {
        Ok(rows) => rows.iter().map(|r| r.get(0)).collect(),
        Err(_) => vec![],
    }
}
