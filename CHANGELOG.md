# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.0] - 2026-06-29

### Added

- **评论系统**：完整的访客评论功能，包含昵称/邮箱/URL、嵌套回复、管理后台审核（通过/标垃圾/删除）、待审评论 localStorage 持久化与 pending 状态轮询、相对时间显示。
- **文章回收站**：删除文章进入回收站，支持恢复、彻底删除、批量操作与清空；新增 `/admin/trash` 管理页面、`settings` 键值表与可配置的自动清理后台任务（保留天数、上限数量）。
- **主题切换动画**：基于 View Transitions API 的圆形展开动画（纯 CSS 实现，从点击点扩散），支持 `prefers-reduced-motion` 降级。
- **编辑器图片上传协调器**：Tiptap 自定义 Image 扩展，带上传中占位图（模糊 + spinner + 错误态）、失败重试、保存拦截、上传计数；支持 slash 命令、粘贴、拖拽。
- **封面图上传**：文章编辑器支持封面图上传（拖拽/点击/粘贴），封面区空态矮横条 + 可滚动主体布局。
- **Blur-up 渐进式图片加载**：Markdown 图片包裹双层结构（低清模糊 + 高清淡入），基于图片尺寸缓存的宽高比占位。
- **Lightbox 子工程**：`libs/lightbox/` TypeScript 项目，图库导航（淡入、箭头、键盘）、原点感知缩放、滚动关闭、防止背景滚动与闪烁。
- **yggdrasil-core 子工程**：核心 JS bundle 子工程，迁移 `post-content` 复制按钮与主题切换逻辑，删除 `public/js/`。
- **新增语法高亮**：TypeScript、JSX、TSX、Zig；补全 Swift 高亮；大小写不敏感的语法名匹配。
- **Markdown 源码视图切换**：编辑器可在富文本与源码视图间切换，按滚动比例同步位置。
- **健康检查端点**：新增 `healthz` 与 `readyz`。
- **内嵌数据库迁移**：启动时自动运行迁移（advisory lock + 逐迁移事务），迁移失败以友好退出 + 可配置重试窗口（`MIGRATE_STARTUP_TIMEOUT_SECS`）替代 panic；启动期自动创建目标数据库。
- **会话安全**：Session token 以 SHA-256 哈希存储（不再存明文）；可配置的单用户 session 数量上限（行锁串行化）；角色/状态变更通过 generation 失效所有会话。
- **CSRF 防护**：基于 Origin 的写接口 CSRF 检查，`APP_BASE_URL` 未设置时启动告警。
- **Cookie 安全**：`COOKIE_SECURE` 环境变量控制 session cookie 的 Secure 标志。
- **真实客户端 IP**：从 `X-Forwarded-For` 按 `TRUSTED_PROXY_COUNT` 提取真实 IP；未知 IP 时使用宽松限流桶。
- **图片响应缓存**：`Cache-Control` 与 `ETag` 头，支持 `If-None-Match` 304（RFC 7232 合规）；图片内存缓存改用 `bytes::Bytes`，新增按文件年龄与总大小淘汰的磁盘缓存定时清理任务。
- **图片上限可配置**：`MAX_IMAGE_DIMENSION`、`MAX_IMAGE_PIXELS`、`WEBP_QUALITY`、`WEBP_METHOD` 等环境变量，默认值翻倍；统一各格式大小上限。
- **`posts` 表字数与阅读时间**：新增 `word_count`/`reading_time` 列并在写入时维护；列表/搜索接口不再返回正文，读取预存的字数与阅读时间。
- **`PostListItem` 轻量 DTO**：列表/标签/搜索接口不再返回完整正文，显著降低缓存与序列化体积。
- **会话与搜索缓存**：基于 moka 的会话内存缓存（缓存对象不含密码哈希）；搜索结果短 TTL 缓存（10 秒，key 规范化）。
- **空状态与配图**：首页、归档、标签、搜索、后台文章/评论/回收站均接入 `EmptyState` 组件与装饰性配图（线条小狗等），配图圆角 + 暗色模式降亮。
- **UI 重新设计**：温暖色调 + 鼠尾草绿主色，统一 `paper-*` 主题变量；后台对齐前台主题变量，新增次要按钮冷调玫瑰色。
- **后台交互改进**：文章管理分页、重建内容按钮（带 tooltip）、重建缓存栏；评论状态筛选 `FilterTabs` 组件带滑动指示动画；回收站自动清理面板带滑入动画。
- **HTTP 压缩**：默认启用所有压缩算法，可通过 `COMPRESSION_ALGORITHMS` 配置；公共页面与静态资源 `Cache-Control`。
- **Dockerfile**：静态 musl 镜像构建（release 二进制 strip 符号）。
- **Gitea Actions CI**：CI 工作流。

### Changed

