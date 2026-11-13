use crate::model::TaskId;
use chrono::NaiveDate;

/// 复习进度：任务在遗忘曲线上的位置。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewProgress {
    /// 所属任务。
    pub task_id: TaskId,
    /// 当前级别，0 表示未开始复习。
    pub level: u8,
    /// 下次复习日。
    pub next_review_on: Option<NaiveDate>,
    /// 是否已走完序列。
    pub graduated: bool,
}

impl ReviewProgress {
    /// 未开始复习的初始进度。
    #[must_use]
    pub fn fresh(task_id: TaskId) -> Self {
        ReviewProgress {
            task_id,
            level: 0,
            next_review_on: None,
            graduated: false,
        }
    }
}

/// 提示记录。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReminderState {
    /// 最近一次弹出提示的本地日期。
    pub last_notified_on: Option<NaiveDate>,
    /// 最近一次查看今日清单的本地日期。
    pub last_viewed_on: Option<NaiveDate>,
}

/// 删除操作的影响范围，供确认对话框展示。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DeletionImpact {
    /// 将被一并删除的分类数量。
    pub categories: usize,
    /// 将被一并删除的任务数量。
    pub tasks: usize,
    /// 将被置为过期的未完成行动数量。
    pub pending_actions: usize,
}
