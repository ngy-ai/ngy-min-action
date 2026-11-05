//! 主题偏好的存储契约。

use std::path::PathBuf;

use ngy_min_action::model::ThemeChoice;
use ngy_min_action::store::Store;

fn open_store(tag: &str) -> (Store, PathBuf) {
    let dir = std::env::temp_dir().join(format!(
        "ngy-min-action-theme-test-{}-{tag}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("创建临时目录");
    let path = dir.join("data.db");
    let store = Store::open(&path).expect("应能创建临时数据库");
    (store, path)
}

#[test]
fn theme_choice_defaults_to_system() {
    let (store, _) = open_store("default");
    assert_eq!(store.theme_choice().expect("读取偏好"), ThemeChoice::System);
}

#[test]
fn theme_choice_round_trips_all_three_values() {
    for (tag, choice) in [
        ("system", ThemeChoice::System),
        ("light", ThemeChoice::Light),
        ("dark", ThemeChoice::Dark),
    ] {
        let (store, _) = open_store(tag);
        store.set_theme_choice(choice).expect("写入偏好");
        assert_eq!(store.theme_choice().expect("读取偏好"), choice);
    }
}

#[test]
fn an_unknown_persisted_value_falls_back_to_system() {
    let (store, path) = open_store("unknown");
    let connection = rusqlite::Connection::open(&path).expect("直连数据库");
    connection
        .execute(
            "INSERT INTO settings (key, value) VALUES ('theme', 'neon')",
            [],
        )
        .expect("写入未知值");
    drop(connection);
    assert_eq!(store.theme_choice().expect("读取偏好"), ThemeChoice::System);
}

#[test]
fn writing_theme_does_not_touch_other_settings_or_business_rows() {
    let (store, path) = open_store("isolated");
    store.set_daily_cap(7).expect("设置上限");
    let goal = store.create_goal("学习英语", None).expect("创建目标");
    store.create_category(goal, "口语").expect("创建分类");

    store.set_theme_choice(ThemeChoice::Dark).expect("写入偏好");

    assert_eq!(store.theme_choice().expect("读取偏好"), ThemeChoice::Dark);
    assert_eq!(store.daily_cap().expect("读取上限"), 7);
    let connection = rusqlite::Connection::open(&path).expect("直连数据库");
    let categories: i64 = connection
        .query_row("SELECT COUNT(*) FROM categories", [], |row| row.get(0))
        .expect("查询分类数");
    assert_eq!(categories, 1);
}

#[test]
fn theme_choice_survives_a_new_store_instance() {
    let (store, path) = open_store("reopen");
    store
        .set_theme_choice(ThemeChoice::Light)
        .expect("写入偏好");
    drop(store);
    let reopened = Store::open(&path).expect("重新打开");
    assert_eq!(
        reopened.theme_choice().expect("读取偏好"),
        ThemeChoice::Light
    );
}
