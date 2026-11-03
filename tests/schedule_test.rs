//! FR-008 / FR-010 ~ FR-015 每日生成与频率判定。

use std::collections::HashSet;

use chrono::NaiveDate;
use ngy_min_action::model::{
    Action, ActionKind, ActionStatus, CategoryId, Frequency, ReviewProgress, Task, TaskId,
    TaskStatus,
};
use ngy_min_action::schedule::{PlanInput, is_due, plan_day};

fn date(year: i32, month: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, month, day).expect("测试用的日期必须合法")
}

fn task(id: i64, frequency: Frequency, last: Option<NaiveDate>) -> Task {
    Task {
        id: TaskId(id),
        category_id: CategoryId(1),
        name: format!("任务{id}"),
        frequency,
        estimated_minutes: None,
        status: TaskStatus::Active,
        last_generated_on: last,
        created_at: "2026-11-01T00:00:00+00:00".to_owned(),
    }
}

fn review(task_id: i64, level: u8, due: NaiveDate) -> ReviewProgress {
    ReviewProgress {
        task_id: TaskId(task_id),
        level,
        next_review_on: Some(due),
        graduated: false,
    }
}

fn existing(task_id: i64, on_date: NaiveDate) -> Action {
    Action {
        id: ngy_min_action::model::ActionId(task_id),
        task_id: TaskId(task_id),
        on_date,
        kind: ActionKind::Regular,
        review_level: None,
        status: ActionStatus::Pending,
        seq: 1,
    }
}

#[test]
fn daily_task_is_due_after_one_day() {
    let today = date(2026, 11, 2);
    let item = task(1, Frequency::Daily, Some(date(2026, 11, 1)));
    assert!(is_due(&item, today));
}

#[test]
fn daily_task_is_not_due_twice_on_the_same_day() {
    let today = date(2026, 11, 2);
    let item = task(1, Frequency::Daily, Some(today));
    assert!(!is_due(&item, today));
}

#[test]
fn weekly_task_waits_for_its_interval() {
    let item = task(1, Frequency::WeeklyN(1), Some(date(2026, 11, 2)));
    assert!(!is_due(&item, date(2026, 11, 3)));
    assert!(!is_due(&item, date(2026, 11, 8)));
    assert!(is_due(&item, date(2026, 11, 9)));
}

#[test]
fn weekly_seven_times_degrades_to_daily() {
    let item = task(1, Frequency::WeeklyN(7), Some(date(2026, 11, 2)));
    assert!(is_due(&item, date(2026, 11, 3)));
}

#[test]
fn review_action_wins_over_regular_for_the_same_task() {
    let today = date(2026, 11, 2);
    let tasks = vec![task(1, Frequency::Daily, Some(date(2026, 11, 1)))];
    let progress = vec![review(1, 1, date(2026, 11, 2))];
    let plan = plan_day(&PlanInput {
        today,
        daily_cap: 5,
        tasks: &tasks,
        progress: &progress,
        existing: &[],
    });
    assert_eq!(plan.actions.len(), 1, "同一任务当天只上线一条行动");
    assert_eq!(plan.actions[0].kind, ActionKind::Review);
    assert_eq!(
        plan.actions[0].review_level,
        Some(1),
        "进度里的 level 就是当前这一级，界面上不应再加一"
    );
}

#[test]
fn tasks_with_an_existing_action_are_skipped() {
    let today = date(2026, 11, 2);
    let tasks = vec![task(1, Frequency::Daily, None)];
    let plan = plan_day(&PlanInput {
        today,
        daily_cap: 5,
        tasks: &tasks,
        progress: &[],
        existing: &[existing(1, today)],
    });
    assert!(plan.actions.is_empty(), "已有行动的任务不应重复生成");
}

