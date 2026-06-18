//! 会话过期清理后台任务。
//!
//! 仅在 `server` feature 启用时编译，每小时运行一次。

use std::time::Duration;

use tokio::time::interval;

use crate::db::pool::get_conn;

/// 启动会话清理循环，每小时删除 `expires_at < NOW()` 的过期会话。
pub async fn run_cleanup() {
    // 每小时触发一次
    let mut ticker = interval(Duration::from_secs(3600));
    loop {
        match get_conn().await {
            Ok(client) => {
                // 删除已过期会话
                match client
                    .execute("DELETE FROM sessions WHERE expires_at < NOW()", &[])
                    .await
                {
                    Ok(_) => {
                        // 同时清空内存中的会话缓存，避免已失效会话继续命中。
                        crate::cache::SESSION_CACHE.invalidate_all();
                    }
                    Err(e) => {
                        tracing::error!("Session cleanup error: {:?}", e);
                    }
                }
            }
            Err(e) => {
                tracing::error!("Failed to get DB connection for cleanup: {:?}", e);
            }
        }
        ticker.tick().await;
    }
}
