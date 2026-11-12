//! 任务字段与进度：暂停、频率、预计耗时、未结算行动与批量进度。

use chrono::NaiveDate;
use ngy_min_action::model::{ActionStatus, DomainError, Frequency, TaskDraft, TaskStatus};
use ngy_min_action::store::Store;

fn date(year: i32, month: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, month, day).expect("测试用的日期必须合法")
}

fn open_store(tag: &str) -> Store {
    let dir =
        std::env::temp_dir().join(format!("ngy-min-action-test-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    Store::open(&dir.join("data.db")).expect("应能创建临时数据库")
}

fn draft(name: &str) -> TaskDraft {
    TaskDraft {
        name: name.to_owned(),
        frequency: Frequency::Daily,
        estimated_minutes: None,
        description: None,
    }
}

#[test]
fn task_open_actions_counts_unsettled_actions() {
    let store = open_store("task-open-actions");
    let goal = store.create_goal("学习英语", None).expect("创建目标");
    let category = store.create_category(goal, "口语").expect("创建分类");
    let task = store
        .create_task(category, draft("连读"))
        .expect("创建任务");

    assert_eq!(
        store.task_open_actions(task).expect("无行动"),
        0,
        "没有行动时应为 0"
    );

    let actions = store.generate_day(date(2026, 11, 2), 5).expect("生成行动");
    assert_eq!(
        store.task_open_actions(task).expect("一条待完成"),
        1,
        "一条 pending 行动应计为 1"
    );

    store
        .settle_action(actions[0].id, ActionStatus::Done)
        .expect("完成行动");
    assert_eq!(
        store.task_open_actions(task).expect("结算后"),
        0,
        "已完成行动不应计入"
    );

    store.delete_task(task).expect("删除任务");
    assert_eq!(
        store.task_open_actions(task).expect("删除后"),
        0,
        "已删除任务返回 0"
    );
}

#[test]
fn all_tasks_includes_paused_tasks_for_name_resolution() {
    let store = open_store("all-tasks");
    let goal = store.create_goal("学习英语", None).expect("创建目标");
    let category = store.create_category(goal, "口语").expect("创建分类");
    let task = store
        .create_task(category, draft("连读"))
        .expect("创建任务");
    store
        .set_task_status(task, TaskStatus::Paused)
        .expect("暂停任务");
    let names: Vec<String> = store
        .all_tasks()
        .expect("全部任务")
        .into_iter()
        .map(|item| item.name)
        .collect();
    assert_eq!(
        names,
        vec!["连读".to_string()],
        "暂停任务的名称仍必须可解析，否则今日清单会退化为任务#N"
    );
}

#[test]
fn tasks_are_stored_with_their_frequency() {
    let store = open_store("task-frequency");
    let goal = store.create_goal("学习英语", None).expect("创建目标");
    let category = store.create_category(goal, "口语").expect("创建分类");
    store
        .create_task(
            category,
            TaskDraft {
                name: "隔三天".to_string(),
                frequency: Frequency::EveryNDays(3),
                estimated_minutes: None,
                description: None,
            },
        )
        .expect("创建任务");
    store
        .create_task(
            category,
            TaskDraft {
                name: "每周两次".to_string(),
                frequency: Frequency::WeeklyN(2),
                estimated_minutes: None,
                description: None,
            },
        )
        .expect("创建任务");
    let tasks = store.tasks(category).expect("列出任务");
    assert_eq!(tasks[0].frequency, Frequency::EveryNDays(3));
    assert_eq!(tasks[1].frequency, Frequency::WeeklyN(2));
}

#[test]
fn progress_for_category_returns_progress_of_live_tasks_only() {
    let store = open_store("progress-batch");
    let goal = store.create_goal("学习英语", None).expect("创建目标");
    let category = store.create_category(goal, "口语").expect("创建分类");
    assert!(
        store
            .progress_for_category(category)
            .expect("空分类")
            .is_empty(),
        "空分类应返回空向量"
    );

    let task = store
        .create_task(category, draft("连读"))
        .expect("创建任务");
    let actions = store.generate_day(date(2026, 11, 2), 5).expect("生成行动");
    store
        .settle_action(actions[0].id, ActionStatus::Done)
        .expect("完成行动");

    let progress = store.progress_for_category(category).expect("读取进度");
    assert_eq!(progress.len(), 1);
    assert_eq!(progress[0].task_id, task);
    assert_eq!(progress[0].level, 1);

    store.delete_task(task).expect("删除任务");
    assert!(
        store
            .progress_for_category(category)
            .expect("读取进度")
            .is_empty(),
        "已删除任务的进度不应出现"
    );
}

#[test]
fn update_task_frequency_only_touches_the_frequency_columns() {
    let store = open_store("update-frequency");
    let goal = store.create_goal("学习英语", None).expect("创建目标");
    let category = store.create_category(goal, "口语").expect("创建分类");
    let task = store
        .create_task(category, draft("连读"))
        .expect("创建任务");

    store
        .update_task_frequency(task, Frequency::EveryNDays(3))
        .expect("改成每 3 天");

    let tasks = store.tasks(category).expect("列出任务");
    assert_eq!(tasks[0].frequency, Frequency::EveryNDays(3));
    assert_eq!(tasks[0].name, "连读", "名称不应变化");
    assert_eq!(tasks[0].status, TaskStatus::Active, "状态不应变化");
}

#[test]
fn update_task_frequency_rejects_deleted_task() {
    let store = open_store("update-frequency-deleted");
    let goal = store.create_goal("学习英语", None).expect("创建目标");
    let category = store.create_category(goal, "口语").expect("创建分类");
    let task = store
        .create_task(category, draft("连读"))
        .expect("创建任务");
    store.delete_task(task).expect("删除任务");

    let error = store
        .update_task_frequency(task, Frequency::Daily)
        .expect_err("已删除任务必须拒绝");
    assert!(
        matches!(error, DomainError::NotFound { .. }),
        "应返回 NotFound，实际为 {error:?}"
    );
}

#[test]
fn update_task_estimated_minutes_sets_and_clears_the_value() {
    let store = open_store("update-estimate");
    let goal = store.create_goal("学习英语", None).expect("创建目标");
    let category = store.create_category(goal, "口语").expect("创建分类");
    let task = store
        .create_task(category, draft("连读"))
        .expect("创建任务");

    store
        .update_task_estimated_minutes(task, Some(25))
        .expect("设置耗时");
    let tasks = store.tasks(category).expect("列出任务");
    assert_eq!(tasks[0].estimated_minutes, Some(25));
    assert_eq!(tasks[0].name, "连读", "名称不应变化");
    assert_eq!(tasks[0].frequency, Frequency::Daily, "频率不应变化");
    assert_eq!(tasks[0].status, TaskStatus::Active, "状态不应变化");

    store
        .update_task_estimated_minutes(task, None)
        .expect("清空耗时");
    let tasks = store.tasks(category).expect("列出任务");
    assert_eq!(tasks[0].estimated_minutes, None, "清空后应为未估时");
    assert_eq!(tasks[0].name, "连读", "清空耗时不应影响其它字段");
}

#[test]
fn update_task_estimated_minutes_rejects_deleted_task() {
    let store = open_store("update-estimate-deleted");
    let goal = store.create_goal("学习英语", None).expect("创建目标");
    let category = store.create_category(goal, "口语").expect("创建分类");
    let task = store
        .create_task(category, draft("连读"))
        .expect("创建任务");
    store.delete_task(task).expect("删除任务");

    let error = store
        .update_task_estimated_minutes(task, Some(10))
        .expect_err("已删除任务必须拒绝");
    assert!(
        matches!(error, DomainError::NotFound { .. }),
        "应返回 NotFound，实际为 {error:?}"
    );
}
