//! gpui-kit 根视图（迁移骨架 → Phase 1 数据桥接）。
//!
//! 数据层（`Store` / `Model` / `Attachment`）不变，以 `Arc<Store>` 注入。本阶段把原 egui 版
//! `MinActionApp` 的状态与 `reload()` 桥接过来：根视图持有目标/分类/任务数据与当前选择，并负责
//! 首次与每次刷新时从 `Store` 重新载入。后续阶段把结构树、三页签、对话框等逐个从 egui 平移到
//! gpui-kit。

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use gpui_kit::base::StyledExt;
use gpui_kit::*;

use crate::clock::Clock;
use crate::config::Bootstrap;
use crate::model::{Category, CategoryId, Goal, GoalId, Task, TaskId};
use crate::store::Store;

/// 主窗口标题。
pub const WINDOW_TITLE: &str = "最小行动";

/// 后台 ticker 投递的事件（跨日检测由根视图消费）。
#[derive(Debug, Clone, Copy)]
pub enum TickEvent {
    /// 定时心跳。
    Tick,
}

/// 中央区页签（Phase 2 渲染时消费）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TodoTab {
    /// 任务管理：建任务、调频率与进度。
    Tasks,
    /// 今天触发的待办。
    Today,
    /// 今天之前的历史待办。
    History,
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
    #[allow(dead_code)]
    pub(crate) selected_goal: Option<GoalId>,
    /// 当前选中的分类。
    #[allow(dead_code)]
    pub(crate) selected_category: Option<CategoryId>,
    /// 当前选中的任务。
    #[allow(dead_code)]
    pub(crate) selected_task: Option<TaskId>,
    /// 当前中央区页签。
    #[allow(dead_code)]
    pub(crate) tab: TodoTab,
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
        };
        app.reload()?;
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
        Ok(())
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
    #[allow(dead_code)]
    pub(crate) fn goal_task_count(&self, goal: GoalId) -> usize {
        self.all_categories
            .iter()
            .filter(|category| category.goal_id == goal)
            .map(|category| self.task_counts.get(&category.id).copied().unwrap_or(0))
            .sum()
    }
}

#[cfg(test)]
mod tests;

impl Render for MinActionApp {
    fn render(
        &mut self,
        _window: &mut Window,
        _cx: &mut Context<'_, MinActionApp>,
    ) -> impl IntoElement {
        let total_tasks: usize = self.task_counts.values().sum();
        div()
            .size_full()
            .v_flex()
            .items_center()
            .justify_center()
            .gap_2()
            .child("最小行动 · gpui-kit 迁移中")
            .child(format!(
                "已载入：目标 {} · 分类 {} · 任务 {}",
                self.all_goals.len(),
                self.all_categories.len(),
                total_tasks,
            ))
    }
}
