//! 删除确认对话框行为无头验收（T014）。
//!
//! 对话框确认即调用 `Store` 的 `delete_goal` / `delete_category`（软删除）；
//! 这里验证软删除的效果与返回的影响范围（无需 GPU）。

use crate::model::{Frequency, TaskDraft};
use crate::store::Store;

/// 在临时目录打开一个空库，便于测试构造数据。
fn temp_store(tag: &str) -> (std::path::PathBuf, Store) {
    let dir = std::env::temp_dir().join(format!("ngy-min-action-015-delete-{tag}"));
    let _ = std::fs::remove_dir_all(&dir);
    let store = Store::open(&dir.join("data.db")).expect("应能创建临时数据库");
    (dir, store)
}

#[test]
fn delete_goal_soft_deletes_and_reports_impact() {
    let (_dir, store) = temp_store("goal");
    let goal = store.create_goal("G", None).expect("目标");
    let cat = store.create_category(goal, "C").expect("分类");
    let draft = TaskDraft {
        name: "t".to_owned(),
        frequency: Frequency::Daily,
        estimated_minutes: None,
        description: None,
    };
    let _ = store.create_task(cat, draft);

    let impact = store.delete_goal(goal).expect("删除目标");
    assert_eq!(impact.categories, 1, "应波及 1 个分类");
    assert_eq!(impact.tasks, 1, "应波及 1 条任务");

    // 软删除后常规列表应不再包含该目标/分类/任务。
    assert!(store.goals().expect("目标").is_empty(), "目标应已被软删除");
    assert!(
        store.all_categories().expect("分类").is_empty(),
        "分类应被级联软删除"
    );
    assert!(
        store.all_tasks().expect("任务").is_empty(),
        "任务应被级联软删除"
    );
}

#[test]
fn delete_category_soft_deletes_tasks() {
    let (_dir, store) = temp_store("cat");
    let goal = store.create_goal("G", None).expect("目标");
    let cat = store.create_category(goal, "C").expect("分类");
    let draft = TaskDraft {
        name: "t".to_owned(),
        frequency: Frequency::Daily,
        estimated_minutes: None,
        description: None,
    };
    let _ = store.create_task(cat, draft);

    let impact = store.delete_category(cat).expect("删除分类");
    assert_eq!(impact.tasks, 1, "应波及 1 条任务");
    assert!(
        store.all_categories().expect("分类").is_empty(),
        "分类应被软删除"
    );
    assert!(
        store.all_tasks().expect("任务").is_empty(),
        "任务应被级联软删除"
    );
}
