//! 主视图：目标/分类树，以及任务区（新建、列表、暂停恢复、复习进度）。

use eframe::egui;

use crate::app::MinActionApp;
use crate::model::{CategoryId, Frequency, GoalId, TaskDraft, TaskStatus};

impl MinActionApp {
    pub(crate) fn ui_main_view(&mut self, ui: &mut egui::Ui) {
        ui.heading("目标与分类");

        let mut select_goal: Option<GoalId> = None;
        let mut select_category: Option<CategoryId> = None;
        let mut drop_goal: Option<GoalId> = None;
        let mut drop_category: Option<CategoryId> = None;

        egui::ScrollArea::vertical()
            .id_salt("structure-tree")
            .max_height(300.0)
            .show(ui, |ui| {
                if self.goals.is_empty() {
                    ui.label("还没有目标。在下方新建一个，例如「学习英语」。");
                }
                for goal in &self.goals {
                    let selected = self.selected_goal == Some(goal.id);
                    ui.horizontal(|ui| {
                        if ui.selectable_label(selected, &goal.name).clicked() {
                            select_goal = Some(goal.id);
                        }
                        if ui.small_button("×").on_hover_text("删除目标").clicked() {
                            drop_goal = Some(goal.id);
                        }
                    });
                    if selected {
                        for category in &self.categories {
                            ui.horizontal(|ui| {
                                ui.add_space(18.0);
                                let picked = self.selected_category == Some(category.id);
                                if ui.selectable_label(picked, &category.name).clicked() {
                                    select_category = Some(category.id);
                                }
                                if ui.small_button("×").on_hover_text("删除分类").clicked() {
                                    drop_category = Some(category.id);
                                }
                            });
                        }
                    }
                }
            });

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
            self.apply(|app| {
                app.store.delete_goal(goal).map(|impact| {
                    format!(
                        "已删除目标：分类 {} 个、任务 {} 个、未完成行动 {} 条",
                        impact.categories, impact.tasks, impact.pending_actions
                    )
                })
            });
        }
        if let Some(category) = drop_category {
            self.apply(|app| {
                app.store.delete_category(category).map(|impact| {
                    format!(
                        "已删除分类：任务 {} 个、未完成行动 {} 条",
                        impact.tasks, impact.pending_actions
                    )
                })
            });
        }

        ui.separator();
        let mut create_goal = false;
        ui.horizontal(|ui| {
            ui.label("新目标");
            ui.text_edit_singleline(&mut self.new_goal);
            create_goal = ui.button("创建").clicked();
        });
        if create_goal {
            let name = self.new_goal.clone();
            if self
                .apply(|app| {
                    app.store
                        .create_goal(&name, None)
                        .map(|_| format!("已创建目标「{name}」"))
                })
                .is_some()
            {
                self.new_goal.clear();
            }
        }

        let mut create_category = false;
        ui.horizontal(|ui| {
            ui.label("新分类");
            ui.text_edit_singleline(&mut self.new_category);
            create_category = ui.button("创建").clicked();
        });
        if create_category {
            match self.selected_goal {
                Some(goal) => {
                    let name = self.new_category.clone();
                    if self
                        .apply(|app| {
                            app.store
                                .create_category(goal, &name)
                                .map(|_| format!("已创建分类「{name}」"))
                        })
                        .is_some()
                    {
                        self.new_category.clear();
                    }
                }
                None => self.error = Some("请先创建或选择一个目标，再添加分类".to_owned()),
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
            .max_height(200.0)
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
}
