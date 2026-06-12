//! IP 与用户代理信息定期清理后台任务。
//!
//! 仅在 `server` feature 启用时编译，每天运行一次。

use std::time::Duration;

use tokio::time::interval;

use crate::db::pool::get_conn;

/// 启动 IP 信息清理循环，每天将 90 天前的评论的 `ip_address` 与 `user_agent` 置空。
pub async fn run_purge() {
    // 每天触发一次
    let mut ticker = interval(Duration::from_secs(86400));
    loop {
        ticker.tick().await;
        match get_conn().await {
            Ok(client) => {
                // 仅清理 90 天前且仍保留 IP 的评论
                if let Err(e) = client
                    .execute("UPDATE comments SET ip_address = NULL, user_agent = NULL WHERE created_at < NOW() - INTERVAL '90 days' AND ip_address IS NOT NULL", &[])
                    .await
                {
                    tracing::error!("IP purge error: {:?}", e);
                }
            }
            Err(e) => {
                tracing::error!("Failed to get DB connection for IP purge: {:?}", e);
            }
        }
    }
}
