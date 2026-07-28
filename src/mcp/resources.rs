//! MCP Resources：把已发布文章暴露为可分页、可读取的资源（知识库）。
//!
//! 提供：
//! - [`list_published_resources`]：游标分页枚举已发布文章为 rmcp `Resource`；
//! - [`read_post_resource`]：按 `post://{slug}` URI 读取单篇文章 Markdown 正文；
//! - [`post_resource_template`]：`post://{slug}` 模板，供 `resources/templates/list`。
//!
//! 游标设计：不透明字符串 = 文章 `id` 的 hex 编码（URL 安全，且无需额外依赖 base64）。
//! 分页按 `id` 升序（稳定、无偏移放大问题）：`WHERE id > $last_id ORDER BY id LIMIT n+1`，
//! 取到 `n+1` 行说明还有下一页，下一页游标 = 第 n 行的 id。
//!
//! 这里只提供数据访问函数 + 纯类型；`server.rs`（Main 装配）把它们接到
//! `ServerHandler::list_resources` / `read_resource`。

use rmcp::model::{Resource, ResourceTemplate};

use crate::api::error::AppError;
use crate::db::pool::get_conn;

/// `resources/list` 的默认每页数量。
///
/// 与 `api::posts::list::MAX_PER_PAGE`（50）保持同一量级，但 MCP 客户端通常
/// 一次只拉一小批做上下文注入，20 是读取效率与上下文体积的折中。
pub const DEFAULT_PAGE_SIZE: usize = 20;

/// 单页允许的最大资源数量（防 DoS：无认证的枚举不应无界扫描）。
pub const MAX_PAGE_SIZE: usize = 100;

/// `post://{slug}` URI 前缀。
pub const POST_URI_PREFIX: &str = "post://";

/// 把 slug 规范化为 MCP 资源 URI。
pub fn post_uri(slug: &str) -> String {
    format!("{POST_URI_PREFIX}{slug}")
}

/// 从 MCP 资源 URI 解析出 slug。
///
/// 接受 `post://{slug}`；非法 scheme / 空 slug 返回 None。
pub fn slug_from_uri(uri: &str) -> Option<&str> {
    let rest = uri.strip_prefix(POST_URI_PREFIX)?;
    let slug = rest.trim();
    if slug.is_empty() {
        None
    } else {
        Some(slug)
    }
}

/// 构造 `post://{slug}` 资源模板（供 `resources/templates/list`）。
pub fn post_resource_template() -> ResourceTemplate {
    ResourceTemplate::new("{POST_URI_PREFIX}{slug}", "post")
        .with_title("已发布文章")
        .with_description("按 slug 读取单篇已发布文章的 Markdown 正文。URI 形如 post://my-post-slug。")
        .with_mime_type("text/markdown")
}

/// 把分页大小钳制到 `[1, MAX_PAGE_SIZE]`，默认 [`DEFAULT_PAGE_SIZE`]。
fn clamp_page_size(n: Option<usize>) -> usize {
    match n {
        Some(n) if n >= 1 => n.min(MAX_PAGE_SIZE),
        _ => DEFAULT_PAGE_SIZE,
    }
}

/// 游标（不透明字符串）与文章 id 的双向转换。
///
/// 游标 = `{id}` 的 hex 编码。对外是黑盒字符串；客户端只原样回传。
mod cursor {
    /// 把文章 id 编码为不透明游标。
    pub(crate) fn encode(id: i32) -> String {
        hex::encode(id.to_le_bytes())
    }

    /// 把不透明游标解码为文章 id；非法输入返回 None。
    pub(crate) fn decode(s: &str) -> Option<i32> {
        let bytes = hex::decode(s.trim()).ok()?;
        if bytes.len() != 4 {
            return None;
        }
        let mut arr = [0u8; 4];
        arr.copy_from_slice(&bytes);
        Some(i32::from_le_bytes(arr))
    }
}

/// `list_published_resources` 的结果。
pub struct ResourcePage {
    /// 本页资源（rmcp `Resource`，供直接返回给客户端）。
    pub resources: Vec<Resource>,
    /// 下一页游标；None 表示已到末页。
    pub next_cursor: Option<String>,
}

