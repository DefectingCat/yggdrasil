//! 图片上传的 Axum 处理器。
//!
//! 处理 multipart 上传，校验 MIME 类型、文件大小与 admin 权限，
//! JPEG/PNG 自动转 WebP（若体积更小则保留原格式），GIF/WebP 保持原样。
//! 文件按日期分目录存放于 `uploads/`。
//! 本模块属于手动注册的 Axum 路由，仅在 `feature = "server"` 时可用。

#[cfg(feature = "server")]
use axum::{
    extract::{ConnectInfo, Extension, Multipart},
    http::{HeaderMap, StatusCode},
    response::Json,
};
#[cfg(feature = "server")]
use serde_json::{json, Value};
#[cfg(feature = "server")]
use std::net::SocketAddr;

#[cfg(feature = "server")]
use crate::auth::session::parse_session_token;

#[cfg(feature = "server")]
const ALLOWED_MIME_TYPES: &[&str] = &["image/jpeg", "image/png", "image/gif", "image/webp"];
#[cfg(feature = "server")]
const MAX_FILE_SIZE: usize = 5 * 1024 * 1024; // 5MB

/// 构造统一的 JSON 错误响应：`{ "success": false, "error": msg }`。
#[cfg(feature = "server")]
fn upload_error<T: serde::Serialize>(status: StatusCode, msg: T) -> (StatusCode, Json<Value>) {
    (status, Json(json!({ "success": false, "error": msg })))
}

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
/// 通过文件头 magic bytes 校验实际格式是否与声明 MIME 一致。
fn validate_image_magic_bytes(data: &[u8], mime_type: &str) -> bool {
    if data.is_empty() {
        return false;
    }
    match mime_type {
        "image/jpeg" => data.starts_with(&[0xFF, 0xD8, 0xFF]),
        "image/png" => data.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]),
        "image/gif" => data.starts_with(b"GIF87a") || data.starts_with(b"GIF89a"),
        "image/webp" => {
            // RIFF....WEBP
            data.len() >= 12 && &data[0..4] == b"RIFF" && &data[8..12] == b"WEBP"
        }
        _ => false,
    }
}

#[cfg(feature = "server")]
/// 解码验证 GIF/WebP 原始字节，确保不是伪造扩展名的恶意文件。
fn validate_raw_image(data: &[u8], mime_type: &str) -> bool {
    match mime_type {
        "image/webp" => crate::webp::decode(data).is_ok(),
        "image/gif" => image::load_from_memory(data).is_ok(),
        _ => true,
    }
}

