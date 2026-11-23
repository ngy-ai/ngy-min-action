//! 顶部菜单栏 / 底部状态栏渲染与对应入口。
//!
//! 菜单栏提供「编辑」（新建目标/分类/任务、首选项）与「统计」，以及中央区的
//! 三个页签（任务 / 今日待办 / 历史待办）；主题在配置窗口里切换。

use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

use super::config_dialog::ConfigTab;
use super::create_dialog::CreateKind;
use super::style::{SurfaceExt, TEXT_LG, TEXT_XS};
use super::{MinActionApp, TodoTab, WINDOW_TITLE, theme_label};
use crate::palette::Palette;

impl MinActionApp {
    /// 渲染顶部菜单栏（应用标题 + 迁移版标识）。
    pub(crate) fn render_menu_bar(&self) -> impl IntoElement {
        div()
            .id("menu-bar")
            .flex()
            .items_center()
            .justify_between()
            .px_3()
            .py_2()
            .border_b_1()
            .bg(self.palette.surface)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .w(px(20.))
                            .h(px(20.))
                            .rounded_md()
                            .bg(self.palette.accent)
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(
                                div()
                                    .text_size(TEXT_XS)
                                    .text_color(self.palette.accent_fg)
                                    .child("✓"),
                            ),
                    )
                    .child(
                        div()
                            .text_size(TEXT_LG)
                            .font_weight(gpui_kit::FontWeight::BOLD)
                            .child(WINDOW_TITLE),
                    ),
            )
            .child(
                div()
                    .text_size(TEXT_XS)
                    .text_color(self.palette.text_muted)
                    .child("gpui-kit 迁移版"),
            )
    }

    /// 渲染菜单栏：编辑（新建 / 首选项）、统计与三个页签。
    pub(crate) fn render_toolbar(&self, entity: Entity<MinActionApp>) -> impl IntoElement {
        let p = self.palette;
        let open = self.menu_open;
        let goal_scope = self
            .selected_goal
            .or_else(|| self.all_goals.first().map(|goal| goal.id));
        let category_scope = self
            .selected_category
            .or_else(|| self.all_categories.first().map(|category| category.id));
        let goal_label = goal_scope
            .and_then(|id| self.all_goals.iter().find(|goal| goal.id == id))
            .map(|goal| format!("归属：{}", goal.name))
            .unwrap_or_else(|| "请先创建目标".to_owned());
        let category_label = category_scope
            .and_then(|id| self.all_categories.iter().find(|c| c.id == id))
            .map(|c| format!("归属：{}", c.name))
            .unwrap_or_else(|| "请先创建目标与分类".to_owned());

        div()
            .id("toolbar")
            .relative()
            .flex()
            .items_center()
            .gap_2()
            .px_3()
            .py_2()
            .border_b_1()
            .bg(p.surface)
            .child(menu_trigger(entity.clone(), "menu-edit", "编辑", open, &p))
            .child(divider(&p))
            .child(menu_command("menu-stats", "统计", &p, {
                let entity = entity.clone();
                move |window, cx| {
                    entity.update(cx, |root, cx| {
                        root.close_menu();
                        let app_entity = cx.entity();
                        root.open_stats(app_entity, window, cx);
                    });
                }
            }))
            .child(divider(&p))
            .children(
                [TodoTab::Tasks, TodoTab::Today, TodoTab::History]
                    .into_iter()
                    .map(|tab| tab_button(entity.clone(), tab, self.tab, &p)),
            )
            // 下拉必须包在 `deferred` 里：它按绝对定位盖住下方区域，但工具栏本身比
            // 结构树与内容区先绘制，直接挂子节点会被后绘制的面板整块遮住（只剩一条边）。
            .when(open, |this| {
                this.child(deferred(edit_dropdown(
                    entity.clone(),
                    goal_scope,
                    goal_label.clone(),
                    category_scope,
                    category_label.clone(),
                    &p,
                )))
            })
    }

    /// 渲染底部状态栏：每日上限、主题、今日完成进度、历史条数。
    pub(crate) fn render_status_bar(&self) -> impl IntoElement {
        let done = self
            .today_actions
            .iter()
            .filter(|a| a.status.is_settled())
            .count();
        let total = self.today_actions.len();
        let progress = if total == 0 {
            0.
        } else {
            done as f32 / total as f32
        };
        let palette = self.palette;
        div()
            .id("status-bar")
            .flex()
            .items_center()
            .gap_3()
            .px_4()
            .py_2()
            .border_t_1()
            .bg(palette.status_bar)
            .text_size(TEXT_XS)
            .text_color(palette.text_muted)
            .child(format!("每日上限 {}", self.daily_cap))
            .child(divider(&palette))
            .child(format!("主题 {}", theme_label(self.theme_choice)))
            .child(divider(&palette))
            .child(format!("今日 完成 {done}/{total}"))
            .child(
                div()
                    .w(px(96.))
                    .h(px(4.))
                    .rounded_full()
                    .bg(palette.inactive_pill)
                    .child(
                        div()
                            .h_full()
                            .w(px(96.) * progress)
                            .rounded_full()
                            .bg(palette.success),
                    ),
            )
            .child(divider(&palette))
            .child(format!("历史 {}", self.history_actions.len()))
    }
}

