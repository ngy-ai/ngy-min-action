//! 新建目标 / 分类 / 任务的写库行为验收。

use crate::app::create_dialog::{CreateKind, create_entity};
use crate::model::Frequency;
use crate::store::Store;

/// 在临时目录打开一个空库，便于测试构造数据。
fn temp_store(tag: &str) -> Store {
    let dir = std::env::temp_dir().join(format!("ngy-min-action-016-create-{tag}"));
    let _ = std::fs::remove_dir_all(&dir);
    Store::open(&dir.join("data.db")).expect("应能创建临时数据库")
}

#[test]
fn create_goal_writes_store() {
    let store = temp_store("goal");
    create_entity(&store, CreateKind::Goal, "写作").expect("创建目标应成功");

    let goals = store.goals().expect("读取目标");
    assert_eq!(goals.len(), 1, "应写入 1 个目标");
    assert_eq!(goals[0].name, "写作", "名称应写库");
}

#[test]
fn create_category_writes_store_under_chosen_goal() {
    let store = temp_store("category");
    let goal = store.create_goal("健康", None).expect("创建目标");
    create_entity(&store, CreateKind::Category { goal: Some(goal) }, "运动")
        .expect("创建分类应成功");

    let categories = store.all_categories().expect("读取分类");
    assert_eq!(categories.len(), 1, "应写入 1 个分类");
    assert_eq!(categories[0].name, "运动", "名称应写库");
    assert_eq!(categories[0].goal_id, goal, "分类应归属所选目标");
}

#[test]
fn create_task_writes_store_under_chosen_category() {
    let store = temp_store("task");
    let goal = store.create_goal("健康", None).expect("创建目标");
    let category = store.create_category(goal, "运动").expect("创建分类");
    create_entity(
        &store,
        CreateKind::Task {
            category: Some(category),
        },
        "晨跑",
    )
    .expect("创建任务应成功");

    let tasks = store.all_tasks().expect("读取任务");
    assert_eq!(tasks.len(), 1, "应写入 1 条任务");
    assert_eq!(tasks[0].name, "晨跑", "名称应写库");
    assert_eq!(tasks[0].category_id, category, "任务应归属所选分类");
    assert_eq!(tasks[0].frequency, Frequency::Daily, "新任务默认每天一次");
}

#[test]
fn create_without_scope_fails_with_readable_hint() {
    let store = temp_store("no-scope");
    let category_error = create_entity(&store, CreateKind::Category { goal: None }, "分类")
        .expect_err("无目标时创建分类应失败");
    let task_error = create_entity(&store, CreateKind::Task { category: None }, "任务")
        .expect_err("无分类时创建任务应失败");

    assert!(category_error.contains("目标"), "提示应指向先建目标");
    assert!(task_error.contains("分类"), "提示应指向先建分类");
    assert!(
        store.all_categories().expect("读取分类").is_empty(),
        "不应写入分类"
    );
    assert!(
        store.all_tasks().expect("读取任务").is_empty(),
        "不应写入任务"
    );
}

#[test]
fn create_rejects_duplicate_name() {
    let store = temp_store("duplicate");
    create_entity(&store, CreateKind::Goal, "写作").expect("首次创建应成功");
    let error =
        create_entity(&store, CreateKind::Goal, "写作").expect_err("同名目标应被数据层拒绝");

    assert!(error.contains("写作"), "错误文案应含冲突名称");
    assert_eq!(
        store.goals().expect("读取目标").len(),
        1,
        "不应写入重复目标"
    );
}
