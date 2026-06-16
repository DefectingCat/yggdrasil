//! 回收站与站点配置模型。

/// 默认保留天数（天）。
pub const DEFAULT_RETENTION_DAYS: i32 = 30;
/// 默认不启用自动清理。
pub const DEFAULT_AUTO_PURGE_ENABLED: bool = false;
/// 保留天数下限（天）。
pub const MIN_RETENTION_DAYS: i32 = 1;
/// 保留天数上限（天）。防止误填超大值导致永不清理。
pub const MAX_RETENTION_DAYS: i32 = 365;

/// 回收站配置。
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TrashSettings {
    /// 是否启用自动定时清理。
    pub auto_purge_enabled: bool,
    /// 已删除文章保留天数，超过后被后台任务物理删除。
    pub retention_days: i32,
}

impl Default for TrashSettings {
    fn default() -> Self {
        Self {
            auto_purge_enabled: DEFAULT_AUTO_PURGE_ENABLED,
            retention_days: DEFAULT_RETENTION_DAYS,
        }
    }
}

impl TrashSettings {
    /// 将保留天数钳制到合法范围 [MIN, MAX]。
    pub fn clamp_retention(days: i32) -> i32 {
        days.clamp(MIN_RETENTION_DAYS, MAX_RETENTION_DAYS)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_disabled_30_days() {
        let s = TrashSettings::default();
        assert!(!s.auto_purge_enabled);
        assert_eq!(s.retention_days, 30);
    }

    #[test]
    fn clamp_retention_keeps_valid() {
        assert_eq!(TrashSettings::clamp_retention(7), 7);
        assert_eq!(TrashSettings::clamp_retention(30), 30);
    }

    #[test]
    fn clamp_retention_clamps_below_min() {
        assert_eq!(TrashSettings::clamp_retention(0), MIN_RETENTION_DAYS);
        assert_eq!(TrashSettings::clamp_retention(-5), MIN_RETENTION_DAYS);
    }

    #[test]
    fn clamp_retention_clamps_above_max() {
        assert_eq!(TrashSettings::clamp_retention(366), MAX_RETENTION_DAYS);
        assert_eq!(TrashSettings::clamp_retention(i32::MAX), MAX_RETENTION_DAYS);
    }

    #[test]
    fn clamp_retention_boundary() {
        assert_eq!(TrashSettings::clamp_retention(MIN_RETENTION_DAYS), MIN_RETENTION_DAYS);
        assert_eq!(TrashSettings::clamp_retention(MAX_RETENTION_DAYS), MAX_RETENTION_DAYS);
    }
}
