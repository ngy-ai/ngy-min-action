//! `plan_day` 在规划规模下的微基准。
//!
//! 规模与 `specs/001-min-action-core/plan.md` 的假设一致：200 个任务、90 天滚动。
//! 运行 `cargo bench --bench plan_day`。

use chrono::{Duration, NaiveDate};
use criterion::{Criterion, criterion_group, criterion_main};
use ngy_min_action::model::{
    Action, ActionId, ActionKind, ActionStatus, CategoryId, Frequency, ReviewProgress, Task,
    TaskId, TaskStatus,
};
use ngy_min_action::schedule::{PlanInput, plan_day};

/// 规划规模下的任务数。
const TASK_COUNT: i64 = 200;

/// 基准使用的固定日期，避免结果随运行日漂移。
fn today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 11, 2).expect("固定日期必须合法")
}

/// 构造 200 个任务：三分之一常规到期、三分之一复习到期、三分之一当天已生成。
fn tasks() -> Vec<Task> {
    (1..=TASK_COUNT)
        .map(|id| {
            let last_generated_on = match (id - 1) % 3 {
                0 => Some(today() - Duration::days(2)),
                1 => Some(today()),
                _ => Some(today() - Duration::days(2)),
            };
            Task {
                id: TaskId(id),
                category_id: CategoryId(1),
                name: format!("任务 {id}"),
                frequency: Frequency::Daily,
                estimated_minutes: Some(15),
                status: TaskStatus::Active,
                last_generated_on,
                created_at: "2026-10-01T00:00:00+08:00".to_owned(),
            }
        })
        .collect()
}

/// 复习到期的那三分之一任务各有一条已到期的进度。
fn progress() -> Vec<ReviewProgress> {
    (1..=TASK_COUNT)
        .filter(|id| (id - 1) % 3 == 2)
        .map(|id| ReviewProgress {
            task_id: TaskId(id),
            level: 2,
            next_review_on: Some(today() - Duration::days(1)),
            graduated: false,
        })
        .collect()
}

/// 当天已有行动：模拟同一批次重跑生成。
fn existing_actions() -> Vec<Action> {
    (1..=5)
        .map(|id| Action {
            id: ActionId(id),
            task_id: TaskId(id),
            on_date: today(),
            kind: ActionKind::Regular,
            review_level: None,
            status: ActionStatus::Pending,
            seq: u32::try_from(id - 1).expect("序号非负"),
        })
        .collect()
}

fn bench_plan_day(criterion: &mut Criterion) {
    let tasks = tasks();
    let progress = progress();
    let existing = existing_actions();

    let mut group = criterion.benchmark_group("plan_day");

    group.bench_function("200_tasks_cap5_first_run", |bencher| {
        let input = PlanInput {
            today: today(),
            daily_cap: 5,
            tasks: &tasks,
            progress: &progress,
            existing: &[],
        };
        bencher.iter(|| std::hint::black_box(plan_day(std::hint::black_box(&input))));
    });

    group.bench_function("200_tasks_cap20_first_run", |bencher| {
        let input = PlanInput {
            today: today(),
            daily_cap: 20,
            tasks: &tasks,
            progress: &progress,
            existing: &[],
        };
        bencher.iter(|| std::hint::black_box(plan_day(std::hint::black_box(&input))));
    });

    group.bench_function("200_tasks_cap5_rerun", |bencher| {
        let input = PlanInput {
            today: today(),
            daily_cap: 5,
            tasks: &tasks,
            progress: &progress,
            existing: &existing,
        };
        bencher.iter(|| std::hint::black_box(plan_day(std::hint::black_box(&input))));
    });

    group.finish();
}

criterion_group!(benches, bench_plan_day);
criterion_main!(benches);
