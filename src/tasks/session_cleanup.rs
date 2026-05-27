use std::time::Duration;

use tokio::time::interval;

use crate::db::pool::DB_POOL;

pub async fn run_cleanup() {
    let mut ticker = interval(Duration::from_secs(3600));
    loop {
        ticker.tick().await;
        match DB_POOL.get().await {
            Ok(client) => {
                if let Err(e) = client
                    .execute("DELETE FROM sessions WHERE expires_at < NOW()", &[])
                    .await
                {
                    tracing::error!("Session cleanup error: {:?}", e);
                }
            }
            Err(e) => {
                tracing::error!("Failed to get DB connection for cleanup: {:?}", e);
            }
        }
    }
}
