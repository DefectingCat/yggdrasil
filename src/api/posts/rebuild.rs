use dioxus::prelude::*;

#[cfg(feature = "server")]
use super::helpers::get_current_admin_user;
#[cfg(feature = "server")]
use crate::api::error::AppError;
use crate::api::posts::RebuildResult;
use crate::db::pool::get_conn;

const REBUILD_BATCH_LIMIT: i64 = 500;
const MAX_DISPLAY_ERRORS: usize = 5;

#[server(RebuildContentHtml, "/api")]
pub async fn rebuild_content_html(rebuild_all: bool) -> Result<RebuildResult, ServerFnError> {
    let _user = get_current_admin_user().await?;

    #[cfg(feature = "server")]
    {
        let client = get_conn().await.map_err(AppError::db_conn)?;

        let query = if rebuild_all {
            format!(
                "SELECT id, content_md FROM posts WHERE deleted_at IS NULL ORDER BY id LIMIT {REBUILD_BATCH_LIMIT}"
            )
        } else {
            format!(
                "SELECT id, content_md FROM posts WHERE deleted_at IS NULL AND content_html IS NULL ORDER BY id LIMIT {REBUILD_BATCH_LIMIT}"
            )
        };

        let rows = client.query(&query, &[]).await.map_err(AppError::query)?;

        let mut rebuilt: u64 = 0;
        let mut failed: u64 = 0;
        let mut errors: Vec<String> = Vec::new();

        for row in &rows {
            let id: i32 = row.get(0);
            let content_md: String = row.get(1);

            let rendered = match std::panic::catch_unwind(|| {
                crate::api::markdown::render_markdown_enhanced(&content_md)
            }) {
                Ok(r) => r,
                Err(_) => {
                    failed += 1;
                    if errors.len() < MAX_DISPLAY_ERRORS {
                        errors.push(format!("文章 #{id}: 渲染异常"));
                    }
                    continue;
                }
            };

            let toc_html = if rendered.toc_html.is_empty() {
                None::<String>
            } else {
                Some(rendered.toc_html)
            };

            match client
                .execute(
                    "UPDATE posts SET content_html = $1, toc_html = $2 WHERE id = $3",
                    &[&rendered.html, &toc_html, &id],
                )
                .await
            {
                Ok(_) => rebuilt += 1,
                Err(_) => {
                    failed += 1;
                    if errors.len() < MAX_DISPLAY_ERRORS {
                        errors.push(format!("文章 #{id}: DB 写入失败"));
                    }
                }
            }
        }

        if rebuilt > 0 || failed > 0 {
            crate::cache::invalidate_all_post_caches();
        }

        Ok(RebuildResult {
            rebuilt,
            failed,
            errors,
        })
    }

    #[cfg(not(feature = "server"))]
    {
        Ok(RebuildResult {
            rebuilt: 0,
            failed: 0,
            errors: vec![],
        })
    }
}
