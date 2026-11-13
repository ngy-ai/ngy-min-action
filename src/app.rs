//! gpui-kit 根视图（Phase 2：结构树 + 三页签骨架）。
//!
//! 数据层（`Store` / `Model` / `Attachment`）不变，以 `Arc<Store>` 注入。根视图持有目标/分类/任务
//! 数据与当前选择，并在 `reload()` 时从 `Store` 重新载入。本阶段把布局搭出来：左侧结构树（目标/
//! 分类 + 任务计数），右侧三页签（任务 / 今日待办 / 历史待办），交互通过 gpui 实体更新驱动重绘。

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use chrono::NaiveDate;
use gpui::{AnyElement, App, ClickEvent, Window};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

use crate::clock::Clock;
use crate::common::{DAILY_CAP_DEFAULT, DAILY_CAP_MAX, DAILY_CAP_MIN, HISTORY_PAGE_SIZE};
use crate::config::Bootstrap;
use crate::model::{
    Action, ActionId, ActionStatus, Category, CategoryId, DeletionImpact, Frequency, Goal, GoalId,
    Task, TaskId, ThemeChoice,
};
use crate::store::Store;
use gpui_kit::component::theme::{Theme, ThemeMode};

mod delete_dialog;
mod frequency_dialog;
mod history_view;
mod note_dialog;
mod rename_dialog;
mod reset_dialog;
mod rich_content;
mod stats_dialog;
mod storage_dialog;
mod task_editor;
mod today_view;

/// 主窗口标题。
pub const WINDOW_TITLE: &str = "最小行动";

/// 后台 ticker 投递的事件（跨日检测由根视图消费）。
#[derive(Debug, Clone, Copy)]
pub enum TickEvent {
    /// 定时心跳。
    Tick,
}

/// 结构树中被重命名/删除的对象：目标或分类。
#[derive(Debug, Clone, Copy)]
pub(crate) enum StructTarget {
    /// 目标。
    Goal(GoalId),
    /// 分类。
    Category(CategoryId),
}

/// 当前打开的结构树对话框（重命名/删除/频率/重置进度），与任务内容编辑器可叠加显示。
#[derive(Clone)]
pub(crate) enum StructDialog {
    /// 重命名目标或分类。
    Rename(Entity<rename_dialog::RenameDialog>),
    /// 删除确认目标或分类。
    Delete(Entity<delete_dialog::DeleteDialog>),
    /// 编辑任务频率。
    Frequency(Entity<frequency_dialog::FrequencyDialog>),
    /// 复习统计展示。
    Stats(Entity<stats_dialog::StatsDialog>),
    /// 存储目录与占用展示。
    Storage(Entity<storage_dialog::StorageDialog>),
    /// 行动备注（左右两栏：来源任务 / 当前备注）。
    Note(Entity<note_dialog::NoteDialog>),
    /// 重置任务复习进度（遗忘曲线归零）。
    Reset(Entity<reset_dialog::ResetDialog>),
}

/// 中央区页签。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TodoTab {
    /// 任务管理：建任务、调频率与进度。
    Tasks,
    /// 今天触发的待办。
    Today,
    /// 今天之前的历史待办。
    History,
}

impl TodoTab {
    /// 页签展示文案。
    pub fn label(self) -> &'static str {
        match self {
            TodoTab::Tasks => "任务",
            TodoTab::Today => "今日待办",
            TodoTab::History => "历史待办",
        }
    }
}

/// 应用根视图：持有数据层与界面状态。
pub struct MinActionApp {
    store: Arc<Store>,
    #[allow(dead_code)]
    clock: Arc<dyn Clock>,
    #[allow(dead_code)]
    ticks: std::sync::mpsc::Receiver<TickEvent>,
    #[allow(dead_code)]
    bootstrap: Bootstrap,

