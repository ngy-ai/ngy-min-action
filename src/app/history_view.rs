//! 历史待办页（T010）：分页加载历史行动，支持完成/跳过/撤销。
//!
//! 数据来自 `MinActionApp::history_actions`（初始化时由 `actions_page` 载入首页，可继续加载更多）。
//! 每条行动经 `settle_history_action` / `undo_history_action` 流转状态，结算后就地更新内存状态以保留分页。

use crate::palette::Palette;
use gpui::{ClickEvent, Entity, Window};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

use super::style::{SurfaceExt, TEXT_MD, Tone, empty_state};
use crate::app::MinActionApp;
use crate::app::today_view::status_text;
use crate::model::{Action, ActionId, ActionStatus};

/// 渲染历史待办列表：每条行动展示任务名+日期、状态与可用操作，底部「加载更多」。
pub(crate) fn render(entity: Entity<MinActionApp>, app: &MinActionApp) -> impl IntoElement {
    let rows: Vec<(Action, String)> = app
        .history_actions
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
        .id("history")
        .flex()
        .flex_col()
        .gap_2()
        .when(is_empty, |this| {
            this.child(empty_state(
                "还没有历史记录",
                "完成或跳过行动后，记录会出现在这里。",
                &palette,
            ))
        })
        .children(
            rows.into_iter()
                .map(|(action, name)| action_row(entity.clone(), action, name, &palette)),
        )
        .when(app.history_has_more, |this| {
            this.child(load_more_button(entity.clone(), &palette))
        })
}

/// 单行：任务名+日期 + 状态相关的操作按钮（完成/跳过/撤销）。
fn action_row(
    entity: Entity<MinActionApp>,
    action: Action,
    name: String,
    palette: &Palette,
) -> impl IntoElement {
    let action_id = action.id;
    let date = action.on_date.format("%Y-%m-%d").to_string();
    let status_label = status_text(action.status);
    let badge_color = match action.status {
        ActionStatus::Done => palette.success_fg,
        ActionStatus::Skipped | ActionStatus::Deferred | ActionStatus::Expired => {
            palette.warning_fg
        }
        ActionStatus::Pending => palette.text_muted2,
    };
    div()
        .id(format!("history-action-{}", action_id.0))
        .list_card(palette)
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
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(
                            div()
                                .text_size(TEXT_MD)
                                .font_weight(FontWeight::MEDIUM)
                                .child(name),
                        )
                        .child(
                            div()
                                .text_size(gpui_kit::px(11.))
                                .text_color(palette.text_muted)
                                .child(date),
                        ),
                )
                .child(
                    div()
                        .badge(palette)
                        .text_color(badge_color)
                        .child(status_label),
                ),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .when(action.status.is_settled(), |this| {
                    this.child(undo_button(entity.clone(), action_id, palette))
                })
                .when(action.status == ActionStatus::Pending, |this| {
                    this.child(action_button(
                        "history-done",
                        "完成",
                        Tone::Primary,
                        entity.clone(),
                        action_id,
                        ActionStatus::Done,
                        palette,
                    ))
                    .child(action_button(
                        "history-skip",
                        "跳过",
                        Tone::Neutral,
                        entity.clone(),
                        action_id,
                        ActionStatus::Skipped,
                        palette,
                    ))
                }),
        )
}

/// 完成/跳过按钮：点击经 `settle_history_action` 结算并就地刷新。
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
                    let _ = root.settle_history_action(action_id, next);
                    cx.notify();
                });
            },
        )
}

/// 撤销按钮：回滚最近一次结算并就地刷新。
fn undo_button(
    entity: Entity<MinActionApp>,
    action_id: ActionId,
    palette: &Palette,
) -> impl IntoElement {
    div()
        .id(format!("history-undo-{}", action_id.0))
        .tone_button(palette, Tone::Accent)
        .child("撤销")
        .on_click(
            move |_ev: &ClickEvent, _window: &mut Window, cx: &mut App| {
                entity.update(cx, |root, cx| {
                    let _ = root.undo_history_action(action_id);
                    cx.notify();
                });
            },
        )
}

/// 「加载更多」按钮：追加更早的一页历史行动。
fn load_more_button(entity: Entity<MinActionApp>, palette: &Palette) -> impl IntoElement {
    div()
        .id("history-more")
        .tone_button(palette, Tone::Neutral)
        .mt_2()
        .child("加载更多")
        .on_click(
            move |_ev: &ClickEvent, _window: &mut Window, cx: &mut App| {
                entity.update(cx, |root, cx| {
                    let _ = root.load_history_more();
                    cx.notify();
                });
            },
        )
}
