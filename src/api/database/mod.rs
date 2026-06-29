//! 数据库管理 server functions 与 Axum 处理器。
//!
//! 按功能拆分子模块：
//! - [`status`]：数据库运行状态聚合（表/连接/死元组/迁移版本）。
//!
//! 后续 task 会新增：`system_status`（服务器状态）、`sql_console`（SQL 执行+护栏）、
//! `schema`（SQL 补全数据）、`export`（流式导出）、`backup`/`tasks`（备份恢复+进度）。

/// 数据库运行状态聚合查询。
pub mod status;
/// 服务器状态聚合查询（应用内 + 主机层）。
pub mod system_status;
