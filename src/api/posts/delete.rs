use dioxus::prelude::*;

#[cfg(feature = "server")]
use super::helpers::get_current_admin_user;
use super::types::CreatePostResponse;
#[cfg(feature = "server")]
use crate::api::error::AppError;
use crate::db::pool::get_conn;

#[server(DeletePost, "/api")]
pub async fn delete_post(post_id: i32) -> Result<CreatePostResponse, ServerFnError> {
    let _user = get_current_admin_user().await?;

    #[cfg(feature = "server")]
    {
        let client = get_conn().await.map_err(AppError::db_conn)?;

        let result = client
            .execute(
                "UPDATE posts SET deleted_at = NOW() WHERE id = $1 AND deleted_at IS NULL",
                &[&post_id],
            )
            .await
            .map_err(AppError::query)?;

        if result == 0 {
            return Ok(CreatePostResponse {
                success: false,
                message: "文章不存在".to_string(),
                post_id: None,
                slug: None,
            });
        }

        crate::cache::invalidate_all_post_caches();

        Ok(CreatePostResponse {
            success: true,
            message: "删除成功".to_string(),
            post_id: Some(post_id),
            slug: None,
        })
    }

    #[cfg(not(feature = "server"))]
    {
        Ok(CreatePostResponse {
            success: false,
            message: "server only".to_string(),
            post_id: None,
            slug: None,
        })
    }
}
