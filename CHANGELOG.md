# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.6.2] - 2026-07-24

### Fixed

- **web-only 构建失败**：`sleep_ms` 原用 `#[cfg(not(target_arch = "wasm32"))]` guard tokio 分支，但 tokio 是 server-only optional 依赖。该 guard 在「非 wasm32 主机 + 仅 web feature」组合下误激活导致编译失败（长期被 dev-dependencies 中的 tokio 掩盖，只有排除 dev-deps 的生产构建才暴露）。修复为 `#[cfg(all(feature = "server", not(target_arch = "wasm32")))]`，符合 dual-target gating 规范。
- **404 页「返回首页」卡死**：文章详情页对不存在的 slug 抛出 404 错误后，Dioxus `ErrorBoundary` 捕获错误渲染 fallback，但点击「返回首页」仅更新 URL 不切换页面（`ErrorBoundary` 需显式 `clear_errors()` 才能恢复渲染 children）。修复：返回首页改用 button，onclick 内先 `clear_errors()` 再导航。

## [0.6.1] - 2026-07-23

### Changed

- **消除非测试代码中的裸 `unwrap()`**：在 `panic = "abort"` 全局下，任何裸 `unwrap()` 都会直接崩溃整个进程且无法恢复。将所有非测试代码中的 `unwrap()` 改写为带不变量说明的 `expect()`（消息需解释*为何*不可能失败），并在 AGENTS.md 规范 #16 中固化该约束。

### Fixed

- **mhchem 转译器三处 panic 修复**：修复 `panic = "abort"` 下化学公式转译器的三处崩溃：`". __* "` 正则转录错误、`find_observe_end` 逐字节扫描多字节字符越界、以及 `re!` 宏编译失败时未降级为不匹配导致直接 panic。前两处为输入触发的运行时 panic，第三处补上缺失的防御性边界。
- **Docker 构建缺 `patches/` 目录**：构建镜像时未复制 `patches/` 目录导致 `pnpm install` 报 ENOENT，补充复制修复。

## [0.6.0] - 2026-07-23

### Added

- **编辑器 mermaid 实时预览**：Tiptap 代码块在编辑器内实时渲染 mermaid 流程图，所见即所得。
- **编辑器脚注所见即所得**：Tiptap 富文本编辑器内脚注直接可见，不再依赖 Markdown 源码。
- **mhchem 化学公式**：移植 mhchem 转译器，支持 `\ce`/`\pu` 化学方程式语法转 LaTeX 渲染。
- **KaTeX 物理学宏表**：注册 16 个物理学宏（如 `\d`、`\od`、`\textsubscript` 等），适配物理学公式写作习惯。
- **新建文章默认直接发布**：`/admin/write` 新建文章的默认发布状态改为直接发布。

### Changed

- **mermaid 主题变量下沉**：将 Catppuccin 主题变量从 tiptap-editor 下沉到 `@yggdrasil/shared` 共享包，统一管理。
- **中间件抽取**：将 `ssr_generation`/`version_headers` 中间件从 `main.rs` 抽出至 `src/middleware.rs`。

### Fixed

- **窄表格边框裁切**：修复窄表格在 `table-wrap` 容器内边框被裁切的问题。
- **移动端表格滚动**：修复移动端表格无法横向滚动的问题。
- **空工具栏顶栏**：隐藏无语言标识代码块的空工具栏顶栏。
- **KaTeX sanitizer 拦截**：允许 KaTeX 渲染所需的 `svg`、`path` 标签及绘图属性通过 sanitizer。
- **tiptap-markdown 过度转义**：修复 tiptap-markdown 过度转义导致内容显示损坏。
- **脚注序列化转义失效**：修复 tiptap 序列化转义脚注语法 `[^id]` → `\[^id\]` 导致脚注失效。
- **SQL 控制台写后缓存失效**：SQL 控制台执行写操作后全量失效相关缓存。

## [0.5.0] - 2026-07-22

### Added

