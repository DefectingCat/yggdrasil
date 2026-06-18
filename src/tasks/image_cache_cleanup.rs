//! 图片磁盘缓存定期清理任务。
//!
//! 仅在 `server` feature 启用时编译，每小时运行一次。
//! 删除超过保留时间的文件，并在总大小超过上限时按修改时间删除最旧的文件。

use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use tokio::time::interval;

const CACHE_DIR: &str = "uploads/.cache";

/// 启动图片磁盘缓存清理循环，每小时触发一次。
pub async fn run_cleanup() {
    let mut ticker = interval(Duration::from_secs(3600));
    loop {
        if let Err(e) = cleanup_image_cache().await {
            tracing::error!("Image disk cache cleanup error: {:?}", e);
        }
        ticker.tick().await;
    }
}

/// 读取环境变量并清理默认磁盘缓存目录。
pub async fn cleanup_image_cache() -> io::Result<()> {
    let base = Path::new(CACHE_DIR);
    let max_mb = std::env::var("IMAGE_DISK_CACHE_MAX_MB")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1024);
    let max_age_hours = std::env::var("IMAGE_DISK_CACHE_MAX_AGE_HOURS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(168);
    let (deleted, bytes_freed) = cleanup_image_cache_at(base, max_mb, max_age_hours).await?;
    if !deleted.is_empty() {
        tracing::info!(
            "Image disk cache cleanup: removed {} files, freed {} bytes",
            deleted.len(),
            bytes_freed
        );
    }
    Ok(())
}

/// 清理指定目录下的图片磁盘缓存。
///
/// 返回被删除文件的路径列表以及释放的总字节数。
pub async fn cleanup_image_cache_at(
    base: &Path,
    max_mb: u64,
    max_age_hours: u64,
) -> io::Result<(Vec<PathBuf>, u64)> {
    if !base.exists() {
        return Ok((Vec::new(), 0));
    }

    let max_age = Duration::from_secs(max_age_hours * 3600);
    let now = SystemTime::now();
    let cutoff = now - max_age;

    let mut entries: Vec<(PathBuf, u64, SystemTime)> = Vec::new();
    collect_files(base, &mut entries).await?;

    let mut deleted = Vec::new();
    let mut bytes_freed: u64 = 0;

    // 第一轮：删除超过保留期限的文件。
    let mut remaining: Vec<(PathBuf, u64, SystemTime)> = Vec::new();
    for (path, size, mtime) in entries {
        if mtime < cutoff {
            match tokio::fs::remove_file(&path).await {
                Ok(_) => {
                    deleted.push(path);
                    bytes_freed += size;
                }
                Err(e) => {
                    tracing::warn!("Failed to remove expired cache file {:?}: {:?}", path, e);
                }
            }
        } else {
            remaining.push((path, size, mtime));
        }
    }

    // 第二轮：若总大小仍超过上限，按修改时间从旧到新删除。
    let max_bytes = max_mb.saturating_mul(1024 * 1024);
    let mut total: u64 = remaining.iter().map(|(_, size, _)| size).sum();
    if total > max_bytes {
        remaining.sort_by_key(|a| a.2);
        for (path, size, _) in remaining {
            if total <= max_bytes {
                break;
            }
            match tokio::fs::remove_file(&path).await {
                Ok(_) => {
                    total -= size;
                    deleted.push(path);
                    bytes_freed += size;
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to remove cache file {:?} for size cap: {:?}",
                        path,
                        e
                    );
                }
            }
        }
    }

    Ok((deleted, bytes_freed))
}

/// 递归收集目录下的所有常规文件，返回路径、大小与修改时间。
async fn collect_files(
    base: &Path,
    entries: &mut Vec<(PathBuf, u64, SystemTime)>,
) -> io::Result<()> {
    let mut stack = vec![base.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let mut reader = tokio::fs::read_dir(&dir).await?;
        while let Some(entry) = reader.next_entry().await? {
            let metadata = entry.metadata().await?;
            if metadata.is_file() {
                let mtime = metadata.modified()?;
                entries.push((entry.path(), metadata.len(), mtime));
            } else if metadata.is_dir() {
                stack.push(entry.path());
            }
        }
    }
    Ok(())
}