- 写路径缓存失效从「全量清空」改为「精确到 slug / tag / 列表页」，并在读取 slug/tag 元数据时使用事务 + `FOR UPDATE` 避免并发竞态；批量恢复、清空、重建等路径均采用精确失效。
- `get_post_stats` 将 3 次独立 `COUNT(*)` 合并为单次条件聚合查询。
- 图片解码、缩放、编码逻辑通过 `tokio::task::spawn_blocking` 移至阻塞线程池；Argon2 hash/verify、Markdown 渲染、GIF/WebP 原始校验同样 offload 到阻塞线程池。
- 为非图片路由启用 `CompressionLayer` 与 `TimeoutLayer`，`/uploads/*` 图片路由跳过压缩与全局超时。
- 连接池回收方法从 `Verified` 改为 `Fast`；重试从固定 2s 改为指数退避 + 抖动；新增 `statement_timeout`（`STATEMENT_TIMEOUT_SECS`）。
- 为 `deadpool-postgres` 显式指定 Tokio1 runtime；删除无效的 trgm GIN 索引。
- Tiptap 编辑器升级到 Vite 8 / Vitest 4 / TypeScript 6 / Tiptap 3.27（Rolldown），并从 `js_sys::eval` 迁移到 wasm-bindgen 绑定层（`tiptap_bridge`），EditorOptions/onReady/onUploadEvent 回调替代轮询。
- Lightbox 与 post-content 从 `include_str!` 内联改为配置驱动初始化；移除旧 `ImageViewer` 组件。
- JS 子项目从 npm 迁移到 pnpm。
- 封面图比例统一为 21:9；卡片重构使用原生 `blur-img` 结构。
- 邮箱正则、sanitizer allowlist 用 `LazyLock` 静态化以避免每次调用分配。
- Markdown 渲染读取 DB 中预渲染的 `content_html`/`toc_html`，写入时存储 `toc_html`。
- 大量「上帝组件」拆分为子组件：`CoverUploader`、`FilterTabs`、`AutoPurgeSettings`、`RebuildCacheBar`，共享 `EmptyState` 组件。

### Fixed

- 修复 `/uploads/{*path}` 路径因缺少 `ConnectInfo` 扩展而返回 HTTP 500 的问题。
- 修复并发重复评论提交（advisory lock）；重复检查改为事务内原子操作并对 `content_hash` 建索引。
- 修复评论表单：服务端 honeypot、a11y label、回复布局；待审评论嵌套显示于父评论下；评论项添加 `md-content` 类修复高亮与空行。
- 修复时序枚举攻击：不存在用户执行 dummy Argon2 verify；对 `check_pending_status` 限流防止状态枚举。
- 修复图片磁盘缓存清理跳过符号链接，防止遍历到缓存目录外部；过期 session 清理同时失效会话内存缓存。
- 修复 SSR hydration 不匹配、暗色模式 FOUC、ThemeToggle 状态同步。
- 修复 Tiptap 二次导航空白、链接命令顺序与 URL scheme 校验、blob 泄漏（节点销毁时 revoke）。
- 修复 404 页面、首页卡片嵌套锚点、封面图高度塌陷、暗色模式封面灰背景。
- 修复 WASM 构建假阳性 warning（27 处 `cfg` gate）、Tailwind v4 与 Tiptap 构建问题。
- 修复 dev 模式 SSR 缓存导致渲染陈旧 HTML；`/doc` 路由与静态托管冲突的 panic。
- 修复 `b7afd12` 起的 highlight 大小写匹配导致 Haskell 等高亮失效。
- 其他构建、CI、格式化与测试修复。

### Security

- Session token 以 SHA-256 哈希存储；可配置单用户 session 上限并以行锁串行化。
- 基于 Origin 的写接口 CSRF 防护；`COOKIE_SECURE` 控制 Secure 标志。
- 不存在用户执行 dummy Argon2 verify 防时序枚举；`check_pending_status` 限流。
- 图片路径 `canonicalize` 前缀检查（纵深防御）；拒绝不可解码图片返回 422，原始文件上限 20MB；所有图片响应加 `X-Content-Type-Options: nosniff`。
- 分页参数 `per_page` 与 `page` 上限钳制，消除公开接口 DoS 与无界 OFFSET / 缓存键扇出。
- 磁盘缓存写入改为 temp-file + rename 原子操作。

### Internal

- 新增 vitest 测试套件：`tiptap-editor`（UploadCoordinator、UploadImageNodeView、isValidUrl）、`lightbox`（geometry、lightbox 生命周期）、`yggdrasil-core`（post-content、theme-transition 降级）。
- 扩充 Rust 单元测试覆盖：AppError、sanitizer、WebP、theme、cache、db（迁移注册校验）、PostListItem 等。
- 补齐大量中文文档注释；更新 AGENTS.md、新增 DEVELOPMENT.md 与生产部署指南。
- Makefile 完善 `test`（cargo test + vitest）、`clippy`、`fix`、`doc`（ayu 主题）、`build-lightbox` 等目标。
- 仓库安全审查（`449a545`）修复多项关键问题。

