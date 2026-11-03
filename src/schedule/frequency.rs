//! 频率判定：某个任务在目标日期是否需要生成常规行动。

use chrono::NaiveDate;

use crate::model::Task;

/// 判断任务在 `today` 是否需要生成常规行动。
///
/// 判定只依赖频率间隔与 `last_generated_on`，因此对同一输入恒定。
/// 任务是否暂停由调用方先行过滤，本函数不感知状态。
#[must_use]
pub fn is_due(task: &Task, today: NaiveDate) -> bool {
    let Some(last) = task.last_generated_on else {
        return true;
    };
    let elapsed = (today - last).num_days();
    elapsed > 0 && elapsed >= task.frequency.interval_days()
}
