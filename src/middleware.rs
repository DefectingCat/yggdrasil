//! Axum 中间件与启动期纯函数。
//!
//! 从 `main.rs` 抽出的、可独立测试的服务端 HTTP 中间件（cache-control、admin
//! 守卫）与压缩层构造逻辑。整体 server-only——WASM 构建不会编译本模块。
//!
//! 这些函数此前散落在 `main.rs`，既无法单独测试也使入口职责过载。迁移后
//! `main.rs` 的路由组装以全路径 `crate::middleware::xxx` 引用，语义不变。

#![cfg(feature = "server")]

/// 压缩算法配置。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CompressionAlgorithms {
    gzip: bool,
    brotli: bool,
    deflate: bool,
    zstd: bool,
}

impl CompressionAlgorithms {
    fn all_enabled() -> Self {
        Self {
            gzip: true,
            brotli: true,
            deflate: true,
            zstd: true,
        }
    }

    fn is_empty(&self) -> bool {
        !self.gzip && !self.brotli && !self.deflate && !self.zstd
    }
}

/// 解析 COMPRESSION_ALGORITHMS 环境变量值。
/// ""、"none"、"off" 返回 None；"all" 或未识别到任何算法时启用全部。
pub(crate) fn parse_compression_algorithms(env: &str) -> Option<CompressionAlgorithms> {
    let env = env.trim();
    if env.is_empty() || env.eq_ignore_ascii_case("none") || env.eq_ignore_ascii_case("off") {
        return None;
    }

    let mut all = false;
    let mut gzip = false;
    let mut brotli = false;
    let mut deflate = false;
    let mut zstd = false;

    for part in env.split(',') {
        match part.trim().to_lowercase().as_str() {
            "all" => all = true,
            "gzip" => gzip = true,
            "brotli" | "br" => brotli = true,
            "deflate" => deflate = true,
            "zstd" => zstd = true,
            other => tracing::warn!(
                "Unknown compression algorithm in COMPRESSION_ALGORITHMS: '{}'",
                other
            ),
        }
    }

    if all {
        return Some(CompressionAlgorithms::all_enabled());
    }

    let algorithms = CompressionAlgorithms {
        gzip,
        brotli,
        deflate,
        zstd,
    };
    if algorithms.is_empty() {
        return None;
    }

    Some(algorithms)
}

/// 根据 COMPRESSION_ALGORITHMS 环境变量构造 CompressionLayer。
/// 未设置或设置为 "all" 时启用全部算法；设置为 ""、"none" 或 "off" 时禁用。
///
/// CompressionLayer 使用 tower-http 的 `DefaultPredicate`，开箱即用即：
/// - 跳过 `image/*` content-type（WebP/PNG/JPEG/GIF 等已是压缩格式，再压浪费 CPU，
///   唯一例外是 `image/svg+xml`，作为 XML 文本可被压缩）；
/// - 跳过 gRPC 与 `text/event-stream`（SSE）；
/// - 跳过小于 32 字节的响应。
///
/// 因此无需在此处对图片响应做额外的 content-type 过滤。另：图片实际挂在
/// `static_routes`（无中间件），根本不经此层，详见 main.rs 路由 merge 处。
pub(crate) fn compression_layer_from_env() -> Option<tower_http::compression::CompressionLayer> {
    use tower_http::compression::CompressionLayer;

    let env = std::env::var("COMPRESSION_ALGORITHMS").unwrap_or_else(|_| "all".to_string());
    let algorithms = parse_compression_algorithms(&env)?;

    Some(
        CompressionLayer::new()
            .gzip(algorithms.gzip)
            .br(algorithms.brotli)
            .deflate(algorithms.deflate)
            .zstd(algorithms.zstd),
    )
}

