//! 任务内容编辑器无头验收（T011）。
//!
//! 保存写回逻辑抽成了自由函数 `apply_edits`（不依赖 GPU/窗口），
//! 这里直接验证它把名称 / 内容 / 预计耗时正确写回 `Store`。

use std::path::{Path, PathBuf};

use crate::model::{Frequency, TaskDraft, TaskMediaKind};
use crate::store::Store;

use super::{add_media_file, apply_edits, file_uri, infer_kind, remove_media_file};

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

    apply_edits(
        &store,
        task_id,
        "新名称",
        "新的内容",
        Some(30),
        Frequency::EveryNDays(3),
    )
    .expect("写回应成功");

    let updated = store.task(task_id).expect("读取任务");
    assert_eq!(updated.name, "新名称", "名称应更新");
    assert_eq!(
        updated.description.as_deref(),
        Some("新的内容"),
        "内容应更新"
    );
    assert_eq!(updated.estimated_minutes, Some(30), "预计耗时应更新");
    assert_eq!(updated.frequency, Frequency::EveryNDays(3), "频率应更新");
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

    apply_edits(&store, task_id, "T", "新内容", None, Frequency::WeeklyN(2)).expect("写回应成功");

    let updated = store.task(task_id).expect("读取任务");
    assert_eq!(updated.estimated_minutes, None, "清空后应写回 None 而非 0");
    assert_eq!(updated.description.as_deref(), Some("新内容"), "内容应更新");
    assert_eq!(updated.frequency, Frequency::WeeklyN(2), "频率应更新");
}

/// `file_uri` 应把本地路径转成可被系统打开的 `file://` URI，
/// 并把 Windows 反斜杠规范为正斜杠。
#[test]
fn file_uri_normalizes_windows_separators() {
    let path = Path::new("C:\\Users\\me\\任务\\a.jpg");
    assert_eq!(
        file_uri(path),
        "file:///C:/Users/me/任务/a.jpg",
        "反斜杠应规范为正斜杠，并加 file:/// 前缀"
    );
}

/// `file_uri` 对已是正斜杠的路径保持原样。
#[test]
fn file_uri_keeps_forward_slashes() {
    let path = Path::new("/data/task_content/1/clip.mp4");
    assert_eq!(file_uri(path), "file:////data/task_content/1/clip.mp4");
}

/// `infer_kind` 应依据扩展名给出媒体类型，未知类型退化为图片。
#[test]
fn infer_kind_maps_extensions() {
    assert_eq!(infer_kind(Path::new("a.png")), TaskMediaKind::Image);
    assert_eq!(infer_kind(Path::new("a.JPG")), TaskMediaKind::Image);
    assert_eq!(infer_kind(Path::new("a.mp3")), TaskMediaKind::Audio);
    assert_eq!(infer_kind(Path::new("a.mov")), TaskMediaKind::Video);
    assert_eq!(infer_kind(Path::new("a.unknown")), TaskMediaKind::Image);
}

/// 添加媒体应把文件拷入任务内容目录并写入库记录；删除应清库并删磁盘文件。
#[test]
fn add_then_remove_media_writes_and_clears() {
    let (dir, store) = open_temp_store("media");
    let goal = store.create_goal("G", None).expect("目标");
    let category = store.create_category(goal, "分类").expect("分类");
    let task_id = store
        .create_task(
            category,
            TaskDraft {
                name: "t".to_owned(),
                frequency: Frequency::Daily,
                estimated_minutes: None,
                description: None,
            },
        )
        .expect("任务");

    let src = dir.join("clip.png");
    std::fs::write(&src, b"\x89PNG fake bytes").expect("写临时源文件");

    let media = add_media_file(&store, task_id, &src).expect("添加媒体");
    assert_eq!(media.kind, TaskMediaKind::Image, "png 应判为图片");
    assert_eq!(media.original_name, "clip.png", "应保留原始文件名");

    let on_disk = store
        .data_dir()
        .join("task_content")
        .join(task_id.0.to_string())
        .join(&media.file_name);
    assert!(on_disk.exists(), "媒体文件应已拷入任务内容目录");

    let listed = store.task_media(task_id).expect("读取媒体");
    assert_eq!(listed.len(), 1, "库中应有 1 条媒体记录");

    remove_media_file(&store, task_id, media.id);
    assert!(!on_disk.exists(), "删除后磁盘文件应被清理");
    assert!(
        store.task_media(task_id).expect("读取媒体").is_empty(),
        "删除后库中应无媒体记录"
    );
}
