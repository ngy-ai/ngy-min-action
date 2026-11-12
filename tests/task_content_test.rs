//! 013：任务内容校验、按 id 取任务、内容读写、媒体增删与文件落盘。

use image::{DynamicImage, RgbaImage};
use ngy_min_action::attachment::MediaStore;
use ngy_min_action::model::{
    CategoryId, DomainError, Frequency, TaskDraft, TaskId, TaskMediaKind, validate_description,
};
use ngy_min_action::store::Store;

/// 写一张 1×1 的合法 PNG 当图片源，返回其路径。
fn write_sample_png(dir: &std::path::Path) -> std::path::PathBuf {
    let path = dir.join("sample.png");
    DynamicImage::ImageRgba8(RgbaImage::new(1, 1))
        .save(&path)
        .expect("应能写出测试用 PNG");
    path
}

fn open_store(tag: &str) -> (Store, std::path::PathBuf) {
    let dir = std::env::temp_dir().join(format!("ngy-min-action-013-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let store = Store::open(&dir.join("data.db")).expect("应能创建临时数据库");
    (store, dir)
}

fn draft(name: &str) -> TaskDraft {
    TaskDraft {
        name: name.to_owned(),
        frequency: Frequency::Daily,
        estimated_minutes: None,
        description: None,
    }
}

fn add_category(store: &Store) -> CategoryId {
    let goal = store.create_goal("目标", None).expect("应能创建目标");
    store.create_category(goal, "分类").expect("应能创建分类")
}

#[test]
fn empty_description_normalizes_to_none() {
    assert_eq!(validate_description("   \t  ").unwrap(), None);
}

#[test]
fn normal_description_preserved() {
    assert_eq!(
        validate_description("背单词 10 个 https://example.com").unwrap(),
        Some("背单词 10 个 https://example.com".to_owned())
    );
}

#[test]
fn newline_in_description_kept() {
    assert_eq!(
        validate_description("第一行\n第二行").unwrap(),
        Some("第一行\n第二行".to_owned())
    );
}

#[test]
fn oversized_description_rejected() {
    let big = "x".repeat(4001);
    assert!(validate_description(&big).is_err());
}

#[test]
fn control_char_in_description_rejected() {
    assert!(validate_description("abc\u{0007}def").is_err());
}

#[test]
fn task_by_id_round_trips() {
    let (store, _) = open_store("task_by_id");
    let category = add_category(&store);
    let id = store.create_task(category, draft("写周报")).unwrap();
    let task = store.task(id).unwrap();
    assert_eq!(task.name, "写周报");
    assert_eq!(task.description, None);
}

#[test]
fn unknown_task_is_not_found() {
    let (store, _) = open_store("missing_task");
    assert!(matches!(
        store.task(TaskId(999)),
        Err(DomainError::NotFound { .. })
    ));
}

#[test]
fn description_round_trips() {
    let (store, _) = open_store("description");
    let category = add_category(&store);
    let id = store
        .create_task(
            category,
            TaskDraft {
                name: "x".to_owned(),
                frequency: Frequency::Daily,
                estimated_minutes: None,
                description: Some("初始说明".to_owned()),
            },
        )
        .unwrap();
    assert_eq!(
        store.task_description(id).unwrap(),
        Some("初始说明".to_owned())
    );
    store.set_task_description(id, "更新说明").unwrap();
    assert_eq!(
        store.task_description(id).unwrap(),
        Some("更新说明".to_owned())
    );
    store.set_task_description(id, "   ").unwrap();
    assert_eq!(store.task_description(id).unwrap(), None);
}

#[test]
fn set_description_on_missing_task_fails() {
    let (store, _) = open_store("desc_missing");
    assert!(matches!(
        store.set_task_description(TaskId(999), "x"),
        Err(DomainError::NotFound { .. })
    ));
}

#[test]
fn media_add_list_delete() {
    let (store, data_dir) = open_store("media");
    let category = add_category(&store);
    let id = store.create_task(category, draft("带媒体的任务")).unwrap();

    let source = write_sample_png(&data_dir);

    let media_store = MediaStore::new(&data_dir);
    let stored = media_store
        .store(id, &source, TaskMediaKind::Image)
        .unwrap();
    let recorded = store
        .add_task_media(
            id,
            TaskMediaKind::Image,
            &stored.file_name,
            &stored.original_name,
            stored.byte_size,
        )
        .unwrap();

    let listed = store.task_media(id).unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].file_name, stored.file_name);

    let path = media_store.path(id, &stored.file_name).unwrap();
    assert!(path.exists());

    let removed = store.delete_task_media(recorded.id).unwrap();
    assert_eq!(removed.file_name, stored.file_name);
    media_store.remove(id, &removed.file_name).unwrap();
    assert!(!path.exists());
    assert!(store.task_media(id).unwrap().is_empty());
}

#[test]
fn unsupported_audio_extension_rejected() {
    let (store, data_dir) = open_store("audio_bad");
    let category = add_category(&store);
    let id = store.create_task(category, draft("a")).unwrap();
    let source = data_dir.join("clip.txt");
    std::fs::write(&source, b"nope").unwrap();
    let media_store = MediaStore::new(&data_dir);
    assert!(
        media_store
            .store(id, &source, TaskMediaKind::Audio)
            .is_err()
    );
}
