//! MCP 服务器：rmcp `ServerHandler` 实现，暴露博客工具。
//!
//! T1（tracer bullet）仅暴露一个工具 `search_posts`，证明：
//! - rmcp handler 经 `tool_router`/`tool_handler` 宏装配成功；
//! - 工具内能经 `Extension<http::request::Parts>` 读取鉴权中间件注入的 `McpPrincipal`；
//! - 作用域鉴权可生效（此处 search_posts 要求 read）。
//!
//! T3 会扩展为完整 read 工具集 + Resources；T4/T5 扩展 write/admin 工具。
//! 搜索 SQL 与 `src/api/posts/search.rs` 的 server-fn 一致（pg_trgm word_similarity），
//! T3 会把这段查询抽成共享 helper 供两条路径复用，避免重复维护。

use rmcp::handler::server::tool::Extension;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ContentBlock, TextContent};
use rmcp::{schemars, tool, tool_handler, tool_router, ErrorData as McpError, ServerHandler};
use serde::Deserialize;

use crate::mcp::auth::McpPrincipal;
use crate::models::mcp_token::TokenScope;

/// search_posts 入参。
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SearchPostsParams {
    /// 搜索关键词（会做 SQL 通配符转义；空串返回空结果）。
    pub query: String,
}

/// MCP 服务器实例（无状态：每个请求由 service_factory 新建一份）。
///
/// 共享状态（DB 连接等）通过 `get_conn()` 全局池获取，无需在实例里持有。
#[derive(Clone)]
pub struct YggMcpServer;

#[tool_router]
impl YggMcpServer {
    /// 搜索已发布文章（知识库）。要求 read 作用域。
    #[tool(description = "全文搜索已发布文章，作为知识库。返回标题/slug/摘要/标签。")]
    async fn search_posts(
        &self,
        Parameters(SearchPostsParams { query }): Parameters<SearchPostsParams>,
        Extension(parts): Extension<http::request::Parts>,
    ) -> Result<CallToolResult, McpError> {
        // 鉴权 + 作用域校验：read 工具要求 token.scope >= read。
        let principal = parts
            .extensions
            .get::<McpPrincipal>()
            .ok_or_else(|| McpError::invalid_request("missing MCP principal", None))?;
        if !principal.scope.grants(TokenScope::Read) {
            return Err(McpError::invalid_request(
                "insufficient_scope: search_posts requires read",
                None,
            ));
        }

        let hits = search_published(&query)
            .await
            .map_err(|e| McpError::internal_error(format!("search failed: {e}"), None))?;

        // 输出为 JSON 文本块（客户端 LLM 可解析）。T3 会改用 resource_link + 结构化输出。
        let text = serde_json::to_string_pretty(&hits)
            .map_err(|e| McpError::internal_error(format!("encode failed: {e}"), None))?;
        Ok(CallToolResult::success(vec![ContentBlock::Text(
            TextContent::new(text),
        )]))
    }
}

#[tool_handler(name = "yggdrasil", version = "0.1.0")]
impl ServerHandler for YggMcpServer {}

/// 已发布文章的精简命中（MCP 工具输出）。
#[derive(Debug, serde::Serialize)]
struct SearchHit {
    id: i32,
    title: String,
    slug: String,
    summary: Option<String>,
    tags: Vec<String>,
}

/// 直查 DB：与 src/api/posts/search.rs 的 server-fn 一致的 pg_trgm 查询。
/// 这里不复用 server-fn（后者依赖 FullstackContext 做限流，MCP 路径不走 cookie 鉴权），
/// 直接走连接池。T3 会抽出共享 helper。
async fn search_published(query: &str) -> Result<Vec<SearchHit>, String> {
    let q = query.trim();
    if q.is_empty() || q.chars().count() > 200 {
        return Ok(Vec::new());
    }
    let client = crate::db::pool::get_conn()
        .await
        .map_err(|e| format!("db conn: {e}"))?;

    let escaped = q
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_");

    let rows = client
        .query(
            "SELECT p.id, p.title, p.slug, p.summary,
                    COALESCE(array_agg(t.name) FILTER (WHERE t.name IS NOT NULL), '{}') as tags
             FROM posts p
             LEFT JOIN post_tags pt ON p.id = pt.post_id
             LEFT JOIN tags t ON pt.tag_id = t.id
             WHERE p.status = 'published' AND p.deleted_at IS NULL
               AND p.search_text ILIKE '%' || $1 || '%' ESCAPE '\\'
             GROUP BY p.id, p.search_text
             ORDER BY word_similarity(p.search_text, $2) DESC, p.published_at DESC
             LIMIT 50",
            &[&escaped, &q],
        )
        .await
        .map_err(|e| format!("query: {e}"))?;

    let hits = rows
        .iter()
        .map(|r| SearchHit {
            id: r.get(0),
            title: r.get(1),
            slug: r.get(2),
            summary: r.get(3),
            tags: r.get(4),
        })
        .collect();
    Ok(hits)
}