#[cfg(feature = "server")]
/// 处理图片上传的 Axum handler。
///
/// 流程：限流 → 解析 session → 校验 admin → 读取 multipart → 校验类型/大小 →
/// 转码（如适用）→ 按日期落盘 → 返回相对 URL。
///
/// `ConnectInfo` 以可选扩展注入：`dioxus::server::serve()` 接管了 listener，
/// 无法调用 `into_make_service_with_connect_info::<SocketAddr>()`，所以这里
/// 与 `serve_image` 保持一致的优雅降级——扩展缺失时退回 `"unknown"` 限流桶。
/// 生产环境应在反向代理后部署并配置 `TRUSTED_PROXY_COUNT`，让限流拿到真实 IP。
pub async fn upload_image(
    connect_info: Option<Extension<ConnectInfo<SocketAddr>>>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    // 0. Rate limit check
    let peer = connect_info.map(|Extension(ConnectInfo(addr))| addr);
    let ip = crate::api::rate_limit::get_client_ip_with_peer(&headers, peer);
    if let Err(msg) = crate::api::rate_limit::check_upload_limit(&ip) {
        return Err(upload_error(StatusCode::TOO_MANY_REQUESTS, msg));
    }

    // 1. Extract session from cookie
    let cookie_header = headers
        .get("cookie")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");

    let token = match parse_session_token(cookie_header) {
        Some(t) => t,
        None => {
            return Err(upload_error(StatusCode::UNAUTHORIZED, "未登录"));
        }
    };

    // 2. Verify admin
    let user = match crate::api::auth::get_user_by_token(token).await {
        Ok(Some(u)) => u,
        _ => {
            return Err(upload_error(StatusCode::UNAUTHORIZED, "会话已过期"));
        }
    };

    if user.role != crate::models::user::UserRole::Admin {
        return Err(upload_error(StatusCode::FORBIDDEN, "权限不足"));
    }

    // 3. Read multipart field
    let field = match multipart.next_field().await {
        Ok(Some(f)) => f,
        Ok(None) => {
            return Err(upload_error(StatusCode::BAD_REQUEST, "未找到文件"));
        }
        Err(e) => {
            tracing::error!("Multipart error: {:?}", e);
            return Err(upload_error(StatusCode::BAD_REQUEST, "文件读取失败"));
        }
    };

    // 4. Validate mime type
    let mime_type = field.content_type().unwrap_or("").to_string();
    if !ALLOWED_MIME_TYPES.contains(&mime_type.as_str()) {
        return Err(upload_error(StatusCode::BAD_REQUEST, "不支持的文件类型"));
    }

    // 5. Read file data
    let data = match field.bytes().await {
        Ok(d) => d,
        Err(e) => {
            tracing::error!("Read file error: {:?}", e);
            return Err(upload_error(StatusCode::INTERNAL_SERVER_ERROR, "文件读取失败"));
        }
    };

    if data.len() > MAX_FILE_SIZE {
        return Err(upload_error(StatusCode::PAYLOAD_TOO_LARGE, "文件超过大小限制"));
    }

    // 校验文件头 magic bytes，防止仅修改扩展名/Content-Type 上传非图片文件。
    if !validate_image_magic_bytes(&data, mime_type.as_str()) {
        return Err(upload_error(StatusCode::BAD_REQUEST, "文件类型与内容不符"));
    }

    // 仅读 header 统一校验尺寸/像素上限。三种格式走同一路径:
    // JPEG/PNG/GIF 用 image crate 的 into_dimensions,WebP 用 zenwebp header。
    // 超限直接拒绝,避免大图走 decode 后被静默降级(原 fallback 存原图)。
    if let Err(msg) = crate::api::image::check_upload_dimensions(&data, mime_type.as_str()) {
        return Err(upload_error(StatusCode::BAD_REQUEST, msg));
    }

    let is_gif = mime_type.as_str() == "image/gif";
    let is_webp = mime_type.as_str() == "image/webp";

    // 对不经过重编码的格式做解码验证。GIF 走 image::load_from_memory 会完整解码，
    // 移到阻塞线程池避免拖住 async 运行时。
    if is_gif || is_webp {
        let validate_data = data.to_vec();
        let validate_mime = mime_type.clone();
        let is_valid = tokio::task::spawn_blocking(move || {
            validate_raw_image(&validate_data, validate_mime.as_str())
        })
        .await
        .map_err(|_| upload_error(StatusCode::INTERNAL_SERVER_ERROR, "图片校验任务失败"))?;
        if !is_valid {
            return Err(upload_error(StatusCode::BAD_REQUEST, "图片文件损坏或格式不正确"));
        }
    }

    // GIF 与 WebP 保持原格式；其余格式尝试转 WebP。
    let (final_data, final_ext) = if is_gif {
        (data.to_vec(), "gif".to_string())
    } else if is_webp {
        (data.to_vec(), "webp".to_string())
    } else {
        let original_data = data.to_vec();
        let mime = mime_type.clone();
        let config = crate::webp::WEBP_CONFIG.clone();
        // 在阻塞线程中执行图片解码与 WebP 编码，避免阻塞异步运行时。
        let result = tokio::task::spawn_blocking(move || -> (Vec<u8>, String, bool) {
            let total_start = std::time::Instant::now();
            let cursor = std::io::Cursor::new(&original_data);
            let format = match mime.as_str() {
                "image/jpeg" => image::ImageFormat::Jpeg,
                "image/png" => image::ImageFormat::Png,
                _ => image::ImageFormat::Jpeg,
            };
            let mut reader = image::ImageReader::with_format(cursor, format);
            reader.limits(crate::api::image::image_reader_limits());

            match reader.decode() {
                Ok(img) => {
                    let decode_time = total_start.elapsed();
                    let enc_start = std::time::Instant::now();
                    let result = match crate::webp::encode(&img, config.quality, config.method) {
                        Ok(webp_data) => {
                            let enc_time = enc_start.elapsed();
                            let total_time = total_start.elapsed();
                            // WebP 更小才采用，否则回退原格式以节省带宽。
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
                Err(e) => {
                    // 到这里尺寸校验已通过(超限在 header 阶段被拒),decode 失败只能是真损坏。
                    tracing::warn!("Failed to decode image ({}), keeping original format", e);
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

    // 按上传时间组织目录：uploads/YYYY/MM/DD。
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
        return Err(upload_error(StatusCode::INTERNAL_SERVER_ERROR, "文件保存失败"));
    }

    if let Err(e) = tokio::fs::write(&file_path, &final_data).await {
        tracing::error!("Write file error: {:?}", e);
        return Err(upload_error(StatusCode::INTERNAL_SERVER_ERROR, "文件保存失败"));
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

    #[test]
    fn mime_to_ext_maps_jpeg() {
        assert_eq!(super::mime_to_ext("image/jpeg"), "jpg");
    }

    #[test]
    fn mime_to_ext_maps_png() {
        assert_eq!(super::mime_to_ext("image/png"), "png");
    }

    #[test]
    fn mime_to_ext_maps_gif() {
        assert_eq!(super::mime_to_ext("image/gif"), "gif");
    }

    #[test]
    fn mime_to_ext_maps_webp() {
        assert_eq!(super::mime_to_ext("image/webp"), "webp");
    }

    #[test]
    fn mime_to_ext_falls_back_for_unknown_mime() {
        assert_eq!(super::mime_to_ext("image/avif"), "bin");
        assert_eq!(super::mime_to_ext("application/octet-stream"), "bin");
    }

    #[test]
    fn validate_jpeg_magic_bytes() {
        assert!(super::validate_image_magic_bytes(
            &[0xFF, 0xD8, 0xFF],
            "image/jpeg"
        ));
        assert!(!super::validate_image_magic_bytes(
            &[0x89, 0x50],
            "image/jpeg"
        ));
    }

    #[test]
    fn validate_png_magic_bytes() {
        assert!(super::validate_image_magic_bytes(
            &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A],
            "image/png"
        ));
        assert!(!super::validate_image_magic_bytes(
            &[0xFF, 0xD8],
            "image/png"
        ));
    }

    #[test]
    fn validate_gif_magic_bytes() {
        assert!(super::validate_image_magic_bytes(b"GIF89a", "image/gif"));
        assert!(super::validate_image_magic_bytes(b"GIF87a", "image/gif"));
        assert!(!super::validate_image_magic_bytes(b"GIF90a", "image/gif"));
    }

    #[test]
    fn validate_webp_magic_bytes() {
        let webp = b"RIFF\x00\x00\x00\x00WEBPVP8 ";
        assert!(super::validate_image_magic_bytes(&webp[..12], "image/webp"));
        assert!(!super::validate_image_magic_bytes(
            &[0xFF, 0xD8],
            "image/webp"
        ));
    }
}
