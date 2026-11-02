//! 每日生成与复习调度。
//!
//! 本模块全部是纯函数：输入是当天的任务、复习进度与已有行动，输出是应当写入的方案。
//! 不碰数据库、不读系统时间，因此同一输入的结果恒定，可直接写基准与单测。

mod frequency;
mod generate;
mod review;

use chrono::NaiveDate;

use crate::model::{Action, ActionKind, ReviewProgress, Task, TaskId};

pub use frequency::is_due;
pub use generate::plan_day;
pub use review::advance_review;

/// 调度失败的类别。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScheduleError {
    /// 复习级别超出遗忘曲线序列。
    InvalidLevel { level: u8, max: u8 },
}

impl std::fmt::Display for ScheduleError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScheduleError::InvalidLevel { level, max } => write!(
                formatter,
                "复习级别 {level} 超出序列上限 {max}，请把该任务的复习进度重置后重试"
            ),
        }
    }
}

impl std::error::Error for ScheduleError {}

/// 生成方案的输入。
#[derive(Debug)]
pub struct PlanInput<'a> {
    /// 目标日期。
    pub today: NaiveDate,
    /// 每日行动上限。
    pub daily_cap: u8,
    /// 参与竞争的候选任务。
    pub tasks: &'a [Task],
    /// 候选任务的复习进度。
    pub progress: &'a [ReviewProgress],
    /// 当天已经存在的行动，用于幂等。
    pub existing: &'a [Action],
}

/// 单条待写入的行动。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedAction {
    /// 所属任务。
    pub task_id: TaskId,
    /// 类型。
    pub kind: ActionKind,
    /// 复习级别，仅复习行动有值。
    pub review_level: Option<u8>,
}

/// 生成方案的结果。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PlanOutput {
    /// 当天应当写入的行动，顺序即展示顺序。
    pub actions: Vec<PlannedAction>,
    /// 因上限被顺延的行动，落库为 `deferred` 记录。
    pub deferred: Vec<PlannedAction>,
}

impl PlanOutput {
    /// 被顺延的条数，供状态条展示。
    #[must_use]
    pub fn deferred_count(&self) -> usize {
        self.deferred.len()
    }
}
