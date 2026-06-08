#![allow(clippy::unused_unit)]

#[cfg(feature = "server")]
use axum::{
    extract::{Path, Query},
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
#[cfg(feature = "server")]
use serde::Deserialize;
#[cfg(feature = "server")]
use std::sync::LazyLock;
#[cfg(feature = "server")]
use moka::future::Cache;

#[cfg(feature = "server")]
const MAX_IMAGE_DIMENSION: u32 = 4096;
#[cfg(feature = "server")]
const DEFAULT_JPEG_QUALITY: u8 = 85;

#[cfg(feature = "server")]
#[derive(Debug, Clone)]
struct CachedImage {
    data: Vec<u8>,
    content_type: HeaderValue,
}

#[cfg(feature = "server")]
static IMAGE_CACHE: LazyLock<Cache<String, CachedImage>> = LazyLock::new(|| {
    Cache::builder()
        .max_capacity(100)
        .time_to_idle(std::time::Duration::from_secs(300))
        .build()
});

#[cfg(feature = "server")]
#[derive(Debug, Deserialize, Clone, Hash, Eq, PartialEq, Default)]
pub struct ImageParams {
    pub w: Option<u32>,
    pub h: Option<u32>,
    pub thumb: Option<String>,
    pub rotate: Option<u16>,
    pub format: Option<String>,
    pub quality: Option<u8>,
}

#[cfg(feature = "server")]
impl ImageParams {
    fn is_empty(&self) -> bool {
        self.w.is_none()
            && self.h.is_none()
            && self.thumb.is_none()
            && self.rotate.is_none()
            && self.format.is_none()
    }

    fn cache_key(&self, path: &str) -> String {
        let mut key = format!(
            "{}?w={:?}&h={:?}&thumb={:?}&rotate={:?}&format={:?}",
            path, self.w, self.h, self.thumb, self.rotate, self.format,
        );
        if let Some(q) = self.quality {
            key.push_str(&format!("&quality={}", q));
        }
        key
    }

    fn validate(&self) -> Result<(), StatusCode> {
        if let Some(dim) = self.w {
            if dim == 0 || dim > MAX_IMAGE_DIMENSION {
                return Err(StatusCode::BAD_REQUEST);
            }
        }
        if let Some(dim) = self.h {
            if dim == 0 || dim > MAX_IMAGE_DIMENSION {
                return Err(StatusCode::BAD_REQUEST);
            }
        }
        if let Some(r) = self.rotate {
            if !matches!(r, 0 | 90 | 180 | 270) {
                return Err(StatusCode::BAD_REQUEST);
            }
        }
        if let Some(ref fmt) = self.format {
            if !matches!(fmt.as_str(), "jpeg" | "jpg" | "png" | "webp") {
                return Err(StatusCode::BAD_REQUEST);
            }
        }
        if let Some(q) = self.quality {
            if q == 0 || q > 100 {
                return Err(StatusCode::BAD_REQUEST);
            }
        }
        Ok(())
    }
}

#[cfg(feature = "server")]
fn detect_format(path: &str) -> image::ImageFormat {
    let lower = path.to_lowercase();
    if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
        image::ImageFormat::Jpeg
    } else if lower.ends_with(".png") {
        image::ImageFormat::Png
    } else if lower.ends_with(".webp") {
        image::ImageFormat::WebP
    } else if lower.ends_with(".gif") {
        image::ImageFormat::Gif
    } else {
        image::ImageFormat::Jpeg
    }
}

#[cfg(feature = "server")]
fn content_type(format: image::ImageFormat) -> HeaderValue {
    match format {
        image::ImageFormat::Jpeg => HeaderValue::from_static("image/jpeg"),
        image::ImageFormat::Png => HeaderValue::from_static("image/png"),
        image::ImageFormat::WebP => HeaderValue::from_static("image/webp"),
        image::ImageFormat::Gif => HeaderValue::from_static("image/gif"),
        _ => HeaderValue::from_static("application/octet-stream"),
    }
}

