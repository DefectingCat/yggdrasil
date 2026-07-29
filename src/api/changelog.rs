//! 更新日志接口。
//!
//! `CHANGELOG.md` 在编译期经 `include_str!` 内嵌进二进制（scratch 运行时镜像
//! 只有 binary + public/ + uploads/，磁盘上没有该文件；与 migrations、
//! highlight.css 的内嵌原则一致），首次请求时在 `spawn_blocking` 中经全站统一的
//! [`crate::api::markdown::render_markdown_enhanced`] 管线渲染（标题锚点、TOC、
//! sanitizer 清理），随后由 `LazyLock` 永久缓存——内容随二进制版本固定，
//! 部署即更新，无需任何缓存失效逻辑。

// 与 settings 等模块一致：Dioxus `#[server]` 宏触发 deprecated/unit 提示，按项目惯例放行。
#![allow(clippy::unused_unit, deprecated)]

use dioxus::prelude::*;

/// CHANGELOG.md 原文，编译期内嵌。
#[cfg(feature = "server")]
const CHANGELOG_MD: &str = include_str!("../../CHANGELOG.md");

/// 渲染结果（正文 HTML + TOC HTML），进程生命周期内只计算一次。
#[cfg(feature = "server")]
static CHANGELOG_RENDERED: std::sync::LazyLock<crate::api::markdown::RenderedContent> =
    std::sync::LazyLock::new(|| {
        crate::api::markdown::render_markdown_enhanced(changelog_body(CHANGELOG_MD))
    });

/// 剥去 CHANGELOG.md 开头的 `# Changelog` 标题与 Keep a Changelog 说明段，
/// 从第一个 `## ` 版本段起返回；不符合预期格式时回退全文。
///
/// 剥离原因：页面已有「更新日志」页头，原文 preamble 属于仓库元信息，
/// 直接渲染会造成标题重复。
#[cfg(feature = "server")]
fn changelog_body(full: &str) -> &str {
    match full.find("\n## ") {
        Some(i) => &full[i + 1..],
        None => full,
    }
}

/// 更新日志响应。
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ChangelogResponse {
    /// 正文 HTML（已过 sanitizer）。
    pub html: String,
    /// 版本索引 TOC HTML（无标题时为空字符串）。
    pub toc_html: String,
}

/// 获取渲染后的更新日志。
///
/// 公开接口；首次调用渲染后永久缓存，之后均为一次 `LazyLock` 读取 + clone。
#[server(GetChangelog, "/api")]
pub async fn get_changelog() -> Result<ChangelogResponse, ServerFnError> {
    #[cfg(feature = "server")]
    {
        // Markdown 渲染是 CPU 密集操作，即使只发生一次也放 spawn_blocking，
        // 不占用 async worker（与文章保存时的渲染路径同一约定）。
        let resp = tokio::task::spawn_blocking(|| {
            let r = &*CHANGELOG_RENDERED;
            ChangelogResponse {
                html: r.html.clone(),
                toc_html: r.toc_html.clone(),
            }
        })
        .await
        .map_err(|_| crate::api::error::AppError::Internal("更新日志渲染任务失败"))?;
        Ok(resp)
    }

    #[cfg(not(feature = "server"))]
    {
        Ok(ChangelogResponse {
            html: String::new(),
            toc_html: String::new(),
        })
    }
}

#[cfg(all(test, feature = "server"))]
mod tests {
    use super::*;

    #[test]
    fn changelog_body_strips_preamble() {
        let md = "# Changelog\n\n基于 Keep a Changelog。\n\n## [Unreleased]\n\n### Added\n- x\n\n## [0.1.0] - 2026-01-01\n";
        let body = changelog_body(md);
        assert!(body.starts_with("## [Unreleased]"));
        assert!(!body.contains("# Changelog"));
        assert!(!body.contains("Keep a Changelog"));
    }

    #[test]
    fn changelog_body_falls_back_to_full_text_without_version_section() {
        let md = "# Changelog\n\n这里还没有任何版本段。\n";
        assert_eq!(changelog_body(md), md);
    }

    /// 端到端守护：include_str! 路径有效 + Keep a Changelog 格式假设成立
    /// （`## [x.y.z]` 版本段）+ 渲染器产出带锚点的版本标题与 TOC。
    /// 若 CHANGELOG.md 被移动、改名或格式偏离约定，此测试会在编译/运行期失败。
    #[test]
    fn changelog_renders_anchored_versions() {
        let rendered = &*CHANGELOG_RENDERED;
        assert!(rendered.html.contains("<h2"), "应渲染出版本 h2 标题");
        assert!(
            rendered.html.contains("0.6.2"),
            "正文应包含当前最新版本号"
        );
        assert!(
            rendered.html.contains("Unreleased"),
            "应保留 [Unreleased] 段"
        );
        assert!(
            !rendered.html.contains("keepachangelog.com"),
            "preamble 应被剥离"
        );
        assert!(!rendered.toc_html.is_empty(), "应生成版本索引 TOC");
    }
}
