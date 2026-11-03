//! FR-002 / FR-004 / FR-005 / FR-009 / FR-011 / FR-017 / FR-034 存储行为。

use chrono::NaiveDate;
use ngy_min_action::model::{
    ActionStatus, DomainError, Frequency, TaskDraft, TaskStatus, validate_name,
};
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

#[test]
fn duplicate_category_in_the_same_goal_is_rejected() {
    let store = open_store("dup-category");
    let goal = store.create_goal("学习英语", None).expect("创建目标");
    store.create_category(goal, "口语").expect("首次创建分类");
    let error = store
        .create_category(goal, "口语")
        .expect_err("同目标下重名必须被拒绝");
    assert!(matches!(error, DomainError::DuplicateName { .. }));
    assert_eq!(store.categories(goal).expect("列出分类").len(), 1);
}

#[test]
fn the_same_category_name_is_allowed_across_goals() {
    let store = open_store("cross-goal");
    let first = store.create_goal("学习英语", None).expect("创建目标");
    let second = store.create_goal("健身", None).expect("创建目标");
    store.create_category(first, "口语").expect("首次创建分类");
    store
        .create_category(second, "口语")
        .expect("跨目标允许重名");
}

#[test]
fn blank_and_control_character_names_are_rejected() {
    assert!(validate_name("   ", "目标名称", 80).is_err());
    assert!(validate_name("学习\n英语", "目标名称", 80).is_err());
    assert!(validate_name(&"长".repeat(81), "目标名称", 80).is_err());
    assert!(validate_name("  学习英语  ", "目标名称", 80).is_ok());
}

#[test]
fn soft_deleted_category_disappears_from_lists() {
    let store = open_store("soft-delete");
    let goal = store.create_goal("学习英语", None).expect("创建目标");
    let category = store.create_category(goal, "口语").expect("创建分类");
    let impact = store.delete_category(category).expect("删除分类");
    assert_eq!(impact.categories, 0);
    assert!(store.categories(goal).expect("列出分类").is_empty());
    store
        .create_category(goal, "口语")
        .expect("删除后应允许重建同名分类");
}

#[test]
fn deleting_a_goal_reports_its_impact() {
    let store = open_store("goal-impact");
    let goal = store.create_goal("学习英语", None).expect("创建目标");
    let category = store.create_category(goal, "口语").expect("创建分类");
    store
        .create_task(category, draft("每周学习一个连读"))
        .expect("创建任务");
    store.generate_day(date(2026, 11, 2), 5).expect("生成行动");

    let impact = store.delete_goal(goal).expect("删除目标");
    assert_eq!(impact.categories, 1);
    assert_eq!(impact.tasks, 1);
    assert_eq!(impact.pending_actions, 1);
    assert!(store.goals().expect("列出目标").is_empty());
}

#[test]
fn completed_actions_survive_goal_deletion() {
    let store = open_store("history-kept");
    let goal = store.create_goal("学习英语", None).expect("创建目标");
    let category = store.create_category(goal, "口语").expect("创建分类");
    store
        .create_task(category, draft("每周学习一个连读"))
        .expect("创建任务");
    let actions = store.generate_day(date(2026, 11, 2), 5).expect("生成行动");
    store
        .settle_action(actions[0].id, ActionStatus::Done)
        .expect("标记完成");

    store.delete_goal(goal).expect("删除目标");
    let remaining = store.actions_on(date(2026, 11, 2)).expect("查询当天行动");
    assert_eq!(remaining.len(), 1, "已完成的历史记录必须保留");
    assert_eq!(remaining[0].status, ActionStatus::Done);
}

#[test]
fn generate_day_is_idempotent() {
    let store = open_store("idempotent");
    let goal = store.create_goal("学习英语", None).expect("创建目标");
    let category = store.create_category(goal, "口语").expect("创建分类");
    store
        .create_task(category, draft("背 10 个单词"))
        .expect("创建任务");

    let first = store.generate_day(date(2026, 11, 2), 5).expect("首次生成");
    let second = store.generate_day(date(2026, 11, 2), 5).expect("重复生成");
    assert_eq!(first.len(), 1);
    assert_eq!(
        first.iter().map(|item| item.id).collect::<Vec<_>>(),
        second.iter().map(|item| item.id).collect::<Vec<_>>()
    );
}

#[test]
fn daily_cap_limits_generated_actions() {
    let store = open_store("cap");
    let goal = store.create_goal("学习英语", None).expect("创建目标");
    let category = store.create_category(goal, "综合").expect("创建分类");
    for index in 1..=8 {
        store
            .create_task(category, draft(&format!("任务{index}")))
            .expect("创建任务");
    }

    let actions = store.generate_day(date(2026, 11, 2), 5).expect("生成行动");
    let pending = actions
        .iter()
        .filter(|item| item.status == ActionStatus::Pending)
        .count();
    assert_eq!(pending, 5, "全局上限必须生效");
    assert_eq!(
        store.deferred_count(date(2026, 11, 2)).expect("顺延计数"),
        3
    );
}

