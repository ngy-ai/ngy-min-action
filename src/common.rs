//! 跨模块共享的常量、配置项与日志辅助。

mod constants;

pub use constants::{
    APP_DATA_DIR, DAILY_CAP_DEFAULT, DAILY_CAP_MAX, DAILY_CAP_MIN, DEFERRED_RETENTION_DAYS,
    ENV_PREFIX, HISTORY_PAGE_SIZE, IMAGE_MAX_BYTES, IMAGE_MAX_DIMENSION, MAX_EVERY_N_DAYS,
    MAX_TASK_DESCRIPTION, MAX_TASK_NAME, MAX_WEEKLY_N, MEDIA_MAX_BYTES, NOTE_MAX_CHARS,
    REVIEW_INTERVALS_DAYS, TASKS_PER_DAY_DEFAULT, TICK_INTERVAL_SECS,
};

/// 把操作耗时转成日志字段 `elapsed_ms` 的值，超出 `u64` 范围时饱和而不是回绕。
///
/// 各层统一用它填 `elapsed_ms`，避免同一字段在不同模块出现不同单位或精度。
#[must_use]
pub fn elapsed_ms(started: std::time::Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX)
}