#[cfg(all(test, feature = "server"))]
mod tests {
    use super::*;
    use std::time::{Duration, UNIX_EPOCH};
    use tokio::time::sleep;

    fn temp_cache_dir() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("yggdrasil_image_cache_test_{}_{}", nanos, std::process::id()))
    }

    #[tokio::test]
    async fn cleanup_ignores_missing_directory() {
        let dir = temp_cache_dir();
        let (deleted, freed) = cleanup_image_cache_at(&dir, 1024, 168).await.unwrap();
        assert!(deleted.is_empty());
        assert_eq!(freed, 0);
    }

    #[tokio::test]
    async fn cleanup_removes_expired_files_by_age() {
        let dir = temp_cache_dir();
        tokio::fs::create_dir_all(&dir).await.unwrap();

        let old = dir.join("old.dat");
        tokio::fs::write(&old, b"old content").await.unwrap();
        // 确保文件的修改时间严格早于清理时计算的截止时间。
        sleep(Duration::from_millis(1100)).await;

        let (deleted, freed) = cleanup_image_cache_at(&dir, 1024, 0).await.unwrap();
        assert_eq!(deleted.len(), 1);
        assert!(!old.exists());
        assert!(freed > 0);

        tokio::fs::remove_dir_all(&dir).await.unwrap();
    }

    #[tokio::test]
    async fn cleanup_keeps_recent_files() {
        let dir = temp_cache_dir();
        tokio::fs::create_dir_all(&dir).await.unwrap();

        let recent = dir.join("recent.dat");
        tokio::fs::write(&recent, b"recent content").await.unwrap();

        let (deleted, freed) = cleanup_image_cache_at(&dir, 1024, 168).await.unwrap();
        assert!(deleted.is_empty());
        assert_eq!(freed, 0);
        assert!(recent.exists());

        tokio::fs::remove_dir_all(&dir).await.unwrap();
    }

    #[tokio::test]
    async fn cleanup_enforces_size_cap_by_mtime() {
        let dir = temp_cache_dir();
        tokio::fs::create_dir_all(&dir).await.unwrap();

        let f1 = dir.join("oldest.dat");
        tokio::fs::write(&f1, vec![0u8; 1024 * 1024]).await.unwrap();
        sleep(Duration::from_millis(1100)).await;

        let f2 = dir.join("middle.dat");
        tokio::fs::write(&f2, vec![0u8; 1024 * 1024]).await.unwrap();
        sleep(Duration::from_millis(1100)).await;

        let f3 = dir.join("newest.dat");
        tokio::fs::write(&f3, vec![0u8; 1024 * 1024]).await.unwrap();

        // 上限 2 MB，当前 3 MB，应删除最旧的一个文件。
        let (deleted, freed) = cleanup_image_cache_at(&dir, 2, 1000).await.unwrap();
        assert_eq!(deleted.len(), 1);
        assert!(!f1.exists());
        assert!(f2.exists());
        assert!(f3.exists());
        assert_eq!(freed, 1024 * 1024);

        tokio::fs::remove_dir_all(&dir).await.unwrap();
    }

    #[tokio::test]
    async fn cleanup_recurses_into_subdirectories() {
        let dir = temp_cache_dir();
        let sub = dir.join("nested");
        tokio::fs::create_dir_all(&sub).await.unwrap();

        let nested = sub.join("nested.dat");
        tokio::fs::write(&nested, b"nested content").await.unwrap();
        sleep(Duration::from_millis(1100)).await;

        let (deleted, _freed) = cleanup_image_cache_at(&dir, 1024, 0).await.unwrap();
        assert_eq!(deleted.len(), 1);
        assert!(!nested.exists());

        tokio::fs::remove_dir_all(&dir).await.unwrap();
    }
}
