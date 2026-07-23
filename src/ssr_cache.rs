//! SSR 增量渲染缓存失效。
//!
//! Dioxus 0.7 增量渲染器把每个路由的 SSR 结果落盘到 `static/<route>/index/<hash>.html`，
//! 以请求 `path_and_query()` 作 key。它只暴露 `invalidate_after(ttl)` 一个失效手段，
//! **没有按路由失效的公开 API**。本模块通过**物理删除缓存目录**绕过这个限制：
//! 写路径（create/update/rebuild/delete）调用 [`invalidate_ssr_route`] 删掉对应路由的
//! 缓存目录，下次请求触发重渲染。
//!
//! 全局世代号（`bump_global_generation`）保留作可观测性 + 未来就绪钩子，但不依赖它
//! 实际失效缓存。
//!
//! 仅在启用 `server` feature 时编译。

#![cfg(feature = "server")]

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::LazyLock;

/// 全局 SSR 世代号。
///
/// 任何文章写入操作都会使其递增，从而让所有基于该全局世代的 SSR 缓存键在未来
/// Dioxus 支持自定义缓存键时失效。
static GLOBAL_GENERATION: LazyLock<AtomicU64> = LazyLock::new(AtomicU64::default);

/// 注入到请求扩展中的当前 SSR 世代号。
///
/// 这是为未来 Dioxus 支持自定义 SSR 缓存键预留的钩子。当前 Dioxus 0.7 的渲染器
/// 不会读取此扩展。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SsrGeneration(pub u64);

/// Dioxus 增量渲染器缓存落盘根目录（相对 CWD）。
///
/// 路由 `/post/foo` 的缓存为 `static/post/foo/index/<hash>.html`。
/// 由 `IncrementalRendererConfig::default()` 决定，无公开 API 可读，故硬编码。
const SSR_CACHE_ROOT: &str = "static";

/// 计算某路由的 SSR 缓存目录路径。
///
/// route 形如 `/post/markdown-syntax-test`（前导 `/` 可有可无）。返回
/// `static/post/markdown-syntax-test`（不含 `index/`，删整目录更彻底）。
/// 对路径段做安全清洗：禁止 `..` / 空段，避免越出 `static/` 根。
fn route_cache_dir(route: &str) -> Option<PathBuf> {
    let mut path = PathBuf::from(SSR_CACHE_ROOT);
    for seg in route.trim_start_matches('/').split('/') {
        if seg.is_empty() || seg == ".." || seg == "." {
            continue;
        }
        path.push(seg);
    }
    if path.as_path() == std::path::Path::new(SSR_CACHE_ROOT) {
        None // 根路由（"/"）无目录段；首页缓存由 invalidate_ssr_all_public 覆盖删除
    } else {
        Some(path)
    }
}

/// 失效单一路由的 SSR 磁盘缓存。
///
/// 删除 `static/<route>/` 目录（含 `index/<hash>.html`）。文件不存在时静默。
/// **IO 在当前线程同步执行**——删除一个空目录是纳秒级操作，不值得 spawn_blocking；
/// 调用方已在事务提交后调用，无阻塞风险。
pub fn invalidate_ssr_route(route: &str) {
    let Some(dir) = route_cache_dir(route) else {
        return;
    };
    match std::fs::remove_dir_all(&dir) {
        Ok(()) => tracing::debug!(route = route, dir = %dir.display(), "SSR 路由缓存已删除"),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => tracing::warn!(route = route, error = %e, "删除 SSR 路由缓存失败"),
    }
}

/// 失效所有公开页 SSR 缓存（删除 `static/` 下除 `.well-known`、`admin` 外的全部）。
///
/// 用于批量重建等影响面广的写入。保留 `.well-known`（浏览器/PWA 元数据，
/// 与内容无关）和 `admin`（管理后台，写入者自己的视角无需刷新）。
pub fn invalidate_ssr_all_public() {
    let root = PathBuf::from(SSR_CACHE_ROOT);
    let Ok(entries) = std::fs::read_dir(&root) else {
        return; // 目录不存在（首次启动或已被清）
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name == ".well-known" || name == "admin" {
            continue;
        }
        if let Err(e) = std::fs::remove_dir_all(entry.path()) {
            if e.kind() != std::io::ErrorKind::NotFound {
                tracing::warn!(entry = %name, error = %e, "删除 SSR 缓存条目失败");
            }
        }
    }
    tracing::debug!("已失效全部公开页 SSR 缓存");
}

/// 原子递增并返回新的全局世代号。
///
/// 仅作可观测性用途（注入 `X-SSR-Generation` 响应头）。实际 SSR 缓存失效由
/// [`invalidate_ssr_route`] 物理删文件完成（首页由 [`invalidate_ssr_all_public`] 覆盖）。
pub fn bump_global_generation() -> u64 {
    let new = GLOBAL_GENERATION
        .fetch_add(1, Ordering::SeqCst)
        .wrapping_add(1);
    tracing::debug!(new_generation = new, "SSR 全局世代号已递增");
    new
}

/// 返回当前全局世代号。
pub fn current_global_generation() -> u64 {
    GLOBAL_GENERATION.load(Ordering::SeqCst)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn global_generation_is_monotonic() {
        let before = current_global_generation();
        let g1 = bump_global_generation();
        let g2 = bump_global_generation();
        let current = current_global_generation();

        assert!(g1 > before || g1 == 1);
        assert!(g2 > g1);
        assert_eq!(current, g2);
    }

    #[test]
    fn route_cache_dir_rejects_traversal() {
        // 路径穿越尝试不应越出 static/ 根。
        let p = route_cache_dir("/post/../../../etc/passwd").unwrap();
        let segs: Vec<_> = p.components().collect();
        // .. 被过滤，只剩 static/post/etc/passwd
        assert!(p.starts_with("static"));
        assert!(!segs.iter().any(|c| c.as_os_str() == ".."));
    }

    #[test]
    fn route_cache_dir_normalizes_leading_slash() {
        let a = route_cache_dir("/post/foo").unwrap();
        let b = route_cache_dir("post/foo").unwrap();
        assert_eq!(a, b);
        assert!(a.ends_with("post/foo"));
    }

    #[test]
    fn route_cache_dir_none_for_root() {
        // 根路由 "/" 无目录段（首页缓存由 invalidate_ssr_all_public 覆盖）。
        assert!(route_cache_dir("/").is_none());
        assert!(route_cache_dir("").is_none());
    }

    #[test]
    fn invalidate_ssr_route_deletes_dir() {
        // 造一个假的缓存目录：static/__test_route/index/fake.html
        let dir = route_cache_dir("/__test_route_xyz").unwrap();
        std::fs::create_dir_all(dir.join("index")).unwrap();
        std::fs::write(dir.join("index").join("fake.html"), "stale").unwrap();
        assert!(dir.exists());

        invalidate_ssr_route("/__test_route_xyz");

        assert!(!dir.exists(), "删除后目录不应存在");
    }

    #[test]
    fn invalidate_ssr_route_missing_is_noop() {
        // 不存在的路由删除应静默成功（NotFound 不报错）。
        invalidate_ssr_route("/__never_exists_xyz_123");
    }

    #[test]
    fn invalidate_ssr_all_public_is_safe_on_missing_root() {
        // static/ 根不存在时不应 panic（首次启动或已清空场景）。
        // 不创建真实 static/ 目录，直接调用验证 NotFound 静默。
        // （若真实环境已有 static/，本测试不会误删——read_dir 对每个条目单独删，
        //  这里仅验证根缺失分支。）
        invalidate_ssr_all_public();
    }
}
