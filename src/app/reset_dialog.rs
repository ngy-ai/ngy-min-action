//! 重置复习进度对话框（T016 / 补 015 计划遗漏的 `reset_dialog.rs`）。
//!
//! 以独立 gpui 视图渲染一个居中确认面板：确认时写回 `Store::reset_review_progress`，
//! 把该任务的复习进度（遗忘曲线级别）归零；任务不存在时保留对话框并给出可读错误。

use std::sync::Arc;

use gpui::{Context, Entity, Render, Window};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

use crate::app::MinActionApp;
use crate::model::TaskId;
use crate::store::Store;

/// 重置复习进度对话框视图。
pub(crate) struct ResetDialog {
    task_id: TaskId,
    task_name: String,
    store: Arc<Store>,
    root: Entity<MinActionApp>,
    error: Option<String>,
}

impl ResetDialog {
    /// 构造对话框，展示来源任务名。
    pub(crate) fn new(
        task_id: TaskId,
        task_name: String,
        store: Arc<Store>,
        root: Entity<MinActionApp>,
    ) -> Self {
        Self {
            task_id,
            task_name,
            store,
            root,
            error: None,
        }
    }
}

impl Render for ResetDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let dialog_entity = cx.entity().clone();
        let task_id = self.task_id;
        let store = self.store.clone();
        let root = self.root.clone();
        div()
            .id("reset-dialog")
            .flex()
            .flex_col()
            .gap_3()
            .p_4()
            .w(px(360.))
            .child(div().text_size(px(18.)).child("重置复习进度"))
            .child(div().text_size(px(13.)).child(format!(
                "确认将《{}》的复习进度重置为第 0 级？此操作不可撤销。",
                self.task_name
            )))
            .when_some(self.error.clone(), |this, message| {
                this.child(div().text_color(rgb(0xff_55_55)).child(message))
            })
            .child(
                div()
                    .flex()
                    .gap_2()
                    .justify_end()
                    .child(cancel_button(root.clone()))
                    .child(confirm_button(task_id, store, root, dialog_entity)),
            )
    }
}

/// 「取消」按钮：仅关闭对话框。
fn cancel_button(root: Entity<MinActionApp>) -> impl IntoElement {
    div()
        .id("reset-cancel")
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

/// 「确认重置」按钮：写回归零后关闭；任务不存在时保留对话框并提示。
fn confirm_button(
    task_id: TaskId,
    store: Arc<Store>,
    root: Entity<MinActionApp>,
    dialog: Entity<ResetDialog>,
) -> impl IntoElement {
    div()
        .id("reset-confirm")
        .px_3()
        .py_1()
        .cursor_pointer()
        .rounded_md()
        .bg(rgb(0x33_33_38))
        .child("确认重置")
        .on_click(
            move |_ev, _window, cx| match store.reset_review_progress(task_id) {
                Ok(()) => {
                    root.update(cx, |root, cx| {
                        let _ = root.reload();
                        root.close_dialog();
                        cx.notify();
                    });
                }
                Err(error) => {
                    dialog.update(cx, |dialog, cx| {
                        dialog.error = Some(error.to_string());
                        cx.notify();
                    });
                }
            },
        )
}

#[cfg(test)]
mod tests;
