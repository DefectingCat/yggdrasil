# Yggdrasil

基于 Dioxus 0.7 的全栈博客系统，Rust 单一代码库同时编译为 WASM 前端和原生服务端。

## 技术栈

- **框架**: Dioxus 0.7 (fullstack)
- **数据库**: PostgreSQL + tokio-postgres
- **样式**: Tailwind CSS v4
- **密码**: Argon2
- **会话**: UUID token + cookie

## 功能

- 邮箱注册 / 登录（单管理员模式，首次注册后关闭）
- 会话管理与自动过期清理
- 暗色 / 亮色主题切换
- 后台文章撰写（Tiptap Markdown 编辑器）
- 文章归档与标签浏览

## 开发

依赖 Rust 1.95+、wasm32 目标、`dx` CLI、tailwindcss CLI v4 和 PostgreSQL。

```bash
# 配置数据库
DATABASE_URL=postgres://postgres:postgres@localhost:5432/yggdrasil

# 运行迁移
psql $DATABASE_URL -f migrations/001_init.sql

# 启动开发服务器
make dev
```

## 构建

```bash
make build
```
