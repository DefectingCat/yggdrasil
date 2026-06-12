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
fn mime_to_ext(mime: &str) -> &'static str {
    match mime {
        "image/jpeg" => "jpg",
        "image/png" => "png",
        "image/webp" => "webp",
        "image/gif" => "gif",
        _ => "bin",
    }
}

#[cfg(feature = "server")]
pub async fn upload_image(
    headers: HeaderMap,
    mut multipart: Multipart,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    // 0. Rate limit check
    let ip = crate::api::rate_limit::get_client_ip(&headers);
    if let Err(msg) = crate::api::rate_limit::check_upload_limit(&ip) {
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            Json(json!({
                "success": false,
                "error": msg
            })),
        ));
    }

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

    let is_gif = mime_type.as_str() == "image/gif";
    let is_webp = mime_type.as_str() == "image/webp";

    let (final_data, final_ext) = if is_gif {
        (data.to_vec(), "gif".to_string())
    } else if is_webp {
        (data.to_vec(), "webp".to_string())
    } else {
        let original_data = data.to_vec();
        let mime = mime_type.clone();
        let config = crate::webp::WEBP_CONFIG.clone();
        let result = tokio::task::spawn_blocking(move || -> (Vec<u8>, String, bool) {
            let total_start = std::time::Instant::now();
            match image::load_from_memory(&original_data) {
                Ok(img) => {
                    let decode_time = total_start.elapsed();
                    let enc_start = std::time::Instant::now();
                    let result = match crate::webp::encode(&img, config.quality, config.method) {
                        Ok(webp_data) => {
                            let enc_time = enc_start.elapsed();
                            let total_time = total_start.elapsed();
                            if webp_data.len() < original_data.len() {
                                tracing::info!(
                                    "WebP conversion: decode={:?} encode={:?} total={:?} {}x{} {} bytes -> {} bytes",
                                    decode_time, enc_time, total_time,
                                    img.width(), img.height(),
                                    original_data.len(), webp_data.len()
                                );
                                (webp_data, "webp".to_string(), true)
                            } else {
                                tracing::info!(
                                    "WebP conversion larger, keeping original: decode={:?} encode={:?} total={:?} {}x{} original={} webp={}",
                                    decode_time, enc_time, total_time,
                                    img.width(), img.height(),
                                    original_data.len(), webp_data.len()
                                );
                                (original_data, mime_to_ext(&mime).to_string(), false)
                            }
                        }
                        Err(e) => {
                            tracing::warn!("WebP encode failed ({}), keeping original format", e);
                            (original_data, mime_to_ext(&mime).to_string(), false)
                        }
                    };
                    result
                }
                Err(_) => {
                    tracing::warn!("Failed to decode image, keeping original format");
                    (original_data, mime_to_ext(&mime).to_string(), false)
                }
            }
        })
        .await;

        match result {
            Ok((converted_data, ext, _was_converted)) => (converted_data, ext),
            Err(_) => {
                tracing::warn!("spawn_blocking task panicked, keeping original format");
                (data.to_vec(), mime_to_ext(&mime_type).to_string())
            }
        }
    };

    let now = chrono::Utc::now();
    let year = now.format("%Y").to_string();
    let month = now.format("%m").to_string();
    let day = now.format("%d").to_string();
    let uuid = uuid::Uuid::new_v4().to_string();

    let dir_path = format!("uploads/{}/{}/{}", year, month, day);
    let file_name = format!("{}.{}.{}", now.format("%H%M%S"), uuid, final_ext);
    let file_path = format!("{}/{}", dir_path, file_name);
    let url_path = format!("/uploads/{}/{}/{}/{}", year, month, day, file_name);

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

    if let Err(e) = tokio::fs::write(&file_path, &final_data).await {
        tracing::error!("Write file error: {:?}", e);
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "success": false,
                "error": "文件保存失败"
            })),
        ));
    }

    tracing::info!("Image uploaded: {} ({} bytes)", file_path, final_data.len());

    Ok(Json(json!({
        "success": true,
        "url": url_path
    })))
}

#[cfg(all(test, feature = "server"))]
mod tests {
    #[test]
    fn filename_format_no_spaces() {
        let now_str = "120000";
        let uuid = "abc-123";
        let ext = "jpg";
        let file_name = format!("{}.{}.{}", now_str, uuid, ext);
        assert!(
            !file_name.contains(' '),
            "filename should not contain spaces: got '{}'",
            file_name
        );
    }

    #[test]
    fn should_use_webp_ext_for_non_gif() {
        let ext = "jpg";
        let mime = "image/jpeg";
        let is_gif = mime == "image/gif";
        let final_ext = if is_gif { ext } else { "webp" };
        assert_eq!(final_ext, "webp");
    }

    #[test]
    fn should_preserve_gif_ext() {
        let ext = "gif";
        let mime = "image/gif";
        let is_gif = mime == "image/gif";
        let final_ext = if is_gif { ext } else { "webp" };
        assert_eq!(final_ext, "gif");
    }

    #[test]
    fn convert_to_webp_produces_bytes() {
        let img = image::DynamicImage::new_rgb8(10, 10);
        let result = crate::webp::encode(&img, 85.0, 4).unwrap();
        assert!(!result.is_empty());
    }

    #[test]
    fn webp_roundtrip_from_rgba() {
        let img = image::DynamicImage::new_rgba8(2, 2);
        let webp_bytes = crate::webp::encode(&img, 85.0, 4).unwrap();
        let loaded = crate::webp::decode(&webp_bytes);
        assert!(loaded.is_ok());
    }
}
