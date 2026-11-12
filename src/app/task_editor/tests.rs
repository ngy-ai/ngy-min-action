//! `task_editor` 的无头验收：保存逻辑（`apply_edits`）写回 `Store`。
//!
//! 不依赖 GPU/窗口，直接调用 `apply_edits` 并断言 `Store` 中任务字段已更新。

use super::apply_edits;
use crate::model::{Frequency, TaskDraft};
use crate::store::Store;

/// 在临时目录打开一个空库，便于测试构造数据。
fn open_temp_store(tag: &str) -> (std::path::PathBuf, Store) {
    let dir = std::env::temp_dir().join(format!("ngy-min-action-015-editor-{tag}"));
    let _ = std::fs::remove_dir_all(&dir);
    let store = Store::open(&dir.join("data.db")).expect("应能创建临时数据库");
    (dir, store)
}

#[test]
fn apply_edits_writes_back_to_store() {
    let (_dir, store) = open_temp_store("apply");
    let goal = store.create_goal("AI", None).unwrap();
    let category = store.create_category(goal, "口语").unwrap();
    let task = store
        .create_task(
            category,
            TaskDraft {
                name: "原名称".into(),
                frequency: Frequency::Daily,
                estimated_minutes: None,
                description: None,
            },
        )
        .unwrap();

    apply_edits(&store, task, "新名称", "新内容", Some(15)).unwrap();

    let tasks = store.tasks(category).unwrap();
    let updated = tasks.iter().find(|t| t.id == task).expect("任务应存在");
    assert_eq!(updated.name, "新名称", "名称应被写回");
    assert_eq!(
        updated.description.as_deref(),
        Some("新内容"),
        "内容应被写回"
    );
    assert_eq!(updated.estimated_minutes, Some(15), "预计耗时应被写回");
}

#[test]
fn apply_edits_handles_blank_inputs() {
    let (_dir, store) = open_temp_store("blank");
    let goal = store.create_goal("G", None).unwrap();
    let category = store.create_category(goal, "C").unwrap();
    let task = store
        .create_task(
            category,
            TaskDraft {
                name: "X".into(),
                frequency: Frequency::Daily,
                estimated_minutes: None,
                description: None,
            },
        )
        .unwrap();

    // 空名/空内容不应导致 panic；结果由 store 的校验决定（OK 或 Err 均可）。
    let result = apply_edits(&store, task, "   ", "", None);
    assert!(result.is_ok() || result.is_err());

    // 确保未 panic 且 store 仍可读。
    let _ = store.tasks(category).unwrap();
}