- **文章数学公式 SSR 渲染(KaTeX)**：引入 `katex-rs` 在服务端把 `$...$`/`$$...$$` TeX 渲染成视觉层 HTML span(`OutputFormat::Html`,不含 MathML,XSS 面最小;`throw_on_error=false` 坏公式渲染成红色错误而非中断);自托管 KaTeX CSS + woff2 字体到 `public/katex/`(`make katex-css` 从 npm `katex` dist 拷贝)。
- **评论数学公式渲染**：评论路径同步开启 `ENABLE_MATH`,span 白名单加 `style` 保留 KaTeX 内联定位样式。
- **编辑器数学公式节点**：Tiptap 数学公式节点带 KaTeX 预览,根治编辑器序列化破坏 LaTeX 的问题。
- **mermaid 流程图懒加载渲染**：文章页 `language-mermaid` 代码块经 IntersectionObserver 视口可见时动态 `import('/mermaid/mermaid.js')`(独立 IIFE bundle,~3.4MB / gzip ~900KB,非全局注入),`mermaid.render` 产 SVG 注入;主题经 `__initMermaid` 传入,`securityLevel: 'strict'`,幂等守卫防重复渲染;渲染失败保留源码并加 `.mermaid-error` class。
- **流程图配色对齐 Catppuccin**：mermaid 配色对齐 Catppuccin 主题,容器美化。
- **流程图主题切换动画**：流程图主题切换跟随 View Transitions 圆形扩散动画,主题切换时重渲染已渲染的流程图。
- **bun 代码运行器**：新增 `yggdrasil-runner-bun` 沙箱镜像(官方 `bun.sh/install` 脚本 + musl 变体 + `libstdc++`/`libgcc` C++ 运行时);admin 代码试运行沙箱加 bun 语言按钮;CodeMirror 加 TypeScript 模式;语言别名归一化(`ts`/`typescript`→`bun`,在 `parse_fence_info` 统一,`LANGUAGES.get` 只见规范化 key)。
- **文章页脚注完整支持**：语义化 + back-link + 样式。
- **Vue SFC 语法高亮**：文章页代码块支持 Vue SFC 语法高亮。
- **搜索入口改为图标按钮**：header 搜索入口从文字改为图标按钮。
- **正文折叠块卡片化**：`<details>` 折叠块卡片化,自绘 chevron + hover/focus 态。
- **task-list checkbox 自绘**：文章页 task-list checkbox 改用 `appearance:none` 自绘圆角方框。
- **代码块字号调整**：文章页代码块字号从 13.6px 调整为 16px。
- **响应头暴露版本信息**：server 通过 `Server`/`X-Yggdrasil-Version`/`X-Yggdrasil-Git` 响应头主动暴露版本与 git 描述信息(`EXPOSE_VERSION_HEADERS` 可关)。
- **Docker multi-arch 构建目标**：`make docker-amd64` 与 `make docker-apple` 构建 x86_64 镜像。
- **服务器端口占用优雅退出**：端口被占用时优雅退出而非 panic。
- **压缩算法默认 off**：`COMPRESSION_ALGORITHMS` 中间件默认值改为 off。

### Changed

- **mimalloc 全局分配器**：用 mimalloc 替换系统全局分配器(`#[global_allocator]`,双 cfg 门控:server feature + 非 wasm32;musl 静态链接友好)。
- **性能优化系列**：`escape_html` 链式 5 次 replace 改单遍扫描;`slugify` 单遍状态机重写(分配 4→1);Markdown 渲染消除双解析 + `format!` 改 `write!` 直写;upload 消除 `data.to_vec()` 多次全文件深拷贝;`cache_key` 单次拼接 + `detect_format` 零分配后缀匹配;posts list/search 零 capacity Vec 改 collect 预分配 + helpers retain。
- **重构系列**：admin `/admin/posts` 与 `/admin/posts/trash` 合并为单路由 + tab 切换;`Pagination` 支持可选 `on_prev`/`on_next` 回调;抽取 `@yggdrasil/shared` 内部源共享包消除跨 IIFE 库的类型/常量重复;抽 `main.rs` 中间件到 `src/middleware.rs`;为 `Response` 类型添加构造器消除 51 处样板;抽 `invalidate_post_metadata()`/`upload_error()` 等消除样板;统一 WASM sleep 到 `utils::time::sleep_ms`;删除死代码(`delayed_loading.rs`/`ui.rs EmptyState`/`CommentActions`/未用 re-export);拆分 `system.rs` 为 `system/` 目录(按 tab 分文件);图片处理合并维度读取函数共享 `image_reader_limits`。
- **依赖升级**：TypeScript 升级至 7.0.2(Go 原生编译器)。

### Fixed