/// 顶层菜单项：点击展开 / 收起下拉。
fn menu_trigger(
    app: Entity<MinActionApp>,
    id: &'static str,
    label: &'static str,
    open: bool,
    p: &Palette,
) -> impl IntoElement {
    div()
        .id(id)
        .soft_button(p, open)
        .child(format!("{label} ▾"))
        .on_click(move |_ev, _window, cx| {
            app.update(cx, |root, cx| {
                root.toggle_menu();
                cx.notify();
            });
        })
}

/// 顶层命令菜单项：点击直接触发动作。
fn menu_command(
    id: &'static str,
    label: &'static str,
    p: &Palette,
    action: impl Fn(&mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .soft_button(p, false)
        .child(label)
        .on_click(move |_ev, window, cx| action(window, cx))
}

/// 工具栏里的页签按钮：切换中央区显示任务 / 今日待办 / 历史待办。
fn tab_button(
    app: Entity<MinActionApp>,
    tab: TodoTab,
    current: TodoTab,
    p: &Palette,
) -> impl IntoElement {
    div()
        .id(format!("tab-{}", tab.label()))
        .soft_button(p, tab == current)
        .child(tab.label())
        .on_click(move |_ev, _window, cx| {
            app.update(cx, |root, cx| {
                root.switch_tab(tab);
                cx.notify();
            });
        })
}

/// 「编辑」下拉：新建目标 / 分类 / 任务，以及首选项。
fn edit_dropdown(
    app: Entity<MinActionApp>,
    goal_scope: Option<crate::model::GoalId>,
    goal_label: String,
    category_scope: Option<crate::model::CategoryId>,
    category_label: String,
    p: &Palette,
) -> impl IntoElement {
    div()
        .id("edit-dropdown")
        .absolute()
        .top(px(34.))
        .left(px(8.))
        .flex()
        .flex_col()
        .min_w(px(180.))
        .p_1()
        .surface_card(p)
        .shadow_lg()
        .child(dropdown_item("edit-add-goal", "添加目标", p, false, {
            let app = app.clone();
            move |window, cx| {
                app.update(cx, |root, cx| {
                    root.close_menu();
                    let entity = cx.entity();
                    root.open_create(CreateKind::Goal, "新建目标".to_owned(), entity, window, cx);
                });
            }
        }))
        .child(dropdown_item(
            "edit-add-category",
            "添加分类",
            p,
            false,
            {
                let app = app.clone();
                move |window, cx| {
                    app.update(cx, |root, cx| {
                        root.close_menu();
                        let entity = cx.entity();
                        root.open_create(
                            CreateKind::Category { goal: goal_scope },
                            goal_label.clone(),
                            entity,
                            window,
                            cx,
                        );
                    });
                }
            },
        ))
        .child(dropdown_item("edit-add-task", "添加任务", p, false, {
            let app = app.clone();
            move |window, cx| {
                app.update(cx, |root, cx| {
                    root.close_menu();
                    let entity = cx.entity();
                    root.open_create(
                        CreateKind::Task {
                            category: category_scope,
                        },
                        category_label.clone(),
                        entity,
                        window,
                        cx,
                    );
                });
            }
        }))
        .child(menu_separator(p))
        .child(dropdown_item("edit-preferences", "首选项", p, false, {
            let app = app.clone();
            move |window, cx| {
                app.update(cx, |root, cx| {
                    root.close_menu();
                    let entity = cx.entity();
                    root.open_config(ConfigTab::DailyCap, entity, window, cx);
                });
            }
        }))
}

/// 工具栏 / 状态栏中的竖直分隔线。
fn divider(palette: &Palette) -> Div {
    div()
        .w(px(1.))
        .h(px(16.))
        .flex_shrink_0()
        .bg(palette.border)
}

/// 下拉菜单内的可点击项；`active` 用于标记当前选中项。
fn dropdown_item(
    id: impl Into<ElementId>,
    label: &'static str,
    p: &Palette,
    active: bool,
    action: impl Fn(&mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(id.into())
        .soft_button(p, active)
        .child(label)
        .on_click(move |_ev, window, cx| action(window, cx))
}

/// 下拉菜单内的横向分隔线。
fn menu_separator(p: &Palette) -> Div {
    div().h(px(1.)).mx_1().my_1().bg(p.border)
}
