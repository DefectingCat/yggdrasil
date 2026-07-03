//! 全站路由配置
//!
//! 使用 Dioxus Router 定义前端路由层级，包含前台布局、后台管理布局、
//! 独立的登录与注册页面。`Route` 枚举上的 `#[route("/path")]` 属性
//! 既用于生成 URL 匹配规则，也用于组件导航。

use dioxus::prelude::*;
use std::sync::Arc;

use crate::components::admin_layout::AdminLayout;
use crate::components::frontend_layout::FrontendLayout;
use crate::context::UserContext;
use crate::pages::about::About;
use crate::pages::admin::{
    Admin, AdminComments, AdminCommentsPage, Posts, PostsPage, Runner, System, Trash, TrashPage,
    Write, WriteEdit,
};
use crate::pages::archives::Archives;
use crate::pages::home::{Home, HomePage};
use crate::pages::login::Login;
use crate::pages::not_found::NotFound;
use crate::pages::post_detail::PostDetail;
use crate::pages::register::Register;
use crate::pages::search::Search;
use crate::pages::tags::{TagDetail, Tags};
use crate::theme::{use_theme_provider, ThemePreload};

/// 全站路由枚举，每个变体对应一个页面路径
#[derive(Clone, Routable, Debug, PartialEq)]
#[rustfmt::skip]
pub enum Route {
    // 前台页面共享布局
    #[layout(FrontendLayout)]
        /// 首页
        #[route("/")]
        Home {},
        /// 首页分页
        #[route("/page/:page")]
        HomePage { page: i32 },
        /// 文章归档页
        #[route("/archives")]
        Archives {},
        /// 标签列表页
        #[route("/tags")]
        Tags {},
        /// 单个标签下的文章列表
        #[route("/tags/:tag")]
        TagDetail { tag: String },
        /// 文章详情页，按 slug 匹配
        #[route("/post/:slug")]
        PostDetail { slug: String },
        /// 搜索页
        #[route("/search")]
        Search {},
        /// 关于页面
        #[route("/about")]
        About {},
        /// 404 兜底路由，匹配所有未命中路径
        #[route("/:..segments")]
        NotFound { segments: Vec<String> },
    #[end_layout]

    // 后台管理路由嵌套在 `/admin` 下
    #[nest("/admin")]
    // 后台页面共享管理布局
    #[layout(AdminLayout)]
        /// 后台仪表盘
        #[route("/")]
        Admin {},
        /// 写文章页
        #[route("/write")]
        Write {},
        /// 编辑文章页
        #[route("/write/:id")]
        WriteEdit { id: i32 },
        /// 文章管理列表
        #[route("/posts")]
        Posts {},
        /// 文章管理列表分页
        #[route("/posts/:page")]
        PostsPage { page: i32 },
        /// 评论管理
        #[route("/comments")]
        AdminComments {},
        /// 评论管理分页
        #[route("/comments/:page")]
        AdminCommentsPage { page: i32 },
        /// 回收站
        #[route("/trash")]
        Trash {},
        /// 回收站分页
        #[route("/trash/:page")]
        TrashPage { page: i32 },
        /// 系统管理（数据库 + 服务器状态 + SQL 控制台 + 导出 + 备份）
        #[route("/system")]
        System {},
        /// 代码试运行沙箱（作者预览可运行代码块输出）
        #[route("/runner")]
        Runner {},
    #[end_layout]
    #[end_nest]

    /// 登录页面
    #[route("/login")]
    Login {},
    /// 注册页面
    #[route("/register")]
    Register {},
}

/// 应用路由器组件
///
/// 初始化主题提供者、全局用户上下文，并挂载样式表与 `Router`。
#[component]
pub fn AppRouter() -> Element {
    let _theme = use_theme_provider();

    // 提供全局用户上下文，供登录状态与路由守卫使用
    let user = use_signal(|| None::<Arc<crate::models::user::PublicUser>>);
    let checked = use_signal(|| false);
    use_context_provider(|| UserContext { user, checked });

    rsx! {
        document::Stylesheet { href: "/style.css" }
        document::Stylesheet { href: "/highlight.css" }
        document::Title { "Yggdrasil Blog" }
        document::Link { rel: "icon", href: "/favicon.ico" }
        div {
            ThemePreload {}
            Router::<Route> {}
        }
    }
}