- **SSR 缓存失效根治**：文章写入后物理删除 SSR 磁盘缓存目录,根治「重建后内容不更新」(Dioxus 0.7 增量渲染器只暴露 TTL 失效手段,通过删文件绕过限制);build 前清除 `static/` SSR 缓存目录。
- **Docker 构建/部署**：Docker 构建透传 git 信息修复 `x-yggdrasil-git` 恒为 unknown;预装 binaryen 避免 dx 运行时下载 wasm-opt 失败;补齐 Dockerfile 缺失的 katex-css 与 restore-webp 步骤;升级 builder 至 trixie 满足 dx 对 GLIBC_2.39 的需求;用 tmpfs `mode=1777` 替换 uid/gid 选项兼容 Podman;GitHub Releases 下载最终改用直连(移除 gh-proxy)。
- **线上代码高亮缺失**：编译期内嵌自定义语法,修复线上 Docker 镜像代码高亮缺失(原先运行时加载语法文件在打包镜像中找不到)。
- **mermaid 渲染**：改用 script 标签加载 IIFE bundle 修正全局变量取值;修复 tsc 类型错误;主题切换时重渲染已渲染的流程图。
- **文章锚点导航**：`scrollToHash` 增加一次性守卫,切主题不再跳回 URL hash;增加 ResizeObserver 布局稳定期,修正 mermaid 异步渲染导致的锚点落点偏移。
- **WASM 双端编译**：hooks 模块移出 server gate 双端可见(原先 WASM 编译失败)。
- **评论代码块转义**：统一 `escape_html`,修复代码块单引号未转义。
- **Docker daemon 容错**：Docker daemon 不可用/断连时不再 panic(集成测试在无 daemon 环境优雅跳过)。
- **后台布局**：重建结果消息改绝对定位,避免撑高容器顶起按钮。
- **图片 cfg 门控**：为 `ImageFmt` 别名补上 `#[cfg(feature = "server")]` 门控。
- **clippy/lint**：修复 rust-1.97 clippy `useless_borrows_in_formatting` 告警、Biome 告警。
- **安全测试**：补齐安全关键路径的单元测试盲区。

### Internal

- **skills 体系**：新增 `optimizing-rust-performance` 与 `rust-advanced-performance` 性能优化技能;清理已卸载的第三方 skills 及 lock 注册;`deploy-to-linux` 添加手动部署模式。
- **部署脚本**：新增 xun 服务器全量部署脚本。
- **文档**：AGENTS.md 补充别名归一化与 bun 镜像说明、数学公式与流程图架构说明、xterm-terminal/shared 库说明;补全 `.env.example` 缺失的 5 个环境变量。

## [0.4.0] - 2026-07-13

### Added

