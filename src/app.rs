//! 界面层：状态、事件循环与窗口装配。
//!
//! 界面不做业务判断，只调用 `store` 与 `schedule` 并把结果摆出来；
//! 跨日推进由后台 tokio 任务的 tick 驱动，界面侧只负责重绘。

mod delete_dialog;
mod describe;
mod estimate_dialog;
mod frequency_dialog;
mod history_view;
mod icons;
mod main_view;
mod menu_bar;
mod note_dialog;
mod popup;
mod rename_dialog;
mod reset_dialog;
mod stats_dialog;
mod status_bar;
mod storage_dialog;
mod structure_menu;
#[cfg(test)]
mod tests;
mod theme;
mod today_view;
mod toolbar;

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use chrono::NaiveDate;
use eframe::egui;
use tracing::{debug, info, warn};

use crate::clock::Clock;
use crate::common::elapsed_ms;
use crate::config::Bootstrap;
use crate::model::{
    Action, ActionId, ActionImage, Category, CategoryId, DeletionImpact, Goal, GoalId,
    ReminderState, ReviewProgress, ReviewStats, Task, TaskId, ThemeChoice,
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PendingDelete {
    /// 待删除的目标。
    Goal(GoalId),
    /// 待删除的分类。
    Category(CategoryId),
    /// 待删除的任务。
    Task(TaskId),
}

/// 等待用户提交的重命名对象。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RenameTarget {
    /// 待重命名的目标。
    Goal(GoalId),
    /// 待重命名的分类。
    Category(CategoryId),
    /// 待重命名的任务。
    Task(TaskId),
}

/// 新建任务时可选的频率模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FrequencyMode {
    /// 每天。
    Daily,
    /// 每 N 天。
    EveryNDays,
    /// 每周 N 次。
    WeeklyN,
}

impl FrequencyMode {
    /// 切换到该模式时参数输入框的默认值。
    pub(crate) fn default_interval(self) -> &'static str {
        match self {
            FrequencyMode::Daily => "",
            FrequencyMode::EveryNDays => "3",
            FrequencyMode::WeeklyN => "2",
        }
    }

    /// 参数输入框前的说明文字。
    pub(crate) fn interval_label(self) -> &'static str {
        match self {
            FrequencyMode::Daily => "",
            FrequencyMode::EveryNDays => "间隔天数",
            FrequencyMode::WeeklyN => "每周次数",
        }
    }

    /// 交给 `Frequency::from_parts` 的类型标签。
    pub(crate) fn kind(self) -> &'static str {
        match self {
            FrequencyMode::Daily => "daily",
            FrequencyMode::EveryNDays => "every_n_days",
            FrequencyMode::WeeklyN => "weekly_n",
        }
    }
}

/// 中央区页签。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TodoTab {
    /// 今天触发的待办。
    Today,
    /// 今天之前的历史待办。
    History,
}

/// 历史待办的内存页。
#[derive(Debug, Default)]
pub(crate) struct HistoryState {
    pub(crate) actions: Vec<Action>,
    pub(crate) cursor: Option<(NaiveDate, i64)>,
    pub(crate) has_more: bool,
    pub(crate) loaded: bool,
    pub(crate) error: Option<String>,
}

/// 备注窗口的编辑状态。
#[derive(Debug)]
pub(crate) struct NoteEditor {
    pub(crate) action: ActionId,
    pub(crate) title: String,
    pub(crate) text: String,
    pub(crate) images: Vec<ActionImage>,
    pub(crate) read_only: bool,
    pub(crate) error: Option<String>,
}

