//! 根视图数据桥接的无头验收（T005）。
//!
//! 不依赖 GPU/窗口，直接构造 `MinActionApp` 并断言 `reload()` 从 `Store` 正确载入目标、分类、
//! 任务与分类计数。后续阶段的结构树与三页签都依赖这份状态。

use std::sync::Arc;

use crate::app::MinActionApp;
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
