//! 主视图：带根节点的目标/分类树，以及任务区。
//!
//! 新建目标与新建分类都走模态对话框；分类对话框里选择归属目标，
//! 避免「先选中目标才能建分类」这种隐式前置条件。

use eframe::egui;
use tracing::warn;

use crate::app::{MinActionApp, PendingDelete};
use crate::model::{CategoryId, Frequency, GoalId, TaskDraft, TaskStatus};

impl MinActionApp {
    pub(crate) fn ui_main_view(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("目标与分类");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button("＋分类").clicked() {
                    self.open_new_category_dialog();
                }
                if ui.small_button("＋目标").clicked() {
                    self.open_new_goal_dialog();
                }
            });
        });

        ui.horizontal(|ui| {
            if ui.small_button("全部展开").clicked() {
                self.collapsed_goals.clear();
                self.root_collapsed = false;
            }
            if ui.small_button("全部折叠").clicked() {
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

        egui::ScrollArea::vertical()
            .id_salt("structure-tree")
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let arrow = if self.root_collapsed { "▸" } else { "▾" };
                    if ui.small_button(arrow).clicked() {
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
                        ui.weak("还没有目标，先点右上角「＋目标」。");
                    });
                }

                for goal in &self.goals {
                    let expanded = !self.collapsed_goals.contains(&goal.id);
                    let selected = self.selected_goal == Some(goal.id);
                    ui.horizontal(|ui| {
                        ui.add_space(12.0);
                        let arrow = if expanded { "▾" } else { "▸" };
                        if ui.small_button(arrow).clicked() {
                            toggle_goal = Some(goal.id);
                        }
                        if ui.selectable_label(selected, &goal.name).clicked() {
                            select_goal = Some(goal.id);
                        }
                        if ui.small_button("×").on_hover_text("删除目标").clicked() {
                            drop_goal = Some(goal.id);
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
                            if ui.selectable_label(picked, &category.name).clicked() {
                                select_category = Some(category.id);
                            }
                            if ui.small_button("×").on_hover_text("删除分类").clicked() {
                                drop_category = Some(category.id);
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
        if let Some(goal) = drop_goal {
            self.request_delete(PendingDelete::Goal(goal));
        }
        if let Some(category) = drop_category {
            self.request_delete(PendingDelete::Category(category));
        }
    }

    /// 读取删除影响并打开确认对话框。
    fn request_delete(&mut self, target: PendingDelete) {
        let outcome = match target {
            PendingDelete::Goal(goal) => self.store.goal_deletion_impact(goal),
            PendingDelete::Category(category) => self.store.category_deletion_impact(category),
        };
        match outcome {
            Ok(impact) => {
                self.pending_delete = Some(target);
                self.delete_impact = Some(impact);
            }
            Err(error) => {
                warn!(
                    op = "deletion_impact",
                    result = "error",
                    error = %error,
                    "读取删除影响范围失败"
                );
                self.error = Some(error.to_string());
            }
        }
    }

    pub(crate) fn ui_delete_dialog(&mut self, ctx: &egui::Context) {
        let Some(target) = self.pending_delete else {
            return;
        };
        let impact = self.delete_impact.unwrap_or_default();

        let mut confirm = false;
        let mut cancel = false;

        egui::Window::new(match target {
            PendingDelete::Goal(_) => "删除目标",
            PendingDelete::Category(_) => "删除分类",
        })
        .id(egui::Id::unique("dialog-delete"))
        .collapsible(false)
        .resizable(false)
        .default_pos([320.0, 260.0])
        .show(ctx, |ui| {
            ui.set_width(360.0);
            match target {
                PendingDelete::Goal(_) => {
                    ui.label(format!(
                        "将一并删除 {} 个分类、{} 个任务、{} 条未完成行动。",
                        impact.categories, impact.tasks, impact.pending_actions
                    ));
                }
                PendingDelete::Category(_) => {
                    ui.label(format!(
                        "将一并删除 {} 个任务、{} 条未完成行动。",
                        impact.tasks, impact.pending_actions
                    ));
                }
            }
            ui.weak("已完成的行动记录会保留。");
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                if ui.button("删除").clicked() {
                    confirm = true;
                }
                if ui.button("取消").clicked() {
                    cancel = true;
                }
            });
        });

        if confirm {
            let hint = match target {
                PendingDelete::Goal(goal) => self.apply(|app| {
                    app.store.delete_goal(goal).map(|impact| {
                        format!(
                            "已删除目标：分类 {} 个、任务 {} 个、未完成行动 {} 条",
                            impact.categories, impact.tasks, impact.pending_actions
                        )
                    })
                }),
                PendingDelete::Category(category) => self.apply(|app| {
                    app.store.delete_category(category).map(|impact| {
                        format!(
                            "已删除分类：任务 {} 个、未完成行动 {} 条",
                            impact.tasks, impact.pending_actions
                        )
                    })
                }),
            };
            if hint.is_some() {
                match target {
                    PendingDelete::Goal(goal) => {
                        self.collapsed_goals.remove(&goal);
                    }
                    PendingDelete::Category(category) => {
                        if self.selected_category == Some(category) {
                            self.selected_category = None;
                        }
                    }
                }
                self.pending_delete = None;
                self.delete_impact = None;
            }
        } else if cancel {
            self.pending_delete = None;
            self.delete_impact = None;
        }
    }

    pub(crate) fn ui_task_section(&mut self, ui: &mut egui::Ui) {
        ui.heading("任务");
        match self.selected_category {
            Some(_) => {
                let mut create_task = false;
                ui.horizontal(|ui| {
                    ui.label("新任务");
                    ui.text_edit_singleline(&mut self.new_task);
                    create_task = ui.button("添加").clicked();
                });
                if create_task && let Some(category) = self.selected_category {
                    let draft = TaskDraft {
                        name: self.new_task.clone(),
                        frequency: Frequency::Daily,
                        estimated_minutes: None,
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
                    }
                }
            }
            None => {
                ui.label("先在左侧选择分类，再添加任务。");
            }
        }

        let mut toggle: Option<(crate::model::TaskId, TaskStatus)> = None;
        let mut reset: Option<crate::model::TaskId> = None;

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
                        let label = match task.status {
                            TaskStatus::Active => "暂停",
                            TaskStatus::Paused => "恢复",
                        };
                        if ui.small_button(label).clicked() {
                            let next = match task.status {
                                TaskStatus::Active => TaskStatus::Paused,
                                TaskStatus::Paused => TaskStatus::Active,
                            };
                            toggle = Some((task.id, next));
                        }
                        if ui.small_button("重置复习").clicked() {
                            reset = Some(task.id);
                        }
                    });
                }
            });

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
            self.apply(|app| {
                app.store
                    .reset_review_progress(task)
                    .map(|()| "已重置复习进度".to_owned())
            });
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
                    if ui.button("创建").clicked() {
                        submit = true;
                    }
                    if ui.button("取消").clicked() {
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
                    if ui.button("创建").clicked() {
                        submit = true;
                    }
                    if ui.button("取消").clicked() {
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
