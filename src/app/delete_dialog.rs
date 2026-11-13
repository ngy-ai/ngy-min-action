//! 删除确认对话框（T014）：展示影响范围后软删除目标或分类。
//!
//! 以独立 gpui 视图渲染一个居中的确认面板，展示将被波及的分类/任务数量；
//! 确认时调用 `Store` 的 `delete_goal` / `delete_category`（已完成的行动与结算保留），
//! 并通知根视图 `reload()`（会归一化选择）后关闭对话框。

use std::sync::Arc;

use gpui::{App, Context, Entity, Render, Window};
use gpui_kit::*;

use crate::app::MinActionApp;
use crate::app::StructTarget;
use crate::model::DeletionImpact;
use crate::store::Store;

/// 删除确认对话框视图。
pub struct DeleteDialog {
    target: StructTarget,
    store: Arc<Store>,
    root: Entity<MinActionApp>,
    impact: DeletionImpact,
}

impl DeleteDialog {
    /// 构造对话框，需传入事先算好的影响范围。
    pub fn new(
        target: StructTarget,
        impact: DeletionImpact,
        store: Arc<Store>,
        root: Entity<MinActionApp>,
        _window: &mut Window,
        _cx: &mut App,
    ) -> Self {
        Self {
            target,
            store,
            root,
            impact,
        }
    }
}

impl Render for DeleteDialog {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let store = self.store.clone();
        let root = self.root.clone();
        let target = self.target;
        let impact = self.impact;
        div()
            .id("delete-dialog")
            .flex()
            .flex_col()
            .gap_3()
            .p_4()
            .w(px(360.))
            .child(div().text_size(px(18.)).child("删除确认"))
            .child(div().child(format!(
                "将一并删除 {} 个分类、{} 条任务（已完成的行动与结算记录保留）。",
                impact.categories, impact.tasks
            )))
            .child(
                div()
                    .flex()
                    .gap_2()
                    .justify_end()
                    .child(cancel_button(root.clone()))
                    .child(confirm_button(target, store, root)),
            )
    }
}

/// 「取消」按钮：仅关闭对话框。
fn cancel_button(root: Entity<MinActionApp>) -> impl IntoElement {
    div()
        .id("delete-cancel")
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

/// 「删除」按钮：执行删除后关闭。
fn confirm_button(
    target: StructTarget,
    store: Arc<Store>,
    root: Entity<MinActionApp>,
) -> impl IntoElement {
    div()
        .id("delete-confirm")
        .px_3()
        .py_1()
        .cursor_pointer()
        .rounded_md()
        .bg(rgb(0x88_33_33))
        .child("删除")
        .on_click(move |_ev, _window, cx| {
            let result = match target {
                StructTarget::Goal(id) => store.delete_goal(id),
                StructTarget::Category(id) => store.delete_category(id),
            };
            if let Err(e) = result {
                tracing::error!("删除失败：{e}");
            }
            root.update(cx, |root, cx| {
                let _ = root.reload();
                root.close_dialog();
                cx.notify();
            });
        })
}

#[cfg(test)]
mod tests;