#[test]
fn cap_truncates_and_reports_deferred_items() {
    let today = date(2026, 11, 2);
    let tasks: Vec<Task> = (1..=8).map(|id| task(id, Frequency::Daily, None)).collect();
    let plan = plan_day(&PlanInput {
        today,
        daily_cap: 5,
        tasks: &tasks,
        progress: &[],
        existing: &[],
    });
    assert_eq!(plan.actions.len(), 5);
    assert_eq!(plan.deferred_count(), 3);
}

#[test]
fn cap_counts_actions_already_on_the_day() {
    let today = date(2026, 11, 2);
    let tasks = vec![task(9, Frequency::Daily, None)];
    let occupied: Vec<Action> = (1..=5).map(|id| existing(id, today)).collect();
    let plan = plan_day(&PlanInput {
        today,
        daily_cap: 5,
        tasks: &tasks,
        progress: &[],
        existing: &occupied,
    });
    assert!(
        plan.actions.is_empty(),
        "当天已有行动占满上限时，新候选必须顺延而不是插队"
    );
    assert_eq!(plan.deferred_count(), 1);
}

#[test]
fn settled_actions_still_occupy_the_cap() {
    let today = date(2026, 11, 2);
    let tasks = vec![task(9, Frequency::Daily, None)];
    let mut occupied: Vec<Action> = (1..=5).map(|id| existing(id, today)).collect();
    for item in &mut occupied {
        item.status = ActionStatus::Done;
    }
    let plan = plan_day(&PlanInput {
        today,
        daily_cap: 5,
        tasks: &tasks,
        progress: &[],
        existing: &occupied,
    });
    assert!(
        plan.actions.is_empty(),
        "上限约束的是当天行动总数，已完成的行仍占名额"
    );
}

#[test]
fn review_actions_sort_before_regular_ones() {
    let today = date(2026, 11, 2);
    let tasks = vec![
        task(1, Frequency::Daily, None),
        task(2, Frequency::Daily, None),
    ];
    let progress = vec![review(2, 1, date(2026, 11, 2))];
    let plan = plan_day(&PlanInput {
        today,
        daily_cap: 5,
        tasks: &tasks,
        progress: &progress,
        existing: &[],
    });
    assert_eq!(plan.actions[0].task_id, TaskId(2));
    assert_eq!(plan.actions[0].kind, ActionKind::Review);
}

#[test]
fn plan_is_deterministic_for_the_same_input() {
    let today = date(2026, 11, 2);
    let tasks: Vec<Task> = (1..=8).map(|id| task(id, Frequency::Daily, None)).collect();
    let input = PlanInput {
        today,
        daily_cap: 3,
        tasks: &tasks,
        progress: &[],
        existing: &[],
    };
    assert_eq!(plan_day(&input), plan_day(&input));
}

#[test]
fn thirty_days_of_generation_has_no_duplicates_or_losses() {
    let mut tasks = vec![
        task(1, Frequency::Daily, None),
        task(2, Frequency::EveryNDays(3), None),
        task(3, Frequency::WeeklyN(2), None),
        task(4, Frequency::Daily, None),
    ];
    let mut seen: HashSet<(i64, String)> = HashSet::new();
    let mut deferred_total = 0_usize;

    for offset in 0..30_i64 {
        let today = date(2026, 11, 1) + chrono::Duration::days(offset);
        let plan = plan_day(&PlanInput {
            today,
            daily_cap: 3,
            tasks: &tasks,
            progress: &[],
            existing: &[],
        });
        deferred_total += plan.deferred_count();
        for planned in &plan.actions {
            let key = (planned.task_id.0, today.to_string());
            assert!(seen.insert(key), "同一天同一任务不应重复生成");
        }
        for planned in &plan.actions {
            if let Some(item) = tasks.iter_mut().find(|item| item.id == planned.task_id) {
                item.last_generated_on = Some(today);
            }
        }
    }

    assert!(!seen.is_empty(), "三十天内至少应生成过行动");
    assert!(deferred_total > 0, "上限低于候选数时应有顺延");
}
