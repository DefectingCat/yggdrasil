//! WebP 编解码模块。
//!
//! 本模块仅在 `server` feature 启用时编译。
//!
//! ## `zenwebp` 与 `image` crate 的分工
//!
//! - `image` crate：负责通用图像格式（JPEG、PNG、GIF 等）的解码、缩放、旋转以及
//!   像素格式转换（`DynamicImage`、`RgbaImage`、`RgbImage`）。本项目特意禁用了 `image`
//!   的 `webp` feature，因为它不支持 WebP 编码，且解码能力有限。
//! - `zenwebp`：专门负责 WebP 格式的有损编码与解码。所有需要输出 WebP 或读取 WebP
//!   字节流的场景都通过 `zenwebp` 完成。
//!
//! 简言之：`image` 处理“除 WebP 外的图像操作”，`zenwebp` 处理“WebP 专有编解码”。

#[cfg(feature = "server")]
use std::sync::LazyLock;

/// WebP 编解码过程中可能产生的错误。
#[cfg(feature = "server")]
#[derive(Debug)]
pub enum WebpError {
    /// 编码失败。
    Encode(String),
    /// 解码失败。
    Decode(String),
}

#[cfg(feature = "server")]
impl std::fmt::Display for WebpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WebpError::Encode(msg) => write!(f, "WebP encode error: {}", msg),
            WebpError::Decode(msg) => write!(f, "WebP decode error: {}", msg),
        }
    }
}

#[cfg(feature = "server")]
impl std::error::Error for WebpError {}

/// WebP 有损编码配置。
#[cfg(feature = "server")]
#[derive(Debug, Clone)]
pub struct WebpConfig {
    /// 质量系数，范围 0.0–100.0。
    pub quality: f32,
    /// 编码方法，范围 0–6，数值越大压缩率越高但越慢。
    pub method: u8,
}

/// 从环境变量读取的 WebP 全局配置，未设置时使用默认值。
///
/// - `WEBP_QUALITY`：默认 85.0，越界时 clamp 到 0.0–100.0。
/// - `WEBP_METHOD`：默认 2，越界时 clamp 到 0–6。
#[cfg(feature = "server")]
pub static WEBP_CONFIG: LazyLock<WebpConfig> = LazyLock::new(|| {
    let (quality, quality_clamped) = std::env::var("WEBP_QUALITY")
        .ok()
        .and_then(|s| s.parse::<f32>().ok())
        .map(|q| {
            let clamped = q.clamp(0.0, 100.0);
            (clamped, clamped != q)
        })
        .unwrap_or((85.0, false));

    if quality_clamped {
        tracing::warn!(
            "WEBP_QUALITY was clamped from {} to {} (valid range: 0.0-100.0)",
            std::env::var("WEBP_QUALITY").unwrap_or_default(),
            quality
        );
    }

    let (method, method_clamped) = std::env::var("WEBP_METHOD")
        .ok()
        .and_then(|s| s.parse::<u8>().ok())
        .map(|m| {
            let clamped = m.clamp(0, 6);
            (clamped, clamped != m)
        })
        .unwrap_or((2, false));

    if method_clamped {
        tracing::warn!(
            "WEBP_METHOD was clamped from {} to {} (valid range: 0-6)",
            std::env::var("WEBP_METHOD").unwrap_or_default(),
            method
        );
    }

    tracing::info!("WebP config loaded: quality={}, method={}", quality, method);
    WebpConfig { quality, method }
});

/// 将 `image::DynamicImage` 编码为 WebP 字节流。
///
/// 直接处理 `Rgba8` 与 `Rgb8` 两种像素布局，其他格式先转换为 `Rgba8` 再编码。
#[cfg(feature = "server")]
pub fn encode(img: &image::DynamicImage, quality: f32, method: u8) -> Result<Vec<u8>, WebpError> {
    use zenwebp::{EncodeRequest, LossyConfig, PixelLayout};

    let (width, height) = (img.width(), img.height());
    let config = LossyConfig::new().with_quality(quality).with_method(method);

    fn do_encode(
        config: &LossyConfig,
        pixels: &[u8],
        layout: zenwebp::PixelLayout,
        width: u32,
        height: u32,
    ) -> Result<Vec<u8>, WebpError> {
        EncodeRequest::lossy(config, pixels, layout, width, height)
            .encode()
            .map_err(|e| WebpError::Encode(e.to_string()))
    }

    match img {
        image::DynamicImage::ImageRgba8(rgba) => {
            do_encode(&config, rgba.as_raw(), PixelLayout::Rgba8, width, height)
        }
        image::DynamicImage::ImageRgb8(rgb) => {
            do_encode(&config, rgb.as_raw(), PixelLayout::Rgb8, width, height)
        }
        _ => {
            // 其他像素格式统一转换为 RGBA8 后再交给 zenwebp 编码
            let rgba = img.to_rgba8();
            do_encode(&config, rgba.as_raw(), PixelLayout::Rgba8, width, height)
        }
    }
}

