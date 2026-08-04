//! 回归测试：锁定 PostDetail 组件不再用镜像 signal 触发 server future。
//!
//! 背景：src/pages/post_detail.rs 曾用 `use_signal(|| slug.clone())` 镜像 slug prop，
//! 并在 render 期 `if slug_signal() != slug { slug_signal.set(...) }` 触发重取——
//! 这是 Dioxus antipatterns 明确点名的「render body 副作用」。
//! 已在 commit 225bb24 改为直接读 prop。本测试通过源码字符串断言防止回退。

/// PostDetail 组件源码不得包含镜像 slug 的 signal 反模式签名。
#[test]
fn post_detail_does_not_mirror_slug_into_signal() {
    let src = include_str!("../src/pages/post_detail.rs");

    // 反模式签名：用 use_signal 镜像 slug prop。
    assert!(
        !src.contains("slug_signal"),
        "post_detail.rs 重新引入了 slug_signal 镜像——这是 render 期 set signal 反模式。\
         应直接在 use_server_future 闭包里读 slug prop，让 Dioxus 在 prop 变化时自动重跑。\
         详见 .agents/skills/dioxus-render-purity/SKILL.md 规则一。"
    );

    // 反模式签名：render 期对 slug 相关 signal 调用 set。
    assert!(
        !src.contains("slug_signal.set"),
        "post_detail.rs 在 render 期 set signal——违反渲染纯净性。"
    );
}

/// 回归测试：锁定 CommentSection 在上/下一篇切换时强制 remount（issue #10）。
#[test]
fn comment_section_is_remounted_on_post_switch() {
    let src = include_str!("../src/pages/post_detail.rs");

    // CommentSection 必须被单元素 keyed for 循环（key 绑定 post.id）包裹：Dioxus 的
    // keyed diff 只在兄弟节点列表里生效，单个非列表元素按位置复用。若移除该包裹，
    // CommentSection 复用旧实例，use_resource 闭包的冻结 post_id 导致导航后仍请求
    // 旧文章评论（issue #10）。
    assert!(
        src.contains("std::iter::once(post.id)"),
        "post_detail.rs 不再以 keyed 单元素 for-loop 包裹 CommentSection——\
         切换文章会复用 CommentSection 实例，残留旧文章评论（issue #10）。"
    );
}
