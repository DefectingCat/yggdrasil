# 素材管理（Assets）设计规格

> 状态：已确认（2026-07-24 grilling 访谈结论）
> 范围：后台 `/admin/assets` 素材库 + 封面上传联动。编辑器（Tiptap）正文选择器**不在**本期范围。

## 1. 背景与动机

当前图片上传链路（`POST /api/upload` → `uploads/YYYY/MM/DD/`）只有文件没有数据层：

- 图片仅以 URL 字符串形式被 `posts.content_html` / `cover_image` 引用，**引用关系不可查**；
- 编辑器里上传后又删掉的图、被替换的封面图，文件永久残留磁盘，**无孤儿治理**；
- 删除文件无护栏，误删即线上文章 404（且 SSR 缓存会延迟暴露问题）。

本功能建立 `assets` 注册表 + `asset_refs` 引用表，实现：可视化管理、引用状态可见、删除保护、孤儿一键清理、封面图复用。

设计取舍来自 2026 年 DAM/CMS 趋势调研的过滤结论：只采纳「孤儿治理」「扁平无文件夹」「上传即元数据（alt）」三条；AI 语义搜索、标签体系、RBAC 可视、回收站均不采纳（单人博客量级不匹配）。

## 2. 目标 / 非目标

### 目标

1. 所有经 `/api/upload` 上传的图片自动登记入 `assets` 表（编辑器正文图与封面图共用该端点，全覆盖）。
2. 文章保存时同步 `asset_refs`，引用关系精确可查（复刻 `sync_tags` 模式）。
3. `/admin/assets` 管理页：网格浏览、搜索、引用状态筛选、单张删除、alt 编辑、复制 URL。
4. 删除保护：引用中禁删；孤儿（含 7 天保护窗）可一键批量清理。
5. 存量回填：素材页「重建索引」手动按钮，幂等可重跑。
6. 封面联动：`CoverUploader` 支持「从素材库选择」。

### 非目标（明确不做）

- Tiptap 正文「从素材库插入」选择器（后续版本候选）
- 标签体系 / 自由描述字段
- 回收站 / 软删除
- 外链图片（非 `/uploads/` 路径）管理
- 多选 checkbox 批量操作
- AI 打标 / 语义搜索

## 3. 数据模型

### 3.1 migration `015_assets.sql`

```sql
CREATE TABLE IF NOT EXISTS assets (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    path        TEXT NOT NULL UNIQUE,          -- 相对路径 "2026/07/24/153000.<uuid>.webp"
    filename    TEXT NOT NULL,                 -- 原始文件名（客户端提供，仅展示用）
    mime        TEXT NOT NULL,                 -- 落盘后的实际 MIME
    size_bytes  BIGINT NOT NULL,
    width       INTEGER NOT NULL,
    height      INTEGER NOT NULL,
    alt         TEXT,                          -- 管理性 alt，仅作默认值/备注，不回写已有文章
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_assets_created_at ON assets (created_at DESC);

CREATE TABLE IF NOT EXISTS asset_refs (
    asset_id UUID NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
    post_id  INTEGER NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    PRIMARY KEY (asset_id, post_id)
);
CREATE INDEX IF NOT EXISTS idx_asset_refs_post ON asset_refs (post_id);
```

同步在 `src/db/migrate.rs` 的 `MIGRATIONS` 数组注册（编译测试守卫文件/数组一致性）。

注意：`posts.id` 为 `INTEGER`（见 001/002 迁移），`asset_refs.post_id` 类型必须对齐。

### 3.2 一致性语义

- **磁盘是字节唯一存储，DB 是元数据注册表**。两者可能不一致（手动删文件、回填前上传），以「重建索引」自愈：
  - 扫 `uploads/`（跳过 `.cache`/`.trash` 等点目录）→ upsert assets（存在则更新 size/width/height，**不覆盖** alt）；
  - 全表扫 posts（含回收站？——见 §7 边界）→ 重建 asset_refs；
  - DB 中文件已消失的行连同 refs 级联删除。
- 新上传增量入库；重建只是兜底，不是常态路径。

## 4. 写入路径改造

