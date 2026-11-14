//! 重命名对话框（T014 头一项）：对目标或分类重命名。
//!
//! 以独立 gpui 视图渲染一个居中的输入面板，确认时写回 `Store` 的
//! `rename_goal` / `rename_category`，并通知根视图 `reload()` 后关闭对话框。

use std::sync::Arc;

use crate::palette::Palette;
use gpui::{App, Context, Entity, Render, Window};
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::*;

use crate::app::MinActionApp;
use crate::app::StructTarget;
use crate::store::Store;

/// 重命名对话框视图。
pub struct RenameDialog {
    target: StructTarget,
    store: Arc<Store>,
    root: Entity<MinActionApp>,
    name: Entity<InputState>,
}

impl RenameDialog {
    /// 构造对话框，输入框预填当前名称。
    pub fn new(
        target: StructTarget,
        current_name: &str,
        store: Arc<Store>,
        root: Entity<MinActionApp>,
        window: &mut Window,
        cx: &mut App,
    ) -> Self {
        let name = cx.new(|c| InputState::new(window, c).default_value(current_name));
        Self {
            target,
            store,
            root,
            name,
        }
    }
}

impl Render for RenameDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let name_state = self.name.clone();
        let store = self.store.clone();
        let root = self.root.clone();
        let target = self.target;
        let p = Palette::from_cx(cx);
        div()
            .id("rename-dialog")
            .flex()
            .flex_col()
            .gap_3()
            .p_4()
            .w(px(360.))
            .child(div().text_size(px(18.)).child("重命名"))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(div().text_size(px(12.)).child("名称"))
                    .child(Input::new(&name_state)),
            )
            .child(
                div()
                    .flex()
                    .gap_2()
                    .justify_end()
                    .child(cancel_button(root.clone()))
                    .child(confirm_button(target, name_state, store, root, &p)),
            )
    }
}

/// 「取消」按钮：仅关闭对话框。
fn cancel_button(root: Entity<MinActionApp>) -> impl IntoElement {
    div()
        .id("rename-cancel")
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

/// 「确定」按钮：写回新名称后关闭。
fn confirm_button(
    target: StructTarget,
    name_state: Entity<InputState>,
    store: Arc<Store>,
    root: Entity<MinActionApp>,
    palette: &Palette,
) -> impl IntoElement {
    div()
        .id("rename-confirm")
        .px_3()
        .py_1()
        .cursor_pointer()
        .rounded_md()
        .bg(palette.raised)
        .child("确定")
        .on_click(move |_ev, _window, cx| {
            let name = name_state.read(cx).value().trim().to_string();
            if name.is_empty() {
                return;
            }
            let result = match target {
                StructTarget::Goal(id) => store.rename_goal(id, &name),
                StructTarget::Category(id) => store.rename_category(id, &name),
            };
            if let Err(e) = result {
                tracing::error!("重命名失败：{e}");
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
