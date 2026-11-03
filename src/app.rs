//! 界面层：状态、事件循环与窗口装配。
//!
//! 界面不做业务判断，只调用 `store` 与 `schedule` 并把结果摆出来；
//! 跨日推进由后台 tokio 任务的 tick 驱动，界面侧只负责重绘。

mod main_view;
mod popup;
mod today_view;

use std::collections::HashSet;
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use chrono::NaiveDate;
use eframe::egui;
use tracing::{debug, info, warn};

use crate::clock::Clock;
use crate::common::elapsed_ms;
use crate::model::{
    Action, Category, CategoryId, DeletionImpact, Goal, GoalId, ReminderState, Task,
};
use crate::store::Store;

/// 主窗口标题。
pub const WINDOW_TITLE: &str = "最小行动";

/// 主窗口默认尺寸（逻辑像素）。
pub const WINDOW_SIZE: [f32; 2] = [880.0, 560.0];

/// 界面最短重绘间隔。即使没有 tick，也保证跨日判定在 5 秒内生效。
const REPAINT_INTERVAL: Duration = Duration::from_secs(1);

/// 后台 tick 事件。
#[derive(Debug, Clone, Copy)]
pub enum TickEvent {
    /// 每两秒一次，用于发现跨日。
    Tick,
}

/// 提示窗口的共享状态。
#[derive(Debug, Default)]
pub struct PopupState {
    /// 待展示的未完成数量；`None` 表示当前不该显示弹窗。
    pub pending: Option<usize>,
    /// 用户是否已关闭弹窗。
    pub dismissed: bool,
}

/// 等待用户确认的删除目标。
#[derive(Debug, Clone, Copy)]
pub(crate) enum PendingDelete {
    /// 待删除的目标。
    Goal(GoalId),
    /// 待删除的分类。
    Category(CategoryId),
}

/// 应用状态。
pub struct MinActionApp {
    pub(crate) store: Store,
    pub(crate) clock: Arc<dyn Clock>,
    pub(crate) today: NaiveDate,
    pub(crate) goals: Vec<Goal>,
    pub(crate) categories: Vec<Category>,
    pub(crate) tasks: Vec<Task>,
    pub(crate) all_tasks: Vec<Task>,
    pub(crate) all_categories: Vec<Category>,
    pub(crate) actions: Vec<Action>,
    pub(crate) deferred: usize,
    pub(crate) daily_cap: u8,
    pub(crate) selected_goal: Option<GoalId>,
    pub(crate) selected_category: Option<CategoryId>,
    pub(crate) new_task: String,
    pub(crate) show_new_goal: bool,
    pub(crate) show_new_category: bool,
    pub(crate) draft_goal_name: String,
    pub(crate) draft_category_name: String,
    pub(crate) draft_category_goal: Option<GoalId>,
    pub(crate) collapsed_goals: HashSet<GoalId>,
    pub(crate) root_collapsed: bool,
    pub(crate) pending_delete: Option<PendingDelete>,
    pub(crate) delete_impact: Option<DeletionImpact>,
    pub(crate) error: Option<String>,
    pub(crate) ticks: Receiver<TickEvent>,
    pub(crate) popup: Arc<Mutex<PopupState>>,
}

impl MinActionApp {
    /// 创建应用并载入首屏数据。
    ///
    /// # Errors
    ///
    /// 首屏数据读取失败时返回错误，由调用方在边界处记录。
    pub fn new(
        store: Store,
        clock: Arc<dyn Clock>,
        ticks: Receiver<TickEvent>,
    ) -> anyhow::Result<Self> {
        let today = clock.today();
        let mut app = MinActionApp {
            store,
            clock,
            today,
            goals: Vec::new(),
            categories: Vec::new(),
            tasks: Vec::new(),
            all_tasks: Vec::new(),
            all_categories: Vec::new(),
            actions: Vec::new(),
            deferred: 0,
            daily_cap: 5,
            selected_goal: None,
            selected_category: None,
            new_task: String::new(),
            show_new_goal: false,
            show_new_category: false,
            draft_goal_name: String::new(),
            draft_category_name: String::new(),
            draft_category_goal: None,
            collapsed_goals: HashSet::new(),
            root_collapsed: false,
            pending_delete: None,
            delete_impact: None,
            error: None,
            ticks,
            popup: Arc::new(Mutex::new(PopupState::default())),
        };
        app.reload()?;
        app.sync_popup();
        Ok(app)
    }