    /// 全部目标，供结构树渲染。
    pub(crate) all_goals: Vec<Goal>,
    /// 全部分类，供结构树渲染。
    pub(crate) all_categories: Vec<Category>,
    /// 全部任务，供结构树计数与任务页渲染。
    pub(crate) all_tasks: Vec<Task>,
    /// 每个分类下的任务数，供结构树展示条数。
    pub(crate) task_counts: HashMap<CategoryId, usize>,
    /// 当前选中的目标；`None` 表示全部。
    pub(crate) selected_goal: Option<GoalId>,
    /// 当前选中的分类。
    pub(crate) selected_category: Option<CategoryId>,
    /// 当前选中的任务。
    #[allow(dead_code)]
    pub(crate) selected_task: Option<TaskId>,
    /// 当前中央区页签。
    pub(crate) tab: TodoTab,
    /// 当前打开的任务内容编辑器；`None` 表示未打开。
    pub(crate) editing: Option<Entity<task_editor::TaskEditor>>,
    /// 当前打开的结构树对话框（重命名/删除/频率/统计/存储）；`None` 表示未打开。
    pub(crate) dialog: Option<StructDialog>,
    /// 今日行动列表（含已结算与顺延），供今日页渲染与完成/跳过/撤销。
    pub(crate) today_actions: Vec<Action>,
    /// 历史待办（累计已加载页），供历史页渲染与结算。
    pub(crate) history_actions: Vec<Action>,
    /// 历史分页游标，用于「加载更多」。
    pub(crate) history_cursor: Option<(NaiveDate, i64)>,
    /// 历史是否还有更早的页。
    pub(crate) history_has_more: bool,
    /// 每日行动上限（从设置读取，供工具栏与状态栏展示）。
    pub(crate) daily_cap: u8,
    /// 主题偏好（从设置读取，供工具栏高亮与启动时应用）。
    pub(crate) theme_choice: ThemeChoice,
}

impl MinActionApp {
    /// 构造根视图状态；数据层已就绪，构造时即载入一次。
    pub fn new(
        store: Store,
        clock: Arc<dyn Clock>,
        ticks: std::sync::mpsc::Receiver<TickEvent>,
        bootstrap: Bootstrap,
    ) -> Result<Self> {
        let mut app = Self {
            store: Arc::new(store),
            clock,
            ticks,
            bootstrap,
            all_goals: Vec::new(),
            all_categories: Vec::new(),
            all_tasks: Vec::new(),
            task_counts: HashMap::new(),
            selected_goal: None,
            selected_category: None,
            selected_task: None,
            tab: TodoTab::Tasks,
            editing: None,
            dialog: None,
            today_actions: Vec::new(),
            history_actions: Vec::new(),
            history_cursor: None,
            history_has_more: false,
            daily_cap: DAILY_CAP_DEFAULT,
            theme_choice: ThemeChoice::System,
        };
        app.reload()?;
        app.load_history_first()?;
        Ok(app)
    }

    /// 从数据层重新载入全部界面数据，并归一化选择。
    pub fn reload(&mut self) -> Result<()> {
        self.all_goals = self.store.goals()?;
        self.all_categories = self.store.all_categories()?;
        self.all_tasks = self.store.all_tasks()?;
        self.recompute_task_counts();

        if self.selected_goal.is_none() {
            self.selected_goal = self.all_goals.first().map(|goal| goal.id);
        }
        if self.selected_category.is_none() {
            self.selected_category = self
                .all_categories
                .iter()
                .find(|category| Some(category.goal_id) == self.selected_goal)
                .map(|category| category.id);
        }
        self.daily_cap = self.store.daily_cap()?;
        self.theme_choice = self.store.theme_choice()?;
        self.reload_today()?;
        Ok(())
    }

    /// 生成（幂等）并载入今日行动列表，覆盖 `today_actions`。
    pub(crate) fn reload_today(&mut self) -> Result<()> {
        let today = self.clock.today();
        let cap = self.store.daily_cap()?;
        self.store.generate_day(today, cap)?;
        self.today_actions = self.store.actions_on(today)?;
        Ok(())
    }

    /// 结算一条今日行动（完成/跳过），随后刷新今日列表。
    pub(crate) fn settle_today_action(
        &mut self,
        action_id: ActionId,
        next: ActionStatus,
    ) -> Result<()> {
        self.store.settle_action(action_id, next)?;
        self.reload_today()?;
        Ok(())
    }