- **代码运行器（Code Runner）**：读者可在文章页直接运行 ``` ```lang runnable ``` 代码块，在隔离的 Docker 容器中执行，支持 Python / Node.js / Go / Rust 四种语言；admin 侧 `/admin/runner` 试验沙箱页支持任意代码试跑（跳过速率限制）。三层架构：`src/infra/docker.rs`（bollard 执行层，只读 rootfs + tmpfs + 资源/能力限制 + `ContainerGuard` Drop 强制清理）、`src/api/code_runner/`（任务注册表、语言注册表、双速率限制 + 白名单 + 大小检查）、Markdown 渲染层（`PostContent` 拆分 `Html`/`Runnable` 片段，每块渲染为真实 `<CodeRunner>` vdom 元素）。所有 `CODE_RUNNER_*` 环境变量可调，支持 per-IP 速率与每日上限。
- **流式代码执行（SSE + xterm.js）**：CodeRunner 切换为 SSE + xterm.js 流式输出方案。新增 `xterm-terminal` IIFE 子工程（xterm.js 6.0）、`xterm_bridge.rs` wasm-bindgen 绑定、`/api/exec/stream` SSE 端点、Docker 执行层流式路径（wait 与 log 读取并发）；支持无缓冲 stdout（`python -u`）、SSE done 事件回传 `duration_ms`、运行前隐藏输出区 + skeleton 占位。
- **`/admin/system` 管理后台**：全新管理区，5 个 tab —— 数据库状态（表统计/活跃连接/迁移版本）、服务器状态（sysinfo 主机指标 + moka 缓存命中率轮询）、SQL 控制台（全读写，4 道护栏：sqlparser AST 门 + WHERE 缺失拒绝 + 查询超时 + 前端确认；单元格类型化渲染 NULL/布尔/数字、表头 sticky、行截断展开）、数据导出（Axum 流式 SQL/CSV）、备份恢复（`pg_dump` 优先 + COPY 回退、DashMap 任务进度表 + 轮询、备份文件签名校验 + 路径白名单）。
- **UI 重新设计（工业极简 + Catppuccin）**：全站配色迁移到 Catppuccin（Latte/Mocha），移除 Rust 中硬编码颜色，统一语义色阶；后台重设计为现代极简侧边栏布局（写文章页改为左右两栏、编辑器自适应高度）；圆角 token 化为三档梯度（32/16/8）并统一所有组件；Markdown 表格重设计 + 表格单元格圆角防背景溢出；全局路由切换平滑挂载动画 + View Transitions 圆形展开主题切换动画；编辑器背景图（线条小狗）有内容时自动调淡透明度。
- **编辑器可运行代码块 NodeView**：CodeBlock 改用 `CodeBlockLowlight` + Catppuccin 高亮配色；新增 CodeBlockNodeView（语言标签 + 运行按钮 + 结果区），点语言标签可编辑语言与配置；斜杠菜单新增「可运行代码块」条目 + 模态框配置 runnable fence info；`make_run_code_closure` 桥接编辑器内运行代码。
- **编辑器任务列表手动输入**：支持 `- [ ]` 逐字符输入创建任务列表（appendTransaction，非 InputRule 全量替换）；前台与编辑器 checkbox 垂直中线对齐。
- **SQL 控制台 Ctrl+Enter**：`/admin/system` SQL 控制台接通 Ctrl/Cmd+Enter 运行快捷键。
- **CodeMirror Vim 模式**：admin CodeRunner 沙箱编辑器支持 Vim 模式开关，默认开启；CodeMirror 编辑器高度自适应与滚动限制。
- **文章重建内容按钮**：文章列表操作列新增「重建内容」按钮，重建支持并发 loading（spinner 覆盖文字）。
- **中文 slug 自动转拼音**：中文标题自动转拼音生成 URL slug。
- **FreeBSD x86_64 交叉编译**：`make build-freebsd` + `make freebsd-sysroot`，clang + lld + sysroot。
- **构建信息注入**：启动时打印 git/rustc/构建时间信息。
- **404 页面提交 HTTP 404 状态码**（SSR 层）；`ErrorBoundary` 包裹公开路由，文章详情页 404 等错误上抛至 `ErrorLayout`。
- **SSR 层 admin 认证守卫**：未登录访问 admin 直接在 SSR 跳转登录页，避免闪烁。
- **登录表单回车提交**；网站 favicon；评论背景图自动调淡。

### Changed

- **pnpm workspace 重构**：JS 子项目从 npm 迁移到 pnpm workspace，根工作区在 `libs/`，单一 `libs/pnpm-lock.yaml` + 共享 `libs/tsconfig.base.json`；引入 Biome v2.5 monorepo 配置并全量格式化；Makefile 整合 `lint`/`fix`/`test` 目标。
- **消除 `js_sys::eval`**：DOM 互操作全面从字符串求值迁移到 wasm-bindgen 绑定层（`tiptap_bridge`、`codemirror_bridge`、`xterm_bridge`），清理 wasm32 target 残留 clippy lint。
- **通用 hooks 抽取**：新增 `use_paginated`（分页加载）、`use_event_listener`（通用事件监听，解决 `use_hook` + `use_effect` + `use_drop` 资源所有权陷阱）。
- **SQL 控制台组件化**：`SqlConsoleTab` 改用独立 `SqlResultTable` 组件。
- **按钮 token 化**：新增 `BTN_PRIMARY`/`BTN_OUTLINE` 等按钮令牌与 `LoadingButton` 组件，消除样式散落。
- **骨架屏统一延迟**：统一骨架屏延迟机制，200ms 内加载完成不显示骨架，避免快网络闪烁。
- **后台菜单边距收紧**：后台所有页面左右边距统一（`px-10 → px-6`），写文章页移除页头条。
- **回收站合并入文章列表**：`/admin/trash` 合并为 `/admin/posts` 的 URL 驱动 tab，`PostStats` 新增回收站计数 badge。
- **依赖升级**：cargo 与 pnpm 依赖全量升级到最新版本；新增 `tokio-stream`（SSE）、`bollard`（Docker）。
- **Runner 配置**：`CODE_RUNNER_LANGUAGES` 默认开放全部语言；admin runner 页展示 Go/Rust。
- **Tooltip 组件抽取**：文章列表操作按钮用 `Tooltip` 包裹。

### Fixed

- **反应式 hook 不追踪普通 prop 的陷阱**：修复同一路由变体间导航（如 `/post/a → /post/b`）后文章正文/列表/标签不更新的严重 bug —— 根因是 `use_server_future`/`use_memo`/`use_resource` 不追踪非 signal 依赖。改为在闭包内读 `router.current()` signal 或直接内联计算。同理修复评论区 `CommentSection` 依赖追踪与 SSR hydration 不匹配。
- **主题切换动画**：修复 VT 动画期间 CodeMirror/xterm 主题同步（避免圆形展开动画期间直接跳变）、清理 VT 期间的 `animate-page-enter` transform（修复代码块被覆盖）、跟随系统模式系统偏好变化时同步 dark class（改回瞬切避免动画冲突）、`prefers-reduced-motion` 降级。
- **备份恢复假成功**：修复备份恢复实际不写入数据却返回成功（`psql` 未加 `ON_ERROR_STOP=1` 导致语句全错仍 exit 0）；修复备份/恢复任务轮询永不启动导致按钮卡在 loading。
- **可运行代码块容器清理**：修复容器清理失败静默泄漏（重试 + 日志告警）、编译型语言需 `/tmp` tmpfs 执行权、去掉 `nproc` ulimit 修复容器启动 EAGAIN。
- **文章锚点导航**：修复直接访问 `#hash` 时标题被 sticky header 遮住、hydration 后点击标题锚点触发整页刷新、hash 锚点跳转失效。
- **Tiptap 编辑器**：修复斜杠命令创建可运行代码块时模态框被立即关闭、斜杠命令文本残留进新节点（`/code` 带入 codeBlock）、代码块内 Backspace 删整块（`ignoreMutation` 误忽略 contentDOM 编辑）、Backspace 在 lowlight decoration 重建后失效、runnable 块 `classList.add` 抛 `InvalidCharacterError`、语言下拉展开时 Enter 误触发插入、空项 Enter 不退出列表（畸形文档根因）、TaskInputRule 升级后光标被甩到下一行、升级后折叠行被撑高 + 折叠图标垂直不居中、CodeMirror 折叠图标垂直不居中。
- **CodeMirror 编辑器**：修复编辑回退反馈循环（editing reversion loop）、编辑器塌缩导致上下背景割裂（`height:100%` 失效）、行号区背景与代码区割裂、未撑满容器、SQL 编辑器 gutter 与 content 背景割裂。
- **后台骨架屏**：修复后台骨架屏不可见、骨架屏→认证→正常页面布局闪烁、写文章页骨架屏高度不撑满（多次迭代）。
- **路由与分页**：修复 SQL 控制台 Ctrl+Enter 触发 panic（无 dioxus scope）、上下篇切换后可运行代码块消失。
- **UI 细节**：修复 markdown 表格水平填充渲染、admin 布局滚动条位置、写文章页滚动性、`--font-sans` 补齐 CJK sans 字体栈、暗色 type 色值。
- **数据库错误日志**：展开迁移错误的 source chain 全链路。
- 其他构建、CI、Docker 镜像（multi-arch buildx、HTTPS Debian 镜像绕过 HTTP 透明拦截、apk 清华镜像）、格式化与测试修复。

