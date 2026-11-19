//! 新建目标 / 分类 / 任务的对话框与写库逻辑。
//!
//! 沿用历史交互：先选中目标/分类作为归属，再新建下一级对象。新任务默认「每天一次」，
//! 之后可在任务编辑器调整频率与耗时。

use std::sync::Arc;

use gpui::{App, Context, Entity, Render, Window};
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

use crate::app::MinActionApp;
use crate::model::{CategoryId, Frequency, GoalId, TaskDraft};
use crate::palette::Palette;
use crate::store::Store;

/// 新建对象及其归属：目标无归属，分类归属目标，任务归属分类。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CreateKind {
    Goal,
    Category { goal: Option<GoalId> },
    Task { category: Option<CategoryId> },
}

/// 新建对话框视图。
pub(crate) struct CreateDialog {
    kind: CreateKind,
    scope_label: String,
    store: Arc<Store>,
    root: Entity<MinActionApp>,
    name: Entity<InputState>,
    error: Option<String>,
}

impl CreateDialog {
    /// 构造对话框：名称输入框留空，归属说明由调用方给出。
    pub(crate) fn new(
        kind: CreateKind,
        scope_label: String,
        store: Arc<Store>,
        root: Entity<MinActionApp>,
        window: &mut Window,
        cx: &mut App,
    ) -> Self {
        let name = cx.new(|c| InputState::new(window, c));
        Self {
            kind,
            scope_label,
            store,
            root,
            name,
            error: None,
        }
    }
}

impl Render for CreateDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let name_state = self.name.clone();
        let store = self.store.clone();
        let root = self.root.clone();
        let dialog = cx.entity().clone();
        let kind = self.kind;
        let title = match kind {
            CreateKind::Goal => "新建目标",
            CreateKind::Category { .. } => "新建分类",
            CreateKind::Task { .. } => "新建任务",
        };
        let scope = self.scope_label.clone();
        let error = self.error.clone();
        let p = Palette::from_cx(cx);

        div()
            .id("create-dialog")
            .flex()
            .flex_col()
            .gap_3()
            .p_4()
            .w(px(380.))
            .child(div().text_size(px(18.)).child(title))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(p.text_muted)
                            .child(scope),
                    )
                    .child(div().text_size(px(12.)).child("名称"))
                    .child(Input::new(&name_state)),
            )
            .when_some(error, |this, msg| {
                this.child(div().text_color(p.danger_fg).child(msg))
            })
            .child(
                div()
                    .flex()
                    .gap_2()
                    .justify_end()
                    .child(cancel_button(root.clone()))
                    .child(confirm_button(dialog, kind, name_state, store, root, &p)),
            )
    }
}

/// 「取消」按钮：仅关闭对话框。
fn cancel_button(root: Entity<MinActionApp>) -> impl IntoElement {
    div()
        .id("create-cancel")
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

/// 「确定」按钮：写库成功后重载并关闭；失败留在对话框内显示原因。
fn confirm_button(
    dialog: Entity<CreateDialog>,
    kind: CreateKind,
    name_state: Entity<InputState>,
    store: Arc<Store>,
    root: Entity<MinActionApp>,
    palette: &Palette,
) -> impl IntoElement {
    div()
        .id("create-confirm")
        .px_3()
        .py_1()
        .cursor_pointer()
        .rounded_md()
        .bg(palette.raised)
        .child("确定")
        .on_click(move |_ev, _window, cx| {
            let name = name_state.read(cx).value().trim().to_owned();
            if name.is_empty() {
                dialog.update(cx, |this, cx| {
                    this.error = Some("名称不能为空".to_owned());
                    cx.notify();
                });
                return;
            }
            match create_entity(&store, kind, &name) {
                Ok(()) => root.update(cx, |root, cx| {
                    let _ = root.reload();
                    root.close_dialog();
                    cx.notify();
                }),
                Err(e) => dialog.update(cx, |this, cx| {
                    this.error = Some(e);
                    cx.notify();
                }),
            }
        })
}

/// 按层级写入数据层；无归属时给出可执行的提示而不是静默失败。
pub(crate) fn create_entity(store: &Store, kind: CreateKind, name: &str) -> Result<(), String> {
    match kind {
        CreateKind::Goal => store
            .create_goal(name, None)
            .map(|_| ())
            .map_err(|e| e.to_string()),
        CreateKind::Category { goal: Some(goal) } => store
            .create_category(goal, name)
            .map(|_| ())
            .map_err(|e| e.to_string()),
        CreateKind::Category { goal: None } => Err("请先创建目标，再添加分类".to_owned()),
        CreateKind::Task {
            category: Some(category),
        } => store
            .create_task(
                category,
                TaskDraft {
                    name: name.to_owned(),
                    frequency: Frequency::Daily,
                    estimated_minutes: None,
                    description: None,
                },
            )
            .map(|_| ())
            .map_err(|e| e.to_string()),
        CreateKind::Task { category: None } => Err("请先选择分类，再添加任务".to_owned()),
    }
}

#[cfg(test)]
mod tests;
