//! 界面层：状态、事件循环与窗口装配。
//!
//! 界面不做业务判断，只调用 `store` 与 `schedule` 并把结果摆出来；
//! 跨日推进由后台 tokio 任务的 tick 驱动，界面侧只负责重绘。

mod main_view;
mod popup;
mod today_view;

use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use chrono::NaiveDate;
use eframe::egui;

use crate::clock::Clock;
use crate::model::{Action, Category, CategoryId, Goal, GoalId, Task};
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
    pub(crate) new_goal: String,
    pub(crate) new_category: String,
    pub(crate) new_task: String,
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
            new_goal: String::new(),
            new_category: String::new(),
            new_task: String::new(),
            error: None,
            ticks,
            popup: Arc::new(Mutex::new(PopupState::default())),
        };
        app.reload()?;
        Ok(app)
    }

    /// 重新载入目标、分类、任务与当天行动。
    ///
    /// # Errors
    ///
    /// 任一查询失败时返回错误并保留上一次的内存快照。
    pub fn reload(&mut self) -> anyhow::Result<()> {
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
        Ok(())
    }

    /// 执行一次数据变更并在成功后刷新界面。
    ///
    /// 失败时把可执行的错误文案放到状态条，返回值表示成功与否。
    pub(crate) fn apply<T>(
        &mut self,
        operation: impl FnOnce(&Self) -> Result<T, crate::model::DomainError>,
    ) -> Option<T> {
        match operation(self) {
            Ok(value) => {
                if let Err(error) = self.reload() {
                    self.error = Some(format!("刷新失败：{error}"));
                }
                Some(value)
            }
            Err(error) => {
                self.error = Some(error.to_string());
                None
            }
        }
    }

    /// 切换目标或分类后刷新其下内容。
    pub(crate) fn refresh_scope(&mut self) {
        if let Err(error) = self.reload() {
            self.error = Some(format!("刷新失败：{error}"));
        }
    }

    /// 按当前数据库状态刷新提示窗口的判定。
    fn sync_popup(&mut self) {
        let pending = self.store.pending_count(self.today).unwrap_or(0);
        let state = self.store.reminder_state().unwrap_or_default();
        let dismissed = self
            .popup
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .dismissed;

        if dismissed {
            return;
        }

        if crate::notify::should_notify(&state, self.today, pending)
            && self.store.mark_notified(self.today).is_ok()
            && let Ok(mut guard) = self.popup.lock()
        {
            guard.pending = Some(pending);
        }
    }

    fn handle_tick(&mut self, ctx: &egui::Context) {
        let today = self.clock.today();
        if today == self.today {
            return;
        }
        self.today = today;
        if let Err(error) = self.reload() {
            self.error = Some(format!("跨日刷新失败：{error}"));
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
                match self.store.set_daily_cap(cap) {
                    Ok(()) => {
                        self.daily_cap = cap;
                        if let Err(error) = self.reload() {
                            self.error = Some(format!("刷新失败：{error}"));
                        }
                    }
                    Err(error) => self.error = Some(error.to_string()),
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
            let _ = self.store.mark_viewed(self.today);
            ctx.send_viewport_cmd_to(egui::ViewportId::ROOT, egui::ViewportCommand::Focus);
        }
    }
}
