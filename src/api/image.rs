#[cfg(feature = "server")]
use axum::{
    extract::{Path, Query},
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
};
#[cfg(feature = "server")]
use moka::future::Cache;
#[cfg(feature = "server")]
use serde::Deserialize;
#[cfg(feature = "server")]
use std::sync::LazyLock;

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
            && self.quality.is_none()
    }

    fn cache_key(&self, path: &str) -> String {
        let mut parts = vec![path.to_string()];
        if let Some(w) = self.w {
            parts.push(format!("w={}", w));
        }
        if let Some(h) = self.h {
            parts.push(format!("h={}", h));
        }
        if let Some(ref thumb) = self.thumb {
            parts.push(format!("thumb={}", thumb));
        }
        if let Some(r) = self.rotate {
            parts.push(format!("rotate={}", r));
        }
        if let Some(ref fmt) = self.format {
            parts.push(format!("format={}", fmt));
        }
        if let Some(q) = self.quality {
            parts.push(format!("quality={}", q));
        }
        parts.join("|")
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
            if !matches!(fmt.to_lowercase().as_str(), "jpeg" | "jpg" | "png" | "webp") {
                return Err(StatusCode::BAD_REQUEST);
            }
        }
        if let Some(ref thumb) = self.thumb {
            let parts: Vec<&str> = thumb.split('x').collect();
            if parts.len() != 2 {
                return Err(StatusCode::BAD_REQUEST);
            }
            let tw: u32 = parts[0].parse().map_err(|_| StatusCode::BAD_REQUEST)?;
            let th: u32 = parts[1].parse().map_err(|_| StatusCode::BAD_REQUEST)?;
            if tw == 0 || th == 0 || tw > MAX_IMAGE_DIMENSION || th > MAX_IMAGE_DIMENSION {
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

    // Output format (case-insensitive)
    let output_format = match params.format.as_deref().map(str::to_lowercase).as_deref() {
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
use axum::http::HeaderMap;

#[cfg(feature = "server")]
const CACHE_DIR: &str = "uploads/.cache";

#[cfg(feature = "server")]
fn disk_cache_base(cache_key: &str) -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    cache_key.hash(&mut hasher);
    let hash = hasher.finish();
    format!("{}/cache_{:016x}", CACHE_DIR, hash)
}

#[cfg(feature = "server")]
async fn read_disk_cache(cache_key: &str) -> Option<CachedImage> {
    let base = disk_cache_base(cache_key);
    let data = tokio::fs::read(format!("{}.dat", base)).await.ok()?;
    let ct_str = tokio::fs::read_to_string(format!("{}.ct", base))
        .await
        .ok()
        .unwrap_or_else(|| "application/octet-stream".to_string());
    let content_type = HeaderValue::from_str(&ct_str).ok()?;
    Some(CachedImage { data, content_type })
}

#[cfg(feature = "server")]
async fn write_disk_cache(cache_key: &str, cached: &CachedImage) {
    let base = disk_cache_base(cache_key);
    if let Err(e) = tokio::fs::create_dir_all(CACHE_DIR).await {
        tracing::warn!("Failed to create cache dir: {:?}", e);
        return;
    }
    let ct_str = cached.content_type.to_str().unwrap_or("application/octet-stream");
    if let Err(e) = tokio::fs::write(format!("{}.dat", base), &cached.data).await {
        tracing::warn!("Failed to write disk cache data: {:?}", e);
    }
    if let Err(e) = tokio::fs::write(format!("{}.ct", base), ct_str).await {
        tracing::warn!("Failed to write disk cache content type: {:?}", e);
    }
}

#[cfg(feature = "server")]
pub async fn serve_image(
    Path(path): Path<String>,
    Query(params): Query<ImageParams>,
    headers: HeaderMap,
) -> Response {
    let ip = crate::api::rate_limit::get_client_ip(&headers);
    if let Err(status) = crate::api::rate_limit::check_image_limit(&ip) {
        return status.into_response();
    }

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

    let cache_key = params.cache_key(&path);
    if let Some(cached) = IMAGE_CACHE.get(&cache_key).await {
        return (
            StatusCode::OK,
            [(header::CONTENT_TYPE, cached.content_type)],
            cached.data,
        )
            .into_response();
    }

    if let Some(cached) = read_disk_cache(&cache_key).await {
        let _ = IMAGE_CACHE.insert(cache_key.clone(), CachedImage {
            data: cached.data.clone(),
            content_type: cached.content_type.clone(),
        }).await;
        return (
            StatusCode::OK,
            [(header::CONTENT_TYPE, cached.content_type)],
            cached.data,
        )
            .into_response();
    }

    let data = match tokio::fs::read(&file_path).await {
        Ok(d) => d,
        Err(_) => return StatusCode::NOT_FOUND.into_response(),
    };

    let original_format = detect_format(&path);
    let img = match image::load_from_memory_with_format(&data, original_format) {
        Ok(img) => img,
        Err(_) => {
            let ct = content_type(original_format);
            return (StatusCode::OK, [(header::CONTENT_TYPE, ct)], data).into_response();
        }
    };

    let (processed, content_type) = match process_image(img, &params, original_format) {
        Ok(r) => r,
        Err(status) => return status.into_response(),
    };

    let cached = CachedImage {
        data: processed.clone(),
        content_type: content_type.clone(),
    };
    let _ = IMAGE_CACHE.insert(cache_key.clone(), cached.clone()).await;
    write_disk_cache(&cache_key, &cached).await;

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, content_type)],
        processed,
    )
        .into_response()
}

