//! 每日生成：候选收集、排序、双重上限截断。

use crate::model::{ActionKind, ReviewProgress, Task, TaskId};
use crate::schedule::{PlanInput, PlanOutput, PlannedAction, is_due};

/// 计算某一天的完整生成方案。
///
/// 保证：
/// - 同一输入多次调用结果完全一致；
/// - 同一任务在输出中最多出现一次，复习优先于常规；
/// - 输出条数不超过 `daily_cap`，超出部分计入 `deferred`；
/// - 当天已有行动的任务不会再次出现。
#[must_use]
pub fn plan_day(input: &PlanInput<'_>) -> PlanOutput {
    let mut candidates: Vec<Candidate> = Vec::new();

    for task in input.tasks {
        if input
            .existing
            .iter()
            .any(|action| action.task_id == task.id)
        {
            continue;
        }
        if let Some(candidate) = candidate_for(task, input.progress, input.today) {
            candidates.push(candidate);
        }
    }

    candidates.sort_by(|left, right| {
        left.kind_rank()
            .cmp(&right.kind_rank())
            .then_with(|| left.due.cmp(&right.due))
            .then_with(|| left.task_id.cmp(&right.task_id))
    });

    let cap = usize::from(input.daily_cap.max(1));
    let mut actions = Vec::new();
    let mut deferred = Vec::new();
    for (index, candidate) in candidates.into_iter().enumerate() {
        let planned = PlannedAction {
            task_id: candidate.task_id,
            kind: candidate.kind,
            review_level: candidate.review_level,
        };
        if index < cap {
            actions.push(planned);
        } else {
            deferred.push(planned);
        }
    }

    PlanOutput { actions, deferred }
}

#[derive(Debug)]
struct Candidate {
    task_id: TaskId,
    kind: ActionKind,
    review_level: Option<u8>,
    due: Option<chrono::NaiveDate>,
}

impl Candidate {
    fn kind_rank(&self) -> u8 {
        match self.kind {
            ActionKind::Review => 0,
            ActionKind::Regular => 1,
        }
    }
}

fn candidate_for(
    task: &Task,
    progress: &[ReviewProgress],
    today: chrono::NaiveDate,
) -> Option<Candidate> {
    let snapshot = progress.iter().find(|item| item.task_id == task.id);

    if let Some(snapshot) = snapshot
        && !snapshot.graduated
        && let Some(due) = snapshot.next_review_on
        && due <= today
    {
        return Some(Candidate {
            task_id: task.id,
            kind: ActionKind::Review,
            review_level: Some(snapshot.level.saturating_add(1)),
            due: Some(due),
        });
    }

    if is_due(task, today) {
        return Some(Candidate {
            task_id: task.id,
            kind: ActionKind::Regular,
            review_level: None,
            due: None,
        });
    }

    None
}
