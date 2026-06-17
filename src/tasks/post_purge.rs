//! 回收站自动清理后台任务。
//!
//! 仅在 `server` feature 启用时编译，每天运行一次。
//! 每次执行前读取 settings 表：若自动清理关闭则跳过，否则按保留天数物理删除过期文章。

use std::time::Duration;

use tokio::time::interval;

use crate::db::pool::get_conn;
use crate::models::settings::TrashSettings;

/// 启动回收站自动清理循环，每天触发一次。
///
/// 每次读取最新配置：若 `trash_auto_purge_enabled` 关闭则 no-op，
/// 否则删除 `deleted_at < NOW() - retention_days` 的文章。
/// 任何错误只记录日志，不中断循环。
pub async fn run_purge() {
    let mut ticker = interval(Duration::from_secs(86400));
    loop {
        match get_conn().await {
            Ok(client) => {
                match purge_expired(&client).await {
                    Ok(n) => {
                        if n > 0 {
                            tracing::info!("Post auto-purge: removed {} expired trashed posts", n);
                        }
                    }
                    Err(e) => tracing::error!("Post auto-purge error: {:?}", e),
                }
            }
            Err(e) => tracing::error!("Failed to get DB connection for post purge: {:?}", e),
        }
        ticker.tick().await;
    }
}

/// 读取配置并删除过期回收站文章，返回删除行数。
///
/// 抽取为独立函数便于单元测试（虽需 DB，但逻辑集中）。
async fn purge_expired(client: &tokio_postgres::Client) -> Result<u64, tokio_postgres::Error> {
    // 读取配置，缺键时回退默认值。
    let enabled: bool = client
        .query_opt(
            "SELECT value FROM settings WHERE key = 'trash_auto_purge_enabled'",
            &[],
        )
        .await?
        .and_then(|r| r.get::<_, String>("value").parse().ok())
        .unwrap_or(false);

    if !enabled {
        return Ok(0);
    }

    let days: i32 = client
        .query_opt(
            "SELECT value FROM settings WHERE key = 'trash_retention_days'",
            &[],
        )
        .await?
        .and_then(|r| r.get::<_, String>("value").parse().ok())
        .unwrap_or(30);

    let days = TrashSettings::clamp_retention(days);

    let n = client
        .execute(
            "DELETE FROM posts WHERE deleted_at IS NOT NULL AND deleted_at < NOW() - make_interval(days => $1)",
            &[&days],
        )
        .await?;
    Ok(n)
}
