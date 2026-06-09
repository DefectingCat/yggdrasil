---
name: release
description: |
  发布新版本时使用。自动从 git 历史生成 CHANGELOG，更新版本号，
  创建 annotated tag 并推送到远程。触发关键词："发布"、"release"、"新版本"、"打 tag"。
allowed-tools:
  - Bash
  - Read
  - Write
  - Edit
  - Glob
  - Grep
  - AskUserQuestion
metadata:
  trigger: 发布新版本、创建 release、打 tag、更新 changelog
---

# Release: 版本发布流程

将当前代码状态发布为新版本，遵循 [Keep a Changelog](https://keepachangelog.com) 格式和 [Semantic Versioning](https://semver.org)。

## 前置检查

1. 确认工作区干净（`git status` 无未提交更改）
2. 确认在主分支上
3. 检查是否已有 CHANGELOG.md

## 确认版本号

向用户确认版本号（遵循 semver）：
- **PATCH** (x.y.z → x.y.z+1)：bug 修复、小调整
- **MINOR** (x.y.z → x.y+1.0)：新功能，向后兼容
- **MAJOR** (x.y.z → x+1.0.0)：破坏性变更

## 生成 CHANGELOG

### 确定变更范围

- 找到最新 tag：`git tag --sort=-version:refname | head -1`
- 如果没有 tag，从第一个 commit 开始
- 提取范围内的所有 commit：`git log <last_tag>..HEAD --oneline`

### 按类型归类

根据 conventional commit 前缀分类：

| 前缀 | 分类 | 说明 |
|------|------|------|
| `feat:` | **Added** | 新功能 |
| `fix:` | **Fixed** | bug 修复 |
| `perf:` | **Changed** (性能) | 性能优化 |
| `refactor:` | **Changed** | 重构 |
| `test:` | 内部 | 测试（合并到 Changed 或省略） |
| `chore:` / `build:` / `deps:` | 内部 | 构建/依赖（合并到 Changed 或省略） |
| `docs:` | 内部 | 文档（通常省略） |
| 无前缀或中文前缀 | 按内容判断 | 根据描述语义归类 |

### 写入格式

```markdown
## [x.y.z] - YYYY-MM-DD

### Added
- 功能描述（从 commit message 提炼，去重合并）

### Changed
- 变更描述

### Deprecated
- 即将移除的功能（如有）

### Removed
- 已移除的功能（如有）

### Fixed
- 修复描述

### Security
- 安全相关修复（如有）
```

**关键原则：**
- 合并语义相同的 commit（如多次修复同一功能只写一条）
- 用简洁自然的语言描述，不照搬 commit message
- 省略纯内部变更（如 `chore: format`），除非用户有要求
- 新版本条目插在文件最前面（`## [Unreleased]` 之后，如果有的话）

## 更新版本号

在 `Cargo.toml` 中更新 `version` 字段为新版本号。

## 提交与打 Tag

```bash
git add CHANGELOG.md Cargo.toml Cargo.lock
git commit -m "chore: release v<x.y.z>"
git tag -a v<x.y.z> -m "v<x.y.z>: <简短版本描述>"
```

## 推送

向用户确认后执行：

```bash
git push origin <branch> --tags
```

## GitHub Release（可选）

如果用户需要 GitHub Release，使用 `gh release create`：

```bash
gh release create v<x.y.z> --title "v<x.y.z>" --notes "$(sed -n '/## \[<x.y.z>\]/,/## \[/p' CHANGELOG.md | head -n -1)"
```

## 检查清单

完成前确认：
- [ ] CHANGELOG.md 已更新且格式正确
- [ ] Cargo.toml 版本号已更新
- [ ] Cargo.lock 已同步（如有）
- [ ] commit message 符合项目风格
- [ ] annotated tag 已创建
- [ ] 已推送到远程