/// 根据请求路径和方法决定公开页面的 Cache-Control 头。
/// 返回 None 表示不添加缓存头（保留现有行为或避免覆盖）。
pub(crate) fn cache_control_for_path(
    path: &str,
    method: &axum::http::Method,
) -> Option<axum::http::HeaderValue> {
    use axum::http::{HeaderValue, Method};

    // 只对 GET/HEAD 请求添加缓存头
    if *method != Method::GET && *method != Method::HEAD {
        return None;
    }

    // API 接口：不缓存（可能涉及认证、写操作）
    if path.starts_with("/api") {
        return None;
    }

    // 管理后台和认证页面：不缓存
    if path.starts_with("/admin") || path == "/login" || path == "/register" {
        return None;
    }

    // 静态资源：长期缓存（Dioxus/WASM 资源通常带内容哈希）
    if path.starts_with("/_dioxus/")
        || path.starts_with("/wasm/")
        || path.ends_with(".wasm")
        || path.ends_with(".js")
        || path == "/style.css"
        || path == "/highlight.css"
    {
        return Some(HeaderValue::from_static(
            "public, max-age=31536000, immutable",
        ));
    }

    // 公开页面：5 分钟新鲜期，过期后 1 小时内可提供过期内容并后台重新验证
    Some(HeaderValue::from_static(
        "public, max-age=300, stale-while-revalidate=3600",
    ))
}

/// Axum 中间件：为公开页面和静态资源附加 Cache-Control 头。
pub(crate) async fn add_cache_control(
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    use axum::http::header;

    let path = req.uri().path().to_string();
    let method = req.method().clone();
    let cache_value = cache_control_for_path(&path, &method);

    let mut response = next.run(req).await;

    if let Some(value) = cache_value {
        // 仅当响应尚未设置 Cache-Control 时才添加，避免覆盖已有策略
        response
            .headers_mut()
            .entry(header::CACHE_CONTROL)
            .or_insert(value);
    }

    response
}

