//! 013：任务内容编辑弹窗与备注弹窗的来源任务展示。

use super::*;

use crate::attachment::MediaStore;
use crate::model::{TaskId, TaskMediaKind};

fn task_with_content(app: &mut MinActionApp) -> TaskId {
    let category = seed_category(app, "目标", "分类");
    app.store
        .create_task(
            category,
            TaskDraft {
                name: "写周报".to_owned(),
                frequency: Frequency::Daily,
                estimated_minutes: None,
                description: Some("见 https://example.com/guide 完成".to_owned()),
            },
        )
        .expect("创建任务")
}

fn temp_png(tag: &str) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!(
        "ngy-min-action-task-media-{}-{tag}.png",
        std::process::id()
    ));
    image::RgbaImage::new(2, 2).save(&path).expect("写测试图片");
    path
}

#[test]
fn task_detail_loads_description_and_media() {
    let mut app = open_app("task-detail-load");
    let task = task_with_content(&mut app);

    let media_store = MediaStore::new(&app.data_dir);
    let source = temp_png("load");
    let stored = media_store
        .store(task, &source, TaskMediaKind::Image)
        .expect("落盘媒体");
    app.store
        .add_task_media(
            task,
            TaskMediaKind::Image,
            &stored.file_name,
            &stored.original_name,
            stored.byte_size,
        )
        .expect("记录媒体");

    app.open_task_detail(task);
    let editor = app.task_detail.as_ref().expect("弹窗应打开");
    assert_eq!(editor.name, "写周报");
    assert_eq!(editor.description, "见 https://example.com/guide 完成");
    assert_eq!(editor.media.len(), 1);
}

#[test]
fn task_detail_saves_description() {
    let mut app = open_app("task-detail-save");
    let task = task_with_content(&mut app);

    app.open_task_detail(task);
    let id = {
        let editor = app.task_detail.as_mut().expect("弹窗应打开");
        editor.description = "新内容 https://a.com".to_owned();
        editor.task_id
    };
    app.save_task_detail(id, "新内容 https://a.com");

    assert!(app.task_detail.is_none(), "保存后弹窗应关闭");
    assert_eq!(
        app.store.task_description(task).expect("读取内容"),
        Some("新内容 https://a.com".to_owned())
    );
}

#[test]
fn note_shows_source_task() {
    let mut app = open_app("note-source");
    let task = task_with_content(&mut app);
    app.reload().expect("生成今天的行动");
    let action = app
        .actions
        .iter()
        .find(|item| item.task_id == task)
        .expect("应有来自该任务的行动")
        .id;

    app.open_note(action, false);
    let editor = app.note.as_ref().expect("备注弹窗应打开");
    let source = editor.source.as_ref().expect("应加载来源任务");
    assert_eq!(source.name, "写周报");
    assert_eq!(
        source.description.as_deref(),
        Some("见 https://example.com/guide 完成")
    );
    assert!(!editor.read_only);
}

#[test]
fn read_only_note_still_shows_source() {
    let mut app = open_app("note-source-ro");
    let task = task_with_content(&mut app);
    app.reload().expect("生成今天的行动");
    let action = app
        .actions
        .iter()
        .find(|item| item.task_id == task)
        .expect("应有来自该任务的行动")
        .id;

    app.open_note(action, true);
    let editor = app.note.as_ref().expect("备注弹窗应打开");
    assert!(editor.read_only);
    assert!(editor.source.is_some(), "只读态也应展示来源任务");
}
