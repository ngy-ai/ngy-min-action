//! 重置复习进度的确认弹窗。

use eframe::egui;

use crate::app::MinActionApp;
use crate::app::theme;
use crate::model::TaskId;

impl MinActionApp {
    /// 打开重置确认框，并记录目标任务的当前进度级别。
    pub(crate) fn request_reset(&mut self, task: TaskId) {
        self.reset_level = self
            .task_progress
            .iter()
            .find(|item| item.task_id == task)
            .map_or(0, |item| item.level);
        self.reset_target = Some(task);
    }

    pub(crate) fn ui_reset_dialog(&mut self, ctx: &egui::Context) {
        let Some(task) = self.reset_target else {
            return;
        };

        let name = self
            .all_tasks
            .iter()
            .find(|item| item.id == task)
            .map_or_else(|| format!("任务#{}", task.0), |item| item.name.clone());
        let level = self.reset_level;

        let mut confirm = false;
        let mut cancel = false;

        egui::Window::new("重置复习进度")
            .id(egui::Id::unique("dialog-reset"))
            .collapsible(false)
            .resizable(false)
            .default_pos([320.0, 260.0])
            .show(ctx, |ui| {
                ui.set_width(360.0);
                ui.label(format!(
                    "把任务「{name}」的复习进度从第 {level} 级重置回第 0 级？"
                ));
                ui.weak("已完成的行动记录会保留，复习计划从头开始。");
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    if theme::danger(ui, |ui| theme::primary_button(ui, "重置")).clicked() {
                        confirm = true;
                    }
                    if ui.button("取消").clicked() {
                        cancel = true;
                    }
                });
            });

        if confirm {
            self.confirm_reset();
        } else if cancel {
            self.cancel_reset();
        }
    }

    /// 执行重置；失败时保留弹窗。
    pub(crate) fn confirm_reset(&mut self) {
        let Some(task) = self.reset_target else {
            return;
        };
        if self
            .apply(|app| app.store.reset_review_progress(task))
            .is_some()
        {
            self.reset_target = None;
            self.reset_level = 0;
        }
    }

    /// 关闭弹窗，不改动数据。
    pub(crate) fn cancel_reset(&mut self) {
        self.reset_target = None;
        self.reset_level = 0;
    }
}
