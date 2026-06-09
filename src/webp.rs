#[cfg(feature = "server")]
use std::sync::LazyLock;

#[derive(Debug)]
pub enum WebpError {
    Encode(String),
    Decode(String),
}

impl std::fmt::Display for WebpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WebpError::Encode(msg) => write!(f, "WebP encode error: {}", msg),
            WebpError::Decode(msg) => write!(f, "WebP decode error: {}", msg),
        }
    }
}

impl std::error::Error for WebpError {}

#[cfg(feature = "server")]
#[derive(Debug, Clone)]
pub struct WebpConfig {
    pub quality: f32,
    pub method: u8,
}

#[cfg(feature = "server")]
pub static WEBP_CONFIG: LazyLock<WebpConfig> = LazyLock::new(|| {
    let quality = std::env::var("WEBP_QUALITY")
        .ok()
        .and_then(|s| s.parse::<f32>().ok())
        .map(|q| q.clamp(0.0, 100.0))
        .unwrap_or(85.0);

    let method = std::env::var("WEBP_METHOD")
        .ok()
        .and_then(|s| s.parse::<u8>().ok())
        .map(|m| m.clamp(0, 6))
        .unwrap_or(2);

    tracing::info!("WebP config loaded: quality={}, method={}", quality, method);
    WebpConfig { quality, method }
});

#[cfg(feature = "server")]
pub fn encode(img: &image::DynamicImage, quality: f32, method: u8) -> Result<Vec<u8>, WebpError> {
    use zenwebp::{EncodeRequest, LossyConfig, PixelLayout};

    let (width, height) = (img.width(), img.height());
    let config = LossyConfig::new().with_quality(quality).with_method(method);

    match img {
        image::DynamicImage::ImageRgba8(rgba) => {
            let pixels = rgba.as_raw();
            EncodeRequest::lossy(&config, pixels, PixelLayout::Rgba8, width, height)
                .encode()
                .map_err(|e| WebpError::Encode(e.to_string()))
        }
        image::DynamicImage::ImageRgb8(rgb) => {
            let pixels = rgb.as_raw();
            EncodeRequest::lossy(&config, pixels, PixelLayout::Rgb8, width, height)
                .encode()
                .map_err(|e| WebpError::Encode(e.to_string()))
        }
        _ => {
            // Convert other formats to RGBA8
            let rgba = img.to_rgba8();
            let pixels = rgba.as_raw();
            EncodeRequest::lossy(&config, pixels, PixelLayout::Rgba8, width, height)
                .encode()
                .map_err(|e| WebpError::Encode(e.to_string()))
        }
    }
}

#[cfg(feature = "server")]
pub fn decode(data: &[u8]) -> Result<image::DynamicImage, WebpError> {
    use zenwebp::WebPDecoder;

    let mut decoder = WebPDecoder::build(data)
        .map_err(|e| WebpError::Decode(format!("Failed to build decoder: {}", e)))?;

    let info = decoder.info();
    let width = info.width;
    let height = info.height;
    let has_alpha = info.has_alpha;

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
        // For RGB output, the buffer is width * height * 3
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
}
