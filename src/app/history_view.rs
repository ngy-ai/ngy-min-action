//! 历史待办页（T010）：分页加载历史行动，支持完成/跳过/撤销。
//!
//! 数据来自 `MinActionApp::history_actions`（初始化时由 `actions_page` 载入首页，可继续加载更多）。
//! 每条行动经 `settle_history_action` / `undo_history_action` 流转状态，结算后就地更新内存状态以保留分页。

use crate::palette::Palette;
use gpui::{ClickEvent, Entity, Window};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

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

    div()
        .id("history")
        .flex()
        .flex_col()
        .gap_2()
        .children(
            rows.into_iter()
                .map(|(action, name)| action_row(entity.clone(), action, name, &app.palette)),
        )
        .when(app.history_has_more, |this| {
            this.child(load_more_button(entity.clone(), &app.palette))
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
    div()
        .id(format!("history-action-{}", action_id.0))
        .flex()
        .items_center()
        .justify_between()
        .px_2()
        .py_1()
        .rounded_md()
        .bg(palette.panel)
        .child(div().flex_1().child(format!("{name} · {date}")))
        .child(
            div()
                .flex()
                .gap_1()
                .when(action.status.is_settled(), |this| {
                    this.child(undo_button(entity.clone(), action_id, palette))
                })
                .when(action.status == ActionStatus::Pending, |this| {
                    this.child(action_button(
                        "history-done",
                        "完成",
                        entity.clone(),
                        action_id,
                        ActionStatus::Done,
                        palette,
                    ))
                    .child(action_button(
                        "history-skip",
                        "跳过",
                        entity.clone(),
                        action_id,
                        ActionStatus::Skipped,
                        palette,
                    ))
                })
                .when(
                    action.status != ActionStatus::Pending && !action.status.is_settled(),
                    |this| this.child(div().child(status_label)),
                ),
        )
}

/// 完成/跳过按钮：点击经 `settle_history_action` 结算并就地刷新。
fn action_button(
    id: &'static str,
    label: &'static str,
    entity: Entity<MinActionApp>,
    action_id: ActionId,
    next: ActionStatus,
    palette: &Palette,
) -> impl IntoElement {
    div()
        .id(id)
        .px_1()
        .py_0()
        .cursor_pointer()
        .text_size(px(11.))
        .text_color(palette.text_muted)
        .hover(|this| this.text_color(palette.text))
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
        .id("history-undo")
        .px_1()
        .py_0()
        .cursor_pointer()
        .text_size(px(11.))
        .text_color(palette.text_muted)
        .hover(|this| this.text_color(palette.text))
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
        .px_2()
        .py_1()
        .mt_1()
        .cursor_pointer()
        .rounded_md()
        .bg(palette.inactive_pill)
        .text_size(px(12.))
        .text_color(palette.text_muted2)
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
