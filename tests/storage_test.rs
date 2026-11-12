//! 012：启动位置配置、图片附件与数据迁移。

use std::path::PathBuf;

use chrono::NaiveDate;
use ngy_min_action::attachment::AttachmentStore;
use ngy_min_action::config::Bootstrap;
use ngy_min_action::model::{ActionId, Frequency, TaskDraft};
use ngy_min_action::storage::{cleanup_old_data, relocate};
use ngy_min_action::store::Store;

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "ngy-min-action-storage-test-{}-{tag}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("创建测试目录");
    dir
}

fn draft(name: &str) -> TaskDraft {
    TaskDraft {
        name: name.to_owned(),
        frequency: Frequency::Daily,
        estimated_minutes: None,
        description: None,
    }
}

fn seed_action(store: &Store) -> ActionId {
    let goal = store.create_goal("学习英语", None).expect("创建目标");
    let category = store.create_category(goal, "口语").expect("创建分类");
    store
        .create_task(category, draft("跟读十分钟"))
        .expect("创建任务");
    store
        .generate_day(NaiveDate::from_ymd_opt(2026, 11, 7).expect("测试日期"), 5)
        .expect("生成行动")
        .first()
        .expect("当天应有行动")
        .id
}

fn write_png(path: &PathBuf) {
    let image = image::RgbaImage::new(2, 2);
    image.save(path).expect("写测试图片");
}

#[test]
fn location_file_round_trips_and_falls_back() {
    let root = temp_dir("config");
    let bootstrap = Bootstrap::new(root.join("boot"));
    assert_eq!(bootstrap.data_dir(), bootstrap.default_data_dir());
    assert_eq!(bootstrap.lock_path(), bootstrap.dir().join("app.lock"));

    let custom = root.join("custom-data");
    std::fs::create_dir_all(&custom).expect("创建自定义目录");
    bootstrap.set_data_dir(&custom).expect("写入位置");
    assert_eq!(bootstrap.data_dir(), custom);

    std::fs::write(bootstrap.location_path(), "relative/path").expect("写入非法位置");
    assert_eq!(
        bootstrap.data_dir(),
        bootstrap.default_data_dir(),
        "相对路径必须回退默认位置"
    );
}

#[test]
fn attachment_copy_and_remove() {
    let root = temp_dir("attachment");
    let source = root.join("来源 截图.png");
    write_png(&source);

    let attachments = AttachmentStore::new(&root);
    let stored = attachments
        .store(ActionId(7), &source)
        .expect("应能复制图片");
    assert!(stored.file_name.ends_with(".png"));
    assert_eq!(stored.original_name, "来源 截图.png");
    assert!(stored.byte_size > 0);

    let path = attachments
        .path(ActionId(7), &stored.file_name)
        .expect("生成的附件名必须合法");
    assert!(path.exists(), "附件必须落在待办目录下");
    assert!(path.starts_with(root.join("attachments").join("7")));

    attachments
        .remove(ActionId(7), &stored.file_name)
        .expect("应能移除附件");
    assert!(!path.exists());
}

#[test]
fn attachment_rejects_unsupported_oversized_and_traversal() {
    let root = temp_dir("attachment-invalid");
    let attachments = AttachmentStore::new(&root);

    let text = root.join("note.txt");
    std::fs::write(&text, b"not an image").expect("写文本文件");
    assert!(attachments.store(ActionId(1), &text).is_err());

    let huge = root.join("huge.png");
    let file = std::fs::File::create(&huge).expect("创建大文件");
    file.set_len(10 * 1024 * 1024 + 1).expect("扩展文件");
    drop(file);
    assert!(attachments.store(ActionId(1), &huge).is_err());

    assert!(
        attachments.path(ActionId(1), "../escape.png").is_err(),
        "带路径分隔符的文件名必须被拒绝"
    );
    assert!(
        attachments.path(ActionId(1), "sub/escape.png").is_err(),
        "子目录形式的文件名必须被拒绝"
    );
}

#[test]
fn relocation_copies_database_and_attachments() {
    let root = temp_dir("relocate-ok");
    let old = root.join("old");
    let new = root.join("new");
    let store = Store::open(&old.join("data.db")).expect("打开旧库");
    let action = seed_action(&store);
    store
        .set_action_note(action, "迁移前的备注")
        .expect("保存备注");

    let source = root.join("图片.png");
    write_png(&source);
    let attachments = AttachmentStore::new(&old);
    let stored = attachments.store(action, &source).expect("复制附件");
    store
        .add_action_image(
            action,
            &stored.file_name,
            &stored.original_name,
            stored.byte_size,
        )
        .expect("写入图片行");

    let bootstrap = Bootstrap::new(root.join("boot"));
    bootstrap.set_data_dir(&old).expect("记录旧位置");

    let outcome = relocate(&store, &new, &bootstrap).expect("迁移应成功");
    assert_eq!(outcome.data_dir, new);
    assert_eq!(bootstrap.data_dir(), new);
    assert!(new.join("data.db").exists());
    assert!(
        new.join("attachments")
            .join(action.0.to_string())
            .join(&stored.file_name)
            .exists()
    );
    assert_eq!(
        outcome.store.action_note(action).expect("新库读备注"),
        Some("迁移前的备注".to_owned())
    );
    assert_eq!(
        outcome
            .store
            .action_images(action)
            .expect("新库读图片")
            .len(),
        1
    );

    drop(outcome.store);
    drop(store);
    cleanup_old_data(&old).expect("清理旧位置");
    assert!(!old.join("data.db").exists());
    assert!(!old.join("attachments").exists());
}

#[test]
fn relocation_refuses_unsafe_targets() {
    let root = temp_dir("relocate-refuse");
    let old = root.join("old");
    let store = Store::open(&old.join("data.db")).expect("打开旧库");
    let bootstrap = Bootstrap::new(root.join("boot"));
    bootstrap.set_data_dir(&old).expect("记录旧位置");

    assert!(
        relocate(&store, &old, &bootstrap).is_err(),
        "同目录必须拒绝"
    );

    let occupied = root.join("occupied");
    std::fs::create_dir_all(&occupied).expect("创建目标");
    Store::open(&occupied.join("data.db")).expect("写入已有数据库");
    assert!(
        relocate(&store, &occupied, &bootstrap).is_err(),
        "已有 data.db 必须拒绝覆盖"
    );

    let nested = old.join("nested");
    assert!(
        relocate(&store, &nested, &bootstrap).is_err(),
        "源目录内的目标必须拒绝"
    );

    let blocked = root.join("blocked");
    std::fs::write(&blocked, b"not a directory").expect("创建同名文件");
    assert!(
        relocate(&store, &blocked, &bootstrap).is_err(),
        "不可写目标必须失败"
    );
    assert_eq!(bootstrap.data_dir(), old, "失败后位置配置不能改变");
    assert!(store.daily_cap().is_ok(), "失败后旧库连接仍可用");
    assert!(store.path().exists(), "失败后旧库文件必须保留");
}
