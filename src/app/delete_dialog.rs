//! 删除确认弹窗：目标、分类、任务共用一套影响范围展示。

use eframe::egui;
use tracing::warn;

use crate::app::theme;
use crate::app::{MinActionApp, PendingDelete};
use crate::model::DeletionImpact;

impl MinActionApp {
    /// 读取删除影响并打开确认对话框。
    pub(crate) fn request_delete(&mut self, target: PendingDelete) {
        let outcome = match target {
            PendingDelete::Goal(goal) => self.store.goal_deletion_impact(goal),
            PendingDelete::Category(category) => self.store.category_deletion_impact(category),
            PendingDelete::Task(task) => {
                self.store
                    .task_open_actions(task)
                    .map(|pending_actions| DeletionImpact {
                        categories: 0,
                        tasks: 1,
                        pending_actions,
                    })
            }
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
        let task_name = match target {
            PendingDelete::Task(task) => self
                .all_tasks
                .iter()
                .find(|item| item.id == task)
                .map_or_else(|| format!("任务#{}", task.0), |item| item.name.clone()),
            PendingDelete::Goal(_) | PendingDelete::Category(_) => "该任务".to_owned(),
        };

        let mut confirm = false;
        let mut cancel = false;

        egui::Window::new(match target {
            PendingDelete::Goal(_) => "删除目标",
            PendingDelete::Category(_) => "删除分类",
            PendingDelete::Task(_) => "删除任务",
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
                PendingDelete::Task(_) => {
                    ui.label(format!(
                        "删除任务「{task_name}」会作废 {} 条未完成行动，确定删除吗？",
                        impact.pending_actions
                    ));
                }
            }
            ui.weak("已完成的行动记录会保留。");
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                if theme::danger(ui, |ui| theme::primary_button(ui, "删除")).clicked() {
                    confirm = true;
                }
                if ui.button("取消").clicked() {
                    cancel = true;
                }
            });
        });

        if confirm {
            self.confirm_delete();
        } else if cancel {
            self.cancel_delete();
        }
    }

    /// 执行确认的删除；失败时保留弹窗。
    pub(crate) fn confirm_delete(&mut self) {
        let Some(target) = self.pending_delete else {
            return;
        };
        let done = match target {
            PendingDelete::Goal(goal) => self.apply(|app| app.store.delete_goal(goal)).is_some(),
            PendingDelete::Category(category) => self
                .apply(|app| app.store.delete_category(category))
                .is_some(),
            PendingDelete::Task(task) => self.apply(|app| app.store.delete_task(task)).is_some(),
        };
        if !done {
            return;
        }
        match target {
            PendingDelete::Goal(goal) => {
                self.collapsed_goals.remove(&goal);
            }
            PendingDelete::Category(category) => {
                if self.selected_category == Some(category) {
                    self.selected_category = None;
                }
            }
            // 任务删除后由 reload 重新取任务列表，无需额外清理选择。
            PendingDelete::Task(_) => {}
        }
        self.pending_delete = None;
        self.delete_impact = None;
    }

    /// 关闭弹窗，不改动任何数据。
    pub(crate) fn cancel_delete(&mut self) {
        self.pending_delete = None;
        self.delete_impact = None;
    }
}
