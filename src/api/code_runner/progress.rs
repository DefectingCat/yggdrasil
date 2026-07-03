//! 内存任务缓冲表：基于 DashMap 的执行任务注册中心。
//!
//! 异步执行模型：StartExec 立即返回 task_id，容器在后台 tokio task 中运行，
//! 前端通过 GetExecResult 轮询本表读取阶段与结果。任务条目在 TTL 过期后由
//! [`gc_old_tasks`] 回收，避免内存无限增长。

use chrono::{Duration, Utc};
use dashmap::DashMap;
use std::sync::LazyLock;

use crate::api::code_runner::{ExecResult, ExecStatus, ExecTask};
use crate::infra::runner_config::RUNNER_CONFIG;

/// 全局任务注册表。
pub static EXEC_TASKS: LazyLock<DashMap<String, ExecTask>> = LazyLock::new(DashMap::new);

/// 创建一个排队中的新任务。
pub fn insert_task(id: String) {
    let task = ExecTask {
        id: id.clone(),
        status: ExecStatus::Queued,
        stage: "排队中".to_string(),
        created_at: Utc::now(),
        result: None,
    };
    EXEC_TASKS.insert(id, task);
}

/// 更新任务阶段（状态 + 描述），不改写 result。
pub fn update_task_stage(id: &str, status: ExecStatus, stage: &str) {
    if let Some(mut task) = EXEC_TASKS.get_mut(id) {
        task.status = status;
        task.stage = stage.to_string();
    }
}

/// 写入最终结果：状态置为「执行完毕」并填充 result。
pub fn update_task_result(id: &str, status: ExecStatus, result: ExecResult) {
    if let Some(mut task) = EXEC_TASKS.get_mut(id) {
        task.status = status;
        task.stage = "执行完毕".to_string();
        task.result = Some(result);
    }
}

/// 回收超过 `RUNNER_CONFIG.task_ttl_secs` 的历史任务。
pub fn gc_old_tasks() {
    let ttl_secs = RUNNER_CONFIG.task_ttl_secs as i64;
    let now = Utc::now();
    EXEC_TASKS.retain(|_, task| {
        let age = now - task.created_at;
        age < Duration::seconds(ttl_secs)
    });
}

#[cfg(all(test, feature = "server"))]
mod tests {
    use super::*;

    #[test]
    #[serial_test::serial]
    fn test_task_lifecycle_and_gc() {
        let task_id = "test-task-progress-123".to_string();
        insert_task(task_id.clone());
        assert!(EXEC_TASKS.contains_key(&task_id));
        assert_eq!(
            EXEC_TASKS.get(&task_id).unwrap().status,
            ExecStatus::Queued
        );

        update_task_stage(&task_id, ExecStatus::Running, "运行中");
        assert_eq!(
            EXEC_TASKS.get(&task_id).unwrap().status,
            ExecStatus::Running
        );

        let res = ExecResult {
            status: ExecStatus::Success,
            stdout: "hello".to_string(),
            stderr: "".to_string(),
            exit_code: Some(0),
            duration_ms: 50,
            language: "python".to_string(),
        };
        update_task_result(&task_id, ExecStatus::Success, res);
        assert_eq!(
            EXEC_TASKS.get(&task_id).unwrap().status,
            ExecStatus::Success
        );
        assert!(EXEC_TASKS.get(&task_id).unwrap().result.is_some());

        // 修改创建时间以便测试 GC
        if let Some(mut task) = EXEC_TASKS.get_mut(&task_id) {
            task.created_at = Utc::now() - Duration::seconds(1000);
        }
        gc_old_tasks();
        assert!(!EXEC_TASKS.contains_key(&task_id));
    }
}