    /// 撤销一条今日行动最近一次结算，随后刷新今日列表。
    pub(crate) fn undo_today_action(&mut self, action_id: ActionId) -> Result<()> {
        self.store.undo_last_settlement(action_id)?;
        self.reload_today()?;
        Ok(())
    }

    /// 载入历史待办首页（重置游标与累计列表），供初始化与刷新。
    pub(crate) fn load_history_first(&mut self) -> Result<()> {
        let today = self.clock.today();
        let page = self.store.actions_page(today, None, HISTORY_PAGE_SIZE)?;
        self.history_actions = page.actions;
        self.history_cursor = page.next_cursor;
        self.history_has_more = page.has_more;
        Ok(())
    }

    /// 加载更早的一页历史待办，追加到现有列表；无更多页时为空操作。
    pub(crate) fn load_history_more(&mut self) -> Result<()> {
        if !self.history_has_more {
            return Ok(());
        }
        let today = self.clock.today();
        let cursor = self.history_cursor;
        let page = self.store.actions_page(today, cursor, HISTORY_PAGE_SIZE)?;
        self.history_actions.extend(page.actions);
        self.history_cursor = page.next_cursor;
        self.history_has_more = page.has_more;
        Ok(())
    }

    /// 结算一条历史行动（完成/跳过），并就地更新内存状态以保留分页。
    pub(crate) fn settle_history_action(
        &mut self,
        action_id: ActionId,
        next: ActionStatus,
    ) -> Result<()> {
        self.store.settle_action(action_id, next)?;
        if let Some(action) = self.history_actions.iter_mut().find(|a| a.id == action_id) {
            action.status = next;
        }
        Ok(())
    }

    /// 撤销一条历史行动最近一次结算，并就地更新内存状态以保留分页。
    pub(crate) fn undo_history_action(&mut self, action_id: ActionId) -> Result<()> {
        self.store.undo_last_settlement(action_id)?;
        if let Some(action) = self.history_actions.iter_mut().find(|a| a.id == action_id) {
            action.status = ActionStatus::Pending;
        }
        Ok(())
    }

    /// 设置每日行动上限（写入设置并刷新今日列表），越界返回 `InvalidValue`。
    pub(crate) fn set_daily_cap(&mut self, cap: u8) -> Result<()> {
        self.store.set_daily_cap(cap)?;
        self.daily_cap = self.store.daily_cap()?;
        self.reload_today()?;
        Ok(())
    }

    /// 设置主题偏好（仅持久化；实际的 gpui 主题应用在 `apply_theme_choice`）。
    pub(crate) fn set_theme(&mut self, choice: ThemeChoice) -> Result<()> {
        self.store.set_theme_choice(choice)?;
        self.theme_choice = choice;
        Ok(())
    }

    /// 启动阶段读取已保存的主题偏好（无记录则跟随系统）。
    pub fn initial_theme(&self) -> ThemeChoice {
        self.store.theme_choice().unwrap_or_default()
    }

    /// 切换到指定中央区页签。
    pub(crate) fn switch_tab(&mut self, tab: TodoTab) {
        self.tab = tab;
    }

    /// 选中目标，并把当前分类归一化到该目标下第一个分类。
    pub(crate) fn select_goal(&mut self, goal: GoalId) {
        self.selected_goal = Some(goal);
        self.selected_category = self
            .all_categories
            .iter()
            .find(|category| category.goal_id == goal)
            .map(|category| category.id);
    }

    /// 选中分类，并同步其所属目标。
    pub(crate) fn select_category(&mut self, category: CategoryId) {
        self.selected_category = Some(category);
        if let Some(category) = self.all_categories.iter().find(|c| c.id == category) {
            self.selected_goal = Some(category.goal_id);
        }
    }

    /// 关闭任务内容对话框。
    pub(crate) fn close_editor(&mut self) {
        self.editing = None;
    }

    /// 关闭结构树对话框（重命名/删除/频率）。
    pub(crate) fn close_dialog(&mut self) {
        self.dialog = None;
    }