/// 游标分页枚举已发布文章为 MCP `Resource`。
///
/// - `cursor`：上一页返回的 `next_cursor`，`None` 表示首页。
/// - `limit`：每页数量，`None` 取 [`DEFAULT_PAGE_SIZE`]，超过 [`MAX_PAGE_SIZE`] 被钳制。
///
/// 按 `id` 升序稳定分页；草稿/已删除文章不出现。
pub async fn list_published_resources(
    cursor: Option<&str>,
    limit: Option<usize>,
) -> Result<ResourcePage, AppError> {
    let page_size = clamp_page_size(limit);
    // 游标解码失败按"无效游标"处理：不静默回首页（否则跳过数据），直接报错。
    let after_id = match cursor.map(str::trim).filter(|s| !s.is_empty()) {
        Some(c) => Some(cursor::decode(c).ok_or_else(|| {
            AppError::BadRequest(format!("invalid pagination cursor: {c}"))
        })?),
        None => None,
    };
    let fetch = page_size + 1; // 多取 1 行判断是否有下一页

    let client = get_conn().await.map_err(AppError::db_conn)?;

    let rows = if let Some(last_id) = after_id {
        client
            .query(
                "SELECT p.id, p.title, p.slug, p.summary, octet_length(p.content_md) AS size
                 FROM posts p
                 WHERE p.status = 'published' AND p.deleted_at IS NULL AND p.id > $1
                 ORDER BY p.id ASC
                 LIMIT $2",
                &[&last_id, &(fetch as i64)],
            )
            .await
            .map_err(AppError::query)?
    } else {
        client
            .query(
                "SELECT p.id, p.title, p.slug, p.summary, octet_length(p.content_md) AS size
                 FROM posts p
                 WHERE p.status = 'published' AND p.deleted_at IS NULL
                 ORDER BY p.id ASC
                 LIMIT $1",
                &[&(fetch as i64)],
            )
            .await
            .map_err(AppError::query)?
    };

    let has_next = rows.len() > page_size;

    let resources: Vec<Resource> = rows
        .iter()
        .take(page_size)
        .map(|r| {
            let title: String = r.get("title");
            let slug: String = r.get("slug");
            let summary: Option<String> = r.get("summary");
            let size: Option<i64> = r.try_get("size").ok();
            let mut res = Resource::new(post_uri(&slug), title.clone())
                .with_title(title)
                .with_mime_type("text/markdown");
            if let Some(s) = summary {
                res = res.with_description(s);
            }
            if let Some(sz) = size {
                res = res.with_size(sz as u64);
            }
            res
        })
        .collect();

    // 多取 1 行 → 有下一页；游标 = 本页最后一行 id 的 hex 编码（不透明、稳定）。
    let next_cursor = if has_next {
        let last_id: i32 = rows[page_size - 1].get("id");
        Some(cursor::encode(last_id))
    } else {
        None
    };

    Ok(ResourcePage {
        resources,
        next_cursor,
    })
}

/// 按 `post://{slug}` 读取单篇已发布文章的 Markdown 正文。
///
/// 仅返回 `content_md`；草稿/已删除文章返回 None（对客户端表现为资源不存在）。
/// 渲染 HTML 由 web 前端负责，MCP 资源保持 Markdown 以利 LLM 直接消费。
pub async fn read_post_resource(uri: &str) -> Result<Option<String>, AppError> {
    let Some(slug) = slug_from_uri(uri) else {
        return Ok(None);
    };

    let client = get_conn().await.map_err(AppError::db_conn)?;
    let row = client
        .query_opt(
            "SELECT p.content_md
             FROM posts p
             WHERE p.slug = $1 AND p.status = 'published' AND p.deleted_at IS NULL",
            &[&slug],
        )
        .await
        .map_err(AppError::query)?;

    Ok(row.map(|r| r.get::<_, String>("content_md")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_round_trips() {
        for id in [0i32, 1, 42, 1_000_000, i32::MAX] {
            let c = cursor::encode(id);
            assert_eq!(cursor::decode(&c), Some(id), "id {id}");
        }
    }

    #[test]
    fn cursor_rejects_invalid_input() {
        // 非 hex
        assert_eq!(cursor::decode("nothex!!"), None);
        // 长度不对（hex 但字节数 != 4）
        assert_eq!(cursor::decode("deadbeefdeadbeef"), None);
        assert_eq!(cursor::decode("ab"), None);
        // 空串
        assert_eq!(cursor::decode(""), None);
        // 仅空白
        assert_eq!(cursor::decode("   "), None);
    }

    #[test]
    fn cursor_decode_trims_whitespace() {
        let c = cursor::encode(123);
        assert_eq!(cursor::decode(&format!("  {c}  ")), Some(123));
    }

    #[test]
    fn uri_round_trips() {
        assert_eq!(post_uri("hello-world"), "post://hello-world");
        assert_eq!(slug_from_uri("post://hello-world"), Some("hello-world"));
        assert_eq!(slug_from_uri("post://with-trailing/"), Some("with-trailing/"));
    }

    #[test]
    fn slug_from_uri_rejects_bad_input() {
        // 错误 scheme
        assert_eq!(slug_from_uri("http://foo"), None);
        assert_eq!(slug_from_uri("foo://bar"), None);
        // 缺前缀
        assert_eq!(slug_from_uri("hello-world"), None);
        // 空 slug
        assert_eq!(slug_from_uri("post://"), None);
        assert_eq!(slug_from_uri("post://   "), None);
    }

    #[test]
    fn clamp_page_size_defaults_and_caps() {
        assert_eq!(clamp_page_size(None), DEFAULT_PAGE_SIZE);
        assert_eq!(clamp_page_size(Some(0)), DEFAULT_PAGE_SIZE);
        assert_eq!(clamp_page_size(Some(1)), 1);
        assert_eq!(clamp_page_size(Some(50)), 50);
        assert_eq!(clamp_page_size(Some(MAX_PAGE_SIZE)), MAX_PAGE_SIZE);
        assert_eq!(
            clamp_page_size(Some(MAX_PAGE_SIZE + 1000)),
            MAX_PAGE_SIZE
        );
    }

    #[test]
    fn resource_template_has_markdown_mime() {
        let t = post_resource_template();
        assert_eq!(t.uri_template, "{POST_URI_PREFIX}{slug}");
        assert_eq!(t.name, "post");
        assert_eq!(t.mime_type.as_deref(), Some("text/markdown"));
    }
}
