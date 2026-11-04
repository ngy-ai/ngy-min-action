//! 008：`Store::review_stats` 的聚合行为——分项计数、连续天数与阶段分布。

use chrono::NaiveDate;
use ngy_min_action::model::{ActionStatus, Frequency, TaskDraft};
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
    }
}

fn generate(store: &Store, day: NaiveDate) {
    let actions = store.generate_day(day, 5).expect("生成当天行动");
    assert!(!actions.is_empty(), "当天应当生成行动");
}

fn settle_day(store: &Store, day: NaiveDate, status: ActionStatus) {
    let actions = store.generate_day(day, 5).expect("生成当天行动");
    assert!(!actions.is_empty(), "当天应当生成行动");
    for action in &actions {
        store.settle_action(action.id, status).expect("结算行动");
    }
}

#[test]
fn review_stats_counts_settled_actions_by_status() {
    let store = open_store("stats-tally");
    let goal = store.create_goal("学习英语", None).expect("创建目标");
    let category = store.create_category(goal, "口语").expect("创建分类");
    store
        .create_task(category, draft("连读"))
        .expect("创建任务");

    settle_day(&store, date(2026, 11, 1), ActionStatus::Done);
    settle_day(&store, date(2026, 11, 2), ActionStatus::Done);
    settle_day(&store, date(2026, 11, 3), ActionStatus::Done);

    let stats = store.review_stats(date(2026, 11, 4), 30).expect("读取统计");
    assert_eq!(stats.window_days, 30);
    assert_eq!(stats.tally.done, 3);
    assert_eq!(stats.tally.skipped, 0);
    assert_eq!(stats.tally.expired, 0);
    assert_eq!(stats.tally.completion_percent(), Some(100));
}

#[test]
fn review_stats_reports_empty_tally_when_nothing_is_settled() {
    let store = open_store("stats-empty");
    let stats = store.review_stats(date(2026, 11, 4), 30).expect("读取统计");
    assert_eq!(stats.tally.settled(), 0);
    assert_eq!(stats.tally.completion_percent(), None, "没有可统计的行动");
    assert_eq!(stats.current_streak, 0);
    assert_eq!(stats.longest_streak, 0);
}

#[test]
fn review_stats_breaks_the_streak_on_a_skipped_day() {
    let store = open_store("stats-streak-break");
    let goal = store.create_goal("学习英语", None).expect("创建目标");
    let category = store.create_category(goal, "口语").expect("创建分类");
    store
        .create_task(category, draft("连读"))
        .expect("创建任务");

    settle_day(&store, date(2026, 10, 30), ActionStatus::Done);
    settle_day(&store, date(2026, 10, 31), ActionStatus::Done);
    settle_day(&store, date(2026, 11, 1), ActionStatus::Done);
    settle_day(&store, date(2026, 11, 2), ActionStatus::Skipped);
    settle_day(&store, date(2026, 11, 3), ActionStatus::Done);

    let stats = store.review_stats(date(2026, 11, 4), 30).expect("读取统计");
    assert_eq!(stats.current_streak, 1, "从 11-03 起算，11-02 中断");
    assert_eq!(stats.longest_streak, 3, "10-30 到 11-01 是历史最长");
}

#[test]
fn review_stats_skips_days_without_actions() {
    let store = open_store("stats-streak-gap");
    let goal = store.create_goal("学习英语", None).expect("创建目标");
    let category = store.create_category(goal, "口语").expect("创建分类");
    store
        .create_task(category, draft("连读"))
        .expect("创建任务");

    settle_day(&store, date(2026, 11, 1), ActionStatus::Done);
    settle_day(&store, date(2026, 11, 3), ActionStatus::Done);

    let stats = store.review_stats(date(2026, 11, 3), 30).expect("读取统计");
    assert_eq!(stats.current_streak, 2, "没有行动的日子不算中断");
}

#[test]
fn review_stats_keeps_the_streak_when_today_is_unfinished() {
    let store = open_store("stats-streak-today");
    let goal = store.create_goal("学习英语", None).expect("创建目标");
    let category = store.create_category(goal, "口语").expect("创建分类");
    store
        .create_task(category, draft("连读"))
        .expect("创建任务");

    settle_day(&store, date(2026, 11, 2), ActionStatus::Done);
    settle_day(&store, date(2026, 11, 3), ActionStatus::Done);
    generate(&store, date(2026, 11, 4));

    let stats = store.review_stats(date(2026, 11, 4), 30).expect("读取统计");
    assert_eq!(stats.current_streak, 2, "今天还没完成，不应打断");
    assert_eq!(stats.tally.settled(), 2, "今天的 pending 不计入已结算");
}

#[test]
fn review_stats_groups_tasks_by_stage() {
    let store = open_store("stats-stages");
    let goal = store.create_goal("学习英语", None).expect("创建目标");
    let category = store.create_category(goal, "口语").expect("创建分类");
    store.create_task(category, draft("连读")).expect("任务一");
    store.create_task(category, draft("音标")).expect("任务二");
    store.create_task(category, draft("语调")).expect("任务三");

    let today = date(2026, 11, 4);
    let actions = store.generate_day(today, 5).expect("生成行动");
    store
        .settle_action(actions[0].id, ActionStatus::Done)
        .expect("完成一条，把对应任务推到第 1 级");

    let stats = store.review_stats(today, 30).expect("读取统计");
    assert_eq!(stats.untouched_tasks, 2);
    assert_eq!(stats.learning_tasks, 1);
    assert_eq!(stats.graduated_tasks, 0);
}

#[test]
fn review_stats_keeps_expired_actions_but_drops_deleted_tasks_from_stages() {
    let store = open_store("stats-deleted");
    let goal = store.create_goal("学习英语", None).expect("创建目标");
    let category = store.create_category(goal, "口语").expect("创建分类");
    let task = store
        .create_task(category, draft("连读"))
        .expect("创建任务");

    let today = date(2026, 11, 4);
    generate(&store, today);
    store.delete_task(task).expect("删除任务");

    let stats = store.review_stats(today, 30).expect("读取统计");
    assert_eq!(stats.tally.expired, 1, "删除后未完成行动转为过期");
    assert_eq!(stats.tally.completion_percent(), Some(0));
    assert_eq!(
        stats.untouched_tasks + stats.learning_tasks + stats.graduated_tasks,
        0,
        "阶段分布不应包含已删除的任务"
    );
}
