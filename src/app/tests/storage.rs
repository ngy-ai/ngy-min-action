//! 存储位置设置：成功后切换路径，失败保留原位置。

use super::*;

fn seed_note(app: &mut MinActionApp) -> crate::model::ActionId {
    let category = seed_category(app, "学习英语", "口语");
    app.store
        .create_task(category, draft("跟读十分钟"))
        .expect("创建任务");
    app.reload().expect("刷新界面数据");
    let action = app.actions.first().expect("今天应有行动").id;
    app.store
        .set_action_note(action, "迁移前的备注")
        .expect("写备注");
    app.reload().expect("刷新界面数据");
    action
}

#[test]
fn storage_dialog_switches_to_the_new_location() {
    let mut app = open_app("storage-switch");
    let action = seed_note(&mut app);
    let old_dir = app.data_dir.clone();
    let new_dir = old_dir
        .parent()
        .expect("临时目录应有父目录")
        .join(format!("ngy-min-action-migrated-{}", std::process::id()));

    app.storage.target = Some(new_dir.clone());
    app.relocate_storage();

    assert!(
        app.storage.error.is_none(),
        "迁移失败：{:?}",
        app.storage.error
    );
    assert_eq!(app.data_dir, new_dir);
    assert_eq!(app.store.path(), new_dir.join("data.db"));
    assert!(app.storage.notice.is_some());
    assert!(!old_dir.join("data.db").exists(), "旧库应被清理");
    assert_eq!(
        app.store.action_note(action).expect("新库读备注"),
        Some("迁移前的备注".to_owned())
    );
    assert_eq!(app.bootstrap.data_dir(), new_dir);
}

#[test]
fn failed_migration_keeps_the_old_location() {
    let mut app = open_app("storage-failure");
    let old_dir = app.data_dir.clone();
    let blocked = old_dir
        .parent()
        .expect("临时目录应有父目录")
        .join(format!("ngy-min-action-blocked-{}", std::process::id()));
    std::fs::write(&blocked, b"not a directory").expect("创建同名文件");

    app.storage.target = Some(blocked.clone());
    app.relocate_storage();

    assert!(app.storage.error.is_some());
    assert_eq!(app.data_dir, old_dir);
    assert!(old_dir.join("data.db").exists());
    assert_eq!(app.bootstrap.data_dir(), old_dir);
}
