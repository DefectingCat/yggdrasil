# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
