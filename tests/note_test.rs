//! 012：待办备注、图片行、历史分页与 schema 增量升级。

use std::path::PathBuf;

use chrono::NaiveDate;
use ngy_min_action::model::{
    ActionId, ActionStatus, DomainError, Frequency, TaskDraft, validate_note,
};
use ngy_min_action::store::Store;
use rusqlite::Connection;

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "ngy-min-action-note-test-{}-{tag}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn date(year: i32, month: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, month, day).expect("测试日期必须合法")
}

fn draft(name: &str) -> TaskDraft {
    TaskDraft {
        name: name.to_owned(),
        frequency: Frequency::Daily,
        estimated_minutes: None,
    }
}

fn open_store(tag: &str) -> (PathBuf, Store) {
    let dir = temp_dir(tag);
    let store = Store::open(&dir.join("data.db")).expect("应能创建临时数据库");
    (dir, store)
}

fn seed_today_action(store: &Store) -> ActionId {
    let goal = store.create_goal("学习英语", None).expect("创建目标");
    let category = store.create_category(goal, "口语").expect("创建分类");
    store
        .create_task(category, draft("每天跟读十分钟"))
        .expect("创建任务");
    let actions = store.generate_day(date(2026, 11, 7), 5).expect("生成行动");
    actions.first().expect("当天应有行动").id
}

#[test]
fn note_round_trips_and_clears() {
    let (_dir, store) = open_store("note-roundtrip");
    let action = seed_today_action(&store);

    assert_eq!(store.action_note(action).expect("读取备注"), None);
    store
        .set_action_note(action, "  今天练了连读  ")
        .expect("保存备注");
    assert_eq!(
        store.action_note(action).expect("读取备注"),
        Some("今天练了连读".to_owned())
    );

    store.set_action_note(action, "   ").expect("清空备注");
    assert_eq!(store.action_note(action).expect("读取备注"), None);
}

#[test]
fn note_validation_rejects_overlong_and_control_chars() {
    assert!(validate_note("   ").expect("空备注合法").is_none());
    assert!(validate_note(&"长".repeat(2001)).is_err());
    assert!(validate_note("第一行\u{0007}第二行").is_err());
    assert_eq!(
        validate_note("第一行\n第二行").expect("换行合法"),
        Some("第一行\n第二行".to_owned())
    );

    let (_dir, store) = open_store("note-invalid");
    let action = seed_today_action(&store);
    let error = store
        .set_action_note(action, &"长".repeat(2001))
        .expect_err("超长备注必须被拒绝");
    assert!(matches!(error, DomainError::InvalidValue { .. }));
}

#[test]
fn images_can_be_added_listed_and_deleted() {
    let (_dir, store) = open_store("image-rows");
    let action = seed_today_action(&store);

    let first = store
        .add_action_image(action, "1700000000_a.png", "截图 A.png", 1024)
        .expect("新增图片");
    let second = store
        .add_action_image(action, "1700000001_b.jpg", "截图 B.jpg", 2048)
        .expect("新增图片");

    let images = store.action_images(action).expect("列出图片");
    assert_eq!(images.len(), 2);
    assert_eq!(images[0].original_name, "截图 A.png");
    assert_eq!(images[1].byte_size, 2048);

    let removed = store.delete_action_image(second.id).expect("删除图片行");
    assert_eq!(removed.file_name, "1700000001_b.jpg");
    assert_eq!(store.action_images(action).expect("列出图片").len(), 1);

    let missing = store
        .delete_action_image(first.id)
        .expect("删除最后一张图片行");
    assert_eq!(missing.original_name, "截图 A.png");
    assert!(store.action_images(action).expect("列出图片").is_empty());
}

#[test]
fn history_pages_are_ordered_and_do_not_repeat() {
    let (_dir, store) = open_store("history-page");
    let goal = store.create_goal("健身", None).expect("创建目标");
    let category = store.create_category(goal, "力量").expect("创建分类");
    store
        .create_task(category, draft("俯卧撑"))
        .expect("创建任务");

    for day in 1..=5 {
        store
            .generate_day(date(2026, 11, day), 5)
            .expect("生成历史");
    }

    let today = date(2026, 11, 7);
    let first = store.actions_page(today, None, 2).expect("第一页");
    assert_eq!(first.actions.len(), 2);
    assert_eq!(first.actions[0].on_date, date(2026, 11, 5));
    assert_eq!(first.actions[1].on_date, date(2026, 11, 4));
    assert!(first.has_more);

    let second = store
        .actions_page(today, first.next_cursor, 2)
        .expect("第二页");
    assert_eq!(second.actions[0].on_date, date(2026, 11, 3));
    assert_eq!(second.actions[1].on_date, date(2026, 11, 2));
    assert!(second.has_more);

    let third = store
        .actions_page(today, second.next_cursor, 2)
        .expect("第三页");
    assert_eq!(third.actions.len(), 1);
    assert_eq!(third.actions[0].on_date, date(2026, 11, 1));
    assert!(!third.has_more);
}

#[test]
fn schema_v2_upgrade_keeps_existing_actions() {
    let dir = temp_dir("schema-v2");
    std::fs::create_dir_all(&dir).expect("创建测试目录");
    let path = dir.join("data.db");

    {
        let connection = Connection::open(&path).expect("创建旧库");
        connection
            .execute_batch(
                "CREATE TABLE goals (id INTEGER PRIMARY KEY, name TEXT, name_folded TEXT,
                     description TEXT, created_at TEXT, deleted_at TEXT);
                 CREATE TABLE categories (id INTEGER PRIMARY KEY, goal_id INTEGER, name TEXT,
                     name_folded TEXT, created_at TEXT, deleted_at TEXT);
                 CREATE TABLE tasks (id INTEGER PRIMARY KEY, category_id INTEGER, name TEXT,
                     freq_kind TEXT, freq_interval INTEGER, estimated_minutes INTEGER,
                     status TEXT, last_generated_on TEXT, created_at TEXT, deleted_at TEXT);
                 CREATE TABLE actions (id INTEGER PRIMARY KEY, task_id INTEGER, on_date TEXT,
                     kind TEXT, review_level INTEGER, status TEXT, seq INTEGER,
                     created_at TEXT, settled_at TEXT, UNIQUE(task_id, on_date));
                 INSERT INTO goals VALUES (1, '旧目标', '旧目标', NULL, '2026-01-01', NULL);
                 INSERT INTO categories VALUES (1, 1, '旧分类', '旧分类', '2026-01-01', NULL);
                 INSERT INTO tasks VALUES (1, 1, '旧任务', 'daily', 1, NULL, 'active', NULL,
                     '2026-01-01', NULL);
                 INSERT INTO actions VALUES (1, 1, '2026-11-06', 'regular', NULL, 'done',
                     1, '2026-11-06', '2026-11-06');
                 PRAGMA user_version = 2;",
            )
            .expect("写入旧结构");
    }

    let store = Store::open(&path).expect("旧库应能升级");
    let actions = store.actions_on(date(2026, 11, 6)).expect("读取旧行动");
    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0].id, ActionId(1));
    assert_eq!(actions[0].status, ActionStatus::Done);
    assert_eq!(actions[0].note, None);

    store
        .set_action_note(actions[0].id, "升级后写的备注")
        .expect("升级后应可写备注");
    assert!(
        store
            .action_images(actions[0].id)
            .expect("新表应存在")
            .is_empty()
    );

    // 旧库的其它行不能被升级清空。
    assert_eq!(store.goals().expect("读取目标").len(), 1);
    assert_eq!(store.all_tasks().expect("读取任务").len(), 1);
}
