//! 今日行动页（T009）：展示当日「最小行动」列表，支持完成/跳过/撤销，以及备注（T012）。
//!
//! 数据来自 `MinActionApp::today_actions`（已在 `reload` 时由 scheduler 幂等生成）。每条行动经
//! `settle_today_action` / `undo_today_action` 流转状态；「备注」按钮打开左右两栏备注对话框。

use gpui::{ClickEvent, Entity, Window};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

use crate::app::MinActionApp;
use crate::model::{Action, ActionId, ActionStatus};

/// 渲染今日行动列表：每条行动展示任务名、状态与可用操作。
pub(crate) fn render(entity: Entity<MinActionApp>, app: &MinActionApp) -> impl IntoElement {
    let rows: Vec<(Action, String)> = app
        .today_actions
        .iter()
        .map(|action| {
            let name = app
                .all_tasks
                .iter()
                .find(|task| task.id == action.task_id)
                .map(|task| task.name.clone())
                .unwrap_or_else(|| "（未知任务）".to_string());
            (action.clone(), name)
        })
        .collect();

    div().id("today").flex().flex_col().gap_2().children(
        rows.into_iter()
            .map(move |(action, name)| action_row(entity.clone(), action, name)),
    )
}

/// 单行：任务名 + 备注入口 + 状态相关的操作按钮（完成/跳过/撤销）。
fn action_row(entity: Entity<MinActionApp>, action: Action, name: String) -> impl IntoElement {
    let action_id = action.id;
    let status_label = status_text(action.status);
    div()
        .id(format!("today-action-{}", action_id.0))
        .flex()
        .items_center()
        .justify_between()
        .px_2()
        .py_1()
        .rounded_md()
        .bg(rgb(0x24_24_28))
        .child(div().flex_1().child(name))
        .child(
            div()
                .flex()
                .gap_1()
                .child(note_button(entity.clone(), action_id))
                .when(action.status.is_settled(), |this| {
                    this.child(undo_button(entity.clone(), action_id))
                })
                .when(action.status == ActionStatus::Pending, |this| {
                    this.child(action_button(
                        "today-done",
                        "完成",
                        entity.clone(),
                        action_id,
                        ActionStatus::Done,
                    ))
                    .child(action_button(
                        "today-skip",
                        "跳过",
                        entity.clone(),
                        action_id,
                        ActionStatus::Skipped,
                    ))
                })
                .when(
                    action.status != ActionStatus::Pending && !action.status.is_settled(),
                    |this| this.child(div().child(status_label)),
                ),
        )
}

/// 把行动状态映射成中文标签。
pub(crate) fn status_text(status: ActionStatus) -> &'static str {
    match status {
        ActionStatus::Pending => "待完成",
        ActionStatus::Done => "已完成",
        ActionStatus::Skipped => "已跳过",
        ActionStatus::Deferred => "已顺延",
        ActionStatus::Expired => "已过期",
    }
}

/// 完成/跳过按钮：点击经 `settle_today_action` 结算并刷新。
fn action_button(
    id: &'static str,
    label: &'static str,
    entity: Entity<MinActionApp>,
    action_id: ActionId,
    next: ActionStatus,
) -> impl IntoElement {
    div()
        .id(id)
        .px_1()
        .py_0()
        .cursor_pointer()
        .text_size(px(11.))
        .text_color(rgb(0x99_99_99))
        .hover(|this| this.text_color(rgb(0xcc_cc_cc)))
        .child(label)
        .on_click(
            move |_ev: &ClickEvent, _window: &mut Window, cx: &mut App| {
                entity.update(cx, |root, cx| {
                    let _ = root.settle_today_action(action_id, next);
                    cx.notify();
                });
            },
        )
}

/// 撤销按钮：回滚最近一次结算并刷新。
fn undo_button(entity: Entity<MinActionApp>, action_id: ActionId) -> impl IntoElement {
    div()
        .id("today-undo")
        .px_1()
        .py_0()
        .cursor_pointer()
        .text_size(px(11.))
        .text_color(rgb(0x99_99_99))
        .hover(|this| this.text_color(rgb(0xcc_cc_cc)))
        .child("撤销")
        .on_click(
            move |_ev: &ClickEvent, _window: &mut Window, cx: &mut App| {
                entity.update(cx, |root, cx| {
                    let _ = root.undo_today_action(action_id);
                    cx.notify();
                });
            },
        )
}

/// 备注按钮：打开左右两栏备注对话框，带来源任务标题、描述与既有备注。
fn note_button(entity: Entity<MinActionApp>, action_id: ActionId) -> impl IntoElement {
    div()
        .id("today-note")
        .px_1()
        .py_0()
        .cursor_pointer()
        .text_size(px(11.))
        .text_color(rgb(0x99_99_99))
        .hover(|this| this.text_color(rgb(0xcc_cc_cc)))
        .child("备注")
        .on_click(move |_ev: &ClickEvent, window: &mut Window, cx: &mut App| {
            entity.update(cx, |root, cx| {
                let app_entity = cx.entity().clone();
                if let Some(action) = root.today_actions.iter().find(|a| a.id == action_id) {
                    let task = root.all_tasks.iter().find(|t| t.id == action.task_id);
                    let context = super::note_dialog::NoteContext {
                        task_name: task.map(|t| t.name.clone()).unwrap_or_default(),
                        task_description: task.and_then(|t| t.description.clone()),
                        current_note: action.note.clone(),
                    };
                    root.open_note_dialog(app_entity, action_id, context, window, cx);
                    cx.notify();
                }
            });
        })
}
