//! SSR HTML 空白压缩中间件。
//!
//! 对 Dioxus fullstack 返回的 `text/html` 响应做轻量 minify。
//!
//! 注意：这里**不**维护 per-URL 缓存。同一个 URL 在 admin 登录态与匿名态下
//! 渲染出的 HTML 不同（例如是否带管理按钮），按 URL 缓存会导致跨用户内容
//! 泄露。Dioxus 的增量渲染器（ISRG）本身已经按 URL 缓存了 SSR HTML，
//! 这里的 minify 只是对其输出做一次无损空白折叠，无需再叠加一层缓存。

#[cfg(feature = "server")]
use axum::{
    body::Body,
    extract::Request,
    http::{header, StatusCode},
    middleware::Next,
    response::Response,
};
use http_body_util::BodyExt;

/// Axum 中间件入口。
pub async fn layer(request: Request, next: Next) -> Response {
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
