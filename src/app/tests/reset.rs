//! 006：重置复习进度的用例。

use super::*;

use crate::clock::Clock;
use crate::model::{ActionStatus, TaskStatus};

#[test]
fn resetting_review_progress_requires_confirmation() {
    let (mut app, clock) = open_app_with_clock("reset-review");
    let category = seed_category(&mut app, "学习英语", "口语");
    let task = app
        .store
        .create_task(category, draft("连读"))
        .expect("创建任务");
    app.reload().expect("刷新界面数据");

    let first = app.actions[0].id;
    app.store
        .settle_action(first, ActionStatus::Done)
        .expect("完成第一次");
    clock.advance_days(1);
    app.today = clock.today();
    app.reload().expect("跨日刷新");
    let second = app.actions[0].id;
    app.store
        .settle_action(second, ActionStatus::Done)
        .expect("完成第二次");
    app.reload().expect("刷新界面数据");

    let level = |app: &MinActionApp| {
        app.task_progress
            .iter()
            .find(|item| item.task_id == task)
            .map(|item| item.level)
    };
    assert_eq!(level(&app), Some(2), "前置条件：进度应为第 2 级");

    app.request_reset(task);
    assert_eq!(app.reset_target, Some(task), "应打开确认框");
    assert_eq!(level(&app), Some(2), "确认前不应改动数据");

    app.cancel_reset();
    assert!(app.reset_target.is_none(), "取消后应关闭");
    assert_eq!(level(&app), Some(2), "取消后进度不变");

    app.request_reset(task);
    app.confirm_reset();
    assert!(app.reset_target.is_none(), "确认后应关闭");
    assert_eq!(level(&app), Some(0), "确认后应归零");
    assert_eq!(app.tasks[0].status, TaskStatus::Active, "任务状态不变");
}

#[test]
fn resetting_a_missing_task_keeps_the_dialog_open() {
    let mut app = open_app("reset-review-missing");
    let ghost = TaskId(9999);

    app.request_reset(ghost);
    assert_eq!(app.reset_target, Some(ghost), "应打开确认框");

    app.confirm_reset();
    assert_eq!(app.reset_target, Some(ghost), "失败时弹窗应保留");
    let message = app.error.as_deref().unwrap_or_default();
    assert!(message.contains("不存在"), "应说明任务不存在：{message}");
}
