//! 任务内容编辑器无头验收（T011）。
//!
//! 保存写回逻辑抽成了自由函数 `apply_edits`（不依赖 GPU/窗口），
//! 这里直接验证它把名称 / 内容 / 预计耗时正确写回 `Store`。

use std::path::PathBuf;

use crate::model::{Frequency, TaskDraft};
use crate::store::Store;

use super::apply_edits;

/// 在临时目录打开一个空库，便于测试构造数据。
fn open_temp_store(tag: &str) -> (PathBuf, Store) {
    let dir = std::env::temp_dir().join(format!("ngy-min-action-015-editor-{tag}"));
    let _ = std::fs::remove_dir_all(&dir);
    let store = Store::open(&dir.join("data.db")).expect("应能创建临时数据库");
    (dir, store)
}

/// `apply_edits` 应把三项编辑写回 store 中对应任务。
#[test]
fn apply_edits_writes_back_to_store() {
    let (_dir, store) = open_temp_store("apply");
    let goal = store.create_goal("AI", None).expect("目标");
    let category = store.create_category(goal, "口语").expect("分类");
    let draft = TaskDraft {
        name: "原名称".to_owned(),
        frequency: Frequency::Daily,
        estimated_minutes: None,
        description: None,
    };
    let task_id = store.create_task(category, draft).expect("任务");

    apply_edits(&store, task_id, "新名称", "新的内容", Some(30)).expect("写回应成功");

    let updated = store.task(task_id).expect("读取任务");
    assert_eq!(updated.name, "新名称", "名称应更新");
    assert_eq!(
        updated.description.as_deref(),
        Some("新的内容"),
        "内容应更新"
    );
    assert_eq!(updated.estimated_minutes, Some(30), "预计耗时应更新");
}

/// 清空预计耗时应写回 `None`，而非 0。
#[test]
fn apply_edits_clears_estimated_minutes() {
    let (_dir, store) = open_temp_store("clear");
    let goal = store.create_goal("AI", None).expect("目标");
    let category = store.create_category(goal, "口语").expect("分类");
    let draft = TaskDraft {
        name: "T".to_owned(),
        frequency: Frequency::Daily,
        estimated_minutes: Some(15),
        description: Some("旧内容".to_owned()),
    };
    let task_id = store.create_task(category, draft).expect("任务");

    apply_edits(&store, task_id, "T", "新内容", None).expect("写回应成功");

    let updated = store.task(task_id).expect("读取任务");
    assert_eq!(updated.estimated_minutes, None, "清空后应写回 None 而非 0");
    assert_eq!(updated.description.as_deref(), Some("新内容"), "内容应更新");
}