## [0.2.0] - 2026-06-10

### Added

- 404 Not Found 页面
- 基于 moka 的内存缓存模块（文章、标签、统计查询缓存）
- 读写操作缓存支持：读操作命中缓存，写操作自动失效相关缓存
- 文章列表返回准确的总条目数，前台首页/归档/标签页分页使用准确总数
- WebP 图片编码支持（zenwebp），上传图片自动转换为 WebP 以节省存储空间
- 图片变体磁盘级缓存

### Changed

- `posts.rs` API 拆分为模块目录结构
- 统一错误处理为 `AppError` 枚举
- 缓存层优化：直接返回命中结果，COUNT(*) 结果单独缓存
- 提取 WebP 编码辅助函数减少重复代码
- 简化 TraceLayer 配置并重新格式化路由定义

### Fixed

- 修复多处 `#[cfg(feature = "server")]` 门控缺失导致的编译问题
- 缓存正确失效旧 slug 和新 slug
- WebP 解码缓冲区大小限制，防止恶意大分配
- 代码块复制按钮点击处理
- 其他细节修复

### Internal

- 新增缓存、WebP 配置测试用例
- 添加 release 自动化技能

## [0.1.0] - 2026-06-09

### Added

- Dioxus 0.7 全栈项目脚手架
- PostgreSQL 数据库建表（用户、文章、标签、会话）
- 用户认证系统：注册、登录、Session 管理
- 首个注册用户自动成为 admin，后续注册关闭
- HttpOnly cookie 会话机制
- 后台管理页面与路由
- Tiptap 富文本编辑器集成（Markdown 模式）
  - Slash 命令、表格、任务列表、图片和链接扩展
  - 图片粘贴/拖拽上传
- 文章 CRUD：创建、编辑（含数据回填）、列表、删除
- 文章封面图支持
- Markdown 渲染：TOC 目录、锚点链接、字数统计、预计阅读时间
- 代码高亮（syntect + catppuccin 主题），支持 Swift/Kotlin 自定义语法
- XSS 防护（ammonia 清洗 HTML）
- 前台博客页面（PaperMod 风格）
  - 首页（个人简介 + 文章列表 + 分页）
  - 归档页（按年月分组）
  - 标签页（标签云 + 标签详情）
  - 文章详情页（目录、上下篇导航）
  - 搜索页
  - 关于页
- 暗色模式（系统偏好检测 + 手动切换，SSR 安全）
- SSR 预渲染（首页、文章、归档、标签）+ 增量缓存
- 骨架屏加载动画（各页面独立骨架，防闪烁）
- 图片处理：缩放、缩略图、旋转、格式转换（moka 缓存）
- 图片灯箱查看器
- pg_trgm 全文搜索
- Rate limiting（注册、登录、上传接口）
- 数据库连接池重试逻辑
- Session 过期自动清理（每小时）
- 数据库性能索引（posts/tags/sessions）
- 数据库迁移脚本（migrate.sh）
- 122 个单元测试覆盖 12 个模块
- 项目开发指南（AGENTS.md）

### Changed

- Tailwind CSS v4 + 独立 CLI 构建
- admin 模块重构为共享组件 + card 布局
- 全局使用 Dioxus 客户端路由替代原生导航
- 提取公共组件：FormInput/FormLabel/AlertBox、SkeletonLine/SkeletonBox/SkeletonCard
- 提取工具模块：slug、markdown、tags、text、time、session
- API 层 DRY 重构（错误处理、N+1 查询修复 via JOIN+array_agg）
- 文章 slug ASCII 化 + 时间戳回退
- Tiptap 编辑器 Vite 构建输出固定文件名
- 首页 HomeInfo 个人简介替代原始首区

### Fixed

- 修复 admin 路由切换闪烁
- 修复编辑器暗色主题和列表样式
- 修复 Footer 滚动监听器未清理
- 修复 CJK 字数统计
- 修复代码块 Tailwind `.block` 类冲突
- 修复 SSR 水合不匹配（ThemeToggle）
- 修复 WASM 生产环境 404（symlink 修复）
- 修复图片上传 500 错误
- 修复 Markdown 渲染中 data URI 丢失
- 修复暗色模式 FOUC 和状态同步
- 修复登录后 UserContext 未重置
- 修复文章 slug 唯一性检查（含已删除文章）
- 修复 Tiptap 编辑器二次导航空白问题
- 修复模板 hydration 不匹配警告
- 修复 Clippy 和编译器警告