    /// 打开频率编辑对话框：依据当前频率预选种类，构造对话框实体并挂到根视图。
    pub(crate) fn open_frequency_dialog(
        &mut self,
        root: Entity<MinActionApp>,
        editor: Entity<task_editor::TaskEditor>,
        current: Frequency,
        window: &mut Window,
        cx: &mut App,
    ) {
        let dialog = cx.new(|c| {
            frequency_dialog::FrequencyDialog::new(editor, root.clone(), current, window, c)
        });
        root.update(cx, |this, cx| {
            this.dialog = Some(StructDialog::Frequency(dialog));
            cx.notify();
        });
    }

    /// 打开复习统计对话框：从数据层算出 `ReviewStats` 后构造对话框挂在根视图。
    pub(crate) fn open_stats(
        &mut self,
        root: Entity<MinActionApp>,
        _window: &mut Window,
        cx: &mut App,
    ) {
        let today = self.clock.today();
        let stats = match self
            .store
            .review_stats(today, stats_dialog::STATS_WINDOW_DAYS)
        {
            Ok(stats) => stats,
            Err(e) => {
                tracing::error!("读取复习统计失败：{e:#}");
                return;
            }
        };
        let dialog = cx.new(|_c| stats_dialog::StatsDialog::new(stats, root.clone()));
        root.update(cx, |this, cx| {
            this.dialog = Some(StructDialog::Stats(dialog));
            cx.notify();
        });
    }

    /// 打开存储对话框：从数据目录递归算出占用后构造对话框挂在根视图。
    pub(crate) fn open_storage(
        &mut self,
        root: Entity<MinActionApp>,
        _window: &mut Window,
        cx: &mut App,
    ) {
        let info = storage_dialog::collect_storage(self.store.data_dir());
        let dialog = cx.new(|_c| storage_dialog::StorageDialog::new(info, root.clone()));
        root.update(cx, |this, cx| {
            this.dialog = Some(StructDialog::Storage(dialog));
            cx.notify();
        });
    }

    /// 打开行动备注对话框：用来源任务上下文与既有备注构造两栏对话框挂在根视图。
    pub(crate) fn open_note_dialog(
        &mut self,
        root: Entity<MinActionApp>,
        action_id: ActionId,
        context: note_dialog::NoteContext,
        window: &mut Window,
        cx: &mut App,
    ) {
        let dialog = cx.new(|c| {
            note_dialog::NoteDialog::new(
                context,
                action_id,
                self.store.clone(),
                root.clone(),
                window,
                c,
            )
        });
        root.update(cx, |this, cx| {
            this.dialog = Some(StructDialog::Note(dialog));
            cx.notify();
        });
    }

    /// 打开重置复习进度对话框：依据任务名构造确认面板挂在根视图。
    pub(crate) fn open_reset_dialog(
        &mut self,
        root: Entity<MinActionApp>,
        task_id: TaskId,
        _window: &mut Window,
        cx: &mut App,
    ) {
        let task_name = self
            .store
            .task(task_id)
            .map(|task| task.name)
            .unwrap_or_default();
        let dialog = cx.new(|_c| {
            reset_dialog::ResetDialog::new(task_id, task_name, self.store.clone(), root.clone())
        });
        root.update(cx, |this, cx| {
            this.dialog = Some(StructDialog::Reset(dialog));
            cx.notify();
        });
    }

    /// 删除一个目标将波及的范围（分类数、任务数），用于删除确认文案。
    pub(crate) fn goal_deletion_impact(&self, goal_id: GoalId) -> DeletionImpact {
        let cat_ids: Vec<CategoryId> = self
            .all_categories
            .iter()
            .filter(|c| c.goal_id == goal_id)
            .map(|c| c.id)
            .collect();
        let tasks = self
            .all_tasks
            .iter()
            .filter(|t| cat_ids.contains(&t.category_id))
            .count();
        DeletionImpact {
            categories: cat_ids.len(),
            tasks,
            pending_actions: 0,
        }
    }

    /// 删除一个分类将波及的范围（任务数），用于删除确认文案。
    pub(crate) fn category_deletion_impact(&self, category_id: CategoryId) -> DeletionImpact {
        let tasks = self
            .all_tasks
            .iter()
            .filter(|t| t.category_id == category_id)
            .count();
        DeletionImpact {
            categories: 0,
            tasks,
            pending_actions: 0,
        }
    }

