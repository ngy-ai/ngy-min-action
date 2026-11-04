//! 复习统计的领域类型：窗口内的行动分项与一次回顾的结果。

/// 窗口内已结算行动的分项计数。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ActionTally {
    /// 已完成的行动数。
    pub done: usize,
    /// 已跳过的行动数。
    pub skipped: usize,
    /// 已过期的行动数。
    pub expired: usize,
}

impl ActionTally {
    /// 已结算行动的合计；为零表示窗口内没有可统计的行动。
    #[must_use]
    pub fn settled(&self) -> usize {
        self.done + self.skipped + self.expired
    }

    /// 完成率百分比（四舍五入）；已结算为零时返回 `None`。
    #[must_use]
    pub fn completion_percent(&self) -> Option<u32> {
        let settled = self.settled();
        if settled == 0 {
            return None;
        }
        let percent = (self.done * 100 + settled / 2) / settled;
        u32::try_from(percent).ok()
    }
}

/// 一次复习统计的结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewStats {
    /// 统计窗口天数，含今天。
    pub window_days: u32,
    /// 窗口内的行动分项。
    pub tally: ActionTally,
    /// 当前连续达成天数。
    pub current_streak: u32,
    /// 历史最长连续达成天数。
    pub longest_streak: u32,
    /// 未开始复习的任务数。
    pub untouched_tasks: usize,
    /// 复习中的任务数。
    pub learning_tasks: usize,
    /// 已巩固的任务数。
    pub graduated_tasks: usize,
}