    /// 重新载入目标、分类、任务与当天行动。
    ///
    /// # Errors
    ///
    /// 任一查询失败时返回错误并保留上一次的内存快照。
    pub fn reload(&mut self) -> anyhow::Result<()> {
        let started = Instant::now();
        self.daily_cap = self.store.daily_cap()?;
        self.goals = self.store.goals()?;

        if let Some(goal) = self.selected_goal
            && !self.goals.iter().any(|item| item.id == goal)
        {
            self.selected_goal = None;
        }
        self.selected_goal = self
            .selected_goal
            .or(self.goals.first().map(|item| item.id));

        self.categories = match self.selected_goal {
            Some(goal) => self.store.categories(goal)?,
            None => Vec::new(),
        };

        if let Some(category) = self.selected_category
            && !self.categories.iter().any(|item| item.id == category)
        {
            self.selected_category = None;
        }
        self.selected_category = self
            .selected_category
            .or(self.categories.first().map(|item| item.id));

        self.tasks = match self.selected_category {
            Some(category) => self.store.tasks(category)?,
            None => Vec::new(),
        };

        self.all_tasks = self.store.active_tasks()?;
        self.all_categories = self.store.all_categories()?;

        self.actions = self.store.generate_day(self.today, self.daily_cap)?;
        self.deferred = self.store.deferred_count(self.today)?;
        debug!(
            op = "reload",
            result = "ok",
            elapsed_ms = elapsed_ms(started),
            actions = self.actions.len(),
            deferred = self.deferred,
            "界面数据已刷新"
        );
        Ok(())
    }

    /// 执行一次数据变更并在成功后刷新界面。
    ///
    /// 失败时把可执行的错误文案放到状态条，返回值表示成功与否。
    pub(crate) fn apply<T>(
        &mut self,
        operation: impl FnOnce(&Self) -> Result<T, crate::model::DomainError>,
    ) -> Option<T> {
        let started = Instant::now();
        match operation(self) {
            Ok(value) => {
                if let Err(error) = self.reload() {
                    warn!(
                        op = "apply",
                        result = "refresh_error",
                        elapsed_ms = elapsed_ms(started),
                        error = %error,
                        "操作已生效，但随后的刷新失败"
                    );
                    self.error = Some(format!("刷新失败：{error}"));
                } else {
                    debug!(
                        op = "apply",
                        result = "ok",
                        elapsed_ms = elapsed_ms(started),
                        "操作已完成"
                    );
                }
                Some(value)
            }
            Err(error) => {
                // 界面用状态条展示错误，不会向上传播；这里是它唯一的一次日志记录。
                warn!(
                    op = "apply",
                    result = "error",
                    elapsed_ms = elapsed_ms(started),
                    error = %error,
                    "操作失败"
                );
                self.error = Some(error.to_string());
                None
            }
        }
    }

    /// 切换目标或分类后刷新其下内容。
    pub(crate) fn refresh_scope(&mut self) {
        let started = Instant::now();
        if let Err(error) = self.reload() {
            warn!(
                op = "refresh_scope",
                result = "error",
                elapsed_ms = elapsed_ms(started),
                error = %error,
                "切换范围后刷新失败"
            );
            self.error = Some(format!("刷新失败：{error}"));
        }
    }

    /// 按当前数据库状态刷新提示窗口的判定。
    fn sync_popup(&mut self) {
        let pending = self
            .store
            .pending_count(self.today)
            .unwrap_or_else(|error| {
                debug!(
                    op = "popup",
                    result = "count_error",
                    error = %error,
                    "读取待办数量失败，按 0 处理"
                );
                0
            });
        let state = self.store.reminder_state().unwrap_or_else(|error| {
            debug!(
                op = "popup",
                result = "state_error",
                error = %error,
                "读取提示记录失败，按未提示处理"
            );
            ReminderState::default()
        });
        let dismissed = self
            .popup
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .dismissed;

        if dismissed {
            debug!(
                op = "popup",
                result = "dismissed",
                "本次运行已关闭提示，不再弹出"
            );
            return;
        }

        if !crate::notify::should_notify(&state, self.today, pending) {
            debug!(
                op = "popup",
                result = "skipped",
                pending,
                "今天不满足提示条件"
            );
            return;
        }

        match self.store.mark_notified(self.today) {
            Ok(()) => {
                if let Ok(mut guard) = self.popup.lock() {
                    guard.pending = Some(pending);
                    info!(op = "popup", result = "shown", pending, "已弹出今日提示");
                }
            }
            Err(error) => debug!(
                op = "popup",
                result = "persist_error",
                error = %error,
                "未能记录提示日期，本次不弹窗"
            ),
        }
    }

