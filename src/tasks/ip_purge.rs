use std::time::Duration;

use tokio::time::interval;

use crate::db::pool::get_conn;

pub async fn run_purge() {
    let mut ticker = interval(Duration::from_secs(86400));
    loop {
        ticker.tick().await;
        match get_conn().await {
            Ok(client) => {
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
