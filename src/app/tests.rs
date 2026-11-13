//! 根视图数据桥接的无头验收（T005）。
//!
//! 不依赖 GPU/窗口，直接构造 `MinActionApp` 并断言 `reload()` 从 `Store` 正确载入目标、分类、
//! 任务与分类计数。后续阶段的结构树与三页签都依赖这份状态。

use std::sync::Arc;

use crate::app::{MinActionApp, TodoTab};
use crate::clock::SystemClock;
use crate::config::Bootstrap;
use crate::model::{Frequency, TaskDraft};
use crate::store::Store;

/// 在临时目录打开一个空库，便于测试构造数据。
fn open_temp_store(tag: &str) -> (std::path::PathBuf, Store) {
    let dir = std::env::temp_dir().join(format!("ngy-min-action-015-{tag}"));
    let _ = std::fs::remove_dir_all(&dir);
    let store = Store::open(&dir.join("data.db")).expect("应能创建临时数据库");
    (dir, store)
}

/// `reload()` 应载入目标、分类、任务，并聚合出分类任务计数。
#[test]
fn reload_loads_goals_categories_tasks_and_counts() {
    let (dir, store) = open_temp_store("bridge");
    let goal = store.create_goal("AI", None).expect("创建目标");
    let category = store.create_category(goal, "口语").expect("创建分类");
    let draft = TaskDraft {
        name: "朗读课文".to_owned(),
        frequency: Frequency::Daily,
        estimated_minutes: None,
        description: None,
    };
    store.create_task(category, draft).expect("创建任务");

    let bootstrap = Bootstrap::new(&dir);
    let (_tx, rx) = std::sync::mpsc::channel();
    let app = MinActionApp::new(store, Arc::new(SystemClock), rx, bootstrap).expect("构造根视图");

    assert_eq!(app.all_goals.len(), 1, "应载入 1 个目标");
    assert_eq!(app.all_categories.len(), 1, "应载入 1 个分类");
    assert_eq!(app.all_tasks.len(), 1, "应载入 1 个任务");
    assert_eq!(
        app.task_counts.get(&category),
        Some(&1),
        "分类任务计数应为 1"
    );
    assert_eq!(app.selected_goal, Some(goal), "默认应选中唯一目标");
    assert_eq!(app.selected_category, Some(category), "默认应选中唯一分类");
    assert_eq!(app.goal_task_count(goal), 1, "目标任务汇总应为 1");
}

/// 选中目标后，分类应归一化到该目标下第一个分类。
#[test]
fn select_goal_normalizes_category() {
    let (dir, store) = open_temp_store("select");
    let goal1 = store.create_goal("AI", None).expect("目标1");
    let cat1 = store.create_category(goal1, "口语").expect("分类1");
    let goal2 = store.create_goal("健康", None).expect("目标2");
    let cat2 = store.create_category(goal2, "运动").expect("分类2");
    let (_tx, rx) = std::sync::mpsc::channel();
    let bootstrap = Bootstrap::new(&dir);
    let mut app =
        MinActionApp::new(store, Arc::new(SystemClock), rx, bootstrap).expect("构造根视图");

    app.select_goal(goal2);
    assert_eq!(app.selected_goal, Some(goal2), "应选中目标2");
    assert_eq!(app.selected_category, Some(cat2), "分类应归一化到目标2下");

    app.select_category(cat1);
    assert_eq!(app.selected_category, Some(cat1), "应选中分类1");
    assert_eq!(app.selected_goal, Some(goal1), "所属目标应同步为目标1");
}

/// 切换页签应更新当前 tab 状态。
#[test]
fn switch_tab_updates_state() {
    let (dir, store) = open_temp_store("tab");
    let _goal = store.create_goal("AI", None).expect("目标");
    let (_tx, rx) = std::sync::mpsc::channel();
    let bootstrap = Bootstrap::new(&dir);
    let mut app =
        MinActionApp::new(store, Arc::new(SystemClock), rx, bootstrap).expect("构造根视图");

    assert_eq!(app.tab, TodoTab::Tasks, "默认应为任务页");
    app.switch_tab(TodoTab::History);
    assert_eq!(app.tab, TodoTab::History, "应切换到历史页");
}

/// 删除目标的影响范围应汇总其下分类与任务数。
#[test]
fn goal_deletion_impact_sums_categories_and_tasks() {
    let (dir, store) = open_temp_store("impact-goal");
    let goal = store.create_goal("G", None).expect("目标");
    let cat1 = store.create_category(goal, "C1").expect("分类1");
    let cat2 = store.create_category(goal, "C2").expect("分类2");
    let draft = TaskDraft {
        name: "t".to_owned(),
        frequency: Frequency::Daily,
        estimated_minutes: None,
        description: None,
    };
    let _ = store.create_task(cat1, draft.clone());
    let _ = store.create_task(cat2, draft);
    let bootstrap = Bootstrap::new(&dir);
    let (_tx, rx) = std::sync::mpsc::channel();
    let app = MinActionApp::new(store, Arc::new(SystemClock), rx, bootstrap).expect("构造根视图");
    let impact = app.goal_deletion_impact(goal);
    assert_eq!(impact.categories, 2, "应波及 2 个分类");
    assert_eq!(impact.tasks, 2, "应波及 2 条任务");
}

/// 删除分类的影响范围应只含其下任务数。
#[test]
fn category_deletion_impact_counts_tasks() {
    let (dir, store) = open_temp_store("impact-cat");
    let goal = store.create_goal("G", None).expect("目标");
    let cat = store.create_category(goal, "C").expect("分类");
    let draft = TaskDraft {
        name: "t".to_owned(),
        frequency: Frequency::Daily,
        estimated_minutes: None,
        description: None,
    };
    let _ = store.create_task(cat, draft);
    let bootstrap = Bootstrap::new(&dir);
    let (_tx, rx) = std::sync::mpsc::channel();
    let app = MinActionApp::new(store, Arc::new(SystemClock), rx, bootstrap).expect("构造根视图");
    let impact = app.category_deletion_impact(cat);
    assert_eq!(impact.categories, 0, "分类删除不影响其他分类");
    assert_eq!(impact.tasks, 1, "应波及 1 条任务");
}