    fn handle_tick(&mut self, ctx: &egui::Context) {
        let today = self.clock.today();
        if today == self.today {
            return;
        }
        let started = Instant::now();
        let previous = self.today;
        self.today = today;
        match self.reload() {
            Ok(()) => info!(
                op = "day_rollover",
                result = "ok",
                from = %previous,
                to = %today,
                elapsed_ms = elapsed_ms(started),
                "已切换到新的一天"
            ),
            Err(error) => {
                self.error = Some(format!("跨日刷新失败：{error}"));
                warn!(
                    op = "day_rollover",
                    result = "error",
                    from = %previous,
                    to = %today,
                    elapsed_ms = elapsed_ms(started),
                    error = %error,
                    "跨日刷新失败"
                );
            }
        }
        self.sync_popup();
        ctx.request_repaint();
    }

    fn take_dismissed(&self) -> bool {
        let mut guard = self
            .popup
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if guard.dismissed {
            guard.dismissed = false;
            guard.pending = None;
            true
        } else {
            false
        }
    }

    fn status_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading(format!("今日 · {}", self.today.format("%Y-%m-%d")));
            ui.separator();
            let pending = self
                .actions
                .iter()
                .filter(|action| action.status == crate::model::ActionStatus::Pending)
                .count();
            ui.label(format!("共 {} 条待完成", pending));
            if self.deferred > 0 {
                ui.separator();
                ui.colored_label(
                    egui::Color32::from_rgb(0xC0, 0x6A, 0x00),
                    format!("已顺延 {} 条", self.deferred),
                );
            }
            ui.separator();
            ui.label("每日上限");
            let mut cap = self.daily_cap;
            if ui
                .add(egui::DragValue::new(&mut cap).range(1..=20))
                .changed()
            {
                let started = Instant::now();
                match self.store.set_daily_cap(cap) {
                    Ok(()) => {
                        self.daily_cap = cap;
                        info!(
                            op = "set_daily_cap",
                            result = "ok",
                            cap,
                            elapsed_ms = elapsed_ms(started),
                            "每日上限已更新"
                        );
                        if let Err(error) = self.reload() {
                            self.error = Some(format!("刷新失败：{error}"));
                            warn!(
                                op = "set_daily_cap",
                                result = "refresh_error",
                                cap,
                                elapsed_ms = elapsed_ms(started),
                                error = %error,
                                "每日上限已更新，但随后的刷新失败"
                            );
                        }
                    }
                    Err(error) => {
                        self.error = Some(error.to_string());
                        warn!(
                            op = "set_daily_cap",
                            result = "error",
                            cap,
                            elapsed_ms = elapsed_ms(started),
                            error = %error,
                            "每日上限更新失败"
                        );
                    }
                }
            }
        });

        if let Some(message) = self.error.clone() {
            ui.horizontal(|ui| {
                ui.colored_label(egui::Color32::LIGHT_RED, message);
                if ui.small_button("知道了").clicked() {
                    self.error = None;
                }
            });
        }
    }
}

impl eframe::App for MinActionApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        while self.ticks.try_recv().is_ok() {
            self.handle_tick(&ctx);
        }
        ctx.request_repaint_after(REPAINT_INTERVAL);

        egui::Panel::top("status").show(ui, |ui| self.status_bar(ui));
        egui::Panel::left("structure")
            .resizable(true)
            .size_range(egui::Rangef::new(240.0, 460.0))
            .show(ui, |ui| self.ui_main_view(ui));
        egui::CentralPanel::default().show(ui, |ui| self.ui_today_view(ui));

        self.show_popup(&ctx);

        if self.take_dismissed() {
            if let Err(error) = self.store.mark_viewed(self.today) {
                warn!(
                    op = "mark_viewed",
                    result = "error",
                    error = %error,
                    "记录查看状态失败"
                );
            }
            ctx.send_viewport_cmd_to(egui::ViewportId::ROOT, egui::ViewportCommand::Focus);
        }

        self.ui_new_goal_dialog(&ctx);
        self.ui_new_category_dialog(&ctx);
        self.ui_delete_dialog(&ctx);
    }
}
