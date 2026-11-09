//! 主视图：带根节点的目标/分类树，以及任务区。
//!
//! 新建目标与新建分类都走模态对话框；分类对话框里选择归属目标，
//! 避免「先选中目标才能建分类」这种隐式前置条件。

use eframe::egui;

use crate::app::icons::{self, ButtonId};
use crate::app::structure_menu;
use crate::app::{FrequencyMode, MinActionApp, PendingDelete, RenameTarget};
use crate::model::{CategoryId, Frequency, GoalId, TaskDraft, TaskStatus};

impl MinActionApp {
    pub(crate) fn ui_main_view(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("目标与分类");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if icons::add(ui, ButtonId::AddCategory).clicked() {
                    self.open_new_category_dialog();
                }
                if icons::add(ui, ButtonId::AddGoal).clicked() {
                    self.open_new_goal_dialog();
                }
            });
        });

        ui.horizontal(|ui| {
            if icons::add(ui, ButtonId::ExpandAll).clicked() {
                self.collapsed_goals.clear();
                self.root_collapsed = false;
            }
            if icons::add(ui, ButtonId::CollapseAll).clicked() {
                self.collapsed_goals = self.goals.iter().map(|goal| goal.id).collect();
                self.root_collapsed = true;
            }
        });
        ui.separator();

        let mut select_goal: Option<GoalId> = None;
        let mut select_category: Option<CategoryId> = None;
        let mut drop_goal: Option<GoalId> = None;
        let mut drop_category: Option<CategoryId> = None;
        let mut toggle_goal: Option<GoalId> = None;
        let mut toggle_root = false;
        let mut rename: Option<(RenameTarget, String)> = None;

        egui::ScrollArea::vertical()
            .id_salt("structure-tree")
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let arrow = if self.root_collapsed {
                        ButtonId::ChevronRight
                    } else {
                        ButtonId::ChevronDown
                    };
                    if icons::add(ui, arrow).clicked() {
                        toggle_root = true;
                    }
                    ui.label(format!("全部（{} 个目标）", self.goals.len()));
                });

                if self.root_collapsed {
                    return;
                }

                if self.goals.is_empty() {
                    ui.horizontal(|ui| {
                        ui.add_space(18.0);
                        ui.weak("还没有目标，先点右上角的添加目标按钮。");
                    });
                }

                for goal in &self.goals {
                    let expanded = !self.collapsed_goals.contains(&goal.id);
                    let selected = self.selected_goal == Some(goal.id);
                    ui.horizontal(|ui| {
                        ui.add_space(12.0);
                        let arrow = if expanded {
                            ButtonId::ChevronDown
                        } else {
                            ButtonId::ChevronRight
                        };
                        if icons::add(ui, arrow).clicked() {
                            toggle_goal = Some(goal.id);
                        }
                        let name = ui
                            .selectable_label(selected, &goal.name)
                            .on_hover_text("左键选中，右键菜单");
                        name.context_menu(|ui| {
                            if let Some(action) = structure_menu::show(ui) {
                                match action {
                                    structure_menu::StructureAction::Rename => {
                                        rename =
                                            Some((RenameTarget::Goal(goal.id), goal.name.clone()));
                                    }
                                    structure_menu::StructureAction::Delete => {
                                        drop_goal = Some(goal.id);
                                    }
                                }
                            }
                        });
                        if name.clicked() {
                            select_goal = Some(goal.id);
                        }
                    });

                    if !expanded {
                        continue;
                    }

                    let mut shown = 0_usize;
                    for category in self
                        .all_categories
                        .iter()
                        .filter(|category| category.goal_id == goal.id)
                    {
                        shown += 1;
                        ui.horizontal(|ui| {
                            ui.add_space(40.0);
                            let picked = self.selected_category == Some(category.id);
                            let name = ui
                                .selectable_label(picked, &category.name)
                                .on_hover_text("左键选中，右键菜单");
                            name.context_menu(|ui| {
                                if let Some(action) = structure_menu::show(ui) {
                                    match action {
                                        structure_menu::StructureAction::Rename => {
                                            rename = Some((
                                                RenameTarget::Category(category.id),
                                                category.name.clone(),
                                            ));
                                        }
                                        structure_menu::StructureAction::Delete => {
                                            drop_category = Some(category.id);
                                        }
                                    }
                                }
                            });
                            if name.clicked() {
                                select_category = Some(category.id);
                            }
                        });
                    }
                    if shown == 0 {
                        ui.horizontal(|ui| {
                            ui.add_space(40.0);
                            ui.weak("这个目标下还没有分类。");
                        });
                    }
                }
            });

        if toggle_root {
            self.root_collapsed = !self.root_collapsed;
        }
        if let Some(goal) = toggle_goal {
            if self.collapsed_goals.contains(&goal) {
                self.collapsed_goals.remove(&goal);
            } else {
                self.collapsed_goals.insert(goal);
            }
        }
        if let Some(goal) = select_goal {
            self.selected_goal = Some(goal);
            self.selected_category = None;
            self.refresh_scope();
        }
        if let Some(category) = select_category {
            self.selected_category = Some(category);
            self.refresh_scope();
        }
        if let Some((target, current)) = rename {
            self.open_rename_dialog(target, &current);
        }
        if let Some(goal) = drop_goal {
            self.request_delete(PendingDelete::Goal(goal));
        }
        if let Some(category) = drop_category {
            self.request_delete(PendingDelete::Category(category));
        }
    }

    /// 按新建表单当前的状态创建任务；校验失败时保留输入。
    pub(crate) fn submit_new_task(&mut self) {
        let Some(category) = self.selected_category else {
            return;
        };
        let interval = self.new_task_interval.trim().parse::<i64>().unwrap_or(0);
        let parsed =
            Frequency::from_parts(self.new_task_mode.kind(), interval).and_then(|frequency| {
                Self::parse_estimate_text(&self.new_task_estimate)
                    .map(|minutes| (frequency, minutes))
            });
        match parsed {
            Ok((frequency, estimated_minutes)) => {
                let draft = TaskDraft {
                    name: self.new_task.clone(),
                    frequency,
                    estimated_minutes,
                    description: None,
                };
                if self
                    .apply(|app| {
                        app.store
                            .create_task(category, draft)
                            .map(|_| "已添加任务".to_owned())
                    })
                    .is_some()
                {
                    self.new_task.clear();
                    self.new_task_estimate.clear();
                }
            }
            Err(error) => {
                self.error = Some(error.to_string());
            }
        }
    }

    pub(crate) fn ui_task_section(&mut self, ui: &mut egui::Ui) {
        ui.heading("任务");
        match self.selected_category {
            Some(_) => {
                let mut create_task = false;
                ui.horizontal(|ui| {
                    ui.label("新任务");
                    ui.add(egui::TextEdit::singleline(&mut self.new_task).desired_width(170.0));
                    ui.label("耗时");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.new_task_estimate).desired_width(56.0),
                    );
                    ui.label("分钟");
                    create_task = icons::add(ui, ButtonId::AddTask).clicked();
                });
                ui.horizontal(|ui| {
                    ui.label("频率");
                    for (mode, text) in [
                        (FrequencyMode::Daily, "每天"),
                        (FrequencyMode::EveryNDays, "每 N 天"),
                        (FrequencyMode::WeeklyN, "每周 N 次"),
                    ] {
                        if ui
                            .selectable_label(self.new_task_mode == mode, text)
                            .clicked()
                            && self.new_task_mode != mode
                        {
                            self.new_task_mode = mode;
                            self.new_task_interval = mode.default_interval().to_string();
                        }
                    }
                });
                if self.new_task_mode != FrequencyMode::Daily {
                    ui.horizontal(|ui| {
                        ui.label(self.new_task_mode.interval_label());
                        ui.add(
                            egui::TextEdit::singleline(&mut self.new_task_interval)
                                .desired_width(60.0),
                        );
                    });
                }
                if create_task {
                    self.submit_new_task();
                }
            }
            None => {
                ui.label("先在左侧选择分类，再添加任务。");
            }
        }

        let mut toggle: Option<(crate::model::TaskId, TaskStatus)> = None;
        let mut reset: Option<crate::model::TaskId> = None;
        let mut rename: Option<(RenameTarget, String)> = None;
        let mut frequency: Option<(crate::model::TaskId, Frequency)> = None;
        let mut estimate: Option<(crate::model::TaskId, Option<u16>)> = None;
        let mut drop_task: Option<crate::model::TaskId> = None;

        egui::ScrollArea::vertical()
            .id_salt("task-list")
            .max_height(220.0)
            .show(ui, |ui| {
                if self.tasks.is_empty() {
                    ui.label("这个分类下还没有任务。");
                }
                for task in &self.tasks {
                    ui.horizontal(|ui| {
                        ui.label(&task.name);
                        if ui
                            .selectable_label(false, task.frequency.label())
                            .on_hover_text("修改频率")
                            .clicked()
                        {
                            frequency = Some((task.id, task.frequency));
                        }
                        if let Some(minutes) = task.estimated_minutes
                            && ui
                                .selectable_label(false, format!("{minutes} 分钟"))
                                .on_hover_text("修改预计耗时")
                                .clicked()
                        {
                            estimate = Some((task.id, task.estimated_minutes));
                        }
                        let progress = self
                            .task_progress
                            .iter()
                            .find(|item| item.task_id == task.id);
                        let progress_text = match progress {
                            None => "未开始".to_string(),
                            Some(item) if item.graduated => "已巩固".to_string(),
                            // 新任务落库时就有 level = 0 的进度行，等同于未开始。
                            Some(item) if item.level == 0 => "未开始".to_string(),
                            Some(item) => match item.next_review_on {
                                Some(date) => format!(
                                    "第 {} 级，下次复习 {}",
                                    item.level,
                                    date.format("%Y-%m-%d")
                                ),
                                None => format!("第 {} 级", item.level),
                            },
                        };
                        ui.label(progress_text);
                        let toggle_button = match task.status {
                            TaskStatus::Active => ButtonId::Pause,
                            TaskStatus::Paused => ButtonId::Resume,
                        };
                        if icons::add(ui, toggle_button).clicked() {
                            let next = match task.status {
                                TaskStatus::Active => TaskStatus::Paused,
                                TaskStatus::Paused => TaskStatus::Active,
                            };
                            toggle = Some((task.id, next));
                        }
                        if icons::add(ui, ButtonId::ResetProgress).clicked() {
                            reset = Some(task.id);
                        }
                        if icons::add(ui, ButtonId::Rename).clicked() {
                            rename = Some((RenameTarget::Task(task.id), task.name.clone()));
                        }
                        ui.add_space(8.0);
                        if icons::add(ui, ButtonId::DeleteTask).clicked() {
                            drop_task = Some(task.id);
                        }
                    });
                }
            });

        if let Some((task, current)) = frequency {
            self.open_frequency_dialog(task, current);
        }
        if let Some((task, current)) = estimate {
            self.open_estimate_dialog(task, current);
        }
        if let Some((target, current)) = rename {
            self.open_rename_dialog(target, &current);
        }
        if let Some(task) = drop_task {
            self.request_delete(PendingDelete::Task(task));
        }
        if let Some((task, status)) = toggle {
            let hint = match status {
                TaskStatus::Paused => "已暂停任务",
                TaskStatus::Active => "已恢复任务",
            };
            self.apply(|app| {
                app.store
                    .set_task_status(task, status)
                    .map(|()| hint.to_owned())
            });
        }
        if let Some(task) = reset {
            self.request_reset(task);
        }
    }

    fn open_new_goal_dialog(&mut self) {
        self.show_new_goal = true;
        self.draft_goal_name.clear();
    }

    fn open_new_category_dialog(&mut self) {
        self.show_new_category = true;
        self.draft_category_name.clear();
        self.draft_category_goal = self
            .selected_goal
            .or(self.goals.first().map(|goal| goal.id));
    }

    pub(crate) fn ui_new_goal_dialog(&mut self, ctx: &egui::Context) {
        if !self.show_new_goal {
            return;
        }
        let mut submit = false;
        let mut cancel = false;

        egui::Window::new("新建目标")
            .id(egui::Id::unique("dialog-new-goal"))
            .collapsible(false)
            .resizable(false)
            .default_pos([320.0, 220.0])
            .show(ctx, |ui| {
                ui.set_width(320.0);
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.label("名称");
                    ui.text_edit_singleline(&mut self.draft_goal_name);
                });
                ui.weak("例如「学习英语」。目标下可以再建分类。");
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    if icons::add(ui, ButtonId::Create).clicked() {
                        submit = true;
                    }
                    if icons::add(ui, ButtonId::Cancel).clicked() {
                        cancel = true;
                    }
                });
            });

        if submit {
            let name = self.draft_goal_name.clone();
            if self
                .apply(|app| {
                    app.store
                        .create_goal(&name, None)
                        .map(|_| format!("已创建目标「{name}」"))
                })
                .is_some()
            {
                self.show_new_goal = false;
                self.selected_goal = self.goals.last().map(|goal| goal.id);
                self.refresh_scope();
            }
        } else if cancel {
            self.show_new_goal = false;
        }
    }

    pub(crate) fn ui_new_category_dialog(&mut self, ctx: &egui::Context) {
        if !self.show_new_category {
            return;
        }
        if self.goals.is_empty() {
            self.show_new_category = false;
            self.error = Some("请先创建目标，再添加分类".to_owned());
            return;
        }

        let mut submit = false;
        let mut cancel = false;

        egui::Window::new("新建分类")
            .id(egui::Id::unique("dialog-new-category"))
            .collapsible(false)
            .resizable(false)
            .default_pos([320.0, 220.0])
            .show(ctx, |ui| {
                ui.set_width(320.0);
                ui.add_space(8.0);

                let selected_name = self
                    .draft_category_goal
                    .and_then(|id| self.goals.iter().find(|goal| goal.id == id))
                    .map_or_else(|| "请选择目标".to_owned(), |goal| goal.name.clone());
                egui::ComboBox::from_label("归属目标")
                    .selected_text(selected_name)
                    .show_ui(ui, |ui| {
                        for goal in &self.goals {
                            ui.selectable_value(
                                &mut self.draft_category_goal,
                                Some(goal.id),
                                &goal.name,
                            );
                        }
                    });

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.label("名称");
                    ui.text_edit_singleline(&mut self.draft_category_name);
                });
                ui.weak("例如「口语」。同一目标下分类名不能重复。");
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    if icons::add(ui, ButtonId::Create).clicked() {
                        submit = true;
                    }
                    if icons::add(ui, ButtonId::Cancel).clicked() {
                        cancel = true;
                    }
                });
            });

        if submit {
            let Some(goal) = self.draft_category_goal else {
                self.error = Some("请选择分类归属的目标".to_owned());
                return;
            };
            let name = self.draft_category_name.clone();
            if self
                .apply(|app| {
                    app.store
                        .create_category(goal, &name)
                        .map(|_| format!("已创建分类「{name}」"))
                })
                .is_some()
            {
                self.show_new_category = false;
                self.selected_goal = Some(goal);
                self.refresh_scope();
                self.selected_category = self.categories.last().map(|category| category.id);
                self.refresh_scope();
            }
        } else if cancel {
            self.show_new_category = false;
        }
    }
}