### Security

- **SQL 控制台护栏加固**：`DROP DATABASE`/`DROP SCHEMA`/`CREATE DATABASE` 绝对禁止（字符串预检 + AST 门）；`DROP`/`TRUNCATE`/`ALTER` 需确认；`UPDATE`/`DELETE` 无 `WHERE` 拒绝；多语句默认禁用；结果上限 500 行。
- **备份文件校验**：备份文件携带签名头，restore 拒绝非系统文件；`backup_path` 路径穿越漏洞修复（补单测）；`pg_dump --clean --if-exists` 使 restore 幂等（drop+recreate 而非 relation 已存在报错）。
- **备份恢复补单测**：`backup_path` 路径穿越漏洞补表驱动单测。

### Internal

- **新子工程 `xterm-terminal`**：xterm.js 6.0 IIFE 库 + smoke 测试。
- **新子工程 `codemirror-editor`**：CodeMirror 编辑器 + Rust bridge，Ctrl/Cmd+Enter 运行快捷键。
- **Docker runner 镜像**：`docker/build-runners.sh` 构建 base → python → node → go → rust 链；Go 镜像重定向 `GOCACHE`/`GOPATH` 到 `/tmp`，Rust 镜像封装 `run-rust.sh` 两步编译+运行 wrapper。
- **新增 Rust 单测**：SQL 控制台护栏表驱动单测、备份恢复单测、markdown `wrap_images_with_blur` 解耦文件系统依赖。
- **codemirror-editor smoke 测试**、`xterm-terminal` smoke 测试。
- **AGENTS.md 文档扩充**：Code Runner 架构说明、双 target 验证陷阱、custom hook 资源所有权陷阱、反应式 hook 不追踪普通 prop 的踩坑记录、Tiptap 交互 bug Playwright 调试方法论。
- **matt-pocock engineering skills** 引入。

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
