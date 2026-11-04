//! 007：任务预计耗时的用例。

use super::*;

use crate::app::FrequencyMode;
use crate::model::TaskStatus;

#[test]
fn editing_the_estimate_updates_only_that_field() {
    let mut app = open_app("estimate-edit");
    let category = seed_category(&mut app, "学习英语", "口语");
    let task = app
        .store
        .create_task(category, draft("连读"))
        .expect("创建任务");
    app.reload().expect("刷新界面数据");
    let actions_before = app.actions.len();
    let progress_before = app.task_progress.len();

    app.open_estimate_dialog(task, None);
    app.draft_estimate = "25".to_owned();
    app.submit_estimate();

    assert!(app.estimate_target.is_none(), "成功后应关闭弹窗");
    assert_eq!(app.tasks[0].estimated_minutes, Some(25));
    assert_eq!(app.tasks[0].frequency, Frequency::Daily, "频率不应变化");
    assert_eq!(app.tasks[0].status, TaskStatus::Active, "状态不应变化");
    assert_eq!(app.task_progress.len(), progress_before, "复习进度不应变化");
    assert_eq!(app.actions.len(), actions_before, "行动不应变化");

    app.open_estimate_dialog(task, Some(25));
    app.draft_estimate = "0".to_owned();
    app.submit_estimate();
    assert_eq!(app.tasks[0].estimated_minutes, None, "0 应视为清空");
}

#[test]
fn invalid_estimate_keeps_the_dialog_open() {
    let mut app = open_app("estimate-invalid");
    let category = seed_category(&mut app, "学习英语", "口语");
    let task = app
        .store
        .create_task(category, draft("连读"))
        .expect("创建任务");
    app.reload().expect("刷新界面数据");

    app.open_estimate_dialog(task, None);
    app.draft_estimate = "481".to_owned();
    app.submit_estimate();

    assert_eq!(app.estimate_target, Some(task), "失败时弹窗应保留");
    assert_eq!(app.draft_estimate, "481", "失败时输入应保留");
    assert!(
        app.error
            .as_deref()
            .unwrap_or_default()
            .contains("预计耗时"),
        "应指出是预计耗时"
    );
    assert_eq!(app.tasks[0].estimated_minutes, None, "数据不应变化");
}

#[test]
fn creating_a_task_with_an_estimate_keeps_it() {
    let mut app = open_app("estimate-create");
    let category = seed_category(&mut app, "学习英语", "口语");
    app.selected_category = Some(category);
    app.refresh_scope();

    app.new_task = "连读".to_owned();
    app.new_task_mode = FrequencyMode::EveryNDays;
    app.new_task_interval = "3".to_owned();
    app.new_task_estimate = "15".to_owned();
    app.submit_new_task();

    assert_eq!(app.tasks.len(), 1);
    assert_eq!(app.tasks[0].estimated_minutes, Some(15));
    assert_eq!(app.tasks[0].frequency, Frequency::EveryNDays(3));
    assert!(app.new_task.is_empty(), "成功后应清空名称");
    assert!(app.new_task_estimate.is_empty(), "成功后应清空耗时输入");
}

#[test]
fn creating_a_task_with_an_invalid_estimate_is_rejected() {
    let mut app = open_app("estimate-create-invalid");
    let category = seed_category(&mut app, "学习英语", "口语");
    app.selected_category = Some(category);
    app.refresh_scope();

    app.new_task = "连读".to_owned();
    app.new_task_estimate = "481".to_owned();
    app.submit_new_task();

    assert!(app.tasks.is_empty(), "不应创建任务");
    assert_eq!(app.new_task_estimate, "481", "输入应保留");
    assert!(
        app.error
            .as_deref()
            .unwrap_or_default()
            .contains("预计耗时"),
        "应指出是预计耗时"
    );
}