### 4.1 `upload_image`（`src/api/upload.rs`）

文件落盘成功后 INSERT assets：

```text
顺序：写文件 → 读 header 得 width/height（上传时已做过 check_upload_dimensions，复用其读取结果）
     → INSERT assets(path, filename, mime, size, w, h)
失败补偿：INSERT 失败 → 尝试 tokio::fs::remove_file 清理已落盘文件，返回 500
filename：取 multipart field 的 file_name()（客户端原始名），缺失时退化为落盘文件名
```

不改动的部分：限流、鉴权、MIME/magic-bytes/尺寸校验、WebP 转码策略全部保持原样。

### 4.2 `sync_asset_refs`（`src/api/posts/helpers.rs`，镜像 `sync_tags`）

文章 create/update 事务内调用：

```rust
pub(super) async fn sync_asset_refs(
    tx: &deadpool_postgres::Transaction<'_>,
    post_id: i32,
    content_html: &str,
    cover_image: Option<&str>,
) -> Result<(), AppError>
```

逻辑：

1. 正则提取 `content_html` 中全部 `/uploads/<rel>` 路径（去 query），加上 `cover_image`（若为 `/uploads/` 路径）；
2. `DELETE FROM asset_refs WHERE post_id = $1`；
3. `INSERT INTO asset_refs SELECT id, $1 FROM assets WHERE path = ANY($2)`（仅匹配已登记资产；未登记的路径——如回填前的旧图——静默跳过，由重建索引兜底）。

调用点：`create.rs` / `update.rs` 中与 `sync_tags` 相同位置（同事务）。

正则复用/对齐 `markdown.rs` 的 `IMG_RE` 思路，但需匹配任意位置的 `/uploads/` 出现（含 blur-img 的 `data-src`），建议 `r#"/uploads/([0-9]{4}/[0-9]{2}/[0-9]{2}/[^"?#\s]+)"#`。

## 5. Server Functions（`src/api/assets/`）

全部遵循现有约定：`#[server(..., "/api")]`、`get_current_admin_user().await?` 守卫、`AppError` 错误映射、写操作返回后失效对应缓存。

| 函数 | 参数 | 说明 |
|---|---|---|
| `list_assets` | `filter: AssetFilter`（All/Used/Orphan）、`query: String`、`sort: AssetSort`（CreatedDesc/SizeDesc）、`page: i32` | 每页 60。返回 `(Vec<AssetDto>, total)`。`AssetDto` 含 `ref_count` 与 `refs: Vec<(post_id, title)>`（仅详情需要时可拆第二个函数） |
| `update_asset_alt` | `id: Uuid, alt: String` | 更新 alt + updated_at |
| `delete_asset` | `id: Uuid` | **引用中返回 `Ok(success:false, 引用文章列表)`**（业务拒绝走 Ok，遵循仓库约定）；孤儿执行：删文件 → 删 DB 行（refs 级联）→ 清 `uploads/.cache` 中该路径的派生文件 → `IMAGE_DIMENSIONS_CACHE` invalidate |
| `purge_orphan_assets` | 无 | 清理 `无引用 AND created_at < now() - interval '7 days'`。先 SELECT 出清单（事务外逐项删文件，容忍单项失败并记录），返回 `(deleted_count, freed_bytes, failures)` |
| `rebuild_assets_index` | 无 | 全量重建（§3.2）。批处理 + 进度返回，交互复刻 `rebuild_content_html` |

缓存失效注意：删除素材后，引用它的 SSR 页面理论上不存在（引用中禁删）；孤儿图无页面引用，无需 `ssr_cache` 失效。

## 6. 管理页 `/admin/assets`

### 6.1 路由与导航

- `src/router.rs` admin nest 下新增 `#[route("/assets")] Assets {}` 与分页路由（对齐 comments 的 `/comments/:page` 模式）；
- `AdminLayout` 导航加「素材」入口，图标用 Feather 风格线框（image 图标），与现有导航一致。

### 6.2 页面结构

