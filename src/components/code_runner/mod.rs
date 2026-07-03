//! 代码运行器（Code Runner）组件。
//!
//! 提供可运行代码块的交互 UI：显示源码、运行按钮、运行阶段、输出区。
//! 点击 Run 时调用 `StartExec` server function 提交，轮询 `GetExecResult` 直到终态。
//!
//! CodeMirror 编辑器实例由调用方（阅读器扫描 / 后台试运行）在 WASM 端按容器 id
//! 挂载；本组件只负责布局、状态机与轮询，与编辑器桥接保持解耦。

pub mod runner;
pub use runner::CodeRunner;
