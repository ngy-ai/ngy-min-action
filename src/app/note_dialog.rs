//! 行动备注对话框（T012）：左右两栏展示来源任务与当前备注。
//!
//! 左栏为来源任务的标题与富内容（描述），右栏为可编辑的备注文本框；保存时经
//! `Store::set_action_note` 写库（空白被规范为清除），非法内容给出可读错误且不关闭。
//! 数据层零改动。

use std::sync::Arc;

use crate::palette::Palette;
use gpui::{App, Context, Entity, FontWeight, Render, Window};
use gpui_kit::component::input::{Textarea, TextareaState};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

use crate::app::MinActionApp;
use crate::model::{ActionId, DomainError};
use crate::store::Store;

/// 备注对话框的来源任务上下文：标题、富内容描述与既有备注。
pub(crate) struct NoteContext {
    pub task_name: String,
    pub task_description: Option<String>,
    pub current_note: Option<String>,
}

/// 行动备注对话框视图。
pub(crate) struct NoteDialog {
    action_id: ActionId,
    task_name: String,
    task_description: Option<String>,
    note_state: Entity<TextareaState>,
    store: Arc<Store>,
    root: Entity<MinActionApp>,
    error: Option<String>,
}

impl NoteDialog {
    /// 用来源任务上下文与既有备注构造对话框。
    pub(crate) fn new(
        context: NoteContext,
        action_id: ActionId,
        store: Arc<Store>,
        root: Entity<MinActionApp>,
        window: &mut Window,
        cx: &mut App,
    ) -> Self {
        let note_state = cx.new(|c| {
            TextareaState::new(window, c)
                .default_value(context.current_note.unwrap_or_default().as_str())
        });
        Self {
            action_id,
            task_name: context.task_name,
            task_description: context.task_description,
            note_state,
            store,
            root,
            error: None,
        }
    }
}

impl Render for NoteDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let task_name = self.task_name.clone();
        let task_description = self.task_description.clone();
        let note_state = self.note_state.clone();
        let root = self.root.clone();
        let store = self.store.clone();
        let action_id = self.action_id;
        let dialog = cx.entity().clone();
        let error = self.error.clone();
        let p = Palette::from_cx(cx);

        div()
            .id("note-dialog")
            .flex()
            .flex_col()
            .gap_2()
            .p_4()
            .w(px(640.))
            .child(
                div()
                    .text_size(px(18.))
                    .child(format!("备注 · {task_name}")),
            )
            .child(
                div()
                    .flex()
                    .gap_3()
                    .min_h(px(220.))
                    .child(
                        div()
                            .id("src-col")
                            .flex()
                            .flex_col()
                            .gap_1()
                            .flex_1()
                            .border_r_1()
                            .pr_3()
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(p.text_muted)
                                    .child("来源任务"),
                            )
                            .child(div().font_weight(FontWeight::BOLD).child(task_name.clone()))
                            .when_some(task_description.clone(), |this, desc| {
                                this.child(
                                    div()
                                        .text_size(px(12.))
                                        .text_color(p.text_muted2)
                                        .child(desc),
                                )
                            }),
                    )
                    .child(
                        div()
                            .id("note-col")
                            .flex()
                            .flex_col()
                            .gap_1()
                            .flex_1()
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(p.text_muted)
                                    .child("当前备注"),
                            )
                            .child(Textarea::new(&note_state).h(px(200.))),
                    ),
            )
            .when_some(error, |this, msg| {
                this.child(div().text_color(rgb(0xff_88_88)).child(msg))
            })
            .child(
                div()
                    .flex()
                    .gap_2()
                    .justify_end()
                    .child(cancel_button(root.clone()))
                    .child(save_button(dialog, action_id, note_state, store, root, &p)),
            )
    }
}

/// 「取消」按钮：仅关闭对话框。
fn cancel_button(root: Entity<MinActionApp>) -> impl IntoElement {
    div()
        .id("note-cancel")
        .px_3()
        .py_1()
        .cursor_pointer()
        .rounded_md()
        .child("取消")
        .on_click(move |_ev, _window, cx| {
            root.update(cx, |root, cx| {
                root.close_dialog();
                cx.notify();
            });
        })
}

/// 「保存」按钮：写回备注；成功则刷新今日列表并关闭，失败显示错误且不关闭。
fn save_button(
    dialog: Entity<NoteDialog>,
    action_id: ActionId,
    note_state: Entity<TextareaState>,
    store: Arc<Store>,
    root: Entity<MinActionApp>,
    palette: &Palette,
) -> impl IntoElement {
    div()
        .id("note-save")
        .px_3()
        .py_1()
        .cursor_pointer()
        .rounded_md()
        .bg(palette.raised)
        .child("保存")
        .on_click(move |_ev, _window, cx| {
            let note = note_state.read(cx).value();
            match store.set_action_note(action_id, &note) {
                Ok(()) => root.update(cx, |root, cx| {
                    let _ = root.reload_today();
                    root.close_dialog();
                    cx.notify();
                }),
                Err(e) => dialog.update(cx, |this, cx| {
                    this.error = Some(format_note_error(&e));
                    cx.notify();
                }),
            }
        })
}

/// 把写备注失败的错误转成对话框内的可读文案。
pub(crate) fn format_note_error(e: &DomainError) -> String {
    format!("保存失败：{e}")
}

#[cfg(test)]
mod tests;