/// Axum 中间件：`/admin*` 的 SSR 层认证守卫。
///
/// 未登录访问后台时，服务端**直接 302 跳转 `/login`**，根本不进入 Dioxus
/// SSR 渲染器。此前后台鉴权完全在客户端 WASM 完成（SSR 渲染骨架屏 → WASM
/// 下载/编译 → hydrate → 异步 `get_current_user()` → 客户端 `navigator.push`），
/// 整条链串行，未登录用户首屏要"空白好久"才跳登录。
///
/// - 只匹配 `/admin*`，其它路径（`/login`、公开页、`/api/*`）直接放行。
/// - 复用 `get_user_by_token`：命中内存缓存 + 校验 `session_generation`
///   （封禁/降级后旧 session 立即失效），与客户端鉴权同一套语义。
/// - DB 错误时 **fail-open**（放行进入 SSR）：避免数据库抖动把已登录的
///   管理员也踢到登录页；客户端 `AdminLayout` 仍有兜底校验。
/// - `/admin` 与 `/login` 本就不进 `cache_control_for_path` 缓存，302 不会被缓存。
pub(crate) async fn admin_guard(
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    use crate::models::user::UserRole;
    use axum::body::Body;
    use axum::http::{header, StatusCode};
    use axum::response::Response;

    let path = req.uri().path().to_string();
    if !path.starts_with("/admin") {
        return next.run(req).await;
    }

    // 从 Cookie 头读 session token（与 export.rs / upload.rs 同款手法）。
    let cookie = req
        .headers()
        .get("cookie")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");
    let token = crate::auth::session::parse_session_token(cookie);

    let is_admin = match token {
        Some(t) => match crate::api::auth::get_user_by_token(t).await {
            Ok(Some(user)) => user.role == UserRole::Admin,
            // Err（DB 抖动）/ Ok(None)（token 无效）：fail-open，交给客户端兜底。
            _ => true,
        },
        // 无 token：明确未登录，拦截。
        None => false,
    };

    if is_admin {
        next.run(req).await
    } else {
        Response::builder()
            .status(StatusCode::FOUND)
            .header(header::LOCATION, "/login")
            .body(Body::empty())
            .unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::{cache_control_for_path, parse_compression_algorithms, CompressionAlgorithms};
    use axum::http::Method;

    fn cache_value(path: &str, method: Method) -> Option<String> {
        cache_control_for_path(path, &method).map(|v| v.to_str().unwrap().to_string())
    }

    #[test]
    fn public_page_is_cached() {
        assert_eq!(
            cache_value("/", Method::GET),
            Some("public, max-age=300, stale-while-revalidate=3600".to_string())
        );
        assert_eq!(
            cache_value("/post/hello-world", Method::GET),
            Some("public, max-age=300, stale-while-revalidate=3600".to_string())
        );
        assert_eq!(
            cache_value("/tags/rust", Method::GET),
            Some("public, max-age=300, stale-while-revalidate=3600".to_string())
        );
    }

    #[test]
    fn static_assets_are_cached_long_term() {
        assert_eq!(
            cache_value("/style.css", Method::GET),
            Some("public, max-age=31536000, immutable".to_string())
        );
        assert_eq!(
            cache_value("/highlight.css", Method::GET),
            Some("public, max-age=31536000, immutable".to_string())
        );
        assert_eq!(
            cache_value("/wasm/app.wasm", Method::GET),
            Some("public, max-age=31536000, immutable".to_string())
        );
        assert_eq!(
            cache_value("/_dioxus/assets/main.js", Method::GET),
            Some("public, max-age=31536000, immutable".to_string())
        );
    }

    #[test]
    fn api_and_admin_and_auth_are_not_cached() {
        assert_eq!(cache_value("/api/posts", Method::GET), None);
        assert_eq!(cache_value("/admin", Method::GET), None);
        assert_eq!(cache_value("/admin/posts", Method::GET), None);
        assert_eq!(cache_value("/login", Method::GET), None);
        assert_eq!(cache_value("/register", Method::GET), None);
    }

    #[test]
    fn non_get_requests_are_not_cached() {
        assert_eq!(cache_value("/", Method::POST), None);
        assert_eq!(cache_value("/post/hello-world", Method::POST), None);
        assert_eq!(cache_value("/style.css", Method::POST), None);
    }

    #[test]
    fn head_requests_are_cached_like_get() {
        assert_eq!(
            cache_value("/", Method::HEAD),
            Some("public, max-age=300, stale-while-revalidate=3600".to_string())
        );
    }

    #[test]
    fn compression_all_enables_everything() {
        assert_eq!(
            parse_compression_algorithms("all"),
            Some(CompressionAlgorithms::all_enabled())
        );
    }

    #[test]
    fn compression_default_env_is_all() {
        // 模拟未设置环境变量时的默认值
        assert_eq!(
            parse_compression_algorithms("all"),
            Some(CompressionAlgorithms::all_enabled())
        );
    }

    #[test]
    fn compression_empty_none_off_disable() {
        assert_eq!(parse_compression_algorithms(""), None);
        assert_eq!(parse_compression_algorithms("none"), None);
        assert_eq!(parse_compression_algorithms("NONE"), None);
        assert_eq!(parse_compression_algorithms("off"), None);
        assert_eq!(parse_compression_algorithms("OFF"), None);
    }

    #[test]
    fn compression_single_algorithm() {
        assert_eq!(
            parse_compression_algorithms("gzip"),
            Some(CompressionAlgorithms {
                gzip: true,
                brotli: false,
                deflate: false,
                zstd: false,
            })
        );
        assert_eq!(
            parse_compression_algorithms("br"),
            Some(CompressionAlgorithms {
                gzip: false,
                brotli: true,
                deflate: false,
                zstd: false,
            })
        );
    }

    #[test]
    fn compression_multiple_algorithms() {
        assert_eq!(
            parse_compression_algorithms("gzip, zstd"),
            Some(CompressionAlgorithms {
                gzip: true,
                brotli: false,
                deflate: false,
                zstd: true,
            })
        );
    }

    #[test]
    fn compression_case_insensitive_and_whitespace_tolerant() {
        assert_eq!(
            parse_compression_algorithms("GZIP, Brotli, Deflate, Zstd"),
            Some(CompressionAlgorithms::all_enabled())
        );
        assert_eq!(
            parse_compression_algorithms(" gzip , br , deflate , zstd "),
            Some(CompressionAlgorithms::all_enabled())
        );
    }

    #[test]
    fn compression_unknown_algorithms_are_ignored() {
        assert_eq!(
            parse_compression_algorithms("gzip, unknown, lz4"),
            Some(CompressionAlgorithms {
                gzip: true,
                brotli: false,
                deflate: false,
                zstd: false,
            })
        );
    }
}
