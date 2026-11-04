//! 005：任务频率的用例，含跨日推进。

use super::*;

use crate::app::FrequencyMode;
use crate::clock::Clock;
use crate::model::TaskStatus;

#[test]
fn editing_the_frequency_updates_only_the_task_row() {
    let mut app = open_app("frequency-edit");
    let category = seed_category(&mut app, "学习英语", "口语");
    let task = app
        .store
        .create_task(category, draft("连读"))
        .expect("创建任务");
    app.reload().expect("刷新界面数据");
    let actions_before = app.actions.len();
    let progress_before = app.task_progress.len();

    app.open_frequency_dialog(task, Frequency::Daily);
    assert_eq!(
        app.draft_frequency_mode,
        FrequencyMode::Daily,
        "打开时应预选当前模式"
    );
    app.draft_frequency_mode = FrequencyMode::EveryNDays;
    app.draft_frequency_interval = "3".to_owned();
    app.submit_frequency();

    assert!(app.frequency_target.is_none(), "成功后应关闭弹窗");
    assert_eq!(
        app.tasks[0].frequency,
        Frequency::EveryNDays(3),
        "任务行应显示新频率"
    );
    assert_eq!(app.tasks[0].name, "连读", "名称不应变化");
    assert_eq!(app.tasks[0].status, TaskStatus::Active, "状态不应变化");
    assert_eq!(app.task_progress.len(), progress_before, "复习进度不应变化");
    assert_eq!(app.actions.len(), actions_before, "已生成的行动不应变化");

    // 再保存一次相同的值，不应触发任何错误。
    app.open_frequency_dialog(task, Frequency::EveryNDays(3));
    app.submit_frequency();
    assert!(app.frequency_target.is_none(), "改成当前值也应成功");
    assert!(app.error.is_none(), "改成当前值不应报错");
}

#[test]
fn invalid_frequency_parameters_keep_the_dialog_open() {
    let mut app = open_app("frequency-invalid");
    let category = seed_category(&mut app, "学习英语", "口语");
    let task = app
        .store
        .create_task(category, draft("连读"))
        .expect("创建任务");
    app.reload().expect("刷新界面数据");

    app.open_frequency_dialog(task, Frequency::Daily);
    app.draft_frequency_mode = FrequencyMode::EveryNDays;
    app.draft_frequency_interval = "0".to_owned();
    app.submit_frequency();

    assert_eq!(app.frequency_target, Some(task), "失败时弹窗应保留");
    assert_eq!(app.draft_frequency_interval, "0", "失败时输入应保留");
    let message = app.error.as_deref().unwrap_or_default();
    assert!(message.contains("频率间隔"), "应指出是频率间隔：{message}");
    assert_eq!(
        app.tasks[0].frequency,
        Frequency::Daily,
        "被拒后数据不应变化"
    );
}

#[test]
fn paused_tasks_accept_frequency_changes() {
    let mut app = open_app("frequency-paused");
    let category = seed_category(&mut app, "学习英语", "口语");
    let task = app
        .store
        .create_task(category, draft("连读"))
        .expect("创建任务");
    app.store
        .set_task_status(task, TaskStatus::Paused)
        .expect("暂停任务");
    app.reload().expect("刷新界面数据");

    app.open_frequency_dialog(task, Frequency::Daily);
    app.draft_frequency_mode = FrequencyMode::WeeklyN;
    app.draft_frequency_interval = "3".to_owned();
    app.submit_frequency();

    assert_eq!(app.tasks[0].frequency, Frequency::WeeklyN(3));
    assert_eq!(app.tasks[0].status, TaskStatus::Paused, "应仍为暂停");
}

#[test]
fn raising_the_frequency_takes_effect_on_the_next_day() {
    let (mut app, clock) = open_app_with_clock("frequency-cross-day");
    let category = seed_category(&mut app, "学习英语", "口语");
    let task = app
        .store
        .create_task(category, draft("连读"))
        .expect("创建任务");
    app.reload().expect("刷新界面数据");
    assert_eq!(app.actions.len(), 1, "每天任务开局就有一条行动");

    app.open_frequency_dialog(task, Frequency::WeeklyN(1));
    assert_eq!(
        app.draft_frequency_mode,
        FrequencyMode::WeeklyN,
        "应预选当前模式"
    );
    assert_eq!(app.draft_frequency_interval, "1", "应预填当前参数");
    app.submit_frequency();
    assert_eq!(app.tasks[0].frequency, Frequency::WeeklyN(1));
    assert_eq!(app.actions.len(), 1, "今天的行动不应被改动");

    clock.advance_days(1);
    app.today = clock.today();
    app.reload().expect("跨日刷新");
    assert_eq!(app.actions.len(), 0, "每周 1 次的间隔未到，次日不应生成");

    app.open_frequency_dialog(task, Frequency::WeeklyN(1));
    app.draft_frequency_mode = FrequencyMode::Daily;
    app.submit_frequency();
    clock.advance_days(1);
    app.today = clock.today();
    app.reload().expect("跨日刷新");
    assert_eq!(app.actions.len(), 1, "改回每天后次日应生成行动");
}
