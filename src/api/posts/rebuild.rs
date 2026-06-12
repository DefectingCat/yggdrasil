use dioxus::prelude::*;

#[cfg(feature = "server")]
use super::helpers::get_current_admin_user;
#[cfg(feature = "server")]
use crate::api::error::AppError;
use crate::db::pool::get_conn;

#[server(RebuildContentHtml, "/api")]
pub async fn rebuild_content_html(rebuild_all: bool) -> Result<u64, ServerFnError> {
    let _user = get_current_admin_user().await?;

    #[cfg(feature = "server")]
    {
        let client = get_conn().await.map_err(AppError::db_conn)?;

        let query = if rebuild_all {
            "SELECT id, content_md FROM posts WHERE deleted_at IS NULL ORDER BY id"
        } else {
            "SELECT id, content_md FROM posts WHERE deleted_at IS NULL AND content_html IS NULL ORDER BY id"
        };

        let rows = client.query(query, &[]).await.map_err(AppError::query)?;

        let mut count: u64 = 0;

        for row in &rows {
            let id: i32 = row.get(0);
            let content_md: String = row.get(1);

            let rendered = crate::api::markdown::render_markdown_enhanced(&content_md);
            let toc_html = if rendered.toc_html.is_empty() {
                None::<String>
            } else {
                Some(rendered.toc_html)
            };

            client
                .execute(
                    "UPDATE posts SET content_html = $1, toc_html = $2, updated_at = NOW() WHERE id = $3",
                    &[&rendered.html, &toc_html, &id],
                )
                .await
                .map_err(AppError::tx)?;

            count += 1;
        }

        if count > 0 {
            crate::cache::invalidate_post_lists();
            crate::cache::invalidate_post_stats();
        }

        Ok(count)
    }

    #[cfg(not(feature = "server"))]
    {
        Ok(0)
    }
}
