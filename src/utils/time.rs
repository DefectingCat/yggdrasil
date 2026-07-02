//! 跨平台时间/睡眠工具。
//!
//! 根据目标架构分别实现：
//! - `wasm32`：通过 `js_sys` 调用 JavaScript 的 `setTimeout` / `Date.now()`。
//! - 其他平台：使用 `tokio::time::sleep` / `chrono::Utc`。
//!
//! 相对时间分档（`relative_label_from_millis` / `format_relative_time_iso`）由
//! 前端待审核评论展示与服务端评论预渲染共享，保证两端口径一致。

use chrono::DateTime;

/// 异步睡眠指定毫秒数。
#[cfg(target_arch = "wasm32")]
pub async fn sleep_ms(ms: u32) {
    use wasm_bindgen::JsCast;
    let js_code = format!("new Promise(r => setTimeout(r, {}))", ms);
    if let Ok(promise_val) = js_sys::eval(&js_code) {
        if let Ok(promise) = promise_val.dyn_into::<js_sys::Promise>() {
            let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
        }
    }
}

/// 异步睡眠指定毫秒数（原生 tokio 版本）。
#[cfg(not(target_arch = "wasm32"))]
pub async fn sleep_ms(ms: u32) {
    tokio::time::sleep(std::time::Duration::from_millis(ms as u64)).await;
}

/// 获取当前时间戳（毫秒）。
///
/// WASM 端使用 `js_sys::Date::now()`，服务端回退到 `chrono::Utc`。
pub fn now_millis() -> i64 {
    #[cfg(target_arch = "wasm32")]
    {
        js_sys::Date::now() as i64
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        chrono::Utc::now().timestamp_millis()
    }
}

/// 相对时间分档：根据"距现在的毫秒数"返回 (相对文本, 绝对日期 YYYY-MM-DD)。
///
/// 分档规则与服务端 `format_relative_time` 完全一致，前端在展示待审核评论时复用，
/// 保证两类评论的时间展示口径统一。返回绝对日期用于 `title` 悬浮提示。
///
/// - `delta_millis`：目标时间与"现在"的差值（毫秒）。正值表示过去，负值表示未来（兜底按刚刚处理）。
/// - `created_iso`：评论的 RFC3339 创建时间，用于兜底生成绝对日期。
pub fn relative_label_from_millis(delta_millis: i64, created_iso: &str) -> (String, String) {
    let seconds = delta_millis / 1000;

    let label = if seconds < 60 {
        "刚刚".to_string()
    } else {
        let minutes = seconds / 60;
        if minutes < 60 {
            format!("{} 分钟前", minutes)
        } else {
            let hours = minutes / 60;
            if hours < 24 {
                format!("{} 小时前", hours)
            } else {
                let days = hours / 24;
                if days < 30 {
                    format!("{} 天前", days)
                } else {
                    // 超过 30 天直接显示日期，下方 absolute 复用
                    String::new()
                }
            }
        }
    };

    // 绝对日期：优先解析 ISO；解析失败时退化为空串，避免组件报错。
    let absolute = DateTime::parse_from_rfc3339(created_iso)
        .map(|dt| dt.format("%Y-%m-%d").to_string())
        .unwrap_or_default();

    let label = if label.is_empty() {
        absolute.clone()
    } else {
        label
    };
    (label, absolute)
}

/// 前端友好的相对时间格式化：返回相对文本，用于展示待审核评论的创建时间。
///
/// 这是 `relative_label_from_millis` 的薄封装，仅返回相对文本。
pub fn format_relative_time_iso(created_iso: &str) -> String {
    // 解析失败时退化为 "刚刚"，避免组件崩溃。
    let Ok(dt) = DateTime::parse_from_rfc3339(created_iso) else {
        return "刚刚".to_string();
    };
    let delta_millis = now_millis() - dt.timestamp_millis();
    relative_label_from_millis(delta_millis, created_iso).0
}

#[cfg(test)]
mod tests {
    use super::*;

    const ISO: &str = "2026-06-22T05:43:57.565+00:00";

    #[test]
    fn relative_label_just_now_under_60s() {
        let (label, _) = relative_label_from_millis(0, ISO);
        assert_eq!(label, "刚刚");
        let (label, _) = relative_label_from_millis(59_999, ISO);
        assert_eq!(label, "刚刚");
    }

    #[test]
    fn relative_label_minutes() {
        let (label, _) = relative_label_from_millis(60_000, ISO);
        assert_eq!(label, "1 分钟前");
        let (label, _) = relative_label_from_millis(5 * 60_000, ISO);
        assert_eq!(label, "5 分钟前");
        let (label, _) = relative_label_from_millis(59 * 60_000, ISO);
        assert_eq!(label, "59 分钟前");
    }

    #[test]
    fn relative_label_hours() {
        let (label, _) = relative_label_from_millis(60 * 60_000, ISO);
        assert_eq!(label, "1 小时前");
        let (label, _) = relative_label_from_millis(3 * 3_600_000, ISO);
        assert_eq!(label, "3 小时前");
        let (label, _) = relative_label_from_millis(23 * 3_600_000, ISO);
        assert_eq!(label, "23 小时前");
    }

    #[test]
    fn relative_label_days() {
        let (label, _) = relative_label_from_millis(24 * 3_600_000, ISO);
        assert_eq!(label, "1 天前");
        let (label, _) = relative_label_from_millis(7 * 24 * 3_600_000, ISO);
        assert_eq!(label, "7 天前");
        let (label, _) = relative_label_from_millis(29 * 24 * 3_600_000, ISO);
        assert_eq!(label, "29 天前");
    }

    #[test]
    fn relative_label_falls_back_to_date_over_30_days() {
        let (label, absolute) = relative_label_from_millis(60 * 24 * 3_600_000, ISO);
        assert_eq!(label, "2026-06-22");
        assert_eq!(absolute, "2026-06-22");
    }

    #[test]
    fn relative_label_future_falls_back_to_just_now() {
        // 未来时间差为负，秒数 < 60，归为"刚刚"。
        let (label, _) = relative_label_from_millis(-5_000, ISO);
        assert_eq!(label, "刚刚");
    }

    #[test]
    fn relative_label_invalid_iso_still_returns_absolute_empty() {
        // 无法解析时 absolute 为空，但分档逻辑仍按 delta 决定。
        let (label, absolute) = relative_label_from_millis(0, "not-a-date");
        assert_eq!(label, "刚刚");
        assert_eq!(absolute, "");
    }

    #[test]
    fn format_relative_time_iso_invalid_iso_falls_back() {
        // 解析失败退化为"刚刚"，不 panic。
        assert_eq!(format_relative_time_iso("not-a-date"), "刚刚");
    }
}