    /// 按分类聚合任务数，供结构树展示条数。
    fn recompute_task_counts(&mut self) {
        let mut counts: HashMap<CategoryId, usize> = HashMap::new();
        for task in &self.all_tasks {
            *counts.entry(task.category_id).or_default() += 1;
        }
        self.task_counts = counts;
    }

    /// 目标下的任务总数：汇总其所有分类的计数。
    pub(crate) fn goal_task_count(&self, goal: GoalId) -> usize {
        self.all_categories
            .iter()
            .filter(|category| category.goal_id == goal)
            .map(|category| self.task_counts.get(&category.id).copied().unwrap_or(0))
            .sum()
    }

    /// 渲染左侧结构树面板。
    fn render_structure(&self, entity: Entity<Self>) -> impl IntoElement {
        let selected_goal = self.selected_goal;
        let selected_category = self.selected_category;
        div()
            .id("structure")
            .w(px(240.))
            .h_full()
            .flex_shrink_0()
            .border_r_1()
            .overflow_y_scroll()
            .child(div().px_3().py_2().child("结构"))
            .children(
                self.all_goals
                    .iter()
                    .enumerate()
                    .flat_map(move |(gi, goal)| {
                        let goal_id = goal.id;
                        let goal_selected = selected_goal == Some(goal_id);
                        let count = self.goal_task_count(goal_id);
                        let entity_goal = entity.clone();
                        let entity_cat = entity.clone();
                        let rename_entity = entity.clone();
                        let delete_entity = entity.clone();
                        let goal_name = goal.name.clone();
                        let mut rows = vec![{
                            div()
                                .id(format!("goal-{gi}"))
                                .px_3()
                                .py_1()
                                .rounded_md()
                                .when(goal_selected, |this| this.bg(rgb(0x2a_2a_2e)))
                                .flex()
                                .items_center()
                                .justify_between()
                                .on_click(move |_ev, _window, cx| {
                                    entity_goal.update(cx, |this, cx| {
                                        this.select_goal(goal_id);
                                        cx.notify();
                                    });
                                })
                                .child(
                                    div()
                                        .flex_1()
                                        .cursor_pointer()
                                        .child(format!("{}（{}）", goal.name, count)),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .gap_1()
                                        .child(struct_action(
                                            "goal-rename",
                                            "重命名",
                                            move |_ev, window, cx| {
                                                open_rename(
                                                    rename_entity.clone(),
                                                    StructTarget::Goal(goal_id),
                                                    &goal_name,
                                                    window,
                                                    cx,
                                                );
                                            },
                                        ))
                                        .child(struct_action(
                                            "goal-delete",
                                            "删除",
                                            move |_ev, window, cx| {
                                                let impact = delete_entity
                                                    .read(cx)
                                                    .goal_deletion_impact(goal_id);
                                                open_delete(
                                                    delete_entity.clone(),
                                                    StructTarget::Goal(goal_id),
                                                    impact,
                                                    window,
                                                    cx,
                                                );
                                            },
                                        )),
                                )
                        }];
                        if goal_selected {
                            for (ci, category) in self
                                .all_categories
                                .iter()
                                .filter(|category| category.goal_id == goal_id)
                                .enumerate()
                            {
                                let category_id = category.id;
                                let cat_selected = selected_category == Some(category_id);
                                let cat_count =
                                    self.task_counts.get(&category_id).copied().unwrap_or(0);
                                let cat_select = entity_cat.clone();
                                let cat_rename = entity_cat.clone();
                                let cat_delete = entity_cat.clone();
                                let cat_name = category.name.clone();
                                rows.push(
                                    div()
                                        .id(format!("cat-{gi}-{ci}"))
                                        .px_3()
                                        .pl_6()
                                        .py_1()
                                        .rounded_md()
                                        .when(cat_selected, |this| this.bg(rgb(0x2a_2a_2e)))
                                        .flex()
                                        .items_center()
                                        .justify_between()
                                        .on_click(move |_ev, _window, cx| {
                                            cat_select.update(cx, |this, cx| {
                                                this.select_category(category_id);
                                                cx.notify();
                                            });
                                        })
                                        .child(
                                            div().flex_1().cursor_pointer().child(format!(
                                                "{}（{}）",
                                                category.name, cat_count
                                            )),
                                        )
                                        .child(
                                            div()
                                                .flex()
                                                .gap_1()
                                                .child(struct_action(
                                                    "cat-rename",
                                                    "重命名",
                                                    move |_ev, window, cx| {
                                                        open_rename(
                                                            cat_rename.clone(),
                                                            StructTarget::Category(category_id),
                                                            &cat_name,
                                                            window,
                                                            cx,
                                                        );
                                                    },
                                                ))
                                                .child(struct_action(
                                                    "cat-delete",
                                                    "删除",
                                                    move |_ev, window, cx| {
                                                        let impact = cat_delete
                                                            .read(cx)
                                                            .category_deletion_impact(category_id);
                                                        open_delete(
                                                            cat_delete.clone(),
                                                            StructTarget::Category(category_id),
                                                            impact,
                                                            window,
                                                            cx,
                                                        );
                                                    },
                                                )),
                                        ),
                                );
                            }
                        }
                        rows.into_iter()
                    }),
            )
    }