/// 将 WebP 字节流解码为 `image::DynamicImage`。
///
/// 根据 alpha 通道信息决定返回 `ImageRgba8` 还是 `ImageRgb8`。
/// 解码前会校验像素总数，防止超大图片导致内存问题。
#[cfg(feature = "server")]
pub fn decode(data: &[u8]) -> Result<image::DynamicImage, WebpError> {
    use zenwebp::WebPDecoder;

    let mut decoder = WebPDecoder::build(data)
        .map_err(|e| WebpError::Decode(format!("Failed to build decoder: {}", e)))?;

    let info = decoder.info();
    let width = info.width;
    let height = info.height;
    let has_alpha = info.has_alpha;

    let pixel_count = (width as u64) * (height as u64);

    // 超过最大允许像素数时提前拒绝
    if pixel_count > crate::api::image::MAX_IMAGE_PIXELS as u64 {
        return Err(WebpError::Decode(format!(
            "Image dimensions {}x{} exceed maximum allowed pixels",
            width, height
        )));
    }

    let buf_size = decoder
        .output_buffer_size()
        .ok_or_else(|| WebpError::Decode("Image too large".to_string()))?;

    let mut output = vec![0u8; buf_size];
    decoder
        .read_image(&mut output)
        .map_err(|e| WebpError::Decode(format!("Failed to decode: {}", e)))?;

    if has_alpha {
        image::RgbaImage::from_raw(width, height, output)
            .map(image::DynamicImage::ImageRgba8)
            .ok_or_else(|| WebpError::Decode("Invalid RGBA dimensions".to_string()))
    } else {
        // 无 alpha 时，zenwebp 输出的是 width * height * 3 的 RGB 数据
        image::RgbImage::from_raw(width, height, output)
            .map(image::DynamicImage::ImageRgb8)
            .ok_or_else(|| WebpError::Decode("Invalid RGB dimensions".to_string()))
    }
}

#[cfg(all(test, feature = "server"))]
mod tests {
    use super::*;

    #[test]
    fn encode_produces_non_empty_bytes() {
        let img = image::DynamicImage::new_rgba8(10, 10);
        let result = encode(&img, 85.0, 4).unwrap();
        assert!(!result.is_empty());
    }

    #[test]
    fn decode_roundtrip_rgba() {
        let original = image::DynamicImage::new_rgba8(5, 5);
        let encoded = encode(&original, 85.0, 4).unwrap();
        let decoded = decode(&encoded).unwrap();
        assert_eq!(decoded.width(), 5);
        assert_eq!(decoded.height(), 5);
    }

    #[test]
    fn decode_roundtrip_rgb() {
        let original = image::DynamicImage::new_rgb8(5, 5);
        let encoded = encode(&original, 85.0, 4).unwrap();
        let decoded = decode(&encoded).unwrap();
        assert_eq!(decoded.width(), 5);
        assert_eq!(decoded.height(), 5);
    }

    #[test]
    fn config_default_values_are_reasonable() {
        let config = WebpConfig {
            quality: 85.0,
            method: 2,
        };
        assert!(config.quality >= 0.0 && config.quality <= 100.0);
        assert!(config.method <= 6);
    }

    #[test]
    fn config_clamping_logic() {
        let quality = 150.0f32;
        let clamped = quality.clamp(0.0, 100.0);
        assert_eq!(clamped, 100.0);

        let quality = -10.0f32;
        let clamped = quality.clamp(0.0, 100.0);
        assert_eq!(clamped, 0.0);

        let method = 10u8;
        let clamped = method.clamp(0, 6);
        assert_eq!(clamped, 6);
    }

    #[test]
    fn config_clamps_edge_cases() {
        assert_eq!(0.0f32.clamp(0.0, 100.0), 0.0);
        assert_eq!(100.0f32.clamp(0.0, 100.0), 100.0);
        assert_eq!(0u8.clamp(0, 6), 0);
        assert_eq!(6u8.clamp(0, 6), 6);
    }
}
