//! SSR HTML 空白压缩中间件。
//!
//! 对 Dioxus fullstack 返回的 `text/html` 响应做轻量 minify。
//! 为了避免 SSR 增量渲染缓存命中后仍然重复 minify，中间件内部维护了一个按 URL
//! 缓存的内存缓存（容量有限、TTL 较短），minify 后的结果会直接复用。

#[cfg(feature = "server")]
use axum::{
    body::Body,
    extract::Request,
    http::{header, StatusCode},
    middleware::Next,
    response::Response,
};
use http_body_util::BodyExt;
use moka::future::Cache;
use std::time::Duration;

/// 按 URL 缓存 minify 结果，避免 SSR 缓存命中后重复计算。
static MINIFY_CACHE: std::sync::LazyLock<Cache<String, String>> =
    std::sync::LazyLock::new(|| {
        Cache::builder()
            .max_capacity(256)
            .time_to_live(Duration::from_secs(300))
            .build()
    });

/// Axum 中间件入口。
pub async fn layer(request: Request, next: Next) -> Response {
    let uri = request.uri().clone();
    let response = next.run(request).await;

    let is_html = response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|ct| ct.starts_with("text/html"))
        .unwrap_or(false);

    if !is_html {
        return response;
    }

    // 命中缓存则直接返回
    let cache_key = uri.to_string();
    if let Some(cached) = MINIFY_CACHE.get(&cache_key).await {
        return build_response(response, cached);
    }

    let (parts, body) = response.into_parts();
    let bytes = match body.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(_) => {
            return Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::empty())
                .expect("failed to build error response");
        }
    };

    let original = String::from_utf8_lossy(&bytes);
    let minified = crate::utils::html_minify::minify_html(&original);

    // 写入缓存（忽略失败）
    let _ = MINIFY_CACHE.insert(cache_key, minified.clone()).await;

    let mut response = Response::builder()
        .status(parts.status)
        .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
        .body(Body::from(minified.clone()))
        .expect("failed to build minified response");

    // 保留原响应的其他 header，并修正 Content-Length
    *response.headers_mut() = parts.headers;
    response.headers_mut().remove(header::TRANSFER_ENCODING);
    response
        .headers_mut()
        .insert(header::CONTENT_LENGTH, minified.len().into());

    response
}

fn build_response(original: Response, body: String) -> Response {
    let (parts, _body) = original.into_parts();
    let mut response = Response::builder()
        .status(parts.status)
        .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
        .body(Body::from(body.clone()))
        .expect("failed to build cached response");

    *response.headers_mut() = parts.headers;
    response.headers_mut().remove(header::TRANSFER_ENCODING);
    response
        .headers_mut()
        .insert(header::CONTENT_LENGTH, body.len().into());

    response
}