    /// 渲染右侧中央区（页签栏 + 内容）。
    fn render_main(&self, entity: Entity<Self>) -> impl IntoElement {
        let tab = self.tab;
        let tabs = [TodoTab::Tasks, TodoTab::Today, TodoTab::History];
        let entity_for_tabs = entity.clone();
        let tab_buttons: Vec<_> = tabs
            .iter()
            .map(move |t| {
                let active = *t == tab;
                let tab_value = *t;
                let entity = entity_for_tabs.clone();
                div()
                    .id(format!("tab-{}", t.label()))
                    .px_3()
                    .py_2()
                    .cursor_pointer()
                    .when(active, |this| this.border_b_2().bg(rgb(0x33_33_38)))
                    .child(t.label())
                    .on_click(move |_ev, _window, cx| {
                        entity.update(cx, |this, cx| {
                            this.switch_tab(tab_value);
                            cx.notify();
                        });
                    })
            })
            .collect();
        div()
            .id("main")
            .flex_1()
            .h_full()
            .child(
                div()
                    .id("tab-bar")
                    .border_b_1()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_2()
                    .child(div().id("tabs-row").flex().gap_1().children(tab_buttons)),
            )
            .child(
                div()
                    .id("content")
                    .flex_1()
                    .p_3()
                    .child(self.render_tab_content(entity.clone())),
            )
    }

