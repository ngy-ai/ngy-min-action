//! 重置复习进度对话框行为无头验收（T016 / 补 015 计划遗漏的 `reset_dialog.rs`）。
//!
//! 对话框确认即调用 `Store::reset_review_progress`；这里验证该写回把复习进度归零，
//! 以及任务不存在时返回 `NotFound`（对话框据此保留并提示「不存在」）。

use chrono::NaiveDate;

use crate::model::{ActionStatus, DomainError, Frequency, TaskDraft, TaskId};
use crate::store::Store;

/// 在临时目录打开一个空库，便于测试构造数据。
fn temp_store(tag: &str) -> (std::path::PathBuf, Store) {
    let dir = std::env::temp_dir().join(format!("ngy-min-action-015-reset-{tag}"));
    let _ = std::fs::remove_dir_all(&dir);
    let store = Store::open(&dir.join("data.db")).expect("应能创建临时数据库");
    (dir, store)
}

/// 把复习进度归零：结算一次得到 level>=1，再重置应回到 0 且未毕业。
#[test]
fn resetting_review_progress_clears_level() {
    let (_dir, store) = temp_store("clear");
    let goal = store.create_goal("AI", None).expect("目标");
    let category = store.create_category(goal, "口语").expect("分类");
    let draft = TaskDraft {
        name: "连读".to_owned(),
        frequency: Frequency::Daily,
        estimated_minutes: None,
        description: None,
    };
    let task = store.create_task(category, draft).expect("任务");

    let today = NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();
    store.generate_day(today, 5).expect("生成今日行动");
    let actions = store.actions_on(today).expect("读取今日行动");
    store
        .settle_action(actions[0].id, ActionStatus::Done)
        .expect("完成第一次");

    let before = store.review_progress(task).expect("读取进度").level;
    assert!(before >= 1, "前置：结算后进度应大于 0，实际 {before}");

    store.reset_review_progress(task).expect("重置应成功");
    let after = store.review_progress(task).expect("读取进度");
    assert_eq!(after.level, 0, "重置后应归零");
    assert!(!after.graduated, "重置后应取消毕业");
}

/// 重置不存在的任务应返回 `NotFound`（对话框据此保留并提示「不存在」）。
#[test]
fn resetting_missing_task_reports_not_found() {
    let (_dir, store) = temp_store("missing");
    let result = store.reset_review_progress(TaskId(9999));
    assert!(
        matches!(result, Err(DomainError::NotFound { .. })),
        "不存在的任务应返回 NotFound"
    );
}
