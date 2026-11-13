//! 顶部菜单栏 / 工具栏 / 底部状态栏渲染与对应按钮。
//!
//! 这些 `MinActionApp` 方法只负责渲染；变更经 `set_daily_cap` / `set_theme` 等写回数据层，
//! 主题切换同时经 `apply_theme_choice` 立即应用 gpui-kit 主题。

use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

use super::MinActionApp;
use super::WINDOW_TITLE;
use super::apply_theme_choice;
use crate::common::{DAILY_CAP_MAX, DAILY_CAP_MIN};
use crate::model::ThemeChoice;

impl MinActionApp {
    /// 渲染顶部菜单栏（应用标题 + 迁移版标识）。
    pub(crate) fn render_menu_bar(&self) -> impl IntoElement {
        div()
            .id("menu-bar")
            .flex()
            .items_center()
            .justify_between()
            .px_2()
            .py_1()
            .border_b_1()
            .bg(rgb(0x1e_1e_22))
            .child(
                div()
                    .font_weight(gpui_kit::FontWeight::BOLD)
                    .child(WINDOW_TITLE),
            )
            .child(
                div()
                    .text_size(px(11.))
                    .text_color(rgb(0x88_88_88))
                    .child("gpui-kit 迁移版"),
            )
    }

    /// 渲染工具栏：每日上限步进、主题切换、统计与存储入口。
    pub(crate) fn render_toolbar(&self, entity: Entity<MinActionApp>) -> impl IntoElement {
        let cap = self.daily_cap;
        let theme = self.theme_choice;
        div()
            .id("toolbar")
            .flex()
            .items_center()
            .gap_4()
            .px_2()
            .py_1()
            .border_b_1()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(div().child("每日上限"))
                    .child(cap_button(
                        entity.clone(),
                        "cap-dec",
                        "−",
                        cap.saturating_sub(1).max(DAILY_CAP_MIN),
                    ))
                    .child(div().min_w(px(18.)).text_center().child(cap.to_string()))
                    .child(cap_button(
                        entity.clone(),
                        "cap-inc",
                        "+",
                        cap.saturating_add(1).min(DAILY_CAP_MAX),
                    )),
            )
            .child(self.theme_selector(entity.clone(), theme))
            .child(stats_button(entity.clone()))
            .child(storage_button(entity.clone()))
    }

    /// 渲染主题切换三段：跟随系统 / 浅色 / 深色，当前偏好高亮。
    pub(crate) fn theme_selector(
        &self,
        entity: Entity<MinActionApp>,
        current: ThemeChoice,
    ) -> impl IntoElement {
        let choices = [
            (ThemeChoice::System, "跟随系统"),
            (ThemeChoice::Light, "浅色"),
            (ThemeChoice::Dark, "深色"),
        ];
        div()
            .flex()
            .items_center()
            .gap_1()
            .child(div().child("主题"))
            .children(choices.into_iter().map(move |(choice, label)| {
                theme_button(entity.clone(), choice, label, choice == current)
            }))
    }

    /// 渲染底部状态栏：每日上限、主题、今日完成进度、历史条数。
    pub(crate) fn render_status_bar(&self) -> impl IntoElement {
        let done = self
            .today_actions
            .iter()
            .filter(|a| a.status.is_settled())
            .count();
        let total = self.today_actions.len();
        let theme_label = match self.theme_choice {
            ThemeChoice::System => "跟随系统",
            ThemeChoice::Light => "浅色",
            ThemeChoice::Dark => "深色",
        };
        div()
            .id("status-bar")
            .flex()
            .items_center()
            .gap_4()
            .px_2()
            .py_1()
            .border_t_1()
            .text_size(px(11.))
            .text_color(rgb(0x99_99_99))
            .child(format!("每日上限 {}", self.daily_cap))
            .child(format!("主题 {theme_label}"))
            .child(format!("今日 完成 {done}/{total}"))
            .child(format!("历史 {}", self.history_actions.len()))
    }
}

/// 工具栏「统计」入口：打开复习统计对话框（顺带为 T015 工具栏铺底）。
pub(crate) fn stats_button(app: Entity<MinActionApp>) -> impl IntoElement {
    div()
        .id("open-stats")
        .px_2()
        .py_1()
        .cursor_pointer()
        .rounded_md()
        .text_size(px(12.))
        .child("统计")
        .on_click(move |_ev, window, cx| {
            app.update(cx, |root, cx| {
                let app_entity = cx.entity().clone();
                root.open_stats(app_entity, window, cx);
                cx.notify();
            });
        })
}

/// 工具栏「存储」入口：打开存储对话框（顺带为 T015 工具栏铺底）。
pub(crate) fn storage_button(app: Entity<MinActionApp>) -> impl IntoElement {
    div()
        .id("open-storage")
        .px_2()
        .py_1()
        .cursor_pointer()
        .rounded_md()
        .text_size(px(12.))
        .child("存储")
        .on_click(move |_ev, window, cx| {
            app.update(cx, |root, cx| {
                let app_entity = cx.entity().clone();
                root.open_storage(app_entity, window, cx);
                cx.notify();
            });
        })
}

/// 工具栏「每日上限」步进按钮：点击经 `set_daily_cap` 写库并刷新今日列表。
pub(crate) fn cap_button(
    app: Entity<MinActionApp>,
    id: &'static str,
    label: &'static str,
    target: u8,
) -> impl IntoElement {
    div()
        .id(id)
        .px_2()
        .py_0()
        .cursor_pointer()
        .rounded_md()
        .bg(rgb(0x2c2c32))
        .text_size(px(12.))
        .child(label)
        .on_click(move |_ev, _window, cx| {
            app.update(cx, |root, cx| {
                let _ = root.set_daily_cap(target);
                cx.notify();
            });
        })
}

/// 工具栏「主题」单选项：点击经 `set_theme` 持久化并立即应用 gpui 主题。
pub(crate) fn theme_button(
    app: Entity<MinActionApp>,
    choice: ThemeChoice,
    label: &'static str,
    active: bool,
) -> impl IntoElement {
    div()
        .id(format!("theme-{}", choice.as_str()))
        .px_2()
        .py_0()
        .cursor_pointer()
        .rounded_md()
        .when(active, |this| this.bg(rgb(0x3a_3a_42)))
        .when(!active, |this| this.bg(rgb(0x2c2c32)))
        .text_size(px(12.))
        .child(label)
        .on_click(move |_ev, window, cx| {
            apply_theme_choice(choice, window, cx);
            app.update(cx, |root, cx| {
                let _ = root.set_theme(choice);
                cx.notify();
            });
        })
}
