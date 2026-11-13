//! 重命名对话框行为无头验收（T014）。
//!
//! 对话框确认即调用 `Store` 的 `rename_goal` / `rename_category`；这里直接验证
//! 这两个写回操作的效果（无需 GPU）。

use crate::model::{Frequency, TaskDraft};
use crate::store::Store;

/// 在临时目录打开一个空库，便于测试构造数据。
fn temp_store(tag: &str) -> (std::path::PathBuf, Store) {
    let dir = std::env::temp_dir().join(format!("ngy-min-action-015-rename-{tag}"));
    let _ = std::fs::remove_dir_all(&dir);
    let store = Store::open(&dir.join("data.db")).expect("应能创建临时数据库");
    (dir, store)
}

#[test]
fn rename_goal_updates_name() {
    let (_dir, store) = temp_store("goal");
    let goal = store.create_goal("旧名", None).expect("创建目标");
    store.rename_goal(goal, "新名").expect("重命名");
    let goals = store.goals().expect("读取目标");
    assert_eq!(
        goals.iter().find(|g| g.id == goal).map(|g| g.name.as_str()),
        Some("新名")
    );
}

#[test]
fn rename_category_updates_name() {
    let (_dir, store) = temp_store("cat");
    let goal = store.create_goal("G", None).expect("创建目标");
    let category = store.create_category(goal, "旧分类").expect("创建分类");
    let draft = TaskDraft {
        name: "t".to_owned(),
        frequency: Frequency::Daily,
        estimated_minutes: None,
        description: None,
    };
    let _ = store.create_task(category, draft);
    store.rename_category(category, "新分类").expect("重命名");
    let cats = store.all_categories().expect("读取分类");
    assert_eq!(
        cats.iter()
            .find(|c| c.id == category)
            .map(|c| c.name.as_str()),
        Some("新分类")
    );
}