#[test]
fn adding_a_task_does_not_exceed_the_cap() {
    let store = open_store("cap-late-task");
    let goal = store.create_goal("学习英语", None).expect("创建目标");
    let category = store.create_category(goal, "口语").expect("创建分类");
    for index in 1..=5 {
        store
            .create_task(category, draft(&format!("任务{index}")))
            .expect("创建任务");
    }
    let first = store.generate_day(date(2026, 11, 2), 5).expect("首次生成");
    assert_eq!(first.len(), 5);

    store
        .create_task(category, draft("任务6"))
        .expect("创建任务");
    let after = store.generate_day(date(2026, 11, 2), 5).expect("再次生成");
    let pending = after
        .iter()
        .filter(|item| item.status == ActionStatus::Pending)
        .count();
    assert_eq!(pending, 5, "当天已占满上限时，新增任务不得突破上限");
    assert_eq!(
        store.deferred_count(date(2026, 11, 2)).expect("顺延计数"),
        1,
        "被挡下的新任务应计入顺延"
    );
}

#[test]
fn new_actions_append_after_existing_seq() {
    let store = open_store("seq-append");
    let goal = store.create_goal("学习英语", None).expect("创建目标");
    let category = store.create_category(goal, "口语").expect("创建分类");
    for index in 1..=2 {
        store
            .create_task(category, draft(&format!("任务{index}")))
            .expect("创建任务");
    }
    let first = store.generate_day(date(2026, 11, 2), 5).expect("首次生成");
    let max_seq = first.iter().map(|item| item.seq).max().expect("应有行动");

    store
        .create_task(category, draft("任务3"))
        .expect("创建任务");
    let after = store.generate_day(date(2026, 11, 2), 5).expect("再次生成");
    let appended = after.last().expect("应有行动");
    assert!(
        appended.seq > max_seq,
        "新行动必须追加到清单末尾，实际 seq={}，已有最大 seq={max_seq}",
        appended.seq
    );
}

#[test]
fn paused_task_produces_no_action() {
    let store = open_store("paused");
    let goal = store.create_goal("学习英语", None).expect("创建目标");
    let category = store.create_category(goal, "口语").expect("创建分类");
    let task = store
        .create_task(category, draft("每周学习一个连读"))
        .expect("创建任务");
    store
        .set_task_status(task, TaskStatus::Paused)
        .expect("暂停任务");

    let actions = store.generate_day(date(2026, 11, 2), 5).expect("生成行动");
    assert!(actions.is_empty(), "暂停的任务不产生行动");
}

#[test]
fn completing_an_action_advances_review_progress() {
    let store = open_store("review-advance");
    let goal = store.create_goal("学习英语", None).expect("创建目标");
    let category = store.create_category(goal, "口语").expect("创建分类");
    let task = store
        .create_task(category, draft("每周学习一个连读"))
        .expect("创建任务");
    let actions = store.generate_day(date(2026, 11, 2), 5).expect("生成行动");

    store
        .settle_action(actions[0].id, ActionStatus::Done)
        .expect("标记完成");
    let progress = store.review_progress(task).expect("读取复习进度");
    assert_eq!(progress.level, 1);
    assert!(progress.next_review_on.is_some());
}

#[test]
fn undo_restores_the_previous_state() {
    let store = open_store("undo");
    let goal = store.create_goal("学习英语", None).expect("创建目标");
    let category = store.create_category(goal, "口语").expect("创建分类");
    let task = store
        .create_task(category, draft("每周学习一个连读"))
        .expect("创建任务");
    let actions = store.generate_day(date(2026, 11, 2), 5).expect("生成行动");

    store
        .settle_action(actions[0].id, ActionStatus::Done)
        .expect("标记完成");
    store.undo_last_settlement(actions[0].id).expect("撤销结算");

    let progress = store.review_progress(task).expect("读取复习进度");
    assert_eq!(progress.level, 0, "撤销后复习进度必须回退");
    let after = store.actions_on(date(2026, 11, 2)).expect("查询当天行动");
    assert_eq!(after[0].status, ActionStatus::Pending);
}

#[test]
fn only_settled_actions_offer_undo() {
    assert!(ActionStatus::Done.is_settled());
    assert!(ActionStatus::Skipped.is_settled());
    assert!(!ActionStatus::Pending.is_settled(), "待完成没有结算记录");
    assert!(
        !ActionStatus::Deferred.is_settled(),
        "顺延从未结算，界面上不能给撤销入口"
    );
    assert!(
        !ActionStatus::Expired.is_settled(),
        "过期从未结算，界面上不能给撤销入口"
    );
}

#[test]
fn daily_cap_setting_is_validated() {
    let store = open_store("daily-cap");
    assert_eq!(store.daily_cap().expect("默认上限"), 5);
    store.set_daily_cap(3).expect("设置上限");
    assert_eq!(store.daily_cap().expect("读取上限"), 3);
    assert!(store.set_daily_cap(0).is_err(), "0 必须被拒绝");
    assert!(store.set_daily_cap(21).is_err(), "超出上限必须被拒绝");
}