- **顶栏**：搜索框（filename/alt LIKE）+ 筛选 tabs（全部 / 引用中 / 孤儿，显示各计数）+ 排序（最新/最大）+ 「清理孤儿」按钮 + 「重建索引」按钮；
- **网格**：卡片 = 缩略图（`/uploads/<path>?thumb=300x300`，`serve_image` 现成）+ filename + 尺寸/大小 + 引用徽标（`被 N 篇引用` / `孤儿`）；
- **卡片操作**：复制 URL、编辑 alt（inline 或 modal）、查看引用（列出文章标题，点击跳 `/admin/write/:id`）、删除；
- **删除交互**：引用中 → 按钮禁用 + tooltip 列引用文章；孤儿 → 确认框 → 硬删除；
- **「清理孤儿」**：按钮文本带数量与总大小（`清理 23 张孤儿图（45.2 MB）`），确认框说明 7 天保护窗规则；
- 设计语言遵循 `yggdrasil-ui-design-taste`：2rem 圆角卡片、无感阴影、组件挂载式路由动画；用户文案中文。

## 7. 封面联动（`CoverUploader`）

- 空态与预览态加「从素材库选择」入口；
- 点击弹出 modal：素材网格（复用管理页的 list_assets，默认 All 筛选、最新排序），支持搜索，单击选中回填 `cover_image`；
- modal 内保留「上传新图」按钮（复用现有 `spawn_cover_upload` 闭包）；
- 纯 Dioxus 组件，不触碰 Tiptap/bridge。

## 8. 边界 case

| case | 处理 |
|---|---|
| 未保存草稿的图被当孤儿清理 | 7 天保护窗：`purge_orphan_assets` 只清 `created_at < now()-7d`；列表页「孤儿」筛选展示全部但清理按钮有窗口说明 |
| 回收站文章的引用 | refs 照常在保存时建立；**回收站文章的引用同样阻止删除**（purge 文章时 refs 级联删，图变孤儿可被清）。语义自洽，无需特判 |
| 上传成功但 DB INSERT 失败 | 补偿删文件，返回 500（见 §4.1） |
| 外链图（非 /uploads/） | 不入库、不追踪，正文/封面外链保持现状 |
| 重建时文件已消失 | DB 行级联删除 refs 后删行 |
| 重建时 DB 已有行 | upsert 更新技术字段，**保留 alt** |
| `sync_asset_refs` 时图未登记（回填前旧文重存） | 静默跳过，重建索引兜底 |
| 删除素材的缓存清理 | `uploads/.cache/<key>` 派生文件 + `IMAGE_DIMENSIONS_CACHE`；无需 SSR 失效（孤儿无页面引用） |

## 9. 实现阶段（tracer bullets）

1. **数据层**：015 迁移 + models/asset.rs + `upload_image` 入库 + `sync_asset_refs` 接入 create/update → 验证：上传一张图，DB 有行；保存文章，refs 正确
2. **管理页只读**：`list_assets` + 网格/搜索/筛选/排序/分页 + 导航入口 → 验证：页面可用
3. **删除与清理**：`delete_asset` + `purge_orphan_assets` + 缓存清理 + 确认交互 → 验证：引用中禁删；孤儿删除后文件/DB/缓存三清
4. **重建索引**：`rebuild_assets_index` + 批处理进度 → 验证：存量图全部入库，refs 正确，幂等重跑无副作用
5. **封面联动**：选择器 modal → 验证：选图回填封面

每阶段独立可交付、可提交（`feat(assets): ...`）。

## 10. 验收标准

- [ ] 编辑器上传图片 → `assets` 表出现对应行（尺寸/大小/MIME 正确）
- [ ] 保存含图文章 → `asset_refs` 精确反映引用；改文删图后 refs 同步移除
- [ ] 引用中的素材删除被拦截并列出引用文章
- [ ] 孤儿单删后：文件消失、DB 行消失、`.cache` 派生物消失
- [ ] 「清理孤儿」只清 7 天前的无引用图，返回数量与释放字节数
- [ ] 「重建索引」后存量图全部可见，重跑结果不变（幂等）
- [ ] 封面可从素材库选择回填，也可在 modal 内上传新图
- [ ] `cargo test --features server` 全绿；新增迁移通过文件/数组一致性编译测试