    /// 渲染顶部菜单栏（应用标题 + 迁移版标识）。
    fn render_menu_bar(&self) -> impl IntoElement {
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
                    .font_weight(gpui::FontWeight::BOLD)
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
    fn render_toolbar(&self, entity: Entity<MinActionApp>) -> impl IntoElement {
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
    fn theme_selector(
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
    fn render_status_bar(&self) -> impl IntoElement {
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

    /// 渲染当前页签下的内容（Phase 2 仅任务页有真实数据，其余占位）。
    fn render_tab_content(&self, entity: Entity<Self>) -> impl IntoElement {
        match self.tab {
            TodoTab::Tasks => {
                let entity = entity.clone();
                let tasks: Vec<&Task> = match self.selected_category {
                    Some(category) => self
                        .all_tasks
                        .iter()
                        .filter(|task| task.category_id == category)
                        .collect(),
                    None => self.all_tasks.iter().collect(),
                };
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .children(tasks.into_iter().map(move |task| {
                        let entity = entity.clone();
                        let task_id = task.id;
                        let name = task.name.clone();
                        div()
                            .id(format!("task-{}", task_id.0))
                            .px_2()
                            .py_1()
                            .rounded_md()
                            .bg(rgb(0x24_24_28))
                            .cursor_pointer()
                            .child(name)
                            .on_click(move |_ev, window, cx| {
                                let root = entity.clone();
                                let store = root.read(cx).store.clone();
                                let editor = cx.new(|c| {
                                    let task_model = store
                                        .all_tasks()
                                        .ok()
                                        .and_then(|ts| ts.into_iter().find(|t| t.id == task_id));
                                    let task_model = task_model.expect("任务应存在");
                                    task_editor::TaskEditor::new(
                                        &task_model,
                                        store,
                                        root.clone(),
                                        window,
                                        c,
                                    )
                                });
                                root.update(cx, |this, cx| {
                                    this.editing = Some(editor);
                                    cx.notify();
                                });
                            })
                    }))
            }
            TodoTab::Today => div().child(today_view::render(entity.clone(), self)),
            TodoTab::History => div().child(history_view::render(entity.clone(), self)),
        }
    }
}

/// 把领域层主题偏好映射到 gpui-kit 主题并应用；`System` 跟随当前系统外观。
pub fn apply_theme_choice(choice: ThemeChoice, window: &mut Window, cx: &mut App) {
    let mode: ThemeMode = match choice {
        ThemeChoice::Light => ThemeMode::Light,
        ThemeChoice::Dark => ThemeMode::Dark,
        ThemeChoice::System => cx.window_appearance().into(),
    };
    Theme::change(mode, Some(window), cx);
}

#[cfg(test)]
mod tests;

impl Render for MinActionApp {
    fn render(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<'_, MinActionApp>,
    ) -> impl IntoElement {
        let entity = cx.entity();
        div()
            .size_full()
            .relative()
            .flex()
            .flex_col()
            .child(self.render_menu_bar())
            .child(self.render_toolbar(entity.clone()))
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .child(self.render_structure(entity.clone()))
                    .child(self.render_main(entity.clone())),
            )
            .child(self.render_status_bar())
            .when_some(self.editing.clone(), |this, editing| {
                this.child(modal_overlay(editing))
            })
            .when_some(self.dialog.clone(), |this, dialog| {
                let child: AnyElement = match dialog {
                    StructDialog::Rename(d) => d.into_any_element(),
                    StructDialog::Delete(d) => d.into_any_element(),
                    StructDialog::Frequency(d) => d.into_any_element(),
                    StructDialog::Stats(d) => d.into_any_element(),
                    StructDialog::Storage(d) => d.into_any_element(),
                    StructDialog::Note(d) => d.into_any_element(),
                    StructDialog::Reset(d) => d.into_any_element(),
                };
                this.child(modal_overlay(child))
            })
    }
}

/// 居中的模态遮罩：半透明底色 + 中央卡片容器，包裹任意对话框内容。
fn modal_overlay(child: impl IntoElement) -> impl IntoElement {
    div()
        .absolute()
        .inset_0()
        .bg(rgb(0x00_00_00).opacity(0.5))
        .flex()
        .items_center()
        .justify_center()
        .child(div().bg(rgb(0x20_20_24)).rounded_lg().child(child))
}

/// 结构树每行右侧的小操作按钮（重命名/删除）。
fn struct_action(
    id: &'static str,
    label: &'static str,
    handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
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
        .on_click(handler)
}

/// 工具栏「统计」入口：打开复习统计对话框（顺带为 T015 工具栏铺底）。
fn stats_button(app: Entity<MinActionApp>) -> impl IntoElement {
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
fn storage_button(app: Entity<MinActionApp>) -> impl IntoElement {
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
fn cap_button(
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
fn theme_button(
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

/// 打开重命名对话框：构造对话框实体并挂到根视图。
fn open_rename(
    root: Entity<MinActionApp>,
    target: StructTarget,
    current_name: &str,
    window: &mut Window,
    cx: &mut App,
) {
    let store = root.read(cx).store.clone();
    let dialog = cx.new(|c| {
        rename_dialog::RenameDialog::new(target, current_name, store, root.clone(), window, c)
    });
    root.update(cx, |this, cx| {
        this.dialog = Some(StructDialog::Rename(dialog));
        cx.notify();
    });
}

/// 打开删除确认对话框：先算影响范围，再构造对话框实体并挂到根视图。
fn open_delete(
    root: Entity<MinActionApp>,
    target: StructTarget,
    impact: DeletionImpact,
    window: &mut Window,
    cx: &mut App,
) {
    let store = root.read(cx).store.clone();
    let dialog = cx
        .new(|c| delete_dialog::DeleteDialog::new(target, impact, store, root.clone(), window, c));
    root.update(cx, |this, cx| {
        this.dialog = Some(StructDialog::Delete(dialog));
        cx.notify();
    });
}
