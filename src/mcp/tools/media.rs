//! MCP 写作用域工具：媒体上传。
//!
//! 镜像 `src/api/upload.rs` 的上传流水线（MIME 校验 → 尺寸校验 → 内容去重 →
//! WebP 转码 → 落盘 → assets 登记），但输入为 base64 编码的图片字节而非
//! multipart form。返回可直接在 Markdown 正文里引用的 `/uploads/...` URL。
//!
//! 本模块仅 `feature = "server"` 编译。

#![cfg(feature = "server")]

use base64::Engine;
use rmcp::handler::server::tool::Extension;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::{schemars, tool, tool_router, ErrorData as McpError};
use serde::Deserialize;

use crate::db::pool::get_conn;
use crate::models::mcp_token::TokenScope;
use super::common::{internal, ok_json, require_scope};

/// 与 web 上传一致的大小上限。
const MAX_FILE_SIZE: usize = 5 * 1024 * 1024;

#[tool_router(router = media_router, vis = "pub")]
impl crate::mcp::server::YggMcpServer {
    /// 上传一张图片（base64 编码）。自动转 WebP（若更小），返回可直接嵌入
    /// Markdown 正文的 `/uploads/...` URL。要求 write 作用域。
    #[tool(description = "上传图片。输入 base64 编码的字节，返回 /uploads/... URL（可直接用于 Markdown 正文 img）。支持 JPEG/PNG/GIF/WebP。")]
    async fn upload_media(
        &self,
        Parameters(p): Parameters<UploadMediaParams>,
        Extension(parts): Extension<http::request::Parts>,
    ) -> Result<CallToolResult, McpError> {
        let _principal = require_scope(&parts, "upload_media", TokenScope::Write)?;

        // 1. base64 解码（容忍 data URL 前缀与空白）。
        let b64_clean: String = p
            .base64
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect();
        // 去掉 data URL 前缀（如 data:image/png;base64,...）。
        let b64_payload = b64_clean
            .split(',')
            .next_back()
            .unwrap_or(&b64_clean);
        let data = base64::engine::general_purpose::STANDARD
            .decode(b64_payload)
            .map_err(|e| McpError::invalid_request(format!("base64 decode failed: {e}"), None))?;

        if data.is_empty() {
            return Err(McpError::invalid_request("decoded data is empty", None));
        }
        if data.len() > MAX_FILE_SIZE {
            return Err(McpError::invalid_request(
                format!("文件超过大小限制（{} bytes）", MAX_FILE_SIZE),
                None,
            ));
        }

        // 2. 从 magic bytes 检测 MIME（不信任客户端声明的扩展名）。
        let mime_type = detect_mime(&data).ok_or_else(|| {
            McpError::invalid_request(
                "无法识别的图片格式（支持 JPEG/PNG/GIF/WebP）",
                None,
            )
        })?;

        // 3. 尺寸校验（只读 header，不解码像素）。
        let (img_width, img_height) =
            crate::api::image::upload_dimensions(&data, mime_type)
                .map_err(|msg| McpError::invalid_request(msg, None))?;

        let is_gif = mime_type == "image/gif";
        let is_webp = mime_type == "image/webp";

        // 4. 内容去重（CAS）：SHA-256 命中已登记素材直接复用。
        let content_hash = {
            use sha2::Digest;
            hex::encode(sha2::Sha256::digest(&data))
        };
        {
            let client = get_conn()
                .await
                .map_err(|e| internal(e, "db connection"))?;
            let reused = client
                .query_opt(
                    "UPDATE assets SET created_at = NOW(), updated_at = NOW() \
                     WHERE content_hash = $1 RETURNING path",
                    &[&content_hash],
                )
                .await
                .map_err(|e| internal(e, "dedup check"))?;
            if let Some(row) = reused {
                let path: String = row.get(0);
                return ok_json(UploadResult {
                    success: true,
                    url: format!("/uploads/{}", path),
                    reused: true,
                    width: img_width,
                    height: img_height,
                    mime: mime_type.to_string(),
                });
            }
        }

        // 5. GIF/WebP 解码验证 + 转码（CPU 密集 → spawn_blocking）。
        let data_for_transcode = data.clone();
        let mime_for_transcode = mime_type.to_string();
        let (final_data, final_ext) = tokio::task::spawn_blocking(move || {
            transcode_image(&data_for_transcode, &mime_for_transcode, is_gif, is_webp)
        })
        .await
        .map_err(|e| internal(e, "transcode task"))?;

        // 6. 按日期落盘：uploads/YYYY/MM/DD/HHMMSS.<uuid>.<ext>。
        let now = chrono::Utc::now();
        let date = now.format("%Y/%m/%d");
        let uuid_str = uuid::Uuid::new_v4().to_string();
        let dir_path = format!("uploads/{}", date);
        let file_name = format!("{}.{}.{}", now.format("%H%M%S"), uuid_str, final_ext);
        let file_path = format!("{}/{}", dir_path, file_name);
        let url_path = format!("/uploads/{}/{}", date, file_name);

        tokio::fs::create_dir_all(&dir_path)
            .await
            .map_err(|e| internal(e, "create dir"))?;
        tokio::fs::write(&file_path, &final_data)
            .await
            .map_err(|e| internal(e, "write file"))?;

        // 7. 登记 assets 表（ON CONFLICT 兜底并发竞态）。
        let rel_path = format!("{}/{}", date, file_name);
        let final_mime = mime_for_ext(&final_ext);
        let original_filename = p
            .filename
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .unwrap_or_else(|| file_name.clone());

        let registered: Result<Option<String>, McpError> = async {
            let client = get_conn()
                .await
                .map_err(|e| internal(e, "db connection for register"))?;
            let asset_id = uuid::Uuid::new_v4();
            let inserted = client
                .execute(
                    "INSERT INTO assets (id, path, filename, mime, size_bytes, width, height, content_hash)\
                     VALUES ($1, $2, $3, $4, $5, $6, $7, $8) \
                     ON CONFLICT (content_hash) DO NOTHING",
                    &[
                        &asset_id,
                        &rel_path,
                        &original_filename,
                        &final_mime,
                        &(final_data.len() as i64),
                        &(img_width as i32),
                        &(img_height as i32),
                        &content_hash,
                    ],
                )
                .await
                .map_err(|e| internal(e, "register asset"))?;
            if inserted == 0 {
                // 并发竞态落败：复用胜出者路径。
                let row = client
                    .query_one(
                        "SELECT path FROM assets WHERE content_hash = $1",
                        &[&content_hash],
                    )
                    .await
                    .map_err(|e| internal(e, "select reused asset"))?;
                return Ok(Some(row.get(0)));
            }
            Ok(None)
        }
        .await;

        match registered {
            Ok(Some(reused_path)) => {
                // 竞态落败：删除自己刚写的文件，复用胜出者。
                let _ = tokio::fs::remove_file(&file_path).await;
                ok_json(UploadResult {
                    success: true,
                    url: format!("/uploads/{}", reused_path),
                    reused: true,
                    width: img_width,
                    height: img_height,
                    mime: mime_type.to_string(),
                })
            }
            Ok(None) => {
                tracing::info!(
                    "MCP media uploaded: {} ({} bytes)",
                    file_path,
                    final_data.len()
                );
                ok_json(UploadResult {
                    success: true,
                    url: url_path,
                    reused: false,
                    width: img_width,
                    height: img_height,
                    mime: final_mime.to_string(),
                })
            }
            Err(e) => {
                // 登记失败：补偿删除已落盘文件。
                let _ = tokio::fs::remove_file(&file_path).await;
                Err(e)
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 图片处理辅助（镜像 src/api/upload.rs）
// ---------------------------------------------------------------------------

/// 从 magic bytes 检测 MIME 类型。
fn detect_mime(data: &[u8]) -> Option<&'static str> {
    if data.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("image/jpeg")
    } else if data.starts_with(&[
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A,
    ]) {
        Some("image/png")
    } else if data.starts_with(b"GIF87a") || data.starts_with(b"GIF89a") {
        Some("image/gif")
    } else if data.len() >= 12 && &data[0..4] == b"RIFF" && &data[8..12] == b"WEBP" {
        Some("image/webp")
    } else {
        None
    }
}

/// 图片转码：GIF/WebP 保持原格式，JPEG/PNG 尝试转 WebP（更小才采用）。
fn transcode_image(
    data: &[u8],
    mime: &str,
    is_gif: bool,
    is_webp: bool,
) -> (Vec<u8>, String) {
    if is_gif {
        return (data.to_vec(), "gif".to_string());
    }
    if is_webp {
        return (data.to_vec(), "webp".to_string());
    }

    // JPEG/PNG → 尝试 WebP。
    let format = match mime {
        "image/jpeg" => image::ImageFormat::Jpeg,
        "image/png" => image::ImageFormat::Png,
        _ => image::ImageFormat::Jpeg,
    };
    let cursor = std::io::Cursor::new(data);
    let mut reader = image::ImageReader::with_format(cursor, format);
    reader.limits(crate::api::image::image_reader_limits());

    match reader.decode() {
        Ok(img) => {
            let config = crate::webp::WEBP_CONFIG.clone();
            match crate::webp::encode(&img, config.quality, config.method) {
                Ok(webp_data) if webp_data.len() < data.len() => {
                    tracing::info!(
                        "MCP WebP conversion: {}x{} {} -> {} bytes",
                        img.width(),
                        img.height(),
                        data.len(),
                        webp_data.len()
                    );
                    (webp_data, "webp".to_string())
                }
                Ok(_) => {
                    // WebP 更大，保留原格式。
                    (data.to_vec(), mime_to_ext(mime).to_string())
                }
                Err(e) => {
                    tracing::warn!("MCP WebP encode failed ({}), keeping original", e);
                    (data.to_vec(), mime_to_ext(mime).to_string())
                }
            }
        }
        Err(e) => {
            tracing::warn!("MCP image decode failed ({}), keeping original", e);
            (data.to_vec(), mime_to_ext(mime).to_string())
        }
    }
}

fn mime_to_ext(mime: &str) -> &'static str {
    match mime {
        "image/jpeg" => "jpg",
        "image/png" => "png",
        "image/webp" => "webp",
        "image/gif" => "gif",
        _ => "bin",
    }
}

fn mime_for_ext(ext: &str) -> &'static str {
    match ext {
        "jpg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        _ => "image/webp",
    }
}

// ---------------------------------------------------------------------------
// 参数与输出结构
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct UploadMediaParams {
    /// 原始文件名（仅作 assets 表展示字段，不影响落盘路径）。
    #[serde(default)]
    pub filename: Option<String>,
    /// base64 编码的图片字节（支持 data URL 前缀，会自动去除）。
    pub base64: String,
    /// 替代文本（alt），目前未持久化，保留供未来扩展。
    #[serde(default)]
    #[allow(dead_code)] // 面向未来：客户端可传入，assets 表未存 alt 列
    pub alt: Option<String>,
}

#[derive(Debug, serde::Serialize)]
struct UploadResult {
    success: bool,
    url: String,
    reused: bool,
    width: u32,
    height: u32,
    mime: String,
}

