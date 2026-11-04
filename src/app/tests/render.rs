//! headless 渲染烟测：三个视图与三个弹窗各跑一帧。

use super::*;

use crate::app::{PendingDelete, RenameTarget};
use eframe::egui;

/// 跑一帧 headless 渲染，并丢弃纹理增量（无窗口时没人消费它）。
fn render_one_frame(ctx: &egui::Context, mut draw: impl FnMut(&mut egui::Ui)) {
    let mut output = ctx.run_ui(egui::RawInput::default(), |ui| draw(ui));
    output.textures_delta.clear();
}

#[test]
fn dialogs_and_views_render_without_panicking() {
    let mut app = open_app("render");
    let category = seed_category(&mut app, "学习英语", "口语");
    let task = app
        .store
        .create_task(category, draft("连读"))
        .expect("创建任务");
    app.reload().expect("刷新界面数据");

    let ctx = egui::Context::default();
    render_one_frame(&ctx, |ui| app.ui_main_view(ui));
    render_one_frame(&ctx, |ui| app.ui_today_view(ui));

    app.open_rename_dialog(RenameTarget::Task(task), "连读");
    render_one_frame(&ctx, |ui| {
        let ctx = ui.ctx().clone();
        app.ui_rename_dialog(&ctx);
    });

    assert!(!app.rename_select_all, "渲染后应清除待全选标记");
    let edit_id = egui::Id::unique("dialog-rename-name");
    let state =
        egui::widgets::text_edit::TextEditState::load(&ctx, edit_id).expect("输入框应已写入状态");
    let range = state
        .cursor
        .char_range()
        .expect("打开弹窗后应全选输入框内容");
    assert_eq!(
        range.primary.index,
        egui::text::CharIndex(2),
        "「连读」两个字符都应被选中"
    );
    assert_eq!(
        range.secondary.index,
        egui::text::CharIndex(0),
        "选区应从开头起算"
    );

    app.request_delete(PendingDelete::Task(task));
    render_one_frame(&ctx, |ui| {
        let ctx = ui.ctx().clone();
        app.ui_delete_dialog(&ctx);
    });

    let current = app.tasks[0].frequency;
    app.open_frequency_dialog(task, current);
    render_one_frame(&ctx, |ui| {
        let ctx = ui.ctx().clone();
        app.ui_frequency_dialog(&ctx);
    });
}
