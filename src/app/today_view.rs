//! 今日清单：行动行、结算入口与空状态。

use eframe::egui;

use crate::app::MinActionApp;
use crate::model::{Action, ActionId, ActionKind, ActionStatus};

impl MinActionApp {
    pub(crate) fn ui_today_view(&mut self, ui: &mut egui::Ui) {
        ui.heading("今日最小行动");

        let mut settle: Option<(ActionId, ActionStatus)> = None;
        let mut undo: Option<ActionId> = None;

        if self.actions.is_empty() {
            ui.label("今天还没有行动。");
            ui.label(
                "先在左侧建立目标与分类，再在下方添加一个任务，系统会自动生成今天的最小行动。",
            );
        } else {
            egui::ScrollArea::vertical()
                .id_salt("today-actions")
                .max_height(260.0)
                .show(ui, |ui| {
                    for action in &self.actions {
                        let text = self.describe(action);
                        ui.horizontal(|ui| {
                            ui.label(text);
                            if action.status == ActionStatus::Pending {
                                if ui.small_button("完成").clicked() {
                                    settle = Some((action.id, ActionStatus::Done));
                                }
                                if ui.small_button("跳过").clicked() {
                                    settle = Some((action.id, ActionStatus::Skipped));
                                }
                            } else {
                                ui.label(status_text(action.status));
                                if action.status.is_settled() && ui.small_button("撤销").clicked()
                                {
                                    undo = Some(action.id);
                                }
                            }
                        });
                    }
                });
        }

        if let Some((action, next)) = settle {
            let hint = match next {
                ActionStatus::Done => "已完成",
                ActionStatus::Skipped => "已跳过",
                _ => "已更新",
            };
            self.apply(|app| {
                app.store
                    .settle_action(action, next)
                    .map(|()| hint.to_owned())
            });
        }
        if let Some(action) = undo {
            self.apply(|app| {
                app.store
                    .undo_last_settlement(action)
                    .map(|()| "已撤销".to_owned())
            });
        }

        ui.separator();
        self.ui_task_section(ui);
    }

    fn describe(&self, action: &Action) -> String {
        let task = self.all_tasks.iter().find(|item| item.id == action.task_id);
        let task_name = task.map_or_else(
            || format!("任务#{}", action.task_id.0),
            |item| item.name.clone(),
        );
        let category_name = task
            .and_then(|item| {
                self.all_categories
                    .iter()
                    .find(|category| category.id == item.category_id)
            })
            .map_or_else(String::new, |category| category.name.clone());

        match action.kind {
            ActionKind::Review => format!(
                "{category_name} · {task_name}（第 {} 级复习）",
                action.review_level.unwrap_or(1)
            ),
            ActionKind::Regular => format!("{category_name} · {task_name}"),
        }
    }
}

fn status_text(status: ActionStatus) -> &'static str {
    match status {
        ActionStatus::Pending => "待完成",
        ActionStatus::Done => "已完成",
        ActionStatus::Skipped => "已跳过",
        ActionStatus::Deferred => "已顺延",
        ActionStatus::Expired => "已过期",
    }
}
