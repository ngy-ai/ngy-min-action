//! 待办备注：保存、清空、图片附件与超长拒绝。

use super::*;

use eframe::egui;

use crate::app::note_dialog::image_uri;
use crate::model::ActionId;

fn seed_action(app: &mut MinActionApp) -> ActionId {
    let category = seed_category(app, "学习英语", "口语");
    app.store
        .create_task(category, draft("跟读十分钟"))
        .expect("创建任务");
    app.reload().expect("刷新界面数据");
    app.actions.first().expect("今天应有行动").id
}

fn temp_png(tag: &str) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!(
        "ngy-min-action-note-image-{}-{tag}.png",
        std::process::id()
    ));
    image::RgbaImage::new(2, 2).save(&path).expect("写测试图片");
    path
}

#[test]
fn note_saves_and_marks_the_row() {
    let mut app = open_app("note-save");
    let action = seed_action(&mut app);

    app.open_note(action, false);
    let editor = app.note.as_mut().expect("备注窗口应打开");
    assert!(!editor.read_only);
    editor.text = "  今天练了连读  ".to_owned();
    app.save_note(action);

    assert!(app.note.is_none(), "保存后窗口应关闭");
    assert_eq!(
        app.store.action_note(action).expect("读取备注"),
        Some("今天练了连读".to_owned())
    );
    assert!(app.actions[0].note.is_some(), "列表标记应刷新");
}

#[test]
fn blank_note_clears_the_row() {
    let mut app = open_app("note-clear");
    let action = seed_action(&mut app);
    app.store
        .set_action_note(action, "先写一条")
        .expect("写备注");
    app.reload().expect("刷新界面数据");

    app.open_note(action, false);
    app.note.as_mut().expect("备注窗口").text = "   ".to_owned();
    app.save_note(action);

    assert_eq!(app.store.action_note(action).expect("读取备注"), None);
    assert!(app.actions[0].note.is_none());
}

#[test]
fn images_are_copied_and_removed_with_the_record() {
    let mut app = open_app("note-image");
    let action = seed_action(&mut app);
    let source = temp_png("add");

    app.open_note(action, false);
    app.add_note_images(action, vec![source.clone()]);
    let editor = app.note.as_ref().expect("备注窗口");
    assert_eq!(editor.images.len(), 1);
    let image = editor.images[0].clone();
    let stored = app
        .data_dir
        .join("attachments")
        .join(action.0.to_string())
        .join(&image.file_name);
    assert!(stored.exists(), "图片应复制进数据目录");
    assert_eq!(app.store.action_images(action).expect("读图片").len(), 1);

    app.remove_note_image(action, image.id);
    assert!(app.note.as_ref().expect("备注窗口").images.is_empty());
    assert!(!stored.exists(), "移除后磁盘副本应删除");
    assert!(app.store.action_images(action).expect("读图片").is_empty());
}

#[test]
fn note_image_uri_is_loadable_by_the_file_loader() {
    let ctx = egui::Context::default();
    egui_extras::install_image_loaders(&ctx);
    let source = temp_png("uri");
    let uri = image_uri(&source);

    let mut waited = 0;
    let bytes = loop {
        match ctx.try_load_bytes(&uri) {
            Ok(egui::load::BytesPoll::Ready { bytes, .. }) => break bytes,
            Ok(egui::load::BytesPoll::Pending { .. }) => {
                std::thread::sleep(std::time::Duration::from_millis(10));
                waited += 1;
                assert!(waited < 200, "文件加载超时：{uri}");
            }
            Err(error) => panic!("加载器读不到 {uri}：{error}"),
        }
    };
    assert!(!bytes.is_empty(), "读到的图片字节不应为空");
}

#[test]
fn image_uri_keeps_unc_paths() {
    assert_eq!(
        image_uri(std::path::Path::new(r"\\server\share\预览.png")),
        "file://server/share/预览.png"
    );
}

#[test]
fn overlong_note_keeps_dialog_open_with_error() {
    let mut app = open_app("note-too-long");
    let action = seed_action(&mut app);

    app.open_note(action, false);
    app.note.as_mut().expect("备注窗口").text = "长".repeat(2001);
    app.save_note(action);

    let editor = app.note.as_ref().expect("保存失败应保留窗口");
    assert!(
        editor
            .error
            .as_deref()
            .is_some_and(|text| text.contains("备注"))
    );
    assert_eq!(app.store.action_note(action).expect("读取备注"), None);
}