#[cfg(all(test, feature = "server"))]
mod tests {
    use super::*;

    #[test]
    fn image_params_validate_valid_defaults() {
        let params = ImageParams::default();
        assert!(params.validate().is_ok());
    }

    #[test]
    fn image_params_validate_valid_width() {
        let params = ImageParams { w: Some(100), ..Default::default() };
        assert!(params.validate().is_ok());
    }

    #[test]
    fn image_params_validate_zero_width_rejected() {
        let params = ImageParams { w: Some(0), ..Default::default() };
        assert!(params.validate().is_err());
    }

    #[test]
    fn image_params_validate_oversized_width_rejected() {
        let params = ImageParams { w: Some(5000), ..Default::default() };
        assert!(params.validate().is_err());
    }

    #[test]
    fn image_params_validate_valid_rotation() {
        for angle in [0, 90, 180, 270] {
            let params = ImageParams { rotate: Some(angle), ..Default::default() };
            assert!(params.validate().is_ok(), "angle {} should be valid", angle);
        }
    }

    #[test]
    fn image_params_validate_invalid_rotation_rejected() {
        let params = ImageParams { rotate: Some(45), ..Default::default() };
        assert!(params.validate().is_err());
    }

    #[test]
    fn image_params_validate_valid_format() {
        for fmt in &["jpeg", "jpg", "png", "webp", "JPEG", "PNG"] {
            let params = ImageParams { format: Some(fmt.to_string()), ..Default::default() };
            assert!(params.validate().is_ok(), "format {} should be valid", fmt);
        }
    }

    #[test]
    fn image_params_validate_invalid_format_rejected() {
        let params = ImageParams { format: Some("gif".to_string()), ..Default::default() };
        assert!(params.validate().is_err());
    }

    #[test]
    fn image_params_validate_valid_thumbnail() {
        let params = ImageParams { thumb: Some("200x150".to_string()), ..Default::default() };
        assert!(params.validate().is_ok());
    }

    #[test]
    fn image_params_validate_invalid_thumbnail_rejected() {
        let params = ImageParams { thumb: Some("200".to_string()), ..Default::default() };
        assert!(params.validate().is_err());
    }

    #[test]
    fn image_params_validate_valid_quality() {
        let params = ImageParams { quality: Some(85), ..Default::default() };
        assert!(params.validate().is_ok());
    }

    #[test]
    fn image_params_validate_zero_quality_rejected() {
        let params = ImageParams { quality: Some(0), ..Default::default() };
        assert!(params.validate().is_err());
    }

    #[test]
    fn image_params_validate_over_100_quality_rejected() {
        let params = ImageParams { quality: Some(101), ..Default::default() };
        assert!(params.validate().is_err());
    }

    #[test]
    fn is_path_safe_normal() {
        assert!(is_path_safe("images/photo.jpg"));
        assert!(is_path_safe("2024/01/photo.png"));
    }

    #[test]
    fn is_path_safe_rejects_parent_dir() {
        assert!(!is_path_safe("../etc/passwd"));
        assert!(!is_path_safe("foo/../../bar"));
    }

    #[test]
    fn is_path_safe_rejects_null_bytes() {
        assert!(!is_path_safe("foo\0bar"));
    }

    #[test]
    fn is_path_safe_rejects_absolute_path() {
        assert!(!is_path_safe("/etc/passwd"));
    }

    #[test]
    fn detect_format_jpeg() {
        assert!(matches!(detect_format("photo.jpg"), image::ImageFormat::Jpeg));
        assert!(matches!(detect_format("photo.jpeg"), image::ImageFormat::Jpeg));
        assert!(matches!(detect_format("PHOTO.JPG"), image::ImageFormat::Jpeg));
    }

    #[test]
    fn detect_format_png() {
        assert!(matches!(detect_format("icon.png"), image::ImageFormat::Png));
    }

    #[test]
    fn detect_format_webp() {
        assert!(matches!(detect_format("anim.webp"), image::ImageFormat::WebP));
    }

    #[test]
    fn detect_format_defaults_to_jpeg() {
        assert!(matches!(detect_format("file.xyz"), image::ImageFormat::Jpeg));
    }

    #[test]
    fn cache_key_differs_for_different_params() {
        let p1 = ImageParams { w: Some(100), ..Default::default() };
        let p2 = ImageParams { w: Some(200), ..Default::default() };
        assert_ne!(p1.cache_key("img.jpg"), p2.cache_key("img.jpg"));
    }

    #[test]
    fn is_empty_true_when_all_none() {
        let params = ImageParams::default();
        assert!(params.is_empty());
    }

    #[test]
    fn is_empty_false_when_any_set() {
        let params = ImageParams { w: Some(100), ..Default::default() };
        assert!(!params.is_empty());
    }

    #[test]
    fn disk_cache_base_is_deterministic() {
        let key = "path|w=800";
        let base1 = disk_cache_base(key);
        let base2 = disk_cache_base(key);
        assert_eq!(base1, base2);
        assert!(base1.starts_with("uploads/.cache/cache_"));
    }

    #[test]
    fn disk_cache_base_differs_for_different_keys() {
        let base1 = disk_cache_base("path|w=800");
        let base2 = disk_cache_base("path|w=1200");
        assert_ne!(base1, base2);
    }
}
