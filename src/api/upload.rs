#![allow(clippy::unused_unit)]

#[cfg(feature = "server")]
use axum::{
    extract::Multipart,
    http::{HeaderMap, StatusCode},
    response::Json,
};
#[cfg(feature = "server")]
use serde_json::{json, Value};

#[cfg(feature = "server")]
use crate::auth::session::parse_session_token;

#[cfg(feature = "server")]
const ALLOWED_MIME_TYPES: &[&str] = &["image/jpeg", "image/png", "image/gif", "image/webp"];
#[cfg(feature = "server")]
const MAX_FILE_SIZE: usize = 5 * 1024 * 1024; // 5MB

#[cfg(feature = "server")]
pub async fn upload_image(
    headers: HeaderMap,
    mut multipart: Multipart,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    // 1. Extract session from cookie
    let cookie_header = headers
        .get("cookie")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");

    let token = match parse_session_token(cookie_header) {
        Some(t) => t,
        None => {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(json!({
                    "success": false,
                    "error": "未登录"
                })),
            ));
        }
    };

    // 2. Verify admin
    let user = match crate::api::auth::get_user_by_token(token).await {
        Ok(Some(u)) => u,
        _ => {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(json!({
                    "success": false,
                    "error": "会话已过期"
                })),
            ));
        }
    };

    if user.role != crate::models::user::UserRole::Admin {
        return Err((
            StatusCode::FORBIDDEN,
            Json(json!({
                "success": false,
                "error": "权限不足"
            })),
        ));
    }

    // 3. Read multipart field
    let field = match multipart.next_field().await {
        Ok(Some(f)) => f,
        Ok(None) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "success": false,
                    "error": "未找到文件"
                })),
            ));
        }
        Err(e) => {
            tracing::error!("Multipart error: {:?}", e);
            return Err((
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "success": false,
                    "error": "文件读取失败"
                })),
            ));
        }
    };

    // 4. Validate mime type
    let mime_type = field.content_type().unwrap_or("").to_string();
    if !ALLOWED_MIME_TYPES.contains(&mime_type.as_str()) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "success": false,
                "error": "不支持的文件类型"
            })),
        ));
    }

    // 5. Read file data
    let data = match field.bytes().await {
        Ok(d) => d,
        Err(e) => {
            tracing::error!("Read file error: {:?}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "success": false,
                    "error": "文件读取失败"
                })),
            ));
        }
    };

    if data.len() > MAX_FILE_SIZE {
        return Err((
            StatusCode::PAYLOAD_TOO_LARGE,
            Json(json!({
                "success": false,
                "error": "文件超过大小限制"
            })),
        ));
    }

    // 6. Generate path: uploads/{year}/{month}/{day}/{uuid}.{ext}
    let now = chrono::Utc::now();
    let year = now.format("%Y").to_string();
    let month = now.format("%m").to_string();
    let day = now.format("%d").to_string();
    let uuid = uuid::Uuid::new_v4().to_string();

    let ext = match mime_type.as_str() {
        "image/jpeg" => "jpg",
        "image/png" => "png",
        "image/gif" => "gif",
        "image/webp" => "webp",
        _ => "bin",
    };

    let dir_path = format!("uploads/{}/{}/{}", year, month, day);
    let file_name = format!("{}.{ }.{}", now.format("%H%M%S"), uuid, ext);
    let file_path = format!("{}/{}", dir_path, file_name);
    let url_path = format!("/uploads/{}/{}/{}/{}", year, month, day, file_name);

    // 7. Create directory and write file
    if let Err(e) = tokio::fs::create_dir_all(&dir_path).await {
        tracing::error!("Create dir error: {:?}", e);
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "success": false,
                "error": "文件保存失败"
            })),
        ));
    }

    if let Err(e) = tokio::fs::write(&file_path, &data).await {
        tracing::error!("Write file error: {:?}", e);
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "success": false,
                "error": "文件保存失败"
            })),
        ));
    }

    tracing::info!("Image uploaded: {} ({} bytes)", file_path, data.len());

    Ok(Json(json!({
        "success": true,
        "url": url_path
    })))
}

#[cfg(not(feature = "server"))]
pub async fn upload_image() {}
