//! 008：统计面板的用例。

use super::*;

use crate::model::ActionStatus;

#[test]
fn opening_stats_fills_a_snapshot() {
    let mut app = open_app("stats-open");
    let category = seed_category(&mut app, "学习英语", "口语");
    app.store
        .create_task(category, draft("连读"))
        .expect("创建任务");
    app.reload().expect("刷新界面数据");
    let action = app.actions[0].id;
    app.store
        .settle_action(action, ActionStatus::Done)
        .expect("完成行动");

    app.open_stats();

    let stats = app.stats.clone().expect("应有统计快照");
    assert!(app.show_stats, "应打开面板");
    assert_eq!(stats.window_days, 30);
    assert_eq!(stats.tally.done, 1);
    assert_eq!(stats.tally.completion_percent(), Some(100));
    assert_eq!(stats.current_streak, 1, "今天有完成，连续 1 天");
    assert_eq!(stats.learning_tasks, 1, "完成一次后进入学习中");
}

#[test]
fn opening_stats_on_an_empty_store_keeps_the_empty_state() {
    let mut app = open_app("stats-empty-panel");
    app.open_stats();

    let stats = app.stats.clone().expect("应有统计快照");
    assert_eq!(stats.tally.completion_percent(), None);
    assert_eq!(stats.current_streak, 0);
    assert_eq!(stats.untouched_tasks, 0);
}

#[test]
fn stats_reads_are_stable() {
    let mut app = open_app("stats-stable");
    let category = seed_category(&mut app, "学习英语", "口语");
    app.store
        .create_task(category, draft("连读"))
        .expect("创建任务");
    app.reload().expect("刷新界面数据");

    app.open_stats();
    let first = app.stats.clone();
    app.open_stats();
    assert_eq!(app.stats, first, "重复读取应得到同一份快照");
}
