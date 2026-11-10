//! 行动行的共享文案：任务、分类、复习级别与状态。
//!
//! 今日与历史两个页签必须用同一套描述，避免同一条待办在两处显示不同的名字。

use crate::app::MinActionApp;
use crate::model::{Action, ActionKind, ActionStatus};

impl MinActionApp {
    /// 一条行动在列表中的描述。
    pub(crate) fn describe_action(&self, action: &Action) -> String {
        let task = self.all_tasks.iter().find(|item| item.id == action.task_id);
        let task_name = task.map_or_else(
            || format!("任务#{}", action.task_id.0),
            |item| item.name.clone(),
        );
        let category_name = task
            .and_then(|item| {
                self.all_categories
                    .iter()
                    .find(|category| category.id == item.category_id)
            })
            .map_or_else(String::new, |category| category.name.clone());

        match action.kind {
            ActionKind::Review => format!(
                "{category_name} · {task_name}（第 {} 级复习）",
                action.review_level.unwrap_or(1)
            ),
            ActionKind::Regular => format!("{category_name} · {task_name}"),
        }
    }
}

/// 状态的界面文案。
pub(crate) fn status_text(status: ActionStatus) -> &'static str {
    match status {
        ActionStatus::Pending => "待完成",
        ActionStatus::Done => "已完成",
        ActionStatus::Skipped => "已跳过",
        ActionStatus::Deferred => "已顺延",
        ActionStatus::Expired => "已过期",
    }
}