/// 存储位置窗口的状态。
#[derive(Debug, Default)]
pub(crate) struct StorageDialog {
    pub(crate) target: Option<PathBuf>,
    pub(crate) error: Option<String>,
    pub(crate) notice: Option<String>,
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
    pub(crate) new_task_mode: FrequencyMode,
    pub(crate) new_task_interval: String,
    pub(crate) new_task_estimate: String,
    pub(crate) task_progress: Vec<ReviewProgress>,
    pub(crate) show_new_goal: bool,
    pub(crate) show_new_category: bool,
    pub(crate) draft_goal_name: String,
    pub(crate) draft_category_name: String,
    pub(crate) draft_category_goal: Option<GoalId>,
    pub(crate) collapsed_goals: HashSet<GoalId>,
    pub(crate) root_collapsed: bool,
    pub(crate) pending_delete: Option<PendingDelete>,
    pub(crate) delete_impact: Option<DeletionImpact>,
    pub(crate) rename_target: Option<RenameTarget>,
    pub(crate) draft_rename_name: String,
    pub(crate) rename_select_all: bool,
    pub(crate) frequency_target: Option<TaskId>,
    pub(crate) draft_frequency_mode: FrequencyMode,
    pub(crate) draft_frequency_interval: String,
    pub(crate) reset_target: Option<TaskId>,
    pub(crate) reset_level: u8,
    pub(crate) estimate_target: Option<TaskId>,
    pub(crate) draft_estimate: String,
    pub(crate) show_stats: bool,
    pub(crate) stats: Option<ReviewStats>,
    pub(crate) theme: ThemeChoice,
    pub(crate) error: Option<String>,
    pub(crate) ticks: Receiver<TickEvent>,
    pub(crate) popup: Arc<Mutex<PopupState>>,
    pub(crate) bootstrap: Bootstrap,
    pub(crate) data_dir: PathBuf,
    pub(crate) tab: TodoTab,
    pub(crate) history: HistoryState,
    pub(crate) note: Option<NoteEditor>,
    pub(crate) show_storage: bool,
    pub(crate) storage: StorageDialog,
    pub(crate) show_about: bool,
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
        bootstrap: Bootstrap,
    ) -> anyhow::Result<Self> {
        let today = clock.today();
        let data_dir = store.data_dir().to_path_buf();
        let theme = match store.theme_choice() {
            Ok(choice) => choice,
            Err(error) => {
                warn!(
                    op = "theme_choice",
                    result = "error",
                    error = %error,
                    "读取主题偏好失败，回退跟随系统"
                );
                ThemeChoice::System
            }
        };
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
            new_task_mode: FrequencyMode::Daily,
            new_task_interval: String::new(),
            new_task_estimate: String::new(),
            task_progress: Vec::new(),
            show_new_goal: false,
            show_new_category: false,
            draft_goal_name: String::new(),
            draft_category_name: String::new(),
            draft_category_goal: None,
            collapsed_goals: HashSet::new(),
            root_collapsed: false,
            pending_delete: None,
            delete_impact: None,
            rename_target: None,
            draft_rename_name: String::new(),
            rename_select_all: false,
            frequency_target: None,
            draft_frequency_mode: FrequencyMode::Daily,
            draft_frequency_interval: String::new(),
            reset_target: None,
            reset_level: 0,
            estimate_target: None,
            draft_estimate: String::new(),
            show_stats: false,
            stats: None,
            theme,
            error: None,
            ticks,
            popup: Arc::new(Mutex::new(PopupState::default())),
            bootstrap,
            data_dir,
            tab: TodoTab::Today,
            history: HistoryState::default(),
            note: None,
            show_storage: false,
            storage: StorageDialog::default(),
            show_about: false,
        };
        app.reload()?;
        app.sync_popup();
        Ok(app)
    }

    /// 在首帧前注册两套样式并应用已保存的主题偏好。
    ///
    /// 由启动编排在创建窗口前调用，保证首帧不带默认外观；跟随系统时由框架注入系统主题。
    pub fn apply_theme(&self, ctx: &egui::Context) {
        theme::apply_styles(ctx);
        ctx.set_theme(theme::preference(self.theme));
    }

    /// 切换主题并把选择写入本地设置。
    ///
    /// 写库失败时保留原偏好，只设置一条可关闭的错误提示；错误在界面边界记录一次。
    pub(crate) fn set_theme(&mut self, choice: ThemeChoice, ctx: &egui::Context) {
        let started = Instant::now();
        match self.store.set_theme_choice(choice) {
            Ok(()) => {
                self.theme = choice;
                ctx.set_theme(theme::preference(choice));
                info!(
                    op = "set_theme",
                    result = "ok",
                    theme = choice.as_str(),
                    elapsed_ms = elapsed_ms(started),
                    "主题已切换"
                );
            }
            Err(error) => {
                self.error = Some(format!("切换主题失败：{error}"));
                warn!(
                    op = "set_theme",
                    result = "error",
                    error = %error,
                    elapsed_ms = elapsed_ms(started),
                    "主题偏好写入失败"
                );
            }
        }
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
        self.task_progress = match self.selected_category {
            Some(category) => self.store.progress_for_category(category)?,
            None => Vec::new(),
        };

        self.all_tasks = self.store.all_tasks()?;
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

    /// 切换中央区页签；首次进入历史时加载第一页。
    pub(crate) fn switch_tab(&mut self, tab: TodoTab) {
        self.tab = tab;
        if tab == TodoTab::History {
            self.ensure_history();
        }
    }

    fn ui_tabs(&mut self, ui: &mut egui::Ui) {
        let mut next = None;
        ui.horizontal(|ui| {
            if ui
                .selectable_label(self.tab == TodoTab::Today, "今日待办")
                .clicked()
            {
                next = Some(TodoTab::Today);
            }
            if ui
                .selectable_label(self.tab == TodoTab::History, "历史待办")
                .clicked()
            {
                next = Some(TodoTab::History);
            }
        });
        if let Some(tab) = next {
            self.switch_tab(tab);
        }
        ui.separator();
        match self.tab {
            TodoTab::Today => self.ui_today_view(ui),
            TodoTab::History => {
                self.ensure_history();
                self.ui_history_view(ui);
            }
        }
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
        if self.history.loaded {
            self.reset_history();
        }
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
}

impl eframe::App for MinActionApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        while self.ticks.try_recv().is_ok() {
            self.handle_tick(&ctx);
        }
        ctx.request_repaint_after(REPAINT_INTERVAL);

        egui::Panel::top("menu").show(ui, |ui| self.ui_menu_bar(ui));
        egui::Panel::top("toolbar").show(ui, |ui| self.toolbar(ui));
        egui::Panel::bottom("statusbar").show(ui, |ui| self.ui_status_bar(ui));
        egui::Panel::left("structure")
            .resizable(true)
            .size_range(egui::Rangef::new(240.0, 460.0))
            .show(ui, |ui| self.ui_main_view(ui));
        egui::CentralPanel::default().show(ui, |ui| self.ui_tabs(ui));

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
        self.ui_rename_dialog(&ctx);
        self.ui_frequency_dialog(&ctx);
        self.ui_estimate_dialog(&ctx);
        self.ui_reset_dialog(&ctx);
        self.ui_stats_dialog(&ctx);
        self.ui_delete_dialog(&ctx);
        self.ui_note_dialog(&ctx);
        self.ui_storage_dialog(&ctx);
        self.ui_about_dialog(&ctx);
    }
}
