//! 今日行动页（T009）：展示当日「最小行动」列表，支持完成/跳过/撤销，以及备注（T012）。
//!
//! 数据来自 `MinActionApp::today_actions`（已在 `reload` 时由 scheduler 幂等生成）。每条行动经
//! `settle_today_action` / `undo_today_action` 流转状态；「备注」按钮打开左右两栏备注对话框。

use crate::palette::Palette;
use gpui::{ClickEvent, Entity, Window};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

use super::style::{SurfaceExt, TEXT_MD, Tone, empty_state};
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

    let palette = app.palette;
    let is_empty = rows.is_empty();
    div()
        .id("today")
        .flex()
        .flex_col()
        .gap_2()
        .when(is_empty, |this| {
            this.child(empty_state(
                "今天还没有行动",
                "任务会按频率生成今日待办，先去添加一个任务吧。",
                &palette,
            ))
        })
        .children(
            rows.into_iter()
                .map(move |(action, name)| action_row(entity.clone(), action, name, palette)),
        )
}

/// 单行：任务名 + 备注入口 + 状态相关的操作按钮（完成/跳过/撤销）。
fn action_row(
    entity: Entity<MinActionApp>,
    action: Action,
    name: String,
    palette: Palette,
) -> impl IntoElement {
    let action_id = action.id;
    let status_label = status_text(action.status);
    let badge_color = match action.status {
        ActionStatus::Done => palette.success_fg,
        ActionStatus::Skipped | ActionStatus::Deferred | ActionStatus::Expired => {
            palette.warning_fg
        }
        ActionStatus::Pending => palette.text_muted2,
    };
    div()
        .id(format!("today-action-{}", action_id.0))
        .list_card(&palette)
        .flex()
        .items_center()
        .justify_between()
        .gap_2()
        .px_3()
        .py_2()
        .child(
            div()
                .flex_1()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .text_size(TEXT_MD)
                        .font_weight(FontWeight::MEDIUM)
                        .child(name),
                )
                .child(
                    div()
                        .badge(&palette)
                        .text_color(badge_color)
                        .child(status_label),
                ),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(note_button(entity.clone(), action_id, &palette))
                .when(action.status.is_settled(), |this| {
                    this.child(undo_button(entity.clone(), action_id, &palette))
                })
                .when(action.status == ActionStatus::Pending, |this| {
                    this.child(action_button(
                        "today-done",
                        "完成",
                        Tone::Primary,
                        entity.clone(),
                        action_id,
                        ActionStatus::Done,
                        &palette,
                    ))
                    .child(action_button(
                        "today-skip",
                        "跳过",
                        Tone::Neutral,
                        entity.clone(),
                        action_id,
                        ActionStatus::Skipped,
                        &palette,
                    ))
                }),
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
    tone: Tone,
    entity: Entity<MinActionApp>,
    action_id: ActionId,
    next: ActionStatus,
    palette: &Palette,
) -> impl IntoElement {
    div()
        .id(format!("{id}-{}", action_id.0))
        .tone_button(palette, tone)
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
fn undo_button(
    entity: Entity<MinActionApp>,
    action_id: ActionId,
    palette: &Palette,
) -> impl IntoElement {
    div()
        .id(format!("today-undo-{}", action_id.0))
        .tone_button(palette, Tone::Accent)
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
fn note_button(
    entity: Entity<MinActionApp>,
    action_id: ActionId,
    palette: &Palette,
) -> impl IntoElement {
    div()
        .id(format!("today-note-{}", action_id.0))
        .tone_button(palette, Tone::Neutral)
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
