//! 今日清单：行动行、结算入口与空状态。

use eframe::egui;

use crate::app::MinActionApp;
use crate::app::describe::status_text;
use crate::app::icons::{self, ButtonId};
use crate::model::{ActionId, ActionStatus};

impl MinActionApp {
    pub(crate) fn ui_today_view(&mut self, ui: &mut egui::Ui) {
        ui.heading("今日最小行动");

        let mut settle: Option<(ActionId, ActionStatus)> = None;
        let mut undo: Option<ActionId> = None;
        let mut open_note: Option<ActionId> = None;

        if self.actions.is_empty() {
            ui.label("今天还没有行动。");
            ui.label(
                "先在左侧建立目标与分类，再到「任务」页签添加一个任务，系统会自动生成今天的最小行动。",
            );
        } else {
            egui::ScrollArea::vertical()
                .id_salt("today-actions")
                .max_height(260.0)
                .show(ui, |ui| {
                    for action in &self.actions {
                        let text = self.describe_action(action);
                        ui.horizontal(|ui| {
                            ui.label(text);
                            if action.note.is_some() {
                                ui.label("有备注");
                            }
                            if icons::add(ui, ButtonId::Note).clicked() {
                                open_note = Some(action.id);
                            }
                            if action.status == ActionStatus::Pending {
                                if icons::add(ui, ButtonId::Done).clicked() {
                                    settle = Some((action.id, ActionStatus::Done));
                                }
                                if icons::add(ui, ButtonId::Skip).clicked() {
                                    settle = Some((action.id, ActionStatus::Skipped));
                                }
                            } else {
                                ui.label(status_text(action.status));
                                if action.status.is_settled()
                                    && icons::add(ui, ButtonId::Undo).clicked()
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
        if let Some(action) = open_note {
            self.open_note(action, false);
        }
    }
}
