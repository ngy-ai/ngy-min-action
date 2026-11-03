//! 遗忘曲线推进。

use chrono::NaiveDate;

use crate::model::ReviewProgress;
use crate::schedule::ScheduleError;

/// 完成一次行动后推进复习进度。
///
/// - 未开始复习（`level = 0`）完成常规行动后进入第 1 级；
/// - 完成第 k 级复习后进入第 k+1 级，下次复习日为 `completed_on + intervals[k]`；
/// - 走完序列（`level` 达到 `intervals.len()`）后置 `graduated`，不再自动复习。
///
/// # Errors
///
/// `level` 大于 `intervals.len()` 时返回 [`ScheduleError::InvalidLevel`]。
pub fn advance_review(
    progress: &ReviewProgress,
    completed_on: NaiveDate,
    intervals: &[u16],
) -> Result<ReviewProgress, ScheduleError> {
    let max_level = u8::try_from(intervals.len()).unwrap_or(u8::MAX);
    if progress.level > max_level {
        return Err(ScheduleError::InvalidLevel {
            level: progress.level,
            max: max_level,
        });
    }

    let next_level = progress.level.saturating_add(1);
    if usize::from(next_level) > intervals.len() {
        return Ok(ReviewProgress {
            task_id: progress.task_id,
            level: max_level,
            next_review_on: None,
            graduated: true,
        });
    }

    let offset = i64::from(intervals[usize::from(next_level) - 1]);
    Ok(ReviewProgress {
        task_id: progress.task_id,
        level: next_level,
        next_review_on: Some(completed_on + chrono::Duration::days(offset)),
        graduated: false,
    })
}