#[cfg(feature = "server")]
fn process_image(
    img: image::DynamicImage,
    params: &ImageParams,
    original_format: image::ImageFormat,
) -> Result<(Vec<u8>, HeaderValue), StatusCode> {
    let mut img = img;

    // Rotate first, then resize
    if let Some(degrees) = params.rotate {
        img = match degrees {
            90 => img.rotate90(),
            180 => img.rotate180(),
            270 => img.rotate270(),
            _ => img,
        };
    }

    // Resize by max dimensions (keep aspect ratio)
    if params.w.is_some() || params.h.is_some() {
        let max_w = params.w.unwrap_or(img.width());
        let max_h = params.h.unwrap_or(img.height());
        if img.width() > max_w || img.height() > max_h {
            img = img.resize(max_w, max_h, image::imageops::FilterType::Lanczos3);
        }
    }

    // Thumbnail: fit-in-box (same semantics as resize, but both dimensions required)
    if let Some(ref thumb_spec) = params.thumb {
        let parts: Vec<&str> = thumb_spec.split('x').collect();
        if parts.len() == 2 {
            let tw: u32 = parts[0].parse().map_err(|_| StatusCode::BAD_REQUEST)?;
            let th: u32 = parts[1].parse().map_err(|_| StatusCode::BAD_REQUEST)?;
            if tw > 0 && th > 0 && tw <= MAX_IMAGE_DIMENSION && th <= MAX_IMAGE_DIMENSION {
                img = img.thumbnail(tw, th);
            }
        }
    }

    // Output format
    let output_format = match params.format.as_deref() {
        Some("webp") => image::ImageFormat::WebP,
        Some("png") => image::ImageFormat::Png,
        Some("jpeg") | Some("jpg") => image::ImageFormat::Jpeg,
        _ => original_format,
    };

    let quality = params.quality.unwrap_or(DEFAULT_JPEG_QUALITY);

    let mut buf = std::io::Cursor::new(Vec::new());
    match output_format {
        image::ImageFormat::Jpeg => {
            let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, quality);
            img.write_with_encoder(encoder)
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        }
        _ => {
            img.write_to(&mut buf, output_format)
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        }
    }

    let ct = content_type(output_format);
    Ok((buf.into_inner(), ct))
}

#[cfg(feature = "server")]
fn is_path_safe(path: &str) -> bool {
    // Reject paths with parent directory references or null bytes
    if path.contains("..") || path.contains('\0') {
        return false;
    }
    // Reject absolute paths
    if path.starts_with('/') {
        return false;
    }
    true
}

#[cfg(feature = "server")]
pub async fn serve_image(
    Path(path): Path<String>,
    Query(params): Query<ImageParams>,
) -> Response {
    // Path traversal protection
    if !is_path_safe(&path) {
        return StatusCode::FORBIDDEN.into_response();
    }

    let file_path = format!("uploads/{}", path);

    // Validate params
    if let Err(status) = params.validate() {
        return status.into_response();
    }

    // No processing params: return raw file
    if params.is_empty() {
        return match tokio::fs::read(&file_path).await {
            Ok(data) => {
                let ct = content_type(detect_format(&path));
                (StatusCode::OK, [(header::CONTENT_TYPE, ct)], data).into_response()
            }
            Err(_) => StatusCode::NOT_FOUND.into_response(),
        };
    }

    // Check cache
    let cache_key = params.cache_key(&path);
    if let Some(cached) = IMAGE_CACHE.get(&cache_key).await {
        return (StatusCode::OK, [(header::CONTENT_TYPE, cached.content_type)], cached.data)
            .into_response();
    }

    // Read file
    let data = match tokio::fs::read(&file_path).await {
        Ok(d) => d,
        Err(_) => return StatusCode::NOT_FOUND.into_response(),
    };

    // Load image
    let original_format = detect_format(&path);
    let img = match image::load_from_memory_with_format(&data, original_format) {
        Ok(img) => img,
        Err(_) => {
            // Not a valid image or unsupported format, return raw
            let ct = content_type(original_format);
            return (StatusCode::OK, [(header::CONTENT_TYPE, ct)], data).into_response();
        }
    };

    // Process
    let (processed, content_type) = match process_image(img, &params, original_format) {
        Ok(r) => r,
        Err(status) => return status.into_response(),
    };

    // Cache result
    let cached = CachedImage {
        data: processed.clone(),
        content_type: content_type.clone(),
    };
    let _ = IMAGE_CACHE.insert(cache_key, cached).await;

    (StatusCode::OK, [(header::CONTENT_TYPE, content_type)], processed).into_response()
}

#[cfg(not(feature = "server"))]
pub async fn serve_image() {}
